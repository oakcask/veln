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
    if function.package_name.as_deref() == Some(veln_stdlib::PACKAGE_NAME) {
        return Vec::new();
    }
    let type_parameters = function
        .type_binders
        .iter()
        .map(|binder| binder.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut paths = Vec::new();
    let mut refinement_spans = Vec::new();
    paths.extend(function.return_type_paths.iter());
    refinement_spans.extend(
        function
            .return_type_refinements
            .iter()
            .map(|refinement| &refinement.span),
    );
    for param in &function.params {
        paths.extend(param.ty_paths.iter());
        refinement_spans.extend(
            param
                .ty_refinements
                .iter()
                .map(|refinement| &refinement.span),
        );
    }
    collect_body_annotation_paths(&function.body, &mut paths, &mut refinement_spans);

    paths
        .into_iter()
        .filter(|path| !path_is_within_refinement(path, &refinement_spans))
        .filter_map(|path| {
            unresolved_function_type_diagnostic(path, &type_parameters, function, environment)
        })
        .collect()
}

fn path_is_within_refinement(
    path: &veln_ast::TypePathSegments,
    refinement_spans: &[&veln_source::SourceSpan],
) -> bool {
    refinement_spans.iter().any(|span| {
        span.file == path.application_span.file
            && span.start.offset <= path.application_span.start.offset
            && path.application_span.end.offset <= span.end.offset
    })
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
    let span = path.segment_spans.first()?.clone();
    Some(unresolved_type_diagnostic(name, span))
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
    refinement_spans: &mut Vec<&'a veln_source::SourceSpan>,
) {
    for line in body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                annotation_structure,
                expr,
                ..
            } => {
                paths.extend(annotation_structure.paths.iter());
                refinement_spans.extend(
                    annotation_structure
                        .variant_refinements
                        .iter()
                        .map(|refinement| &refinement.span),
                );
                collect_expr_annotation_paths(expr, paths, refinement_spans);
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_expr_annotation_paths(expr, paths, refinement_spans);
            }
            veln_ast::BodyLineKind::Defer { body, .. } => {
                collect_body_annotation_paths(body, paths, refinement_spans);
            }
        }
    }
}

fn collect_expr_annotation_paths<'a>(
    expr: &'a veln_ast::Expr,
    paths: &mut Vec<&'a veln_ast::TypePathSegments>,
    refinement_spans: &mut Vec<&'a veln_source::SourceSpan>,
) {
    if let veln_ast::ExprKind::Begin { body, .. } = &expr.kind {
        collect_body_annotation_paths(body, paths, refinement_spans);
    } else {
        expr.for_each_child(&mut |child| {
            collect_expr_annotation_paths(child, paths, refinement_spans)
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
