use super::*;

impl TypeEnvironment {
    pub(in crate::types) fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        let functions = selected_standard_facts(&self.functions, module_names, |signature| {
            signature.module_name.as_deref()
        });
        let functions_by_name = facts::function_name_index(&functions);
        let recoveries = self.standard_subset_recoveries(module_names);
        let access_targets = self.standard_subset_access_targets(module_names);
        let (module_values, module_values_by_identity) =
            self.standard_subset_module_values(module_names);
        let uses = selected_standard_facts(&self.uses, module_names, |use_decl| {
            use_decl.module_name.as_deref()
        });
        let bare_indexes = facts::bare_module_value_indexes(
            &module_values,
            &functions,
            &self.unpublished_functions,
            &uses,
        );
        let blocked_exact_module_value_imports = self
            .blocked_exact_module_value_imports
            .iter()
            .filter(|(owner, _)| {
                owner
                    .module
                    .as_ref()
                    .is_none_or(|module| module_names.contains(module))
            })
            .cloned()
            .collect();
        let public_module_value_identities = facts::public_module_value_identities(&module_values);
        let exact_module_value_imports = facts::exact_module_value_import_paths(&uses);
        let qualified_module_value_imports = facts::qualified_module_value_import_index(&uses);
        Self {
            module_values,
            module_values_by_identity,
            bare_module_value_exports: bare_indexes.module_values,
            bare_function_exports: bare_indexes.functions,
            bare_module_value_import_targets: bare_indexes.targets_by_owner,
            module_value_import_owners: bare_indexes.owners_by_target,
            bare_module_value_cache: Arc::new(std::sync::RwLock::new(HashMap::new())),
            qualified_module_value_imports,
            exact_module_value_imports,
            blocked_exact_module_value_imports,
            public_module_value_identities,
            functions,
            functions_by_name,
            unpublished_functions: self.unpublished_functions.clone(),
            function_recovery_signatures: recoveries.function_signatures,
            function_recoveries: recoveries.functions,
            constructor_recoveries: recoveries.constructors,
            import_constructor_recoveries: recoveries.import_constructors,
            effects: selected_standard_facts(&self.effects, module_names, |signature| {
                signature.module_name.as_deref()
            }),
            handlers: selected_standard_facts(&self.handlers, module_names, |signature| {
                signature.module_name.as_deref()
            }),
            schema_symbols: self.schema_symbols.standard_subset(module_names),
            type_symbols: selected_standard_facts(&self.type_symbols, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            uses,
            quarantined_uses: selected_standard_facts(
                &self.quarantined_uses,
                module_names,
                |use_decl| use_decl.module_name.as_deref(),
            ),
            invalid_names: self.invalid_names.clone(),
            adts: self.adts.standard_subset(module_names),
            companion_function_access_targets: access_targets.functions,
            companion_module_value_access_targets: access_targets.module_values,
            companion_schema_access_targets: access_targets.schemas,
            companion_effect_access_targets: access_targets.effects,
        }
    }

    fn standard_subset_module_values(
        &self,
        module_names: &BTreeSet<String>,
    ) -> (
        Vec<ModuleValueSignature>,
        HashMap<ModuleValueIdentity, ModuleValueIndex>,
    ) {
        let module_values = selected_standard_facts(&self.module_values, module_names, |value| {
            value.declaration.module_name.as_deref()
        });
        let mut module_values_by_identity = HashMap::new();
        for (index, value) in module_values.iter().enumerate() {
            let Some(name) = &value.declaration.name else {
                continue;
            };
            let identity = (
                value.declaration.package_name.clone(),
                value.declaration.module_name.clone(),
                name.clone(),
            );
            module_values_by_identity
                .entry(identity)
                .and_modify(|entry| *entry = ModuleValueIndex::Collision)
                .or_insert(ModuleValueIndex::Unique(index));
        }
        (module_values, module_values_by_identity)
    }

    fn standard_subset_recoveries(
        &self,
        module_names: &BTreeSet<String>,
    ) -> StandardSubsetRecoveries {
        StandardSubsetRecoveries {
            function_signatures: self
                .function_recovery_signatures
                .iter()
                .filter(|signature| {
                    optional_module_is_selected(&signature.module_name, module_names)
                })
                .cloned()
                .collect(),
            functions: selected_recovery_counts_by_module(
                &self.function_recoveries,
                module_names,
                |key| &key.module_name,
            ),
            constructors: selected_recovery_counts_by_module(
                &self.constructor_recoveries,
                module_names,
                |key| &key.module_name,
            ),
            import_constructors: selected_recovery_counts_by_module(
                &self.import_constructor_recoveries,
                module_names,
                |key| &key.current_module,
            ),
        }
    }

    fn standard_subset_access_targets(
        &self,
        module_names: &BTreeSet<String>,
    ) -> StandardSubsetAccessTargets {
        StandardSubsetAccessTargets {
            functions: selected_standard_access_targets(
                &self.companion_function_access_targets,
                module_names,
            ),
            module_values: self
                .companion_module_value_access_targets
                .iter()
                .filter(|(owner, target)| {
                    owner
                        .module
                        .as_ref()
                        .is_some_and(|module| module_names.contains(module))
                        && target
                            .module
                            .as_ref()
                            .is_some_and(|module| module_names.contains(module))
                })
                .map(|(owner, target)| (owner.clone(), target.clone()))
                .collect(),
            schemas: selected_standard_access_targets(
                &self.companion_schema_access_targets,
                module_names,
            ),
            effects: self
                .companion_effect_access_targets
                .iter()
                .filter(|(module, access)| {
                    module_names.contains(*module) && module_names.contains(&access.target_module)
                })
                .map(|(module, access)| (module.clone(), access.clone()))
                .collect(),
        }
    }
}

struct StandardSubsetRecoveries {
    function_signatures: Vec<FunctionSignature>,
    functions: BTreeMap<FunctionRecoveryKey, usize>,
    constructors: BTreeMap<ConstructorRecoveryKey, usize>,
    import_constructors: BTreeMap<ImportConstructorRecoveryKey, usize>,
}

struct StandardSubsetAccessTargets {
    functions: BTreeMap<String, String>,
    module_values: HashMap<ResolvedImportOwner, ResolvedImportOwner>,
    schemas: BTreeMap<String, String>,
    effects: BTreeMap<String, CompanionAccessTarget>,
}

fn selected_recovery_counts_by_module<K, F>(
    recoveries: &BTreeMap<K, usize>,
    module_names: &BTreeSet<String>,
    module_name: F,
) -> BTreeMap<K, usize>
where
    K: Clone + Ord,
    F: Fn(&K) -> &Option<String>,
{
    recoveries
        .iter()
        .filter(|(key, _)| optional_module_is_selected(module_name(key), module_names))
        .map(|(key, count)| (key.clone(), *count))
        .collect()
}

fn optional_module_is_selected(module: &Option<String>, module_names: &BTreeSet<String>) -> bool {
    module
        .as_ref()
        .is_none_or(|module| module_names.contains(module))
}
