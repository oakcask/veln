use super::*;

#[derive(Default)]
struct QualifiedImportCandidates {
    exact: Vec<UseDecl>,
    implicit: Vec<UseDecl>,
}

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
) -> HashMap<QualifiedModuleValueKey, QualifiedModuleValueImportSelection> {
    let imports = qualified_import_candidates(uses);

    imports
        .into_iter()
        .filter_map(|(key, candidates)| qualified_import_selection(key, candidates))
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

fn qualified_import_selection(
    key: QualifiedModuleValueKey,
    candidates: QualifiedImportCandidates,
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

    let mut selected = selected;
    selected.sort_by_key(|use_decl| {
        let target = use_decl.resolved_target();
        (
            target.package,
            target.module,
            use_decl.span.file.clone(),
            use_decl.span.start.offset,
        )
    });
    selected.dedup_by(|left, right| {
        let left = left.resolved_target();
        let right = right.resolved_target();
        left.package == right.package && left.module == right.module
    });
    Some((
        key,
        QualifiedModuleValueImportSelection::Ambiguous(selected),
    ))
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

pub(in crate::types::environment) struct BareModuleValueIndexes {
    pub(in crate::types::environment) module_values:
        HashMap<String, BTreeMap<ModuleValueTarget, ModuleValueIdentity>>,
    pub(in crate::types::environment) functions: HashMap<String, BTreeSet<ModuleValueTarget>>,
    pub(in crate::types::environment) targets_by_owner:
        HashMap<ModuleValueOwner, BTreeSet<ModuleValueTarget>>,
    pub(in crate::types::environment) owners_by_target:
        HashMap<ModuleValueTarget, BTreeSet<ModuleValueOwner>>,
    pub(in crate::types::environment) representative_imports:
        HashMap<ModuleValueImportKey, UseDecl>,
}

#[derive(Default)]
struct BareModuleValueImportIndexes {
    targets_by_owner: HashMap<ModuleValueOwner, BTreeSet<ModuleValueTarget>>,
    owners_by_target: HashMap<ModuleValueTarget, BTreeSet<ModuleValueOwner>>,
    representative_imports: HashMap<ModuleValueImportKey, UseDecl>,
}

pub(in crate::types::environment) fn bare_module_value_indexes(
    module_values: &[ModuleValueSignature],
    functions: &[FunctionSignature],
    unpublished_functions: &HashSet<FunctionPublicationKey>,
    uses: &[UseDecl],
) -> BareModuleValueIndexes {
    let imports = bare_module_value_import_indexes(uses);
    BareModuleValueIndexes {
        module_values: public_module_values_by_name(module_values),
        functions: published_functions_by_name(functions, unpublished_functions),
        targets_by_owner: imports.targets_by_owner,
        owners_by_target: imports.owners_by_target,
        representative_imports: imports.representative_imports,
    }
}

fn bare_module_value_import_indexes(uses: &[UseDecl]) -> BareModuleValueImportIndexes {
    let mut indexes = BareModuleValueImportIndexes::default();
    for use_decl in uses
        .iter()
        .filter(|use_decl| crate::name_recovery::use_decl_contributes_bare_exports(use_decl))
    {
        record_module_value_ambiguity_provenance_index_visit();
        indexes.insert(use_decl);
    }
    indexes
}

impl BareModuleValueImportIndexes {
    fn insert(&mut self, use_decl: &UseDecl) {
        let resolved_target = use_decl.resolved_target();
        let target = (resolved_target.package, Some(resolved_target.module));
        let owner = (
            use_decl.importing_package.clone(),
            use_decl.module_name.clone(),
        );
        self.update_representative(owner.clone(), target.clone(), use_decl);
        self.index_owner_target(owner, target);
    }

    fn update_representative(
        &mut self,
        owner: ModuleValueOwner,
        target: ModuleValueTarget,
        use_decl: &UseDecl,
    ) {
        self.representative_imports
            .entry((owner, target))
            .and_modify(|representative| {
                if (use_decl.span.file.as_str(), use_decl.span.start.offset)
                    < (
                        representative.span.file.as_str(),
                        representative.span.start.offset,
                    )
                {
                    *representative = use_decl.clone();
                }
            })
            .or_insert_with(|| use_decl.clone());
    }

    fn index_owner_target(&mut self, owner: ModuleValueOwner, target: ModuleValueTarget) {
        if self
            .targets_by_owner
            .entry(owner.clone())
            .or_default()
            .insert(target.clone())
        {
            record_bare_module_value_reverse_index_entry();
        }
        if self
            .owners_by_target
            .entry(target)
            .or_default()
            .insert(owner)
        {
            record_bare_module_value_reverse_index_entry();
        }
    }
}

fn public_module_values_by_name(
    module_values: &[ModuleValueSignature],
) -> HashMap<String, BTreeMap<ModuleValueTarget, ModuleValueIdentity>> {
    let mut module_values_by_name =
        HashMap::<String, BTreeMap<ModuleValueTarget, ModuleValueIdentity>>::new();
    for value in module_values
        .iter()
        .filter(|value| value.declaration.visibility == Visibility::Public)
    {
        let Some(module) = value.declaration.module_name.clone() else {
            continue;
        };
        let Some(name) = value.declaration.name.clone() else {
            continue;
        };
        let target = (value.declaration.package_name.clone(), Some(module));
        let identity = (target.0.clone(), target.1.clone(), name.clone());
        record_bare_module_value_index_entry();
        module_values_by_name
            .entry(name)
            .or_default()
            .insert(target, identity);
    }
    module_values_by_name
}

fn published_functions_by_name(
    functions: &[FunctionSignature],
    unpublished_functions: &HashSet<FunctionPublicationKey>,
) -> HashMap<String, BTreeSet<ModuleValueTarget>> {
    let mut function_targets_by_name = HashMap::<String, BTreeSet<ModuleValueTarget>>::new();
    for function in functions
        .iter()
        .filter(|function| function.visibility == Visibility::Public)
        .filter(|function| !unpublished_functions.contains(&function_publication_key(function)))
    {
        let Some(module) = function.module_name.clone() else {
            continue;
        };
        record_bare_module_value_index_entry();
        function_targets_by_name
            .entry(function.name.clone())
            .or_default()
            .insert((function.package_name.clone(), Some(module)));
    }
    function_targets_by_name
}
