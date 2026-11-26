use super::*;
use crate::analysis::check_function_body_with_facts;
use crate::semantic_model::Type;
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
        &mut paths,
    );
    for param in &function.params {
        collect_non_refinement_annotation_paths(&param.ty_paths, &param.ty_refinements, &mut paths);
    }
    collect_body_annotation_paths(&function.body, &mut paths);

    paths
        .into_iter()
        .filter_map(|path| {
            unresolved_function_type_diagnostic(path, &type_parameters, function, environment)
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
    output: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    #[cfg(test)]
    ANNOTATION_PATH_CLASSIFICATION_WORK.update(|(path_work, refinement_work)| {
        (path_work + paths.len(), refinement_work + refinements.len())
    });

    // Lowering retains both collections in source order, so one refinement
    // cursor classifies the annotation without comparing every path and span.
    debug_assert!(paths.windows(2).all(|pair| {
        pair[0].application_span.file != pair[1].application_span.file
            || pair[0].application_span.start.offset <= pair[1].application_span.start.offset
    }));
    debug_assert!(refinements.windows(2).all(|pair| {
        pair[0].span.file != pair[1].span.file
            || pair[0].span.start.offset <= pair[1].span.start.offset
    }));
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
        if path.application_span.end.offset > furthest_refinement_end {
            output.push(path);
        }
    }
}

fn unresolved_function_type_diagnostic(
    path: &veln_ast::TypePathSegments,
    type_parameters: &HashSet<&str>,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let name = path.segments.join("::");
    if path.segments.last().is_some_and(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_lowercase())
    }) || is_function_type_parameter_name(&name, type_parameters)
        || is_compiler_known_standard_signature_type_parameter(&name, function)
        || environment.annotation_type_path_is_resolved_in_owner(
            &name,
            path.type_argument_count,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
        )
        || environment.annotation_type_path_has_visible_declaration_in_owner(
            &name,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
        )
    {
        return None;
    }
    let span = if environment.annotation_type_path_has_resolved_qualifier_in_owner(
        &name,
        function.module_name.as_deref(),
        function.package_name.as_deref(),
    ) {
        path.segment_spans.last()?.clone()
    } else {
        path.segment_spans.first()?.clone()
    };
    Some(unresolved_type_diagnostic(name, span))
}

fn is_compiler_known_standard_signature_type_parameter(
    name: &str,
    function: &veln_ast::Function,
) -> bool {
    if name.contains("::") || function.package_name.as_deref() != Some(veln_stdlib::PACKAGE_NAME) {
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

fn is_function_type_parameter_name(name: &str, type_parameters: &HashSet<&str>) -> bool {
    let base = name.split_once("::").map_or(name, |(base, _)| base);
    type_parameters.contains(base)
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
                    paths,
                );
                collect_expr_annotation_paths(expr, paths);
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_expr_annotation_paths(expr, paths);
            }
            veln_ast::BodyLineKind::Defer { body, .. } => {
                collect_body_annotation_paths(body, paths);
            }
        }
    }
}

fn collect_expr_annotation_paths<'a>(
    expr: &'a veln_ast::Expr,
    paths: &mut Vec<&'a veln_ast::TypePathSegments>,
) {
    if let veln_ast::ExprKind::Begin { body, .. } = &expr.kind {
        collect_body_annotation_paths(body, paths);
    } else {
        expr.for_each_child(&mut |child| collect_expr_annotation_paths(child, paths));
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

fn function_type_parameter_shadow_diagnostic(
    ordinal: usize,
    binder: &veln_ast::TypeBinder,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let builtin = crate::type_syntax::compiler_known_type_annotation_arity(&binder.name).is_some();
    let descriptor = if builtin {
        None
    } else {
        environment
            .adts
            .unique_descriptor_for_type_path_any_arity_in_owner(
                &binder.name,
                function.module_name.as_deref(),
                function.package_name.as_deref(),
                &environment.uses,
            )
    };
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
