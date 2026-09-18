use std::collections::{HashMap, HashSet, VecDeque};
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
    let request = ReferenceArguments::new(arguments);
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
    fn new(arguments: &'a Value) -> ReferenceRequest<'a> {
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
static NEXT_PAGINATION_SECRET: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct ReferencePagination {
    secret: [u8; 32],
    next_result_id: u64,
    next_token_id: u64,
    retained: HashMap<String, RetainedReferences>,
    order: VecDeque<u64>,
    stale: HashSet<String>,
}

#[derive(Clone)]
struct RetainedReferences {
    result_id: u64,
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
            next_result_id: 0,
            next_token_id: 0,
            retained: HashMap::new(),
            order: VecDeque::new(),
            stale: HashSet::new(),
        })
    }

    pub(crate) fn clear(&mut self) {
        for token in self.retained.keys().cloned().collect::<Vec<_>>() {
            self.mark_stale(token);
        }
        self.retained.clear();
        self.order.clear();
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
        let Some(retained) = self.retained.remove(cursor) else {
            return if self.stale.contains(cursor) {
                stale_snapshot()
            } else {
                invalid_cursor()
            };
        };
        let result_id = retained.result_id;
        let page = self.page(
            retained.references,
            retained.scope,
            retained.page_size,
            retained.offset,
            Some(result_id),
        );
        page
    }

    fn page(
        &mut self,
        references: Vec<Value>,
        scope: Value,
        page_size: usize,
        offset: usize,
        result_id: Option<u64>,
    ) -> ToolOutcome {
        let end = (offset + page_size).min(references.len());
        let mut result = json!({
            "references": references[offset..end].to_vec(),
            "scope": scope,
        });
        if end < references.len() {
            let result_id = result_id.unwrap_or_else(|| {
                self.next_result_id = self.next_result_id.wrapping_add(1);
                let result_id = self.next_result_id;
                self.order.push_back(result_id);
                result_id
            });
            self.next_token_id = self.next_token_id.wrapping_add(1);
            let token_id = self.next_token_id;
            let token = self.token(token_id);
            self.retained.insert(
                token.clone(),
                RetainedReferences {
                    result_id,
                    references,
                    scope: result["scope"].clone(),
                    page_size,
                    offset: end,
                },
            );
            if self.order.len() > MAX_RETAINED_RESULTS {
                if let Some(evicted) = self.order.pop_front() {
                    let evicted_tokens = self
                        .retained
                        .iter()
                        .filter(|(_, value)| value.result_id == evicted)
                        .map(|(token, _)| token.clone())
                        .collect::<Vec<_>>();
                    for token in evicted_tokens {
                        self.retained.remove(&token);
                        self.mark_stale(token);
                    }
                }
            }
            result["next_cursor"] = Value::String(token);
        } else if let Some(result_id) = result_id {
            self.order.retain(|id| *id != result_id);
        }
        ToolOutcome::Success(result)
    }

    fn mark_stale(&mut self, token: String) {
        self.stale.insert(token);
    }

    fn token(&self, result_id: u64) -> String {
        format!("{result_id:x}.{}", self.mac(result_id))
    }

    fn mac(&self, token_id: u64) -> String {
        let mut mac = Sha256::new();
        mac.update(self.secret);
        mac.update(token_id.to_le_bytes());
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
    fn every_invalidated_cursor_remains_stale_across_repeated_invalidation() {
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

        assert_eq!(pagination.stale.len(), 256);
        for cursor in cursors {
            assert_eq!(pagination.continue_page(&cursor).code(), "stale_snapshot");
        }
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
