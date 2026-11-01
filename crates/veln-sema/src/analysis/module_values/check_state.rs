use super::*;

pub(super) struct ModuleValueCheckState {
    diagnostics: Vec<Diagnostic>,
    dependencies: Vec<Vec<Dependency>>,
    collision_dependencies: Vec<Vec<ModuleValueIdentity>>,
    direct_causes: Vec<Vec<ModuleValuePublicationCause>>,
    invalid: Vec<bool>,
}

impl ModuleValueCheckState {
    pub(super) fn new(
        module: &SurfaceModule,
        intrinsic_invalid: &BTreeSet<ModuleValueIdentity>,
    ) -> Self {
        let value_count = module.module_values.len();
        Self {
            diagnostics: Vec::new(),
            dependencies: vec![Vec::new(); value_count],
            collision_dependencies: vec![Vec::new(); value_count],
            direct_causes: vec![Vec::new(); value_count],
            invalid: initially_invalid_values(module, intrinsic_invalid),
        }
    }

    pub(super) fn check_declarations(
        &mut self,
        module: &SurfaceModule,
        environment: &TypeEnvironment,
        variant_diagnostics: &mut VariantDiagnosticInterner,
        invalid_cased_identities: &BTreeSet<ModuleValueIdentity>,
        value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    ) {
        for (index, value) in module.module_values.iter().enumerate() {
            self.check_declaration(
                index,
                value,
                environment,
                variant_diagnostics,
                invalid_cased_identities,
                value_indices,
            );
        }
    }

    fn check_declaration(
        &mut self,
        index: usize,
        value: &veln_ast::ModuleValueDecl,
        environment: &TypeEnvironment,
        variant_diagnostics: &mut VariantDiagnosticInterner,
        invalid_cased_identities: &BTreeSet<ModuleValueIdentity>,
        value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    ) {
        #[cfg(test)]
        update_module_value_availability_work(|work| work.semantic_visits += 1);
        let mut collected_dependencies = DependencyCollector {
            dependencies: &mut self.dependencies[index],
            collision_dependencies: &mut self.collision_dependencies[index],
        };
        let diagnostic_start = self.diagnostics.len();
        self.invalid[index] |= check_module_value(
            value,
            index,
            value_indices,
            environment,
            variant_diagnostics,
            &mut collected_dependencies,
            &mut self.diagnostics,
        );
        self.record_new_diagnostic_causes(index, value, diagnostic_start);
        self.record_invalid_casing_cause(index, value, invalid_cased_identities);
        self.record_quarantined_import_causes(index, value, environment);
    }

    fn record_new_diagnostic_causes(
        &mut self,
        index: usize,
        value: &veln_ast::ModuleValueDecl,
        diagnostic_start: usize,
    ) {
        self.direct_causes[index].extend(
            self.diagnostics[diagnostic_start..]
                .iter()
                .enumerate()
                .filter_map(|(relative_index, diagnostic)| {
                    publication_cause(
                        value.package_name.clone(),
                        diagnostic,
                        Some(diagnostic_start + relative_index),
                    )
                }),
        );
    }

    fn record_invalid_casing_cause(
        &mut self,
        index: usize,
        value: &veln_ast::ModuleValueDecl,
        invalid_cased_identities: &BTreeSet<ModuleValueIdentity>,
    ) {
        let invalid_cased = value.name.as_ref().is_some_and(|name| {
            invalid_cased_identities.contains(&(
                value.package_name.clone(),
                value.module_name.clone(),
                name.clone(),
            ))
        });
        if invalid_cased && let Some(span) = value.name_span.clone() {
            self.direct_causes[index].push(ModuleValuePublicationCause {
                package_name: value.package_name.clone(),
                diagnostic_id: "name.invalid_case".to_string(),
                diagnostic_message: None,
                span,
                diagnostic_index: None,
            });
        }
    }

    fn record_quarantined_import_causes(
        &mut self,
        index: usize,
        value: &veln_ast::ModuleValueDecl,
        environment: &TypeEnvironment,
    ) {
        for span in quarantined_import_causes(value, environment) {
            self.invalid[index] = true;
            self.direct_causes[index].push(ModuleValuePublicationCause {
                package_name: value.package_name.clone(),
                diagnostic_id: "name.invalid_case".to_string(),
                diagnostic_message: None,
                span,
                diagnostic_index: None,
            });
        }
    }

    pub(super) fn apply_cycle_failures(
        &mut self,
        module: &SurfaceModule,
        intrinsic_invalid: &mut BTreeSet<ModuleValueIdentity>,
    ) {
        let (cycle_diagnostics, cyclic) = cycle_diagnostics(module, &self.dependencies);
        for (owner, diagnostic) in cycle_diagnostics {
            if let Some(cause) = publication_cause(
                module.module_values[owner].package_name.clone(),
                &diagnostic,
                Some(self.diagnostics.len()),
            ) {
                self.direct_causes[owner].push(cause);
            }
            self.diagnostics.push(diagnostic);
        }
        for index in cyclic {
            self.invalid[index] = true;
            let value = &module.module_values[index];
            if let Some(name) = &value.name {
                intrinsic_invalid.insert((
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                ));
            }
        }
    }

    pub(super) fn finish(
        mut self,
        module: &SurfaceModule,
        intrinsic_invalid: BTreeSet<ModuleValueIdentity>,
    ) -> ModuleValueAnalysis {
        propagate_invalid_dependencies(&self.dependencies, &mut self.invalid);
        ModuleValueAnalysis {
            diagnostics: self.diagnostics,
            intrinsic_invalid,
            dependencies: self.dependencies,
            collision_dependencies: self.collision_dependencies,
            direct_causes: self.direct_causes,
            invalid: invalid_module_value_identities(module, &self.invalid),
            invalid_indices: self.invalid,
        }
    }
}
