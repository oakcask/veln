use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use veln_language_service::{
    NavigationResult, NavigationSource, PackageOrigin, SourcePosition, SymbolDeclarationKind,
    SymbolKind, navigate,
};
use veln_source::{SourcePath, SourceSpan};

use crate::check_project::{CapturedProject, capture_navigation_source};
use crate::definition::{Coordinate, coordinate, path_to_uri, valid_position};
use crate::language_resources::LanguageResources;
use crate::outcome::{ToolOutcome, domain_failure};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) fn references(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    arguments: &Value,
) -> ToolOutcome {
    let request = ReferenceArguments::parse(arguments);
    match request {
        ReferenceRequest::Continuation(cursor) => language_resources
            .reference_pagination()
            .continue_page(cursor),
        ReferenceRequest::Initial(request) => {
            let (captured, captured_source, scope) =
                match capture_navigation_source(base, selection, request.source) {
                    Ok(captured) => captured,
                    Err(failure) => return failure,
                };
            let references = match collect_references(
                captured,
                &captured_source,
                &request,
                language_resources,
            ) {
                Ok(references) => references,
                Err(failure) => return failure,
            };
            language_resources.reference_pagination().initial_page(
                references,
                scope.metadata(selection.generation()),
                request.page_size,
            )
        }
    }
}

enum ReferenceRequest<'a> {
    Initial(ReferenceArguments<'a>),
    Continuation(&'a str),
}

struct ReferenceArguments<'a> {
    source: &'a str,
    line: Coordinate,
    column: Coordinate,
    page_size: usize,
    raw_line: &'a Value,
    raw_column: &'a Value,
}

impl<'a> ReferenceArguments<'a> {
    fn parse(arguments: &'a Value) -> ReferenceRequest<'a> {
        if let Some(cursor) = arguments.get("cursor").and_then(Value::as_str) {
            return ReferenceRequest::Continuation(cursor);
        }
        ReferenceRequest::Initial(Self {
            source: arguments["source"]
                .as_str()
                .expect("references input schema requires a string source"),
            line: coordinate(&arguments["line"]),
            column: coordinate(&arguments["column"]),
            page_size: arguments
                .get("page_size")
                .and_then(crate::schema::json_integer_usize)
                .unwrap_or(100),
            raw_line: &arguments["line"],
            raw_column: &arguments["column"],
        })
    }
}

const MAX_RETAINED_RESULTS: usize = 64;
const MAX_ADMISSION_SLOTS: usize = MAX_RETAINED_RESULTS * 2;
static NEXT_PAGINATION_SECRET: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct ReferencePagination {
    secret: [u8; 32],
    retained: HashMap<String, RetainedReferences>,
    order: VecDeque<usize>,
    admissions: Vec<Admission>,
}

#[derive(Clone, Copy)]
enum Admission {
    Free { generation: u64 },
    Live { generation: u64 },
    Stale { generation: u64 },
}

#[derive(Clone)]
struct RetainedReferences {
    references: Vec<Value>,
    scope: Value,
    page_size: usize,
    offset: usize,
}

impl ReferencePagination {
    pub(crate) fn new() -> Result<Self, String> {
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos() as u64);
        let address = (&tick as *const u64 as usize) as u64;
        let counter = NEXT_PAGINATION_SECRET.fetch_add(1, Ordering::Relaxed);
        let mut secret = Sha256::new();
        let mut entropy = [0_u8; 32];
        let mut filled = 0;
        while filled < entropy.len() {
            let read = rustix::rand::getrandom(
                &mut entropy[filled..],
                rustix::rand::GetRandomFlags::empty(),
            )
            .map_err(|error| format!("reference cursor secret initialization failed: {error}"))?;
            if read == 0 {
                return Err(
                    "reference cursor secret initialization returned a short read".to_owned(),
                );
            }
            filled += read;
        }
        secret.update(entropy);
        secret.update(tick.to_le_bytes());
        secret.update(address.to_le_bytes());
        secret.update(counter.to_le_bytes());
        let secret: [u8; 32] = secret.finalize().into();
        Ok(Self {
            secret,
            retained: HashMap::new(),
            order: VecDeque::new(),
            admissions: vec![Admission::Free { generation: 0 }; MAX_ADMISSION_SLOTS],
        })
    }

    pub(crate) fn clear(&mut self) {
        for slot in self.order.drain(..) {
            if let Admission::Live { generation } = self.admissions[slot] {
                self.admissions[slot] = Admission::Stale { generation };
            }
        }
        self.retained.clear();
    }

    pub(crate) fn initial_page(
        &mut self,
        mut references: Vec<Value>,
        scope: Value,
        page_size: usize,
    ) -> ToolOutcome {
        references.sort_by(compare_locations);
        self.page(references, scope, page_size, 0, None)
    }

    fn continue_page(&mut self, cursor: &str) -> ToolOutcome {
        let Some((slot, generation)) = self.parse_token(cursor) else {
            return invalid_cursor();
        };
        let Admission::Live {
            generation: live_generation,
        } = self
            .admissions
            .get(slot)
            .copied()
            .unwrap_or(Admission::Free { generation: 0 })
        else {
            return match self.admissions.get(slot).copied() {
                Some(Admission::Stale {
                    generation: stale_generation,
                }) if stale_generation == generation => stale_snapshot(),
                _ => invalid_cursor(),
            };
        };
        if live_generation != generation {
            return invalid_cursor();
        }
        let Some(retained) = self.retained.remove(cursor) else {
            return invalid_cursor();
        };
        self.page(
            retained.references,
            retained.scope,
            retained.page_size,
            retained.offset,
            Some(slot),
        )
    }

    fn page(
        &mut self,
        references: Vec<Value>,
        scope: Value,
        page_size: usize,
        offset: usize,
        slot: Option<usize>,
    ) -> ToolOutcome {
        let end = (offset + page_size).min(references.len());
        let mut result = json!({
            "references": references[offset..end].to_vec(),
            "scope": scope,
        });
        if end < references.len() {
            let is_new = slot.is_none();
            let slot = slot.unwrap_or_else(|| self.admit_slot());
            let generation = self.admission_generation(slot).wrapping_add(1);
            self.admissions[slot] = Admission::Live { generation };
            if is_new {
                self.order.push_back(slot);
            }
            let token = self.token(slot, generation);
            self.retained.insert(
                token.clone(),
                RetainedReferences {
                    references,
                    scope: result["scope"].clone(),
                    page_size,
                    offset: end,
                },
            );
            result["next_cursor"] = Value::String(token);
        } else if let Some(slot) = slot {
            self.order.retain(|admitted| *admitted != slot);
            self.admissions[slot] = Admission::Free {
                generation: self.admission_generation(slot),
            };
        }
        ToolOutcome::Success(result)
    }

    fn admit_slot(&mut self) -> usize {
        if let Some(slot) = self
            .admissions
            .iter()
            .position(|admission| matches!(admission, Admission::Free { .. }))
        {
            if self.order.len() >= MAX_RETAINED_RESULTS {
                let evicted = self.order.pop_front().expect("live admission exists");
                self.evict_slot(evicted);
            }
            return slot;
        }

        if let Some(slot) = self
            .admissions
            .iter()
            .position(|admission| matches!(admission, Admission::Stale { .. }))
        {
            if self.order.len() >= MAX_RETAINED_RESULTS {
                let evicted = self.order.pop_front().expect("live admission exists");
                self.evict_slot(evicted);
            }
            return slot;
        }

        let evicted = self.order.pop_front().expect("live admission exists");
        self.evict_slot(evicted);
        evicted
    }

    fn evict_slot(&mut self, slot: usize) {
        let tokens = self
            .retained
            .keys()
            .filter(|token| {
                self.parse_token(token)
                    .is_some_and(|(admitted, _)| admitted == slot)
            })
            .cloned()
            .collect::<Vec<_>>();
        for token in tokens {
            self.retained.remove(&token);
        }
        let generation = self.admission_generation(slot);
        self.admissions[slot] = Admission::Stale { generation };
    }

    fn admission_generation(&self, slot: usize) -> u64 {
        match self.admissions[slot] {
            Admission::Free { generation }
            | Admission::Live { generation }
            | Admission::Stale { generation } => generation,
        }
    }

    fn token(&self, slot: usize, generation: u64) -> String {
        format!("{slot:x}.{generation:x}.{}", self.mac(slot, generation))
    }

    fn parse_token(&self, token: &str) -> Option<(usize, u64)> {
        let mut parts = token.split('.');
        let slot = usize::from_str_radix(parts.next()?, 16).ok()?;
        let generation = u64::from_str_radix(parts.next()?, 16).ok()?;
        let mac = parts.next()?;
        if parts.next().is_some()
            || slot >= self.admissions.len()
            || mac != self.mac(slot, generation)
        {
            return None;
        }
        Some((slot, generation))
    }

    fn mac(&self, slot: usize, generation: u64) -> String {
        let mut mac = Sha256::new();
        mac.update(self.secret);
        mac.update((slot as u64).to_le_bytes());
        mac.update(generation.to_le_bytes());
        mac.finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

fn invalid_cursor() -> ToolOutcome {
    domain_failure("invalid_cursor", "cursor is invalid", json!({}))
}

fn stale_snapshot() -> ToolOutcome {
    domain_failure(
        "stale_snapshot",
        "cursor snapshot is no longer available",
        json!({}),
    )
}

fn compare_locations(left: &Value, right: &Value) -> std::cmp::Ordering {
    left["uri"]
        .as_str()
        .unwrap_or_default()
        .as_bytes()
        .cmp(right["uri"].as_str().unwrap_or_default().as_bytes())
        .then_with(|| compare_coordinate(left, right, "start", "line"))
        .then_with(|| compare_coordinate(left, right, "start", "column"))
        .then_with(|| compare_coordinate(left, right, "end", "line"))
        .then_with(|| compare_coordinate(left, right, "end", "column"))
}

fn compare_coordinate(
    left: &Value,
    right: &Value,
    edge: &str,
    coordinate: &str,
) -> std::cmp::Ordering {
    left["range"][edge][coordinate]
        .as_u64()
        .unwrap_or_default()
        .cmp(
            &right["range"][edge][coordinate]
                .as_u64()
                .unwrap_or_default(),
        )
}

fn collect_references(
    captured: CapturedProject,
    captured_source: &str,
    request: &ReferenceArguments<'_>,
    language_resources: &mut LanguageResources,
) -> Result<Vec<Value>, ToolOutcome> {
    let (line, column) = validate_reference_position(&captured, captured_source, request)?;
    let root = captured.project.root.clone();
    let workspace_key = captured.key;
    let snapshot = language_resources.navigation_snapshot(
        captured.project.files,
        &captured.dependencies,
        workspace_key,
    )?;
    Ok(navigate(
        snapshot.as_ref(),
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    )
    .filter(|result| supported_reference_symbol(result) && !result.is_recovery)
    .map(|result| {
        result
            .references
            .iter()
            .map(|span| location_json(&root, span))
            .collect::<Vec<_>>()
    })
    .unwrap_or_default())
}

fn validate_reference_position(
    captured: &CapturedProject,
    captured_source: &str,
    request: &ReferenceArguments<'_>,
) -> Result<(usize, usize), ToolOutcome> {
    let source_file = captured
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured_source)
        .expect("navigation capture contains the requested source");
    if !valid_position(source_file.text(), request.line, request.column) {
        return Err(invalid_position(request));
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) =
        (request.line, request.column)
    else {
        unreachable!("valid positions are addressable")
    };
    Ok((line, column))
}

fn invalid_position(request: &ReferenceArguments<'_>) -> ToolOutcome {
    domain_failure(
        "invalid_position",
        "position is outside the selected source",
        json!({
            "source": request.source,
            "line": request.raw_line.clone(),
            "column": request.raw_column.clone()
        }),
    )
}

fn supported_reference_symbol(result: &NavigationResult) -> bool {
    match result.definition.source {
        NavigationSource::Workspace => supports_workspace_references(result),
        NavigationSource::Package { .. } => supports_package_references(result),
    }
}

fn supports_workspace_references(result: &NavigationResult) -> bool {
    matches!(
        result.selected_symbol.kind,
        SymbolKind::Schema
            | SymbolKind::Type
            | SymbolKind::Function
            | SymbolKind::Constructor
            | SymbolKind::ValueBinding
            | SymbolKind::HandlerContextParameter
            | SymbolKind::HandlerOperationClauseParameter
    )
}

fn supports_package_references(result: &NavigationResult) -> bool {
    supports_package_reference_kind(result)
        && supports_package_reference_origin(result)
        && supports_package_reference_declaration(result)
}

fn supports_package_reference_kind(result: &NavigationResult) -> bool {
    matches!(
        result.selected_symbol.kind,
        SymbolKind::Function | SymbolKind::Type | SymbolKind::Constructor
    ) || (result.selected_symbol.kind == SymbolKind::Schema
        && result.selected_symbol.package_origin == Some(PackageOrigin::DirectDependency)
        && matches!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration | SymbolDeclarationKind::PublicAlias
        ))
}

fn supports_package_reference_origin(result: &NavigationResult) -> bool {
    matches!(
        result.selected_symbol.package_origin,
        Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
    )
}

fn supports_package_reference_declaration(result: &NavigationResult) -> bool {
    result.selected_symbol.declaration_kind == SymbolDeclarationKind::Declaration
        || supports_supported_package_public_alias(result)
}

fn supports_supported_package_public_alias(result: &NavigationResult) -> bool {
    result.selected_symbol.declaration_kind == SymbolDeclarationKind::PublicAlias
        && matches!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
        )
}

fn location_json(root: &std::path::Path, span: &SourceSpan) -> Value {
    json!({
        "uri": path_to_uri(&root.join(span.file.as_str())),
        "range": {
            "start": {
                "line": span.start.line,
                "column": span.start.column
            },
            "end": {
                "line": span.end.line,
                "column": span.end.column
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(uri: &str, line: u64, column: u64) -> Value {
        json!({
            "uri": uri,
            "range": {
                "start": {"line": line, "column": column},
                "end": {"line": line, "column": column + 1}
            }
        })
    }

    #[test]
    fn pagination_sorts_and_concatenates_without_repeating_locations() {
        let mut pagination = ReferencePagination::new().unwrap();
        let first = pagination.initial_page(
            vec![
                location("file://b", 1, 1),
                location("file://a", 2, 1),
                location("file://a", 1, 2),
                location("file://c", 1, 1),
            ],
            json!({"generation": 4}),
            1,
        );
        let first_cursor = first.cursor().unwrap().to_string();
        assert_eq!(first.references(), vec![location("file://a", 1, 2)]);
        let second = pagination.continue_page(&first_cursor);
        let second_cursor = second.cursor().unwrap().to_string();
        assert_eq!(second.references(), vec![location("file://a", 2, 1)]);
        assert_ne!(first_cursor, second_cursor);
        let third = pagination.continue_page(&second_cursor);
        let third_cursor = third.cursor().unwrap().to_string();
        assert_eq!(third.references(), vec![location("file://b", 1, 1)]);
        let final_page = pagination.continue_page(&third_cursor);
        assert_eq!(final_page.references(), vec![location("file://c", 1, 1)]);
        assert!(final_page.cursor().is_none());
        assert_eq!(
            pagination.continue_page(&first_cursor).code(),
            "invalid_cursor"
        );
    }

    #[test]
    fn terminal_results_leave_capacity_for_the_next_unfinished_result() {
        let mut pagination = ReferencePagination::new().unwrap();
        let mut cursors = Vec::new();
        for index in 0..64 {
            cursors.push(
                pagination
                    .initial_page(
                        vec![
                            location(&format!("file://{index}"), 1, 1),
                            location(&format!("file://{index}-tail"), 1, 1),
                        ],
                        json!({}),
                        1,
                    )
                    .into_success_cursor(),
            );
        }
        assert_eq!(pagination.continue_page(&cursors[0]).code(), "success");
        let _new_cursor = pagination
            .initial_page(
                vec![
                    location("file://new", 1, 1),
                    location("file://new-tail", 1, 1),
                ],
                json!({}),
                1,
            )
            .into_success_cursor();
        assert_eq!(pagination.continue_page(&cursors[1]).code(), "success");
    }

    #[test]
    fn refresh_and_eviction_mark_live_cursors_stale() {
        let mut pagination = ReferencePagination::new().unwrap();
        let cursor = pagination
            .initial_page(
                vec![location("file://a", 1, 1), location("file://b", 1, 1)],
                json!({}),
                1,
            )
            .into_success_cursor();
        pagination.clear();
        assert_eq!(pagination.continue_page(&cursor).code(), "stale_snapshot");

        let mut pagination = ReferencePagination::new().unwrap();
        let mut cursors = Vec::new();
        for index in 0..65 {
            cursors.push(
                pagination
                    .initial_page(
                        vec![
                            location(&format!("file://{index}"), 1, 1),
                            location("file://z", 1, 1),
                        ],
                        json!({}),
                        1,
                    )
                    .into_success_cursor(),
            );
        }
        assert_eq!(
            pagination.continue_page(&cursors[0]).code(),
            "stale_snapshot"
        );
        assert_eq!(pagination.continue_page(&cursors[1]).code(), "success");
    }

    #[test]
    fn invalidation_admissions_remain_bounded_across_repeated_refreshes() {
        let mut pagination = ReferencePagination::new().unwrap();
        let mut cursors = Vec::new();
        for index in 0..256 {
            let cursor = pagination
                .initial_page(
                    vec![
                        location(&format!("file://{index}"), 1, 1),
                        location(&format!("file://{index}-tail"), 1, 1),
                    ],
                    json!({}),
                    1,
                )
                .into_success_cursor();
            cursors.push(cursor);
            pagination.clear();
        }

        assert!(pagination.retained.is_empty());
        assert!(
            pagination
                .admissions
                .iter()
                .filter(|admission| matches!(admission, Admission::Stale { .. }))
                .count()
                <= MAX_ADMISSION_SLOTS
        );
        assert_eq!(
            pagination.continue_page(cursors.last().unwrap()).code(),
            "stale_snapshot"
        );
        assert_eq!(
            pagination.continue_page(&cursors[0]).code(),
            "invalid_cursor"
        );
    }

    trait TestOutcome {
        fn code(&self) -> &str;
        fn references(&self) -> Vec<Value>;
        fn cursor(&self) -> Option<&str>;
        fn into_success_cursor(self) -> String;
    }

    impl TestOutcome for ToolOutcome {
        fn code(&self) -> &str {
            match self {
                ToolOutcome::DomainFailure { code, .. } => code,
                ToolOutcome::Success(_) => "success",
            }
        }
        fn references(&self) -> Vec<Value> {
            match self {
                ToolOutcome::Success(value) => value["references"].as_array().unwrap().clone(),
                ToolOutcome::DomainFailure { .. } => panic!("expected success"),
            }
        }
        fn cursor(&self) -> Option<&str> {
            match self {
                ToolOutcome::Success(value) => value["next_cursor"].as_str(),
                ToolOutcome::DomainFailure { .. } => None,
            }
        }
        fn into_success_cursor(self) -> String {
            match self {
                ToolOutcome::Success(value) => value["next_cursor"].as_str().unwrap().to_string(),
                ToolOutcome::DomainFailure { .. } => panic!("expected success"),
            }
        }
    }
}
