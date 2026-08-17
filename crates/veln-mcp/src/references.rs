use serde_json::{Value, json};
use veln_language_service::{EffectiveProjectSnapshot, SourcePosition, SymbolKind, navigate};
use veln_source::SourcePath;

use crate::check_project::capture_navigation_source;
use crate::definition::{Coordinate, DefinitionOutcome, coordinate, path_to_uri, valid_position};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) fn references(
    base: &WorkspaceBase,
    selection: &Selection,
    arguments: &Value,
) -> DefinitionOutcome {
    let source = arguments["source"]
        .as_str()
        .expect("references input schema requires a string source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let (captured, captured_source) = match capture_navigation_source(base, selection, source) {
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
        return DefinitionOutcome::DomainFailure {
            code: "invalid_position",
            message: "position is outside the selected source",
            details: json!({"source": source, "line": arguments["line"].clone(), "column": arguments["column"].clone()}),
        };
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        unreachable!("valid positions are addressable")
    };

    let project_wide = captured.project.manifest.is_some();
    let scope = if project_wide {
        "project"
    } else {
        "single_file"
    };
    let scope_root = if project_wide {
        captured
            .project
            .root
            .strip_prefix(base.path())
            .ok()
            .and_then(|path| path.to_str())
            .filter(|path| !path.is_empty())
            .unwrap_or(".")
            .replace('\\', "/")
    } else {
        source.to_string()
    };
    let root = captured.project.root.clone();
    let snapshot = EffectiveProjectSnapshot::new(captured.project.files);
    let result = navigate(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    );
    let locations = result
        .filter(|result| result.selected_symbol.kind == SymbolKind::Function)
        .into_iter()
        .flat_map(|result| result.references)
        .map(|span| {
            json!({
                "uri": path_to_uri(&root.join(span.file.as_str())),
                "range": {
                    "start": {"line": span.start.line, "column": span.start.column},
                    "end": {"line": span.end.line, "column": span.end.column}
                }
            })
        })
        .collect::<Vec<_>>();

    DefinitionOutcome::Success(json!({
        "references": locations,
        "scope": scope,
        "scope_root": scope_root,
        "project_wide": project_wide
    }))
}
