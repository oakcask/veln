use serde_json::{Value, json};
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
    let (captured, captured_source, scope) =
        match capture_navigation_source(base, selection, request.source) {
            Ok(captured) => captured,
            Err(failure) => return failure,
        };
    let references =
        match collect_references(captured, &captured_source, &request, language_resources) {
            Ok(references) => references,
            Err(failure) => return failure,
        };

    ToolOutcome::Success(json!({
        "references": references,
        "scope": scope.metadata(selection.generation())
    }))
}

struct ReferenceArguments<'a> {
    source: &'a str,
    line: Coordinate,
    column: Coordinate,
    raw_line: &'a Value,
    raw_column: &'a Value,
}

impl<'a> ReferenceArguments<'a> {
    fn new(arguments: &'a Value) -> Self {
        Self {
            source: arguments["source"]
                .as_str()
                .expect("references input schema requires a string source"),
            line: coordinate(&arguments["line"]),
            column: coordinate(&arguments["column"]),
            raw_line: &arguments["line"],
            raw_column: &arguments["column"],
        }
    }
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
    let dependencies = language_resources.admit_dependencies(&captured.dependencies)?;
    let snapshot = language_resources.with_dependency_navigation(
        captured.project.files,
        dependencies,
        workspace_key,
    );
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
    )
}

fn supports_package_reference_origin(result: &NavigationResult) -> bool {
    matches!(
        result.selected_symbol.package_origin,
        Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
    )
}

fn supports_package_reference_declaration(result: &NavigationResult) -> bool {
    result.selected_symbol.declaration_kind == SymbolDeclarationKind::Declaration
        || supports_standard_library_public_alias(result)
}

fn supports_standard_library_public_alias(result: &NavigationResult) -> bool {
    result.selected_symbol.declaration_kind == SymbolDeclarationKind::PublicAlias
        && result.selected_symbol.package_origin == Some(PackageOrigin::StandardLibrary)
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
