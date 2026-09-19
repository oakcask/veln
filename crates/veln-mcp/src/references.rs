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
    include_declaration: bool,
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
            include_declaration: arguments
                .get("include_declaration")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            raw_line: &arguments["line"],
            raw_column: &arguments["column"],
        })
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
        let mut locations = result
            .references
            .iter()
            .map(|span| location_json(&root, span))
            .collect::<Vec<_>>();
        if request.include_declaration {
            locations.push(navigation_location_json(&root, &result.definition));
        }
        locations
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
    ) && (!matches!(
        result.selected_symbol.declaration_kind,
        SymbolDeclarationKind::PublicAlias
    ) || (result.selected_symbol.kind == SymbolKind::Schema && result.reference_eligible)
        || (result.selected_symbol.kind != SymbolKind::Schema && !result.references.is_empty()))
}

fn supports_package_references(result: &NavigationResult) -> bool {
    supports_package_reference_kind(result)
        && supports_package_reference_origin(result)
        && supports_package_reference_declaration(result)
        && (!matches!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        ) || !result.references.is_empty())
}

fn supports_package_reference_kind(result: &NavigationResult) -> bool {
    matches!(
        result.selected_symbol.kind,
        SymbolKind::Function | SymbolKind::Type | SymbolKind::Constructor
    ) || (result.selected_symbol.kind == SymbolKind::Schema
        && matches!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
        )
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

fn navigation_location_json(
    root: &std::path::Path,
    location: &veln_language_service::NavigationLocation,
) -> Value {
    let uri = match &location.source {
        NavigationSource::Workspace => path_to_uri(&root.join(location.span.file.as_str())),
        NavigationSource::Package { uri } => uri.clone(),
    };
    json!({
        "uri": uri,
        "range": {
            "start": {
                "line": location.span.start.line,
                "column": location.span.start.column
            },
            "end": {
                "line": location.span.end.line,
                "column": location.span.end.column
            }
        }
    })
}
