use super::*;

impl TypeEnvironment {
    pub(in crate::types) fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        let functions = selected_standard_facts(&self.functions, module_names, |signature| {
            signature.module_name.as_deref()
        });
        let functions_by_name = facts::function_name_index(&functions);
        let recoveries = self.standard_subset_recoveries(module_names);
        let access_targets = self.standard_subset_access_targets(module_names);
        Self {
            functions,
            functions_by_name,
            function_recovery_signatures: recoveries.function_signatures,
            function_recoveries: recoveries.functions,
            constructor_recoveries: recoveries.constructors,
            import_constructor_recoveries: recoveries.import_constructors,
            codec_calls: selected_standard_facts(&self.codec_calls, module_names, |signature| {
                signature.module_name.as_deref()
            }),
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
            codec_symbols: selected_standard_facts(&self.codec_symbols, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            uses: selected_standard_facts(&self.uses, module_names, |use_decl| {
                use_decl.module_name.as_deref()
            }),
            quarantined_uses: selected_standard_facts(
                &self.quarantined_uses,
                module_names,
                |use_decl| use_decl.module_name.as_deref(),
            ),
            invalid_names: self.invalid_names.clone(),
            adts: self.adts.standard_subset(module_names),
            companion_function_access_targets: access_targets.functions,
            companion_schema_access_targets: access_targets.schemas,
            companion_effect_access_targets: access_targets.effects,
        }
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
