use super::*;

pub(super) fn check_unresolved_adt_payload_type_names(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    module
        .types
        .iter()
        .flat_map(|declaration| check_declaration_payload_type_names(declaration, environment))
        .collect()
}

fn check_declaration_payload_type_names(
    declaration: &veln_ast::TypeDecl,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let type_parameters = declaration
        .params
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    declaration
        .variants
        .iter()
        .flat_map(|variant| &variant.fields)
        .flat_map(|field| {
            let mut paths = Vec::new();
            collect_non_refinement_annotation_paths(
                &field.ty_paths,
                &field.ty_refinements,
                &type_parameters,
                &mut paths,
            );
            paths
                .into_iter()
                .filter_map(|path| {
                    unresolved_payload_type_diagnostic(
                        path,
                        &type_parameters,
                        declaration,
                        environment,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn unresolved_payload_type_diagnostic(
    path: &veln_ast::TypePathSegments,
    type_parameters: &HashSet<&str>,
    declaration: &veln_ast::TypeDecl,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let name = path.segments.join("::");
    if payload_type_path_is_known(path, &name, type_parameters, declaration, environment) {
        return None;
    }
    let span = unresolved_payload_type_span(path, &name, declaration, environment)?;
    Some(unresolved_type_diagnostic(name, span))
}

fn payload_type_path_is_known(
    path: &veln_ast::TypePathSegments,
    name: &str,
    type_parameters: &HashSet<&str>,
    declaration: &veln_ast::TypeDecl,
    environment: &TypeEnvironment,
) -> bool {
    is_unapplied_type_parameter(path, type_parameters)
        || path.segments.last().is_some_and(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|initial| initial.is_ascii_lowercase())
        })
        || environment.annotation_type_path_is_resolved_in_owner(
            name,
            path.type_argument_count,
            declaration.module_name.as_deref(),
            declaration.package_name.as_deref(),
        )
        || environment.annotation_type_path_has_visible_declaration_in_owner(
            name,
            declaration.module_name.as_deref(),
            declaration.package_name.as_deref(),
        )
}

fn is_unapplied_type_parameter(
    path: &veln_ast::TypePathSegments,
    type_parameters: &HashSet<&str>,
) -> bool {
    path.segments.len() == 1
        && path.type_argument_count == 0
        && type_parameters.contains(path.segments[0].as_str())
}

fn unresolved_payload_type_span(
    path: &veln_ast::TypePathSegments,
    name: &str,
    declaration: &veln_ast::TypeDecl,
    environment: &TypeEnvironment,
) -> Option<veln_source::SourceSpan> {
    if environment.annotation_type_path_has_resolved_qualifier_in_owner(
        name,
        declaration.module_name.as_deref(),
        declaration.package_name.as_deref(),
    ) {
        path.segment_spans.last().cloned()
    } else {
        path.segment_spans.first().cloned()
    }
}
