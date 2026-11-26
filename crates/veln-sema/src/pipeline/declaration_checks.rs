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
    module: &SurfaceModule,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> (Vec<Diagnostic>, HashMap<veln_ast::NodeId, Type>) {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_function_type_parameter_shadowing(
        function,
        environment,
    ));
    diagnostics.extend(check_unresolved_function_type_names(
        module,
        function,
        environment,
    ));
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
    module: &SurfaceModule,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let declaration_binder_names = module
        .functions
        .iter()
        .flat_map(|function| function.type_binders.iter())
        .map(|binder| binder.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    if declaration_binder_names.is_empty() {
        return Vec::new();
    }
    let type_parameters = function
        .type_binders
        .iter()
        .map(|binder| binder.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut paths = Vec::new();
    paths.extend(function.return_type_paths.iter());
    for param in &function.params {
        paths.extend(param.ty_paths.iter());
    }
    collect_body_annotation_paths(&function.body, &mut paths);

    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.segments.join("::");
            if !declaration_binder_names.contains(name.as_str()) {
                return None;
            }
            if !name.contains("::")
                && path.type_argument_count == 0
                && type_parameters.contains(name.as_str())
            {
                return None;
            }
            if environment.annotation_type_path_is_resolved_in_owner(
                &name,
                path.type_argument_count,
                function.module_name.as_deref(),
                function.package_name.as_deref(),
            ) {
                return None;
            }
            let span = path.segment_spans.first()?.clone();
            Some(Diagnostic::new(
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
            ))
        })
        .collect()
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
                paths.extend(annotation_structure.paths.iter());
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
            let builtin = crate::type_syntax::builtin_type_syntax_arity(&binder.name).is_some();
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

            let mut diagnostic = Diagnostic::new(
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
            );
            if let Some(descriptor) = descriptor {
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
                            JsonValue::string(format!(
                                "Visible type `{}` is declared here.",
                                binder.name
                            )),
                        ),
                        ("span", crate::diagnostics::span_json(&span)),
                    ]));
                }
            }
            Some(diagnostic)
        })
        .collect()
}
