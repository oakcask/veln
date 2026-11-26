use super::*;

impl AdtRegistry {
    pub(crate) fn local_source_descriptor_with_declaration(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&AdtDescriptor> {
        self.descriptors_by_type_name
            .get(name)
            .into_iter()
            .flatten()
            .rev()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .find(|descriptor| {
                descriptor.module_name.as_deref() == current_module
                    && descriptor.package_name.as_deref() == current_package
                    && self.declaration_span(descriptor).is_some()
            })
    }

    pub(super) fn descriptor_matches_arity(
        descriptor: &AdtDescriptor,
        args_len: Option<usize>,
    ) -> bool {
        args_len.is_none_or(|args_len| descriptor.type_parameters.len() == args_len)
    }

    pub(super) fn descriptor_for_unqualified_type_path(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.descriptor_in_current_module(name, args_len, current_module, uses)
            .or_else(|| self.root_descriptor(name, args_len))
            .or_else(|| self.unique_imported_descriptor(name, args_len, current_module, uses))
    }

    pub(super) fn descriptor_in_current_module(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if let Some(owner) = self.import_owner_for_module(uses, current_module) {
            return self
                .descriptors_by_source_identity
                .get(&(
                    owner.package,
                    current_module.map(str::to_string),
                    name.to_string(),
                ))
                .into_iter()
                .flatten()
                .rev()
                .map(|index| self.descriptor_lookup_candidate(*index))
                .find(|descriptor| Self::descriptor_matches_arity(descriptor, args_len));
        }
        let mut matching_identities =
            self.descriptors_by_source_identity
                .iter()
                .filter(|((_, module, type_name), _)| {
                    module.as_deref() == current_module && type_name == name
                });
        let (_, indices) = matching_identities.next()?;
        if matching_identities.next().is_some() {
            return None;
        }
        indices
            .iter()
            .rev()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .find(|descriptor| Self::descriptor_matches_arity(descriptor, args_len))
    }

    pub(super) fn root_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
    ) -> Option<&AdtDescriptor> {
        self.descriptors_by_source_identity
            .get(&(None, None, name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .find(|descriptor| Self::descriptor_matches_arity(descriptor, args_len))
    }

    pub(super) fn unique_imported_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if self.source_import_indexes_ready {
            crate::adt::lookup_validation::record_import_path_comparison();
            let owner = self.import_owner_for_module(uses, current_module)?;
            let index = self.unique_indexed_imported_descriptor(name, args_len, &owner)?;
            return Some(self.descriptor_lookup_candidate(index));
        }
        let mut imported = self.descriptors_named(name).filter(|descriptor| {
            Self::descriptor_matches_arity(descriptor, args_len)
                && Self::descriptor_is_unqualified_import(descriptor, current_module, uses)
        });
        let descriptor = imported.next()?;
        imported.next().is_none().then_some(descriptor)
    }

    pub(crate) fn imported_type_path_has_consistent_identity(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        self.consistent_imported_type_path_descriptor(
            name,
            Some(args_len),
            current_module,
            current_package,
            uses,
        )
        .is_some()
    }

    pub(crate) fn consistent_imported_type_path_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.consistent_imported_type_path_descriptor(
            name,
            None,
            current_module,
            current_package,
            uses,
        )
    }

    fn consistent_imported_type_path_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if self.source_import_indexes_ready {
            crate::adt::lookup_validation::record_import_path_comparison();
            let owner = veln_ast::ResolvedImportOwner {
                package: current_package.map(str::to_string),
                module: current_module.map(str::to_string),
            };
            let index = self.consistent_indexed_imported_descriptor(name, args_len, &owner)?;
            return Some(self.descriptor_lookup_candidate(index));
        }
        let mut descriptors = self.descriptors_named(name).filter(|descriptor| {
            Self::descriptor_matches_arity(descriptor, args_len)
                && Self::descriptor_is_unqualified_import_in_owner(
                    descriptor,
                    current_module,
                    current_package,
                    uses,
                )
        });
        let first = descriptors.next()?;
        let identity = self
            .terminal_descriptor_for_alias(first)
            .unwrap_or_else(|| self.canonical_descriptor_for(first))
            .identity();
        let mut representative = first;
        for descriptor in descriptors {
            let candidate_identity = self
                .terminal_descriptor_for_alias(descriptor)
                .unwrap_or_else(|| self.canonical_descriptor_for(descriptor))
                .identity();
            if candidate_identity != identity {
                return None;
            }
            if descriptor_stable_source_key(descriptor)
                < descriptor_stable_source_key(representative)
            {
                representative = descriptor;
            }
        }
        Some(representative)
    }

    pub(super) fn unique_indexed_imported_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
        owner: &ResolvedImportOwner,
    ) -> Option<usize> {
        let owner_targets = self.bare_import_targets_by_owner.get(owner)?;
        let targets = if let Some(args_len) = args_len {
            self.imported_descriptor_targets_by_type_arity
                .get(&(name.to_string(), args_len))?
        } else {
            self.imported_descriptor_targets_by_type.get(name)?
        };
        let cache_key = (owner.clone(), name.to_string(), args_len);
        if let Some(cached) = self
            .unique_imported_descriptor_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&cache_key)
            .copied()
        {
            return cached;
        }
        let visible_targets = if owner_targets.len() <= targets.len() {
            owner_targets
                .iter()
                .filter(|target| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    targets.contains(*target)
                })
                .collect::<Vec<_>>()
        } else {
            targets
                .iter()
                .filter(|target| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    owner_targets.contains(*target)
                })
                .collect::<Vec<_>>()
        };
        let mut selected = None;
        let mut ambiguous = false;
        for (package, module) in visible_targets {
            crate::adt::lookup_validation::record_import_path_comparison();
            let candidate = if let Some(args_len) = args_len {
                self.imported_descriptors_by_type_arity.get(&(
                    package.clone(),
                    module.clone(),
                    name.to_string(),
                    args_len,
                ))
            } else {
                self.imported_descriptors_by_type.get(&(
                    package.clone(),
                    module.clone(),
                    name.to_string(),
                ))
            };
            let Some(candidate) = candidate else {
                continue;
            };
            let candidate = (*candidate)?;
            if selected.replace(candidate).is_some() {
                ambiguous = true;
                break;
            }
        }
        if ambiguous {
            selected = None;
        }
        self.unique_imported_descriptor_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(cache_key, selected);
        selected
    }

    pub(super) fn consistent_indexed_imported_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
        owner: &ResolvedImportOwner,
    ) -> Option<usize> {
        let owner_targets = self.bare_import_targets_by_owner.get(owner)?;
        let targets = if let Some(args_len) = args_len {
            self.imported_descriptor_targets_by_type_arity
                .get(&(name.to_string(), args_len))?
        } else {
            self.imported_descriptor_targets_by_type.get(name)?
        };
        let cache_key = (owner.clone(), name.to_string(), args_len);
        if let Some(cached) = self
            .consistent_imported_descriptor_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&cache_key)
            .copied()
        {
            return cached;
        }
        let visible_targets = if owner_targets.len() <= targets.len() {
            owner_targets
                .iter()
                .filter(|target| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    targets.contains(*target)
                })
                .collect::<Vec<_>>()
        } else {
            targets
                .iter()
                .filter(|target| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    owner_targets.contains(*target)
                })
                .collect::<Vec<_>>()
        };
        let mut selected = None;
        let mut terminal_identity = None;
        let mut inconsistent = false;
        for (package, module) in visible_targets {
            crate::adt::lookup_validation::record_import_path_comparison();
            let candidate = if let Some(args_len) = args_len {
                self.consistent_imported_descriptors_by_type_arity.get(&(
                    package.clone(),
                    module.clone(),
                    name.to_string(),
                    args_len,
                ))
            } else {
                self.consistent_imported_descriptors_by_type.get(&(
                    package.clone(),
                    module.clone(),
                    name.to_string(),
                ))
            };
            let Some(candidate) = candidate else {
                continue;
            };
            let candidate = (*candidate)?;
            let descriptor = self.descriptor_lookup_candidate(candidate);
            let candidate_identity = self
                .terminal_descriptor_for_alias(descriptor)
                .unwrap_or_else(|| self.canonical_descriptor_for(descriptor))
                .identity();
            if terminal_identity
                .as_ref()
                .is_some_and(|identity| identity != &candidate_identity)
            {
                inconsistent = true;
                break;
            }
            terminal_identity = Some(candidate_identity);
            if selected.is_none_or(|selected| {
                descriptor_stable_source_key(descriptor)
                    < descriptor_stable_source_key(self.descriptor_lookup_candidate(selected))
            }) {
                selected = Some(candidate);
            }
        }
        if inconsistent {
            selected = None;
        }
        self.consistent_imported_descriptor_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(cache_key, selected);
        selected
    }

    fn descriptor_is_unqualified_import_in_owner(
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        descriptor.visibility == Visibility::Public
            && descriptor.module_name.as_ref().is_some_and(|module| {
                uses.iter().any(|use_decl| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    crate::name_recovery::use_decl_contributes_bare_exports(use_decl)
                        && use_decl.module_name.as_deref() == current_module
                        && use_decl.importing_package.as_deref() == current_package
                        && use_decl.resolved_target().module == *module
                        && descriptor.package_name == use_decl.resolved_target().package
                })
            })
    }

    pub(super) fn descriptor_is_unqualified_import(
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        descriptor.visibility == Visibility::Public
            && descriptor.module_name.as_ref().is_some_and(|module| {
                uses.iter().any(|use_decl| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    crate::name_recovery::use_decl_contributes_bare_exports(use_decl)
                        && use_decl.module_name.as_deref() == current_module
                        && use_decl.resolved_target().module == *module
                        && descriptor.package_name == use_decl.resolved_target().package
                })
            })
    }
}

fn descriptor_stable_source_key(descriptor: &AdtDescriptor) -> (Option<&str>, Option<&str>, &str) {
    (
        descriptor.package_name.as_deref(),
        descriptor.module_name.as_deref(),
        descriptor.type_name.as_str(),
    )
}
