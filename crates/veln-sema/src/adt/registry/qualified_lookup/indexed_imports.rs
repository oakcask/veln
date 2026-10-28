use super::*;

impl AdtRegistry {
    pub(super) fn indexed_imported_descriptor_in_owner(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        current_package: Option<&str>,
        unique: bool,
    ) -> Option<Option<&AdtDescriptor>> {
        if !self.source_import_indexes_ready {
            return None;
        }
        crate::adt::lookup_validation::record_import_path_comparison();
        let owner = ResolvedImportOwner {
            package: current_package.map(str::to_string),
            module: current_module.map(str::to_string),
        };
        if !name.contains("::") {
            return Some(self.indexed_unqualified_descriptor_in_owner(name, args_len, &owner));
        }

        self.indexed_qualified_descriptor_in_owner(name, args_len, current_module, &owner, unique)
    }

    pub(super) fn indexed_imported_descriptor_including_quarantined_in_owner(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<Option<&AdtDescriptor>> {
        if !self.source_import_indexes_ready {
            return None;
        }
        crate::adt::lookup_validation::record_import_path_comparison();
        let owner = ResolvedImportOwner {
            package: current_package.map(str::to_string),
            module: current_module.map(str::to_string),
        };
        if !name.contains("::") {
            return Some(self.indexed_unqualified_descriptor_in_owner(
                name,
                Some(args_len),
                &owner,
            ));
        }

        self.indexed_qualified_descriptor_including_quarantined(
            name,
            args_len,
            current_module,
            &owner,
        )
    }

    fn indexed_qualified_descriptor_including_quarantined<'a>(
        &'a self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        owner: &ResolvedImportOwner,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let module_path = segments[..segments.len() - 1].join("::");
        crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
        if let Some(descriptor) = self.indexed_exact_descriptor_including_quarantined(
            &module_path,
            type_name,
            args_len,
            owner,
        ) {
            return Some(descriptor);
        }
        if let Some(builtin) = self.indexed_qualified_builtin_fallback(
            &module_path,
            type_name,
            Some(args_len),
            current_module,
            owner,
            false,
        ) {
            return Some(builtin);
        }
        Some(self.indexed_recovered_descriptor_including_quarantined(
            &segments,
            &module_path,
            type_name,
            args_len,
            current_module,
            owner,
        ))
    }

    fn indexed_exact_descriptor_including_quarantined<'a>(
        &'a self,
        module_path: &str,
        type_name: &str,
        args_len: usize,
        owner: &ResolvedImportOwner,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let modules = self
            .qualified_import_modules
            .get(&(owner.clone(), module_path.to_string()))?;
        let identity = self.source_identity_from_import_modules(type_name, modules)?;
        Some(
            self.qualified_descriptor_candidates(&identity)
                .rev()
                .find(|descriptor| Self::descriptor_matches_arity(descriptor, Some(args_len))),
        )
    }

    fn indexed_recovered_descriptor_including_quarantined<'a>(
        &'a self,
        segments: &[String],
        module_path: &str,
        type_name: &str,
        args_len: usize,
        current_module: Option<&str>,
        owner: &ResolvedImportOwner,
    ) -> Option<&'a AdtDescriptor> {
        let identity = self
            .indexed_recovered_qualified_source_identity(
                type_name,
                &segments[..segments.len() - 1],
                current_module,
                owner,
            )
            .unwrap_or(QualifiedSourceIdentity::Ambiguous);
        self.qualified_descriptor_candidates(&identity)
            .rev()
            .find(|descriptor| Self::descriptor_matches_arity(descriptor, Some(args_len)))
            .or_else(|| {
                (normalized_qualified_path(module_path) == "prelude")
                    .then(|| self.root_descriptor(type_name, Some(args_len)))
                    .flatten()
            })
    }

    fn indexed_recovered_qualified_source_identity(
        &self,
        type_name: &str,
        qualifiers: &[String],
        current_module: Option<&str>,
        owner: &ResolvedImportOwner,
    ) -> Option<QualifiedSourceIdentity> {
        let normalized = normalized_qualified_segments(qualifiers);
        crate::adt::lookup_validation::record_qualified_path_work(normalized.len());
        let mut selected = current_module
            .filter(|module_path| {
                crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
                normalized_qualified_path(module_path) == normalized
                    && is_single_initial_recovery(qualifiers, module_path)
            })
            .filter(|module_path| {
                self.source_identity_exists(owner.package.as_deref(), Some(module_path), type_name)
            })
            .map(|module_path| {
                QualifiedSourceIdentity::Source((
                    owner.package.clone(),
                    Some(module_path.to_string()),
                    type_name.to_string(),
                ))
            });

        if let Some(candidates) = self
            .qualified_import_recovery_modules
            .get(&(owner.clone(), normalized))
        {
            for candidate in candidates {
                crate::adt::lookup_validation::record_qualified_path_work(
                    candidate.module_path.len(),
                );
                if !is_single_initial_recovery(qualifiers, &candidate.module_path)
                    || current_module == Some(candidate.module_path.as_str())
                {
                    continue;
                }
                let identity =
                    self.source_identity_from_import_modules(type_name, &candidate.modules);
                if merge_recovered_identity(&mut selected, identity) {
                    return Some(QualifiedSourceIdentity::Ambiguous);
                }
            }
        }
        selected
    }

    fn indexed_unqualified_descriptor_in_owner<'a>(
        &'a self,
        name: &str,
        args_len: Option<usize>,
        owner: &ResolvedImportOwner,
    ) -> Option<&'a AdtDescriptor> {
        if let Some(descriptor) = self
            .descriptors_by_source_identity
            .get(&(None, None, name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .find(|descriptor| Self::descriptor_matches_arity(descriptor, args_len))
        {
            return Some(descriptor);
        }
        let index = if let Some(args_len) = args_len {
            self.imported_descriptors_by_type_arity.get(&(
                owner.clone(),
                name.to_string(),
                args_len,
            ))
        } else {
            self.imported_descriptors_by_type
                .get(&(owner.clone(), name.to_string()))
        }
        .copied()
        .flatten();
        index.map(|index| self.descriptor_lookup_candidate(index))
    }

    fn indexed_qualified_descriptor_in_owner<'a>(
        &'a self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        owner: &ResolvedImportOwner,
        unique: bool,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let module_path = segments[..segments.len() - 1].join("::");
        crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
        let Some(modules) = self
            .qualified_import_modules
            .get(&(owner.clone(), module_path.clone()))
        else {
            return self.indexed_qualified_builtin_fallback(
                &module_path,
                type_name,
                args_len,
                current_module,
                owner,
                unique,
            );
        };
        let Some(identity) = self.source_identity_from_import_modules(type_name, modules) else {
            return Some(None);
        };
        self.select_indexed_qualified_descriptor(&identity, args_len, unique)
    }

    fn indexed_qualified_builtin_fallback<'a>(
        &'a self,
        module_path: &str,
        type_name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        owner: &ResolvedImportOwner,
        unique: bool,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let normalized = normalized_qualified_path(module_path);
        if normalized != module_path
            && (current_module.is_some_and(|current_module| {
                normalized_qualified_path(current_module) == normalized
            }) || self
                .qualified_import_recovery_modules
                .contains_key(&(owner.clone(), normalized)))
        {
            return None;
        }
        let mut builtin = self
            .descriptors_by_source_identity
            .get(&(None, None, type_name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .filter(|descriptor| Self::descriptor_matches_arity(descriptor, args_len));
        Some(if unique {
            Self::unique_candidate(builtin).flatten()
        } else {
            builtin.next_back()
        })
    }

    fn select_indexed_qualified_descriptor<'a>(
        &'a self,
        identity: &QualifiedSourceIdentity,
        args_len: Option<usize>,
        unique: bool,
    ) -> Option<Option<&'a AdtDescriptor>> {
        if self
            .qualified_descriptor_candidates(identity)
            .any(|descriptor| {
                descriptor.visibility != Visibility::Public
                    && Self::descriptor_matches_arity(descriptor, args_len)
            })
        {
            return None;
        }
        let mut candidates = self
            .qualified_descriptor_candidates(identity)
            .filter(|descriptor| {
                descriptor.visibility == Visibility::Public
                    && Self::descriptor_matches_arity(descriptor, args_len)
                    && !self.descriptor_is_quarantined(descriptor)
            });
        Some(if unique {
            Self::unique_candidate(candidates).flatten()
        } else {
            candidates.next_back()
        })
    }
}
