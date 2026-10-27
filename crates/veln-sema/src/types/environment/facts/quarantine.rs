use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

type SourceTypeKey = (Option<String>, Option<String>, String);
type DependentTypes = HashMap<String, Vec<(SourceTypeKey, String)>>;

pub(super) fn quarantine_invalid_variant_refinement_signatures(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
) {
    let validation_environment = environment.clone();
    quarantine_invalid_variant_refinement_function_types(
        module,
        &validation_environment,
        &mut environment.functions,
    );
    quarantine_invalid_variant_refinement_effect_types(
        module,
        &validation_environment,
        &mut environment.effects,
    );
    quarantine_invalid_variant_refinement_handler_types(
        module,
        &validation_environment,
        &mut environment.handlers,
    );
    quarantine_invalid_variant_refinement_adt_payload_types(
        module,
        &validation_environment,
        &mut environment.adts,
    );
}

pub(super) fn quarantine_invalid_variant_refinement_declaration_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    effects: &mut [EffectSignature],
    adts: &mut AdtRegistry,
) {
    quarantine_invalid_variant_refinement_effect_types(module, environment, effects);
    quarantine_invalid_variant_refinement_adt_payload_types(module, environment, adts);
    let mut invalid_types = initially_invalid_declaration_types(module, environment);
    let (dependent_types, mut invalid_identities) =
        declaration_type_dependencies(module, environment, &invalid_types);
    propagate_invalid_declaration_types(
        &mut invalid_types,
        &dependent_types,
        &mut invalid_identities,
    );
    adts.quarantine_public_source_types(&invalid_types);
}

fn initially_invalid_declaration_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> HashSet<SourceTypeKey> {
    module
        .types
        .iter()
        .filter(|declaration| {
            crate::analysis::adt_has_private_variant_refinement_exposure(declaration, environment)
        })
        .filter_map(|declaration| {
            Some((
                declaration.package_name.clone(),
                declaration.module_name.clone(),
                declaration.name.clone()?,
            ))
        })
        .collect()
}

fn declaration_type_dependencies(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    invalid_types: &HashSet<SourceTypeKey>,
) -> (DependentTypes, VecDeque<String>) {
    let mut dependent_types = DependentTypes::new();
    let mut invalid_identities = environment
        .adts
        .quarantined_type_identities()
        .cloned()
        .collect::<VecDeque<_>>();
    for declaration in &module.types {
        let Some(name) = declaration.name.as_ref() else {
            continue;
        };
        let Some(descriptor) = environment.adts.descriptor_for_type_path(
            name,
            declaration.params.len(),
            declaration.module_name.as_deref(),
            &environment.uses,
        ) else {
            continue;
        };
        let key = (
            declaration.package_name.clone(),
            declaration.module_name.clone(),
            name.clone(),
        );
        let identity = descriptor.identity();
        if invalid_types.contains(&key) {
            invalid_identities.push_back(identity.clone());
        }
        for dependency in
            crate::analysis::adt_publication_dependency_identities(declaration, environment)
        {
            dependent_types
                .entry(dependency)
                .or_default()
                .push((key.clone(), identity.clone()));
        }
    }
    (dependent_types, invalid_identities)
}

fn propagate_invalid_declaration_types(
    invalid_types: &mut HashSet<SourceTypeKey>,
    dependent_types: &DependentTypes,
    invalid_identities: &mut VecDeque<String>,
) {
    while let Some(identity) = invalid_identities.pop_front() {
        let Some(dependents) = dependent_types.get(&identity) else {
            continue;
        };
        for (key, dependent_identity) in dependents {
            if invalid_types.insert(key.clone()) {
                invalid_identities.push_back(dependent_identity.clone());
            }
        }
    }
}

fn annotation_has_invalid_refinement(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[veln_ast::TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    crate::analysis::annotation_has_invalid_refinement(
        refinements,
        paths,
        current_module,
        environment,
        type_parameters,
    )
}

fn params_have_invalid_refinement(
    params: &[veln_ast::Param],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    params.iter().any(|param| {
        annotation_has_invalid_refinement(
            &param.ty_refinements,
            &param.ty_paths,
            current_module,
            environment,
            &[],
        )
    })
}

pub(super) fn quarantine_invalid_variant_refinement_function_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    functions: &mut [FunctionSignature],
) {
    let mut signatures = functions.iter_mut();
    for function in &module.functions {
        crate::pipeline::record_variant_refinement_base_work(1);
        if function.kind != FunctionKind::Function
            || !function
                .name
                .as_ref()
                .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
        {
            continue;
        }
        let signature = signatures
            .next()
            .expect("ordinary function signature follows its source declaration");
        crate::pipeline::record_variant_refinement_base_work(1);
        debug_assert_eq!(signature.node_id, function.node_id);
        debug_assert_eq!(signature.span, function.span);
        if !function_has_invalid_variant_refinement(module, function, environment) {
            continue;
        }
        signature.params.fill(Type::Unknown);
        if let Some(variadic) = &mut signature.variadic {
            *variadic = Type::Unknown;
        }
        signature.return_type = Type::Unknown;
    }
}

fn function_has_invalid_refinement(function: &Function, environment: &TypeEnvironment) -> bool {
    params_have_invalid_refinement(
        &function.params,
        function.module_name.as_deref(),
        environment,
    ) || annotation_has_invalid_refinement(
        &function.return_type_refinements,
        &function.return_type_paths,
        function.module_name.as_deref(),
        environment,
        &[],
    )
}

pub(super) fn function_has_invalid_variant_refinement(
    _module: &SurfaceModule,
    function: &Function,
    environment: &TypeEnvironment,
) -> bool {
    function_has_invalid_refinement(function, environment)
        || crate::analysis::function_has_private_variant_refinement_exposure(function, environment)
        || crate::analysis::function_depends_on_quarantined_adt(function, environment)
}

fn quarantine_invalid_variant_refinement_effect_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    effects: &mut [EffectSignature],
) {
    let mut signatures = effects.iter_mut();
    for effect in &module.effects {
        let Some(_) = effect.name.as_ref() else {
            continue;
        };
        let signature = signatures
            .next()
            .expect("effect signature follows its source declaration");
        let mut operation_signatures = signature.operations.iter_mut();
        for operation in &effect.operations {
            let Some(_) = operation.name.as_ref() else {
                continue;
            };
            let signature = operation_signatures
                .next()
                .expect("effect operation signature follows its source declaration");
            crate::pipeline::record_variant_refinement_base_work(1);
            let invalid = params_have_invalid_refinement(
                &operation.params,
                effect.module_name.as_deref(),
                environment,
            ) || annotation_has_invalid_refinement(
                &operation.return_type_refinements,
                &operation.return_type_paths,
                effect.module_name.as_deref(),
                environment,
                &[],
            );
            if !invalid {
                continue;
            }
            debug_assert_eq!(signature.node_id, operation.node_id);
            signature.params.fill(Type::Unknown);
            signature.return_type = Type::Unknown;
        }
    }
}

pub(super) fn quarantine_invalid_variant_refinement_handler_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    handlers: &mut [HandlerSignature],
) {
    let mut signatures = handlers.iter_mut();
    for handler in &module.handlers {
        let Some(_) = handler.name.as_ref() else {
            continue;
        };
        let signature = signatures
            .next()
            .expect("handler signature follows its source declaration");
        crate::pipeline::record_variant_refinement_base_work(1);
        if !params_have_invalid_refinement(
            &handler.params,
            handler.module_name.as_deref(),
            environment,
        ) {
            continue;
        }
        debug_assert_eq!(signature.name, handler.name.as_deref().unwrap_or_default());
        debug_assert_eq!(signature.module_name, handler.module_name);
        signature.params.fill(Type::Unknown);
    }
}

fn quarantine_invalid_variant_refinement_adt_payload_types(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    adts: &mut AdtRegistry,
) {
    let mut invalid = HashSet::new();
    for declaration in &module.types {
        let Some(type_name) = declaration.name.as_deref() else {
            continue;
        };
        for variant in &declaration.variants {
            crate::pipeline::record_variant_refinement_base_work(1);
            if !variant.fields.iter().any(|field| {
                annotation_has_invalid_refinement(
                    &field.ty_refinements,
                    &field.ty_paths,
                    declaration.module_name.as_deref(),
                    environment,
                    &declaration.params,
                )
            }) {
                continue;
            }
            let Some(variant_name) = variant.name.as_deref() else {
                continue;
            };
            invalid.insert((
                declaration.package_name.clone(),
                declaration.module_name.clone(),
                type_name.to_string(),
                variant_name.to_string(),
            ));
        }
    }
    let work = adts.quarantine_source_variant_payload_types(&invalid);
    crate::pipeline::record_variant_refinement_base_work(work);
}
