use serde_json::{Value, json};
use veln_language_service::{
    NavigationResult, NavigationSource, PackageOrigin, SourcePosition, SymbolDeclarationKind,
    SymbolKind, navigate,
};
use veln_source::{SourcePath, SourceSpan};

use crate::check_project::capture_navigation_source;
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
    let source = arguments["source"]
        .as_str()
        .expect("references input schema requires a string source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let (captured, captured_source, scope) =
        match capture_navigation_source(base, selection, source) {
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
    let workspace_key = captured.key;
    let dependencies = match language_resources.admit_dependencies(&captured.dependencies) {
        Ok(dependencies) => dependencies,
        Err(error) => return error.into(),
    };
    let snapshot = language_resources.with_dependency_navigation(
        captured.project.files,
        dependencies,
        workspace_key,
    );
    let references = navigate(
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
    .unwrap_or_default();

    ToolOutcome::Success(json!({
        "references": references,
        "scope": scope.metadata(selection.generation())
    }))
}

fn supported_reference_symbol(result: &NavigationResult) -> bool {
    match result.definition.source {
        NavigationSource::Workspace => matches!(
            result.selected_symbol.kind,
            SymbolKind::Type
                | SymbolKind::Function
                | SymbolKind::Constructor
                | SymbolKind::ValueBinding
                | SymbolKind::HandlerContextParameter
                | SymbolKind::HandlerOperationClauseParameter
        ),
        NavigationSource::Package { .. } => {
            matches!(
                result.selected_symbol.kind,
                SymbolKind::Function | SymbolKind::Type
            ) && result.selected_symbol.declaration_kind == SymbolDeclarationKind::Declaration
                && matches!(
                    result.selected_symbol.package_origin,
                    Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                )
        }
    }
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
