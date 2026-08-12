use std::collections::{HashSet, VecDeque, hash_map::RandomState};
use std::hash::{BuildHasher, Hash, Hasher};

use serde_json::{Value, json};
use veln_language_service::{EffectiveProjectSnapshot, NavigationSource, SourcePosition, navigate};
use veln_source::{SourcePath, SourceSpan};

use crate::check_project::{CheckProjectOutcome, capture_navigation_source};
use crate::definition::{Coordinate, coordinate, path_to_uri, valid_position};
use crate::workspace::{Selection, WorkspaceBase};

const DEFAULT_PAGE_SIZE: usize = 100;
const MAX_CONTINUATIONS: usize = 64;
const MAX_STALE_CURSORS: usize = 64;

pub(crate) enum ReferencesOutcome {
    Success(Value),
    DomainFailure {
        code: &'static str,
        message: &'static str,
        details: Value,
    },
}

struct CapturedReferences {
    locations: Vec<Value>,
    scope: Value,
    project_wide: bool,
}

struct Continuation {
    locations: Vec<Value>,
    scope: Value,
    project_wide: bool,
    page_size: usize,
    include_declaration: bool,
    next_offset: usize,
    generation: u64,
}

pub(crate) struct ReferenceCursors {
    hasher: RandomState,
    next_id: u64,
    states: VecDeque<(String, Continuation)>,
    stale_order: VecDeque<String>,
    stale_lookup: HashSet<String>,
}

impl ReferenceCursors {
    pub(crate) fn new() -> Self {
        Self {
            hasher: RandomState::new(),
            next_id: 0,
            states: VecDeque::new(),
            stale_order: VecDeque::new(),
            stale_lookup: HashSet::new(),
        }
    }

    pub(crate) fn call(
        &mut self,
        base: &WorkspaceBase,
        selection: &Selection,
        arguments: &Value,
    ) -> ReferencesOutcome {
        if let Some(cursor) = arguments.get("cursor").and_then(Value::as_str) {
            return self.continue_result(selection, cursor);
        }
        let include_declaration = arguments
            .get("include_declaration")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let page_size = arguments
            .get("page_size")
            .and_then(Value::as_u64)
            .map(|size| size as usize)
            .unwrap_or(DEFAULT_PAGE_SIZE);
        let captured = match capture(base, selection, arguments, include_declaration) {
            Ok(captured) => captured,
            Err(failure) => return failure,
        };
        self.first_page(
            captured,
            page_size,
            include_declaration,
            selection.generation(),
        )
    }

    pub(crate) fn stale_all(&mut self) {
        while let Some((cursor, _)) = self.states.pop_front() {
            self.retain_stale(cursor);
        }
    }

    fn first_page(
        &mut self,
        captured: CapturedReferences,
        page_size: usize,
        include_declaration: bool,
        generation: u64,
    ) -> ReferencesOutcome {
        let continuation = Continuation {
            locations: captured.locations,
            scope: captured.scope,
            project_wide: captured.project_wide,
            page_size,
            include_declaration,
            next_offset: 0,
            generation,
        };
        ReferencesOutcome::Success(self.page(continuation))
    }

    fn continue_result(&mut self, selection: &Selection, cursor: &str) -> ReferencesOutcome {
        let Some(index) = self
            .states
            .iter()
            .position(|(retained, _)| retained == cursor)
        else {
            return if self.stale_lookup.contains(cursor) {
                failure("stale_snapshot", "reference snapshot is no longer retained")
            } else {
                failure("invalid_cursor", "reference cursor is invalid")
            };
        };
        let (_, continuation) = self
            .states
            .remove(index)
            .expect("located continuation remains present");
        if continuation.generation != selection.generation() {
            self.retain_stale(cursor.to_string());
            return failure("stale_snapshot", "reference snapshot is no longer current");
        }
        ReferencesOutcome::Success(self.page(continuation))
    }

    fn page(&mut self, mut continuation: Continuation) -> Value {
        let start = continuation.next_offset;
        let end = start
            .saturating_add(continuation.page_size)
            .min(continuation.locations.len());
        let page = continuation.locations[start..end].to_vec();
        continuation.next_offset = end;
        let scope = continuation.scope.clone();
        let project_wide = continuation.project_wide;
        let cursor = if end < continuation.locations.len() {
            Some(self.retain(continuation))
        } else {
            None
        };
        json!({
            "references": page,
            "scope": scope,
            "project_wide": project_wide,
            "cursor": cursor,
        })
    }

    fn retain(&mut self, continuation: Continuation) -> String {
        if self.states.len() == MAX_CONTINUATIONS
            && let Some((evicted, _)) = self.states.pop_front()
        {
            self.retain_stale(evicted);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let mut hasher = self.hasher.build_hasher();
        id.hash(&mut hasher);
        continuation.generation.hash(&mut hasher);
        continuation.page_size.hash(&mut hasher);
        continuation.include_declaration.hash(&mut hasher);
        continuation.next_offset.hash(&mut hasher);
        continuation.project_wide.hash(&mut hasher);
        serde_json::to_string(&continuation.scope)
            .expect("reference scope is a JSON value")
            .hash(&mut hasher);
        serde_json::to_string(&continuation.locations)
            .expect("reference locations are JSON values")
            .hash(&mut hasher);
        let cursor = format!("r1-{id:016x}-{:016x}", hasher.finish());
        self.states.push_back((cursor.clone(), continuation));
        cursor
    }

    fn retain_stale(&mut self, cursor: String) {
        if !self.stale_lookup.insert(cursor.clone()) {
            return;
        }
        self.stale_order.push_back(cursor);
        while self.stale_order.len() > MAX_STALE_CURSORS {
            if let Some(expired) = self.stale_order.pop_front() {
                self.stale_lookup.remove(&expired);
            } else {
                break;
            }
        }
    }
}

fn capture(
    base: &WorkspaceBase,
    selection: &Selection,
    arguments: &Value,
    include_declaration: bool,
) -> Result<CapturedReferences, ReferencesOutcome> {
    let source = arguments["source"]
        .as_str()
        .expect("references input schema requires a source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let captured =
        capture_navigation_source(base, selection, source).map_err(ReferencesOutcome::from)?;
    let source_file = captured
        .project
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured.source)
        .expect("navigation capture contains the requested source");
    if !valid_position(source_file.text(), line, column) {
        return Err(ReferencesOutcome::DomainFailure {
            code: "invalid_position",
            message: "position is outside the selected source",
            details: json!({"source": source, "line": arguments["line"].clone(), "column": arguments["column"].clone()}),
        });
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        unreachable!("valid positions are addressable")
    };
    let root = captured.project.project.root.clone();
    let snapshot = EffectiveProjectSnapshot::new(captured.project.project.files);
    let mut spans = Vec::new();
    if let Some(result) = navigate(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(&captured.source),
            line,
            column,
        },
    ) {
        if include_declaration && matches!(result.definition.source, NavigationSource::Workspace) {
            spans.push(result.definition.span);
        }
        spans.extend(result.references);
    }
    spans.dedup_by(|left, right| left == right);
    let mut locations = spans
        .iter()
        .map(|span| location(&root, span))
        .collect::<Vec<_>>();
    locations.sort_by(compare_locations);
    locations.dedup_by(|left, right| left == right);
    let scope = if let Some(root) = captured.scope_root {
        json!({"mode": "project", "project": root})
    } else {
        json!({"mode": "single_file", "source": source})
    };
    Ok(CapturedReferences {
        locations,
        scope,
        project_wide: captured.project_wide,
    })
}

fn location(root: &std::path::Path, span: &SourceSpan) -> Value {
    json!({
        "uri": path_to_uri(&root.join(span.file.as_str())),
        "range": {
            "start": {"line": span.start.line, "column": span.start.column},
            "end": {"line": span.end.line, "column": span.end.column}
        }
    })
}

fn compare_locations(left: &Value, right: &Value) -> std::cmp::Ordering {
    location_uri(left)
        .cmp(location_uri(right))
        .then(location_position(left, "start").cmp(&location_position(right, "start")))
        .then(location_position(left, "end").cmp(&location_position(right, "end")))
}

fn location_uri(location: &Value) -> &str {
    location["uri"]
        .as_str()
        .expect("reference location has a URI")
}

fn location_position(location: &Value, key: &str) -> (u64, u64) {
    let position = &location["range"][key];
    (
        position["line"]
            .as_u64()
            .expect("reference line is an unsigned integer"),
        position["column"]
            .as_u64()
            .expect("reference column is an unsigned integer"),
    )
}

fn failure(code: &'static str, message: &'static str) -> ReferencesOutcome {
    ReferencesOutcome::DomainFailure {
        code,
        message,
        details: json!({}),
    }
}

impl From<CheckProjectOutcome> for ReferencesOutcome {
    fn from(outcome: CheckProjectOutcome) -> Self {
        match outcome {
            CheckProjectOutcome::DomainFailure {
                code,
                message,
                details,
            } => Self::DomainFailure {
                code,
                message,
                details,
            },
            CheckProjectOutcome::Success(_) => {
                unreachable!("navigation capture failures are domain failures")
            }
        }
    }
}
