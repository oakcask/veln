use super::*;
use crate::analysis::check_function_body_with_facts;
use crate::semantic_model::Type;

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
