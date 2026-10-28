use super::*;

type ImportedOwnerIndex =
    HashMap<(Option<String>, String), BTreeSet<(Option<String>, Option<String>)>>;

pub(super) fn finish_module_value_facts(module: &SurfaceModule, environment: &mut TypeEnvironment) {
    let first_new = environment.module_values.len();
    append_module_value_signatures(module, &mut environment.module_values);
    canonicalize_new_module_value_types(environment, first_new);
    index_new_module_value_identities(environment, first_new);
    environment.bare_module_values = bare_module_value_index(
        &environment.module_values,
        &environment.functions,
        &environment.uses,
    );
}

pub(in crate::types::environment) fn qualified_module_value_import_index(
    uses: &[UseDecl],
) -> HashMap<QualifiedModuleValueKey, QualifiedModuleValueImports> {
    let mut imports = HashMap::<QualifiedModuleValueKey, QualifiedModuleValueImports>::new();
    for use_decl in uses {
        record_qualified_module_value_import_build_work();
        let owner = use_decl.resolved_owner();
        for path in crate::name_recovery::use_decl_exact_paths(use_decl) {
            imports
                .entry((owner.clone(), path))
                .or_default()
                .exact
                .push(use_decl.clone());
        }
        if crate::name_recovery::use_decl_matches_implicit_import_alias(
            use_decl,
            &use_decl.alias,
            use_decl.module_name.as_deref(),
        ) {
            imports
                .entry((owner, use_decl.alias.clone()))
                .or_default()
                .implicit
                .push(use_decl.clone());
        }
    }
    imports
}

fn append_module_value_signatures(
    module: &SurfaceModule,
    module_values: &mut Vec<ModuleValueSignature>,
) {
    module_values.extend(
        module
            .module_values
            .iter()
            .filter(|value| value.name.is_some())
            .map(|value| ModuleValueSignature {
                declaration: value.clone(),
                ty: parse_type_or_unknown(Some(&value.annotation)),
                available: true,
            }),
    );
}

fn canonicalize_new_module_value_types(environment: &mut TypeEnvironment, first_new: usize) {
    let types = environment.module_values[first_new..]
        .iter()
        .map(|value| {
            environment.canonicalize_type_annotation_in_owner(
                value.ty.clone(),
                value.declaration.module_name.as_deref(),
                value.declaration.package_name.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    for (value, ty) in environment.module_values[first_new..].iter_mut().zip(types) {
        value.ty = ty;
    }
}

fn index_new_module_value_identities(environment: &mut TypeEnvironment, first_new: usize) {
    for (index, value) in environment.module_values.iter().enumerate().skip(first_new) {
        let Some(name) = &value.declaration.name else {
            continue;
        };
        let identity = (
            value.declaration.package_name.clone(),
            value.declaration.module_name.clone(),
            name.clone(),
        );
        environment
            .module_values_by_identity
            .entry(identity)
            .and_modify(|entry| *entry = ModuleValueIndex::Collision)
            .or_insert(ModuleValueIndex::Unique(index));
    }
}

pub(in crate::types::environment) fn bare_module_value_index(
    module_values: &[ModuleValueSignature],
    functions: &[FunctionSignature],
    uses: &[UseDecl],
) -> HashMap<BareModuleValueKey, BareModuleValueExposure> {
    let imported_owners = imported_owner_index(uses);
    let mut selections = public_module_value_selections(module_values, &imported_owners);
    record_function_collisions(&mut selections, functions, &imported_owners);
    selections
}

fn imported_owner_index(uses: &[UseDecl]) -> ImportedOwnerIndex {
    let mut imported_owners = HashMap::new();
    for use_decl in uses
        .iter()
        .filter(|use_decl| crate::name_recovery::use_decl_contributes_bare_exports(use_decl))
    {
        let target = use_decl.resolved_target();
        imported_owners
            .entry((target.package, target.module))
            .or_insert_with(BTreeSet::new)
            .insert((
                use_decl.importing_package.clone(),
                use_decl.module_name.clone(),
            ));
    }
    imported_owners
}

fn public_module_value_selections(
    module_values: &[ModuleValueSignature],
    imported_owners: &ImportedOwnerIndex,
) -> HashMap<BareModuleValueKey, BareModuleValueExposure> {
    let mut selections = HashMap::new();
    for value in module_values
        .iter()
        .filter(|value| value.declaration.visibility == Visibility::Public)
    {
        record_module_value_selections(&mut selections, value, imported_owners);
    }
    selections
}

fn record_module_value_selections(
    selections: &mut HashMap<BareModuleValueKey, BareModuleValueExposure>,
    value: &ModuleValueSignature,
    imported_owners: &ImportedOwnerIndex,
) {
    let (Some(module), Some(name)) = (
        value.declaration.module_name.as_ref(),
        value.declaration.name.as_ref(),
    ) else {
        return;
    };
    let target = (value.declaration.package_name.clone(), module.clone());
    let Some(owners) = imported_owners.get(&target) else {
        return;
    };
    let identity = (
        value.declaration.package_name.clone(),
        value.declaration.module_name.clone(),
        name.clone(),
    );
    for (owner_package, owner_module) in owners {
        let key = (owner_package.clone(), owner_module.clone(), name.clone());
        record_module_value_selection(selections, key, &identity);
    }
}

fn record_module_value_selection(
    selections: &mut HashMap<BareModuleValueKey, BareModuleValueExposure>,
    key: BareModuleValueKey,
    identity: &ModuleValueIdentity,
) {
    selections
        .entry(key)
        .and_modify(|exposure| {
            if !matches!(&exposure.selection, BareModuleValueSelection::Unique(existing) if existing == identity)
            {
                exposure.selection = BareModuleValueSelection::Ambiguous;
            }
        })
        .or_insert_with(|| BareModuleValueExposure {
            selection: BareModuleValueSelection::Unique(identity.clone()),
            function_collision: false,
        });
}

fn record_function_collisions(
    selections: &mut HashMap<BareModuleValueKey, BareModuleValueExposure>,
    functions: &[FunctionSignature],
    imported_owners: &ImportedOwnerIndex,
) {
    for function in functions
        .iter()
        .filter(|function| function.visibility == Visibility::Public)
    {
        record_function_collision(selections, function, imported_owners);
    }
}

fn record_function_collision(
    selections: &mut HashMap<BareModuleValueKey, BareModuleValueExposure>,
    function: &FunctionSignature,
    imported_owners: &ImportedOwnerIndex,
) {
    let Some(module) = function.module_name.as_ref() else {
        return;
    };
    let target = (function.package_name.clone(), module.clone());
    let Some(owners) = imported_owners.get(&target) else {
        return;
    };
    for (owner_package, owner_module) in owners {
        let key = (
            owner_package.clone(),
            owner_module.clone(),
            function.name.clone(),
        );
        if let Some(exposure) = selections.get_mut(&key) {
            exposure.function_collision = true;
        }
    }
}
