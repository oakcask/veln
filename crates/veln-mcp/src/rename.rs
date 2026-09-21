use std::collections::BTreeMap;

use serde_json::{Value, json};
use veln_language_service::{
    NavigationLocation, NavigationSource, RenameAffectedScope, RenameFailure, RenameFailureKind,
    SourcePosition, navigate_for_rename, validate_rename_in_snapshot,
};
use veln_source::{SourcePath, SourceSpan};

use crate::check_project::{CaptureCache, capture_navigation_source};
use crate::definition::{Coordinate, coordinate, path_to_uri, valid_position};
use crate::language_resources::LanguageResources;
use crate::outcome::{ToolOutcome, domain_failure};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) fn rename(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    capture_cache: &mut CaptureCache,
    arguments: &Value,
) -> ToolOutcome {
    let requested_name = arguments["new_name"]
        .as_str()
        .expect("rename input schema requires a string new_name");
    if !is_identifier(requested_name) {
        return domain_failure(
            "rename.invalid_name",
            "replacement is not a valid identifier",
            json!({"requested_name": requested_name}),
        );
    }

    let source = arguments["source"]
        .as_str()
        .expect("rename input schema requires a string source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let (captured, captured_source, _) =
        match capture_navigation_source(base, selection, source, capture_cache) {
            Ok(captured) => captured,
            Err(failure) => return failure,
        };
    let source_file = captured
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured_source)
        .expect("navigation capture contains the requested source");
    if !valid_position(source_file.text(), line, column) {
        return invalid_position(source, arguments);
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        unreachable!("valid positions are addressable")
    };

    let root = captured.project.root.clone();
    let snapshot = language_resources
        .read_only_navigation_snapshot(captured.project.files, &captured.dependencies);
    let Some(result) = navigate_for_rename(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    )
    .filter(|result| {
        matches!(result.definition.source, NavigationSource::Workspace)
            && result.selected_symbol.kind.is_renamable()
    }) else {
        return ToolOutcome::Success(json!({"edits": []}));
    };

    if let Err(failure) = validate_rename_in_snapshot(&snapshot, &result, requested_name) {
        return rename_failure(&root, failure);
    }

    ToolOutcome::Success(json!({
        "edits": edits_json(
            &root,
            std::iter::once(&result.definition.span).chain(&result.references),
            requested_name,
        )
    }))
}

fn edits_json<'a>(
    root: &std::path::Path,
    spans: impl IntoIterator<Item = &'a SourceSpan>,
    requested_name: &str,
) -> Vec<Value> {
    let mut edits = BTreeMap::new();
    for span in spans {
        let uri = path_to_uri(&root.join(span.file.as_str()));
        let key = (
            uri.clone(),
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column,
        );
        edits
            .entry(key)
            .or_insert_with(|| edit_json(uri, span, requested_name));
    }
    edits.into_values().collect()
}

fn invalid_position(source: &str, arguments: &Value) -> ToolOutcome {
    domain_failure(
        "invalid_position",
        "position is outside the selected source",
        json!({
            "source": source,
            "line": arguments["line"].clone(),
            "column": arguments["column"].clone()
        }),
    )
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn edit_json(uri: String, span: &SourceSpan, requested_name: &str) -> Value {
    let mut location = location_json(uri, span);
    location["new_text"] = json!(requested_name);
    location
}

fn location_json(uri: String, span: &SourceSpan) -> Value {
    json!({
        "uri": uri,
        "range": {
            "start": {"line": span.start.line, "column": span.start.column},
            "end": {"line": span.end.line, "column": span.end.column}
        }
    })
}

fn navigation_location_json(root: &std::path::Path, location: &NavigationLocation) -> Value {
    let uri = match &location.source {
        NavigationSource::Workspace => path_to_uri(&root.join(location.span.file.as_str())),
        NavigationSource::Package { uri } => uri.clone(),
    };
    location_json(uri, &location.span)
}

fn rename_failure(root: &std::path::Path, failure: RenameFailure) -> ToolOutcome {
    let details = match failure.kind {
        RenameFailureKind::InvalidCase { required_initial } => json!({
            "symbol_class": failure.symbol_class.as_str(),
            "requested_name": failure.requested_name,
            "required_initial": required_initial.as_str()
        }),
        RenameFailureKind::Conflict {
            conflicting_declaration,
            affected_scope,
        } => json!({
            "symbol_class": failure.symbol_class.as_str(),
            "requested_name": failure.requested_name,
            "conflicting_declaration": navigation_location_json(root, &conflicting_declaration),
            "affected_scope": affected_scope_json(&affected_scope)
        }),
    };
    domain_failure(failure.code, failure.code, details)
}

fn affected_scope_json(scope: &RenameAffectedScope) -> Value {
    match scope {
        RenameAffectedScope::Module { name } => json!({"kind": "module", "name": name}),
        RenameAffectedScope::Lexical {
            file,
            start_offset,
            end_offset,
        } => json!({
            "kind": "lexical",
            "file": file,
            "start_offset": start_offset,
            "end_offset": end_offset
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{edits_json, is_identifier};
    use serde_json::json;
    use veln_source::{SourceFile, TextRange};

    #[test]
    fn replacement_identifier_uses_the_lsp_ascii_boundary() {
        for name in ["value", "Type2", "_", "match"] {
            assert!(is_identifier(name), "{name}");
        }
        for name in ["", "2value", "two words", "punct!", "café"] {
            assert!(!is_identifier(name), "{name}");
        }
    }

    #[test]
    fn edit_serialization_sorts_and_removes_duplicate_locations() {
        let source = SourceFile::new("main.veln", "target target\n");
        let first = source.span(TextRange::new(0, 6));
        let second = source.span(TextRange::new(7, 13));

        let edits = edits_json(
            std::path::Path::new("workspace"),
            [&second, &first, &second, &first],
            "next",
        );

        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0]["range"]["start"], json!({"line": 1, "column": 1}));
        assert_eq!(edits[1]["range"]["start"], json!({"line": 1, "column": 8}));
        assert!(edits.iter().all(|edit| edit["new_text"] == "next"));
    }
}
