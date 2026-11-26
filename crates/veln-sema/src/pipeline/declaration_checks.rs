use super::*;
use crate::analysis::check_function_body_with_facts;
use crate::semantic_model::Type;
use crate::types::TypeParameterIdentityContext;
use veln_diagnostics::{DiagnosticKind, JsonValue};

pub(super) struct SurfaceAnalysis {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) module_value_publication: ModuleValuePublicationProvenance,
    pub(super) state_call_types: crate::semantic_model::StateCallTypes,
}

pub(super) fn retain_function_state_call_types(
    state_call_types: &mut crate::semantic_model::StateCallTypes,
    function: &veln_ast::Function,
    function_state_call_types: HashMap<veln_ast::NodeId, Type>,
) {
    if !function_state_call_types.is_empty() {
        state_call_types.insert(
            (
                function.package_name.clone(),
                function.module_name.clone(),
                function.name.clone().unwrap_or_default(),
            ),
            function_state_call_types,
        );
    }
}

pub(super) fn check_module_declarations(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> (
    Vec<Diagnostic>,
    ModuleValuePublicationProvenance,
    crate::semantic_model::StateCallTypes,
) {
    let module_value_analysis = check_module_values(module, environment, variant_diagnostics);
    let mut module_value_publication = module_value_analysis.publication_provenance(module);
    let mut diagnostics = module_value_analysis.diagnostics;

    diagnostics.extend(check_invalid_name_casing(module, environment));
    append_name_collisions(module, &mut diagnostics, &mut module_value_publication);
    let handler_state_call_types =
        append_boundary_diagnostics(module, environment, &mut diagnostics);
    (
        diagnostics,
        module_value_publication,
        handler_state_call_types,
    )
}

fn append_name_collisions(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
    publication: &mut ModuleValuePublicationProvenance,
) {
    let (callable_diagnostics, callable_collisions) = check_duplicate_function_names(module);
    diagnostics.extend(callable_diagnostics);
    publication.callable_collisions = callable_collisions
        .into_iter()
        .map(|collision| (collision.identity, collision.diagnostic))
        .collect();
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_effect_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
}

fn append_boundary_diagnostics(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) -> crate::semantic_model::StateCallTypes {
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module, environment));
    diagnostics.extend(check_unresolved_adt_payload_type_names(module, environment));
    for declaration in &module.types {
        diagnostics.extend(check_public_adt_variant_refinement_visibility(
            declaration,
            environment,
        ));
    }
    diagnostics.extend(check_schema_field_primitives(module, environment));
    diagnostics.extend(check_schema_type_references(module));
    let handlers = check_handler_declarations(module, environment);
    diagnostics.extend(handlers.diagnostics);
    handlers.state_call_types
}

fn check_unresolved_adt_payload_type_names(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for declaration in &module.types {
        let type_parameters = declaration
            .params
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        for field in declaration
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
        {
            let mut paths = Vec::new();
            collect_non_refinement_annotation_paths(
                &field.ty_paths,
                &field.ty_refinements,
                &type_parameters,
                &mut paths,
            );
            for path in paths {
                let name = path.segments.join("::");
                let own_parameter = path.segments.len() == 1
                    && path.type_argument_count == 0
                    && type_parameters.contains(path.segments[0].as_str());
                let lowercase_recovery = path.segments.last().is_some_and(|segment| {
                    segment
                        .chars()
                        .next()
                        .is_some_and(|initial| initial.is_ascii_lowercase())
                });
                if own_parameter
                    || lowercase_recovery
                    || environment.annotation_type_path_is_resolved_in_owner(
                        &name,
                        path.type_argument_count,
                        declaration.module_name.as_deref(),
                        declaration.package_name.as_deref(),
                    )
                    || environment.annotation_type_path_has_visible_declaration_in_owner(
                        &name,
                        declaration.module_name.as_deref(),
                        declaration.package_name.as_deref(),
                    )
                {
                    continue;
                }
                let span = if environment.annotation_type_path_has_resolved_qualifier_in_owner(
                    &name,
                    declaration.module_name.as_deref(),
                    declaration.package_name.as_deref(),
                ) {
                    path.segment_spans.last()
                } else {
                    path.segment_spans.first()
                };
                if let Some(span) = span {
                    diagnostics.push(unresolved_type_diagnostic(name, span.clone()));
                }
            }
        }
    }
    diagnostics
}

pub(super) fn check_function_declaration_and_body(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> (Vec<Diagnostic>, HashMap<veln_ast::NodeId, Type>) {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_function_type_parameter_shadowing(
        function,
        environment,
    ));
    diagnostics.extend(check_function_type_parameter_signature(function));
    diagnostics.extend(check_unresolved_function_type_names(function, environment));
    diagnostics.extend(check_declared_effect_labels(function, environment));
    let mut visibility_diagnostics = Vec::new();
    if function.visibility == Visibility::Public {
        let (visibility, boundary): (Vec<_>, Vec<_>) =
            check_public_function_boundary(function, environment)
                .into_iter()
                .partition(|diagnostic| diagnostic.id == "type.variant_refinement_private");
        diagnostics.extend(boundary);
        visibility_diagnostics = visibility;
    }
    if function.kind == FunctionKind::Test {
        diagnostics.extend(check_test_declaration_boundary(function));
    }
    let body = check_function_body_with_facts(function, environment, variant_diagnostics);
    diagnostics.extend(body.diagnostics);
    diagnostics.extend(visibility_diagnostics);
    (diagnostics, body.state_call_types)
}

fn check_unresolved_function_type_names(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let type_parameters = function
        .type_binders
        .iter()
        .map(|binder| binder.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut paths = Vec::new();
    collect_non_refinement_annotation_paths(
        &function.return_type_paths,
        &function.return_type_refinements,
        &type_parameters,
        &mut paths,
    );
    for param in &function.params {
        collect_non_refinement_annotation_paths(
            &param.ty_paths,
            &param.ty_refinements,
            &type_parameters,
            &mut paths,
        );
    }
    collect_body_annotation_paths(&function.body, &type_parameters, &mut paths);
    let type_parameter_context = TypeParameterIdentityContext::for_function(function);

    paths
        .into_iter()
        .filter_map(|path| {
            unresolved_function_type_diagnostic(
                path,
                &type_parameters,
                &type_parameter_context,
                function,
                environment,
            )
        })
        .collect()
}

#[cfg(test)]
thread_local! {
    static ANNOTATION_PATH_CLASSIFICATION_WORK: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
pub(crate) fn reset_annotation_path_classification_work() {
    ANNOTATION_PATH_CLASSIFICATION_WORK.set((0, 0));
}

#[cfg(test)]
pub(crate) fn take_annotation_path_classification_work() -> (usize, usize) {
    ANNOTATION_PATH_CLASSIFICATION_WORK.replace((0, 0))
}

fn collect_non_refinement_annotation_paths<'a>(
    paths: &'a [veln_ast::TypePathSegments],
    refinements: &'a [veln_ast::VariantRefinementType],
    type_parameters: &HashSet<&str>,
    output: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    #[cfg(test)]
    ANNOTATION_PATH_CLASSIFICATION_WORK.update(|(path_work, refinement_work)| {
        (path_work + paths.len(), refinement_work + refinements.len())
    });

    // The flattened annotation paths omit some applied paths inside refinement
    // arguments. Keep the established span classifier for refinement bases,
    // restore argument paths from the refinement tree, then merge the two
    // source-ordered streams. Refinement nesting has a parser-owned bound, so
    // each path is visited only a bounded number of times.
    debug_assert!(paths.windows(2).all(|pair| {
        pair[0].application_span.file != pair[1].application_span.file
            || pair[0].application_span.start.offset <= pair[1].application_span.start.offset
    }));
    debug_assert!(refinements.windows(2).all(|pair| {
        pair[0].span.file != pair[1].span.file
            || pair[0].span.start.offset <= pair[1].span.start.offset
    }));
    let mut direct = Vec::new();
    let mut refinement_index = 0;
    let mut furthest_refinement_end = 0;
    for path in paths {
        while refinements.get(refinement_index).is_some_and(|refinement| {
            refinement.span.file == path.application_span.file
                && refinement.span.start.offset <= path.application_span.start.offset
        }) {
            furthest_refinement_end =
                furthest_refinement_end.max(refinements[refinement_index].span.end.offset);
            refinement_index += 1;
        }
        let qualified_function_type_parameter =
            path.segments.len() > 1 && type_parameters.contains(path.segments[0].as_str());
        if path.application_span.end.offset > furthest_refinement_end
            || qualified_function_type_parameter
        {
            direct.push(path);
        }
    }
    let mut nested = Vec::new();
    collect_variant_refinement_argument_paths(refinements, type_parameters, &mut nested);
    merge_annotation_paths(&direct, &nested, output);
}

fn collect_variant_refinement_argument_paths<'a>(
    refinements: &'a [veln_ast::VariantRefinementType],
    type_parameters: &HashSet<&str>,
    output: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    for refinement in refinements {
        for alternative in &refinement.alternatives {
            for argument in &alternative.type_arguments {
                collect_non_refinement_annotation_paths(
                    &argument.ty_paths,
                    &argument.ty_refinements,
                    type_parameters,
                    output,
                );
            }
        }
    }
}

fn merge_annotation_paths<'a>(
    direct: &[&'a veln_ast::TypePathSegments],
    nested: &[&'a veln_ast::TypePathSegments],
    output: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    let mut direct_index = 0;
    let mut nested_index = 0;
    while direct_index < direct.len() || nested_index < nested.len() {
        let direct_path = direct.get(direct_index).copied();
        let nested_path = nested.get(nested_index).copied();
        let selected = match (direct_path, nested_path) {
            (Some(direct_path), Some(nested_path)) => {
                let direct_key = annotation_path_span_key(direct_path);
                let nested_key = annotation_path_span_key(nested_path);
                if direct_key <= nested_key {
                    direct_index += 1;
                    if direct_key == nested_key {
                        nested_index += 1;
                    }
                    direct_path
                } else {
                    nested_index += 1;
                    nested_path
                }
            }
            (Some(direct_path), None) => {
                direct_index += 1;
                direct_path
            }
            (None, Some(nested_path)) => {
                nested_index += 1;
                nested_path
            }
            (None, None) => break,
        };
        output.push(selected);
    }
}

fn annotation_path_span_key(path: &veln_ast::TypePathSegments) -> (&str, usize, usize) {
    (
        path.application_span.file.as_str(),
        path.application_span.start.offset,
        path.application_span.end.offset,
    )
}

fn unresolved_function_type_diagnostic(
    path: &veln_ast::TypePathSegments,
    type_parameters: &HashSet<&str>,
    type_parameter_context: &TypeParameterIdentityContext<'_>,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let name = path.segments.join("::");
    if let Some(diagnostic) = function_type_parameter_application_diagnostic(
        path,
        &name,
        type_parameters,
        type_parameter_context,
        function,
        environment,
    ) {
        return Some(diagnostic);
    }
    if function_type_path_is_known(path, &name, type_parameters, function, environment) {
        return None;
    }
    let span = unresolved_function_type_span(path, &name, function, environment)?;
    Some(unresolved_type_diagnostic(name, span))
}

fn function_type_parameter_application_diagnostic(
    path: &veln_ast::TypePathSegments,
    name: &str,
    type_parameters: &HashSet<&str>,
    type_parameter_context: &TypeParameterIdentityContext<'_>,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let unqualified =
        path.segments.len() == 1 && type_parameters.contains(path.segments[0].as_str());
    if !unqualified {
        return None;
    }
    environment
        .type_path_arity_error_with_context(
            name,
            path.type_argument_count,
            function.module_name.as_deref(),
            type_parameter_context,
        )
        .map(|error| invalid_function_type_parameter_application_diagnostic(path, function, error))
}

fn function_type_path_is_known(
    path: &veln_ast::TypePathSegments,
    name: &str,
    type_parameters: &HashSet<&str>,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> bool {
    path.segments.last().is_some_and(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_lowercase())
    }) || is_function_type_parameter_path(path, type_parameters)
        || is_compiler_known_standard_signature_type_parameter(name, function)
        || environment.annotation_type_path_is_resolved_in_owner(
            name,
            path.type_argument_count,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
        )
        || environment.annotation_type_path_has_visible_declaration_in_owner(
            name,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
        )
}

fn unresolved_function_type_span(
    path: &veln_ast::TypePathSegments,
    name: &str,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<veln_source::SourceSpan> {
    if environment.annotation_type_path_has_resolved_qualifier_in_owner(
        name,
        function.module_name.as_deref(),
        function.package_name.as_deref(),
    ) {
        path.segment_spans.last().cloned()
    } else {
        path.segment_spans.first().cloned()
    }
}

fn invalid_function_type_parameter_application_diagnostic(
    path: &veln_ast::TypePathSegments,
    function: &veln_ast::Function,
    error: String,
) -> Diagnostic {
    Diagnostic::new(
        "type.invalid_annotation",
        Severity::Error,
        DiagnosticKind::Type,
        format!("invalid type annotation `{}`: {error}", path.written),
        Some(path.application_span.clone()),
        crate::diagnostics::type_details(
            function.node_id.display("fn"),
            "valid_type",
            path.written.clone(),
            "source",
            "source",
            "assignable",
            [function.node_id.display("fn")],
        ),
    )
}

fn is_compiler_known_standard_signature_type_parameter(
    name: &str,
    function: &veln_ast::Function,
) -> bool {
    if name.contains("::")
        || !function.type_binders.is_empty()
        || function.package_name.as_deref() != Some(veln_stdlib::PACKAGE_NAME)
    {
        return false;
    }
    let function_name = function.name.as_deref().unwrap_or_default();
    let allowed = match (function.module_name.as_deref(), function_name) {
        (Some("std::prelude"), "vec_fold") => &["A", "B"][..],
        (
            Some("std::prelude"),
            "vec_len" | "vec_is_empty" | "vec_push" | "vec_concat" | "vec_append" | "vec_filter"
            | "vec_filter_step",
        ) => &["A"],
        (Some("std::prelude"), "vec_map" | "vec_map_step") => &["Item", "A"],
        (Some("std::prelude"), "vec_try_map" | "vec_try_map_state" | "vec_try_map_step") => {
            &["Item", "A", "E"]
        }
        (
            Some("std::prelude"),
            "vec_try_map_with" | "vec_try_map_with_state" | "vec_try_map_with_step",
        ) => &["C", "Item", "A", "E"],
        (
            Some("std::prelude"),
            "list_nil" | "list_cons" | "list_is_empty" | "list_reverse" | "list_reverse_step"
            | "list_filter" | "list_filter_step",
        ) => &["A"],
        (Some("std::prelude"), "list_fold") => &["Item", "B"],
        (Some("std::prelude"), "list_map" | "list_map_step") => &["Item", "A"],
        (Some("std::prelude"), "list_try_map" | "list_try_map_step") => &["Item", "A", "E"],
        (
            Some("std::prelude"),
            "dict_get" | "dict_contains" | "dict_insert" | "dict_remove" | "dict_filter",
        ) => &["K", "V"],
        (Some("std::prelude"), "dict_map" | "dict_fold") => &["K", "V", "A"],
        (
            Some("std::prelude"),
            "dict_map_with" | "dict_map_with_step" | "dict_fold_with" | "dict_fold_with_step",
        ) => &["C", "K", "V", "A"],
        (Some("std::prelude"), "dict_filter_with" | "dict_filter_with_step") => &["C", "K", "V"],
        (Some("std::prelude"), "dict_try_map") => &["K", "V", "A", "E"],
        (
            Some("std::prelude"),
            "dict_try_map_with" | "dict_try_map_with_state" | "dict_try_map_with_step",
        ) => &["C", "K", "V", "A", "E"],
        (Some("std::prelude"), "option_map" | "option_and_then") => &["A", "B"],
        (Some("std::prelude"), "option_unwrap_or") => &["A"],
        (Some("std::prelude"), "result_map" | "result_and_then") => &["A", "E", "B"],
        (Some("std::prelude"), "result_map_err") => &["A", "E", "F"],
        (Some("std::io_provider"), "state") => &["T"],
        (Some("std::io_provider"), "update") => &["T", "C", "R"],
        _ => &[],
    };
    allowed.contains(&name)
}

fn is_function_type_parameter_path(
    path: &veln_ast::TypePathSegments,
    type_parameters: &HashSet<&str>,
) -> bool {
    path.segments.len() == 1
        && path.type_argument_count == 0
        && type_parameters.contains(path.segments[0].as_str())
}

fn unresolved_type_diagnostic(name: String, span: veln_source::SourceSpan) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved type `{name}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("symbol", JsonValue::string(name)),
            ("namespace", JsonValue::string("type")),
            ("resolution_status", JsonValue::string("unresolved")),
            ("candidates", JsonValue::array([])),
        ]),
    )
}

fn collect_body_annotation_paths<'a>(
    body: &'a [veln_ast::BodyLine],
    type_parameters: &HashSet<&str>,
    paths: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    for line in body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                annotation_structure,
                expr,
                ..
            } => {
                collect_non_refinement_annotation_paths(
                    &annotation_structure.paths,
                    &annotation_structure.variant_refinements,
                    type_parameters,
                    paths,
                );
                collect_expr_annotation_paths(expr, type_parameters, paths);
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_expr_annotation_paths(expr, type_parameters, paths);
            }
            veln_ast::BodyLineKind::Defer { body, .. } => {
                collect_body_annotation_paths(body, type_parameters, paths);
            }
        }
    }
}

fn collect_expr_annotation_paths<'a>(
    expr: &'a veln_ast::Expr,
    type_parameters: &HashSet<&str>,
    paths: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    if let veln_ast::ExprKind::Begin { body, .. } = &expr.kind {
        collect_body_annotation_paths(body, type_parameters, paths);
    } else {
        expr.for_each_child(&mut |child| {
            collect_expr_annotation_paths(child, type_parameters, paths)
        });
    }
}

fn check_function_type_parameter_shadowing(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    function
        .type_binders
        .iter()
        .enumerate()
        .filter_map(|(ordinal, binder)| {
            function_type_parameter_shadow_diagnostic(ordinal, binder, function, environment)
        })
        .collect()
}

fn check_function_type_parameter_signature(function: &veln_ast::Function) -> Vec<Diagnostic> {
    if function.visibility == Visibility::Public || function.type_binders.is_empty() {
        return Vec::new();
    }

    let mut diagnostics = function
        .params
        .iter()
        .filter(|param| {
            param
                .ty
                .as_deref()
                .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
        })
        .map(|param| {
            Diagnostic::new(
                "type.generic_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!("generic parameter `{}` has no type annotation", param.name),
                Some(param.span.clone()),
                crate::diagnostics::type_details(
                    param.node_id.display("param"),
                    "explicit",
                    "missing",
                    "declared_parameter",
                    "source",
                    "assignable",
                    [function.node_id.display("fn")],
                ),
            )
        })
        .collect::<Vec<_>>();
    if function.return_type.is_none() {
        diagnostics.push(Diagnostic::new(
            "type.generic_signature_missing",
            Severity::Error,
            DiagnosticKind::Type,
            "generic function has no return type annotation",
            Some(function.span.clone()),
            crate::diagnostics::type_details(
                function.node_id.display("fn"),
                "explicit",
                "missing",
                "declared_return",
                "source",
                "return_value",
                [function.node_id.display("fn")],
            ),
        ));
    }
    diagnostics
}

fn function_type_parameter_shadow_diagnostic(
    ordinal: usize,
    binder: &veln_ast::TypeBinder,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let builtin = crate::type_syntax::compiler_known_type_annotation_arity(&binder.name).is_some()
        || crate::adt::is_opaque_builtin_type(&binder.name);
    let descriptor = environment
        .adts
        .unique_descriptor_for_type_path_any_arity_in_owner(
            &binder.name,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
            &environment.uses,
        )
        .or_else(|| {
            environment
                .adts
                .consistent_imported_type_path_descriptor_any_arity(
                    &binder.name,
                    function.module_name.as_deref(),
                    function.package_name.as_deref(),
                    &environment.uses,
                )
        })
        .or_else(|| {
            builtin.then(|| {
                environment.adts.local_source_descriptor_with_declaration(
                    &binder.name,
                    function.module_name.as_deref(),
                    function.package_name.as_deref(),
                )
            })?
        });
    if !builtin && descriptor.is_none() {
        return None;
    }

    let mut diagnostic = type_parameter_shadow_diagnostic(ordinal, binder, builtin);
    if let Some(descriptor) = descriptor {
        append_shadowed_type_declaration(&mut diagnostic, binder, descriptor, environment);
    }
    Some(diagnostic)
}

fn type_parameter_shadow_diagnostic(
    ordinal: usize,
    binder: &veln_ast::TypeBinder,
    builtin: bool,
) -> Diagnostic {
    Diagnostic::new(
        "name.type_parameter_shadow",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "function type parameter `{}` shadows a visible type",
            binder.name
        ),
        Some(binder.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name_resolution")),
            ("boundary", JsonValue::string("function_type_parameter")),
            ("name", JsonValue::string(binder.name.clone())),
            ("ordinal", JsonValue::Number(ordinal as i64)),
            (
                "shadowed_kind",
                JsonValue::string(if builtin {
                    "builtin_type"
                } else {
                    "visible_type"
                }),
            ),
        ]),
    )
}

fn append_shadowed_type_declaration(
    diagnostic: &mut Diagnostic,
    binder: &veln_ast::TypeBinder,
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    environment: &TypeEnvironment,
) {
    let related = environment
        .adts
        .type_alias_exposure_path(descriptor)
        .and_then(|path| path.declarations().next().map(|decl| decl.span.clone()))
        .or_else(|| environment.adts.declaration_span(descriptor).cloned());
    if let Some(span) = related {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("shadowed_type_declaration")),
            (
                "message",
                JsonValue::string(format!("Visible type `{}` is declared here.", binder.name)),
            ),
            ("span", crate::diagnostics::span_json(&span)),
        ]));
    }
}
