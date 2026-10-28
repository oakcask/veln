use super::*;

type ImportedOwnerIndex =
    HashMap<(Option<String>, String), BTreeSet<(Option<String>, Option<String>)>>;

#[derive(Default)]
struct QualifiedImportCandidates {
    exact: Vec<UseDecl>,
    implicit: Vec<UseDecl>,
}

type ModuleValueNamesByTarget =
    HashMap<(Option<String>, Option<String>), HashMap<String, (ModuleValueIdentity, bool)>>;

pub(in crate::types::environment) fn exact_module_value_import_paths(
    uses: &[UseDecl],
) -> HashSet<QualifiedModuleValueKey> {
    uses.iter()
        .flat_map(|use_decl| {
            let owner = use_decl.resolved_owner();
            crate::name_recovery::use_decl_exact_paths(use_decl)
                .into_iter()
                .map(move |path| (owner.clone(), path))
        })
        .collect()
}

pub(super) fn finish_module_value_facts(module: &SurfaceModule, environment: &mut TypeEnvironment) {
    let first_new = environment.module_values.len();
    append_module_value_signatures(module, &mut environment.module_values);
    canonicalize_new_module_value_types(environment, first_new);
    index_new_module_value_identities(environment, first_new);
    environment.rebuild_module_value_exposures();
}

pub(in crate::types::environment) fn qualified_module_value_import_index(
    uses: &[UseDecl],
    module_values: &[ModuleValueSignature],
    companion_access_targets: &BTreeMap<String, String>,
) -> HashMap<QualifiedModuleValueKey, QualifiedModuleValueImportSelection> {
    let imports = qualified_import_candidates(uses);
    let (public_names_by_target, all_names_by_target) =
        available_module_value_names_by_target(module_values);

    imports
        .into_iter()
        .filter_map(|(key, candidates)| {
            qualified_import_selection(
                key,
                candidates,
                companion_access_targets,
                &public_names_by_target,
                &all_names_by_target,
            )
        })
        .collect()
}

fn qualified_import_candidates(
    uses: &[UseDecl],
) -> HashMap<QualifiedModuleValueKey, QualifiedImportCandidates> {
    let mut imports = HashMap::<QualifiedModuleValueKey, QualifiedImportCandidates>::new();
    for use_decl in uses {
        record_qualified_module_value_import_build_work();
        let owner = use_decl.resolved_owner();
        for path in crate::name_recovery::use_decl_exact_paths(use_decl) {
            record_qualified_module_value_import_build_work();
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
            record_qualified_module_value_import_build_work();
            imports
                .entry((owner, use_decl.alias.clone()))
                .or_default()
                .implicit
                .push(use_decl.clone());
        }
    }
    imports
}

fn available_module_value_names_by_target(
    module_values: &[ModuleValueSignature],
) -> (ModuleValueNamesByTarget, ModuleValueNamesByTarget) {
    let mut public_names_by_target = ModuleValueNamesByTarget::new();
    let mut all_names_by_target = ModuleValueNamesByTarget::new();
    for value in module_values {
        record_qualified_module_value_member_materialization_work();
        let Some(name) = &value.declaration.name else {
            continue;
        };
        let target = (
            value.declaration.package_name.clone(),
            value.declaration.module_name.clone(),
        );
        let identity = (
            value.declaration.package_name.clone(),
            value.declaration.module_name.clone(),
            name.clone(),
        );
        all_names_by_target
            .entry(target.clone())
            .or_default()
            .insert(name.clone(), (identity.clone(), value.available));
        if value.declaration.visibility == Visibility::Public {
            public_names_by_target
                .entry(target)
                .or_default()
                .insert(name.clone(), (identity, value.available));
        }
    }
    (public_names_by_target, all_names_by_target)
}

fn qualified_import_selection(
    key: QualifiedModuleValueKey,
    candidates: QualifiedImportCandidates,
    companion_access_targets: &BTreeMap<String, String>,
    public_names_by_target: &ModuleValueNamesByTarget,
    all_names_by_target: &ModuleValueNamesByTarget,
) -> Option<(QualifiedModuleValueKey, QualifiedModuleValueImportSelection)> {
    let selected = if candidates.exact.is_empty() {
        candidates.implicit
    } else {
        candidates.exact
    };
    let targets = selected
        .iter()
        .map(|use_decl| {
            record_qualified_module_value_target_classification_work();
            let target = use_decl.resolved_target();
            (target.package, target.module)
        })
        .collect::<HashSet<_>>();
    if targets.is_empty() {
        return None;
    }
    if targets.len() == 1 {
        return Some((
            key,
            QualifiedModuleValueImportSelection::Unique(Box::new(
                selected.into_iter().next().expect("selected import"),
            )),
        ));
    }

    let (member_candidates, active_members) = ambiguous_import_visible_members(
        &key,
        &selected,
        companion_access_targets,
        public_names_by_target,
        all_names_by_target,
    );
    Some((
        key,
        QualifiedModuleValueImportSelection::Ambiguous {
            candidates: member_candidates,
            active: active_members,
        },
    ))
}

fn ambiguous_import_visible_members(
    key: &QualifiedModuleValueKey,
    selected: &[UseDecl],
    companion_access_targets: &BTreeMap<String, String>,
    public_names_by_target: &ModuleValueNamesByTarget,
    all_names_by_target: &ModuleValueNamesByTarget,
) -> (
    HashMap<String, BTreeSet<ModuleValueIdentity>>,
    HashMap<String, BTreeSet<ModuleValueIdentity>>,
) {
    let mut member_candidates = HashMap::<String, BTreeSet<ModuleValueIdentity>>::new();
    let mut active_members = HashMap::<String, BTreeSet<ModuleValueIdentity>>::new();
    for use_decl in selected {
        record_qualified_module_value_target_classification_work();
        let target = use_decl.resolved_target();
        let target_key = (target.package, Some(target.module));
        let private_access = use_decl.package.is_none()
            && key.0.module.as_ref().is_some_and(|owner_module| {
                companion_access_targets
                    .get(owner_module)
                    .is_some_and(|allowed| allowed == &use_decl.name)
            });
        let names = if private_access {
            all_names_by_target.get(&target_key)
        } else {
            public_names_by_target.get(&target_key)
        };
        if let Some(names) = names {
            for (name, (identity, available)) in names {
                record_qualified_module_value_member_materialization_work();
                member_candidates
                    .entry(name.clone())
                    .or_default()
                    .insert(identity.clone());
                if *available {
                    active_members
                        .entry(name.clone())
                        .or_default()
                        .insert(identity.clone());
                }
            }
        }
    }
    (member_candidates, active_members)
}

pub(in crate::types::environment) fn public_module_value_identities(
    module_values: &[ModuleValueSignature],
) -> HashSet<ModuleValueIdentity> {
    module_values
        .iter()
        .filter(|value| value.available && value.declaration.visibility == Visibility::Public)
        .filter_map(|value| {
            Some((
                value.declaration.package_name.clone(),
                value.declaration.module_name.clone(),
                value.declaration.name.clone()?,
            ))
        })
        .collect()
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
        record_module_value_selection(selections, key, &identity, value.available);
    }
}

fn record_module_value_selection(
    selections: &mut HashMap<BareModuleValueKey, BareModuleValueExposure>,
    key: BareModuleValueKey,
    identity: &ModuleValueIdentity,
    available: bool,
) {
    selections
        .entry(key)
        .and_modify(|exposure| {
            exposure.candidates.insert(identity.clone());
            if available {
                exposure.active.insert(identity.clone());
            }
        })
        .or_insert_with(|| BareModuleValueExposure {
            candidates: BTreeSet::from([identity.clone()]),
            active: if available {
                BTreeSet::from([identity.clone()])
            } else {
                BTreeSet::new()
            },
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
