use super::*;

impl AdtRegistry {
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
        let indices = self
            .descriptors_by_type_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let descriptors = |local| {
            indices
                .iter()
                .copied()
                .filter(move |index| (*index >= self.local_descriptor_start) == local)
                .map(|index| self.descriptor_lookup_candidate(index))
        };
        let local = descriptors(true)
            .filter(|descriptor| descriptor.module_name.as_deref() == current_module);
        let builtin = descriptors(false).filter(|descriptor| descriptor.module_name.is_none());
        let imported = indices
            .iter()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .filter(|descriptor| {
                descriptor.visibility == Visibility::Public
                    && descriptor
                        .module_name
                        .as_ref()
                        .is_some_and(|module| uses.iter().any(|use_decl| &use_decl.name == module))
            });
        Self::unique_candidate(local)
            .or_else(|| Self::unique_candidate(builtin))
            .or_else(|| Self::unique_candidate(imported))
            .flatten()
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
        if current_module == Some(module_path.as_str())
            && self.source_identity_exists(current_module, type_name)
        {
            return Some(QualifiedSourceIdentity::Source((
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
        let indexed = self
            .qualified_import_modules
            .get(&(current_module.map(str::to_string), module_path.to_string()));
        let fallback;
        let modules = if let Some(indexed) = indexed {
            indexed
        } else if self.qualified_import_modules.is_empty() {
            fallback = uses
                .iter()
                .filter_map(|use_decl| {
                    crate::adt::lookup_validation::record_import_path_comparison();
                    (crate::name_recovery::use_decl_matches_import_path(
                        use_decl,
                        module_path,
                        current_module,
                    ) && use_decl.module_name.as_deref() == current_module)
                        .then(|| use_decl.name.clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            &fallback
        } else {
            return None;
        };
        let mut candidates = modules
            .iter()
            .filter(|module| self.source_identity_exists(Some(module), type_name));
        let module = candidates.next()?;
        if candidates.next().is_some() {
            return Some(QualifiedSourceIdentity::Ambiguous);
        }
        Some(QualifiedSourceIdentity::Source((
            Some(module.clone()),
            type_name.to_string(),
        )))
    }

    fn source_identity_exists(&self, module_name: Option<&str>, type_name: &str) -> bool {
        self.descriptors_by_source_identity
            .contains_key(&(module_name.map(str::to_string), type_name.to_string()))
    }

    fn recovered_qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let mut recovered = segments.to_vec();
        let mut selected = None;
        for index in 0..segments.len().saturating_sub(1) {
            let Some(first) = segments[index].as_bytes().first().copied() else {
                continue;
            };
            if !first.is_ascii_uppercase() {
                continue;
            }
            recovered[index].replace_range(..1, &(first as char).to_ascii_lowercase().to_string());
            if let Some(identity) =
                self.exact_qualified_source_identity(type_name, &recovered, current_module, uses)
            {
                if identity == QualifiedSourceIdentity::Ambiguous
                    || selected
                        .as_ref()
                        .is_some_and(|selected| selected != &identity)
                {
                    return Some(QualifiedSourceIdentity::Ambiguous);
                }
                selected = Some(identity);
            }
            recovered[index] = segments[index].clone();
        }
        selected
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
                .get(&(None, type_name.clone()))
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
            QualifiedSourceIdentity::Source((module_name, _)) => {
                if module_name.as_deref() == current_module {
                    return true;
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

    pub(super) fn descriptor_visible(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> bool {
        if descriptor.module_name.is_none() {
            return true;
        }
        let same_module = descriptor.module_name.as_deref() == current_module;
        if same_module {
            return true;
        }
        if !include_imports {
            return false;
        }
        if descriptor.visibility != Visibility::Public {
            return self.companion_private_access_allowed(
                descriptor,
                segments,
                current_module,
                uses,
            );
        }
        let Some(first) = segments.first() else {
            return false;
        };
        if let Some(module_name) = uses
            .iter()
            .find(|use_decl| {
                use_decl.module_name.as_deref() == current_module && use_decl.alias == *first
            })
            .map(|use_decl| use_decl.name.as_str())
        {
            return descriptor.module_name.as_deref() == Some(module_name);
        }
        if imported_descriptor_path_matches(descriptor, segments, current_module, uses) {
            return true;
        }
        segments.len() <= 2
            && uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == current_module
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
            })
    }

    pub(super) fn variant_visible(
        &self,
        descriptor: &AdtDescriptor,
        variant: &AdtVariantDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
        segments: &[String],
    ) -> bool {
        if descriptor.module_name.is_none() || descriptor.module_name.as_deref() == current_module {
            return true;
        }
        if self.companion_private_access_allowed(descriptor, segments, current_module, uses) {
            return true;
        }
        if segments.len() > 2 {
            return variant.visibility == Visibility::Public
                && descriptor.visibility == Visibility::Public;
        }
        variant.visibility == Visibility::Public
    }

    fn companion_private_access_allowed(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        let Some(current_module) = current_module else {
            return false;
        };
        let Some(target_module) = descriptor.module_name.as_deref() else {
            return false;
        };
        if !self
            .companion_access_targets
            .get(current_module)
            .is_some_and(|allowed| allowed == target_module)
        {
            return false;
        }
        let Some(first) = segments.first() else {
            return false;
        };
        uses.iter().any(|use_decl| {
            use_decl.package.is_none()
                && use_decl.module_name.as_deref() == Some(current_module)
                && use_decl.alias == *first
                && use_decl.name == target_module
        })
    }
}
