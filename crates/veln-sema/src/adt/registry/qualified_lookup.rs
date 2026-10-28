use super::*;

mod recovery_helpers;

use recovery_helpers::{
    fallback_recovery_modules, is_single_initial_recovery, merge_recovered_identity,
    normalized_qualified_path, normalized_qualified_segments,
};

impl AdtRegistry {
    fn owner_uses(
        uses: &[UseDecl],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Vec<UseDecl> {
        uses.iter()
            .filter(|use_decl| {
                use_decl.module_name.as_deref() == current_module
                    && use_decl.importing_package.as_deref() == current_package
            })
            .cloned()
            .collect()
    }

    fn indexed_imported_descriptor_in_owner(
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
            if let Some(descriptor) = self
                .descriptors_by_source_identity
                .get(&(None, None, name.to_string()))
                .into_iter()
                .flatten()
                .map(|index| self.descriptor_lookup_candidate(*index))
                .find(|descriptor| Self::descriptor_matches_arity(descriptor, args_len))
            {
                return Some(Some(descriptor));
            }
            let index = if let Some(args_len) = args_len {
                self.imported_descriptors_by_type_arity
                    .get(&(owner, name.to_string(), args_len))
            } else {
                self.imported_descriptors_by_type
                    .get(&(owner, name.to_string()))
            }
            .copied()
            .flatten();
            return Some(index.map(|index| self.descriptor_lookup_candidate(index)));
        }

        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let module_path = segments[..segments.len() - 1].join("::");
        crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
        let Some(modules) = self
            .qualified_import_modules
            .get(&(owner.clone(), module_path.clone()))
        else {
            let normalized = normalized_qualified_path(&module_path);
            if normalized != module_path
                && (current_module.is_some_and(|current_module| {
                    normalized_qualified_path(current_module) == normalized
                }) || self
                    .qualified_import_recovery_modules
                    .contains_key(&(owner, normalized)))
            {
                return None;
            }
            let mut builtin = self
                .descriptors_by_source_identity
                .get(&(None, None, type_name.clone()))
                .into_iter()
                .flatten()
                .map(|index| self.descriptor_lookup_candidate(*index))
                .filter(|descriptor| Self::descriptor_matches_arity(descriptor, args_len));
            return Some(if unique {
                Self::unique_candidate(builtin).flatten()
            } else {
                builtin.next_back()
            });
        };
        let Some(identity) = self.source_identity_from_import_modules(type_name, modules) else {
            return Some(None);
        };
        if self
            .qualified_descriptor_candidates(&identity)
            .any(|descriptor| {
                descriptor.visibility != Visibility::Public
                    && Self::descriptor_matches_arity(descriptor, args_len)
            })
        {
            return None;
        }
        let mut candidates = self
            .qualified_descriptor_candidates(&identity)
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

    fn owner_local_descriptors<'a>(
        &'a self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> impl Iterator<Item = &'a AdtDescriptor> {
        self.descriptors_by_source_identity
            .get(&(
                current_package.map(str::to_string),
                current_module.map(str::to_string),
                name.to_string(),
            ))
            .into_iter()
            .flatten()
            .rev()
            .map(|index| self.descriptor_lookup_candidate(*index))
    }

    pub(crate) fn unique_descriptor_for_type_path_in_owner(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let (qualifier, type_name) = name
            .rsplit_once("::")
            .map_or((None, name), |(qualifier, type_name)| {
                (Some(qualifier), type_name)
            });
        if qualifier.is_none() || qualifier == current_module {
            let local = Self::unique_candidate(
                self.owner_local_descriptors(type_name, current_module, current_package)
                    .filter(|descriptor| {
                        Self::descriptor_matches_arity(descriptor, Some(args_len))
                    }),
            );
            if qualifier.is_some() || local.is_some() {
                return local.flatten();
            }
        }
        if let Some(descriptor) = self.indexed_imported_descriptor_in_owner(
            name,
            Some(args_len),
            current_module,
            current_package,
            true,
        ) {
            return descriptor;
        }
        let owner_uses = Self::owner_uses(uses, current_module, current_package);
        self.unique_descriptor_for_type_path(name, args_len, current_module, &owner_uses)
    }

    pub(crate) fn unique_descriptor_for_type_path_any_arity_in_owner(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let (qualifier, type_name) = name
            .rsplit_once("::")
            .map_or((None, name), |(qualifier, type_name)| {
                (Some(qualifier), type_name)
            });
        if qualifier.is_none() || qualifier == current_module {
            let local = Self::unique_candidate(self.owner_local_descriptors(
                type_name,
                current_module,
                current_package,
            ));
            if qualifier.is_some() || local.is_some() {
                return local.flatten();
            }
        }
        if let Some(descriptor) = self.indexed_imported_descriptor_in_owner(
            name,
            None,
            current_module,
            current_package,
            true,
        ) {
            return descriptor;
        }
        let owner_uses = Self::owner_uses(uses, current_module, current_package);
        self.unique_descriptor_for_type_path_any_arity(name, current_module, &owner_uses)
    }

    pub(crate) fn descriptor_for_type_path_including_quarantined_in_owner(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let segments = name.split("::").collect::<Vec<_>>();
        let type_name = segments.last()?;
        let qualifier = segments[..segments.len().saturating_sub(1)].join("::");
        if (!name.contains("::") || current_module == Some(qualifier.as_str()))
            && let Some(descriptor) = self
                .descriptors_by_source_identity
                .get(&(
                    current_package.map(str::to_string),
                    current_module.map(str::to_string),
                    (*type_name).to_string(),
                ))
                .into_iter()
                .flatten()
                .rev()
                .map(|index| self.descriptor_lookup_candidate(*index))
                .find(|descriptor| Self::descriptor_matches_arity(descriptor, Some(args_len)))
        {
            return Some(descriptor);
        }
        let owner_uses = Self::owner_uses(uses, current_module, current_package);
        self.descriptor_for_type_path_including_quarantined(
            name,
            args_len,
            current_module,
            &owner_uses,
        )
    }

    pub(crate) fn descriptor_for_type_path_in_owner(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let (qualifier, type_name) = name
            .rsplit_once("::")
            .map_or((None, name), |(qualifier, type_name)| {
                (Some(qualifier), type_name)
            });
        if qualifier.is_none() || qualifier == current_module {
            if let Some(descriptor) = self
                .owner_local_descriptors(type_name, current_module, current_package)
                .find(|descriptor| {
                    Self::descriptor_matches_arity(descriptor, Some(args_len))
                        && !self.descriptor_is_quarantined(descriptor)
                })
            {
                return Some(descriptor);
            }
            if qualifier.is_some() {
                return None;
            }
        }
        if let Some(descriptor) = self.indexed_imported_descriptor_in_owner(
            name,
            Some(args_len),
            current_module,
            current_package,
            false,
        ) {
            return descriptor;
        }
        let owner_uses = Self::owner_uses(uses, current_module, current_package);
        self.descriptor_for_type_path(name, args_len, current_module, &owner_uses)
    }

    pub(crate) fn descriptor_for_type_path_including_quarantined(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if !name.contains("::") {
            return self.descriptor_for_type_path(name, args_len, current_module, uses);
        }
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let identity = self.qualified_source_identity(type_name, &segments, current_module, uses);
        self.qualified_descriptor_candidates(&identity)
            .rev()
            .find(|descriptor| Self::descriptor_matches_arity(descriptor, Some(args_len)))
    }

    pub(crate) fn unique_descriptor_for_type_path_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if name.contains("::") {
            self.unique_qualified_descriptor_any_arity(name, current_module, uses)
        } else {
            self.unique_unqualified_descriptor_any_arity(name, current_module, uses)
        }
    }

    fn unique_unqualified_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.unique_local_descriptor_any_arity(name, current_module, uses)
            .or_else(|| self.unique_builtin_descriptor_any_arity(name))
            .or_else(|| self.unique_imported_descriptor_any_arity(name, current_module, uses))
            .flatten()
    }

    fn unique_local_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<Option<&AdtDescriptor>> {
        let owner = import_owner_for_module(uses, current_module);
        let local = self
            .descriptors_by_source_identity
            .iter()
            .filter(|((package, module, type_name), _)| {
                module.as_deref() == current_module
                    && type_name == name
                    && owner.as_ref().is_none_or(|owner| package == &owner.package)
            })
            .flat_map(|(_, indices)| indices.iter().copied())
            .filter(|index| *index >= self.local_descriptor_start)
            .map(|index| self.descriptor_lookup_candidate(index));
        Self::unique_candidate(local)
    }

    fn unique_builtin_descriptor_any_arity(&self, name: &str) -> Option<Option<&AdtDescriptor>> {
        let builtin_indices = self
            .descriptors_by_source_identity
            .get(&(None, None, name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or_default();
        let builtin = builtin_indices
            .iter()
            .copied()
            .filter(|index| *index < self.local_descriptor_start)
            .map(|index| self.descriptor_lookup_candidate(index));
        Self::unique_candidate(builtin)
    }

    fn unique_imported_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<Option<&AdtDescriptor>> {
        if self.source_import_indexes_ready {
            crate::adt::lookup_validation::record_import_path_comparison();
            let owner = import_owner_for_module(uses, current_module)?;
            return Some(
                self.imported_descriptors_by_type
                    .get(&(owner, name.to_string()))
                    .copied()
                    .flatten()
                    .map(|index| self.descriptor_lookup_candidate(index)),
            );
        }
        let imported = self
            .descriptors_by_type_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .filter(|descriptor| {
                Self::descriptor_is_unqualified_import(descriptor, current_module, uses)
            });
        Self::unique_candidate(imported)
    }

    fn unique_qualified_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let identity = self.qualified_source_identity(type_name, &segments, current_module, uses);
        self.unique_qualified_descriptor_for_identity(
            &identity,
            None,
            &segments,
            current_module,
            uses,
        )
    }

    pub(super) fn qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> QualifiedSourceIdentity {
        self.exact_qualified_source_identity(type_name, segments, current_module, uses)
            .unwrap_or_else(|| {
                self.recovered_qualified_source_identity(type_name, segments, current_module, uses)
                    .unwrap_or_else(|| {
                        QualifiedSourceIdentity::CompilerKnown(type_name.to_string())
                    })
            })
    }

    fn exact_qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let module_path = segments[..segments.len().saturating_sub(1)].join("::");
        crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
        let current_package =
            import_owner_for_module(uses, current_module).and_then(|owner| owner.package);
        if current_module == Some(module_path.as_str())
            && self.source_identity_exists(current_package.as_deref(), current_module, type_name)
        {
            return Some(QualifiedSourceIdentity::Source((
                current_package,
                current_module.map(str::to_string),
                type_name.to_string(),
            )));
        }
        self.exact_imported_source_identity(type_name, &module_path, current_module, uses)
    }

    fn exact_imported_source_identity(
        &self,
        type_name: &str,
        module_path: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let owner = import_owner_for_module(uses, current_module);
        let indexed = owner.as_ref().and_then(|owner| {
            self.qualified_import_modules
                .get(&(owner.clone(), module_path.to_string()))
        });
        let fallback;
        let modules = if let Some(indexed) = indexed {
            indexed
        } else if self.source_import_count != uses.len() {
            fallback = fallback_exact_import_modules(module_path, current_module, uses);
            &fallback
        } else {
            return None;
        };
        self.source_identity_from_import_modules(type_name, modules)
    }

    fn source_identity_exists(
        &self,
        package_name: Option<&str>,
        module_name: Option<&str>,
        type_name: &str,
    ) -> bool {
        self.descriptors_by_source_identity.contains_key(&(
            package_name.map(str::to_string),
            module_name.map(str::to_string),
            type_name.to_string(),
        ))
    }

    fn recovered_qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let qualifiers = &segments[..segments.len().saturating_sub(1)];
        let normalized = normalized_qualified_segments(qualifiers);
        crate::adt::lookup_validation::record_qualified_path_work(normalized.len());
        let mut selected = self.recovered_current_module_identity(
            type_name,
            qualifiers,
            &normalized,
            current_module,
            uses,
        );

        let Some(owner) = import_owner_for_module(uses, current_module) else {
            return selected;
        };
        let key = (owner, normalized);
        let Some(candidates) = self.qualified_import_recovery_modules.get(&key) else {
            if self.source_import_count == uses.len() {
                return selected;
            }
            return self.fallback_recovered_import_identity(
                type_name,
                qualifiers,
                current_module,
                uses,
                selected,
            );
        };

        for candidate in candidates {
            crate::adt::lookup_validation::record_qualified_path_work(candidate.module_path.len());
            if !is_single_initial_recovery(qualifiers, &candidate.module_path)
                || self.is_current_module_identity(
                    type_name,
                    current_module,
                    &candidate.module_path,
                    uses,
                )
            {
                continue;
            }
            let identity = self.source_identity_from_import_modules(type_name, &candidate.modules);
            if merge_recovered_identity(&mut selected, identity) {
                return Some(QualifiedSourceIdentity::Ambiguous);
            }
        }
        selected
    }

    fn recovered_current_module_identity(
        &self,
        type_name: &str,
        qualifiers: &[String],
        normalized: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let module_path = current_module.filter(|module_path| {
            crate::adt::lookup_validation::record_qualified_path_work(module_path.len());
            normalized_qualified_path(module_path) == normalized
                && is_single_initial_recovery(qualifiers, module_path)
        })?;
        let package = import_owner_for_module(uses, current_module).and_then(|owner| owner.package);
        self.source_identity_exists(package.as_deref(), Some(module_path), type_name)
            .then(|| {
                QualifiedSourceIdentity::Source((
                    package,
                    Some(module_path.to_string()),
                    type_name.to_string(),
                ))
            })
    }

    fn fallback_recovered_import_identity(
        &self,
        type_name: &str,
        qualifiers: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        mut selected: Option<QualifiedSourceIdentity>,
    ) -> Option<QualifiedSourceIdentity> {
        for (module_path, modules) in fallback_recovery_modules(qualifiers, current_module, uses) {
            if self.is_current_module_identity(type_name, current_module, &module_path, uses) {
                continue;
            }
            let identity = self.source_identity_from_import_modules(type_name, &modules);
            if merge_recovered_identity(&mut selected, identity) {
                return Some(QualifiedSourceIdentity::Ambiguous);
            }
        }
        selected
    }

    fn is_current_module_identity(
        &self,
        type_name: &str,
        current_module: Option<&str>,
        module_path: &str,
        uses: &[UseDecl],
    ) -> bool {
        current_module == Some(module_path)
            && self.source_identity_exists(
                import_owner_for_module(uses, current_module)
                    .and_then(|owner| owner.package)
                    .as_deref(),
                current_module,
                type_name,
            )
    }

    fn source_identity_from_import_modules(
        &self,
        type_name: &str,
        modules: &QualifiedImportModules,
    ) -> Option<QualifiedSourceIdentity> {
        let candidates = if modules.exact.is_empty() {
            &modules.implicit
        } else {
            &modules.exact
        };
        let mut candidates = candidates.iter();
        let (package, module) = candidates.next()?;
        if candidates.next().is_some() {
            return Some(QualifiedSourceIdentity::Ambiguous);
        }
        if !self.source_identity_exists(package.as_deref(), Some(module), type_name) {
            return None;
        }
        Some(QualifiedSourceIdentity::Source((
            package.clone(),
            Some(module.clone()),
            type_name.to_string(),
        )))
    }

    pub(super) fn unique_qualified_descriptor_for_identity(
        &self,
        identity: &QualifiedSourceIdentity,
        args_len: Option<usize>,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let candidates = self
            .qualified_descriptor_candidates(identity)
            .filter(|descriptor| {
                Self::descriptor_matches_arity(descriptor, args_len)
                    && self.qualified_descriptor_visible(
                        descriptor,
                        identity,
                        segments,
                        current_module,
                        uses,
                    )
            });
        Self::unique_candidate(candidates).flatten()
    }

    fn qualified_descriptor_candidates<'a>(
        &'a self,
        identity: &QualifiedSourceIdentity,
    ) -> impl DoubleEndedIterator<Item = &'a AdtDescriptor> {
        let indices = match identity {
            QualifiedSourceIdentity::Source(identity) => self
                .descriptors_by_source_identity
                .get(identity)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            QualifiedSourceIdentity::CompilerKnown(type_name) => self
                .descriptors_by_source_identity
                .get(&(None, None, type_name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default(),
            QualifiedSourceIdentity::Ambiguous => &[],
        };
        indices
            .iter()
            .copied()
            .map(|index| self.descriptor_lookup_candidate(index))
    }

    fn qualified_descriptor_visible(
        &self,
        descriptor: &AdtDescriptor,
        identity: &QualifiedSourceIdentity,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        match identity {
            QualifiedSourceIdentity::Source((package_name, module_name, _)) => {
                if module_name.as_deref() == current_module {
                    let current_package = import_owner_for_module(uses, current_module)
                        .and_then(|owner| owner.package);
                    if package_name == &current_package {
                        return true;
                    }
                }
                descriptor.visibility == Visibility::Public
                    || self.companion_private_access_allowed(
                        descriptor,
                        segments,
                        current_module,
                        uses,
                    )
            }
            QualifiedSourceIdentity::CompilerKnown(_) => descriptor.module_name.is_none(),
            QualifiedSourceIdentity::Ambiguous => false,
        }
    }

    fn unique_candidate<'a>(
        mut candidates: impl Iterator<Item = &'a AdtDescriptor>,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let candidate = candidates.next()?;
        Some(candidates.next().is_none().then_some(candidate))
    }

    pub(crate) fn canonical_descriptor_for<'a>(
        &'a self,
        descriptor: &'a AdtDescriptor,
    ) -> &'a AdtDescriptor {
        self.descriptors_by_identity
            .get(&descriptor.identity())
            .into_iter()
            .flatten()
            .map(|index| &self.descriptors[*index])
            .find(|candidate| {
                candidate.nominal_identity.is_none()
                    && candidate.type_parameters.len() == descriptor.type_parameters.len()
            })
            .unwrap_or(descriptor)
    }

    pub(crate) fn canonical_type_name_for_descriptor(&self, descriptor: &AdtDescriptor) -> String {
        self.canonical_descriptor_for(descriptor).type_name.clone()
    }

    pub(super) fn descriptor_for_type_path_with_arity(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if !name.contains("::") {
            return self.descriptor_for_unqualified_type_path(name, args_len, current_module, uses);
        }
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let identity = self.qualified_source_identity(type_name, &segments, current_module, uses);
        self.qualified_descriptor_candidates(&identity)
            .rev()
            .find(|descriptor| {
                Self::descriptor_matches_arity(descriptor, args_len)
                    && self.qualified_descriptor_visible(
                        descriptor,
                        &identity,
                        &segments,
                        current_module,
                        uses,
                    )
            })
    }

    pub(crate) fn descriptor_for_core_type(&self, ty: &CoreType) -> Option<&AdtDescriptor> {
        let CoreType::Named { name, args } = ty else {
            return None;
        };
        self.descriptors_named(name).find(|descriptor| {
            descriptor.type_name == *name && descriptor.type_parameters.len() == args.len()
        })
    }
}

fn fallback_exact_import_modules(
    module_path: &str,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> QualifiedImportModules {
    let mut modules = QualifiedImportModules::default();
    for use_decl in uses {
        crate::adt::lookup_validation::record_import_path_comparison();
        if crate::name_recovery::use_decl_matches_exact_import_path(
            use_decl,
            module_path,
            current_module,
        ) {
            let target = use_decl.resolved_target();
            modules.exact.insert((target.package, target.module));
        }
        crate::adt::lookup_validation::record_import_path_comparison();
        if crate::name_recovery::use_decl_matches_implicit_import_alias(
            use_decl,
            module_path,
            current_module,
        ) {
            let target = use_decl.resolved_target();
            modules.implicit.insert((target.package, target.module));
        }
    }
    modules
}
