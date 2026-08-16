use serde_json::{Value, json};
use veln_language_service::{EffectiveProjectSnapshot, SourcePosition, SymbolKind, navigate};
use veln_source::SourcePath;

use crate::check_project::{CheckProjectOutcome, capture_navigation_source};
use crate::definition::{Coordinate, coordinate, source_span_to_location_json, valid_position};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) enum ReferencesOutcome {
    Success(Value),
    DomainFailure {
        code: &'static str,
        message: &'static str,
        details: Value,
    },
}

pub(crate) fn references(
    base: &WorkspaceBase,
    selection: &Selection,
    arguments: &Value,
) -> ReferencesOutcome {
    let source = arguments["source"]
        .as_str()
        .expect("references input schema requires a string source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let (captured, captured_source, scope) =
        match capture_navigation_source(base, selection, source) {
            Ok(captured) => captured,
            Err(failure) => return failure.into(),
        };
    let source_file = captured
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured_source)
        .expect("navigation capture contains the requested source");
    if !valid_position(source_file.text(), line, column) {
        return domain_failure(
            "invalid_position",
            "position is outside the selected source",
            json!({"source": source, "line": arguments["line"].clone(), "column": arguments["column"].clone()}),
        );
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        unreachable!("valid positions are addressable")
    };

    let root = captured.project.root.clone();
    let snapshot = EffectiveProjectSnapshot::new(captured.project.files);
    let references = navigate(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    )
    .filter(|result| result.selected_symbol.kind == SymbolKind::Function)
    .map(|result| {
        result
            .references
            .iter()
            .map(|span| source_span_to_location_json(&root, span))
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    ReferencesOutcome::Success(json!({"references": references, "scope": scope.to_json()}))
}

fn domain_failure(code: &'static str, message: &'static str, details: Value) -> ReferencesOutcome {
    ReferencesOutcome::DomainFailure {
        code,
        message,
        details,
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
