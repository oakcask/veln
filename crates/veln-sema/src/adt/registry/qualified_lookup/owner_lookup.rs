use super::*;

impl AdtRegistry {
    fn fallback_descriptor_for_unqualified_type_path<'a>(
        &'a self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        owner_uses: &[UseDecl],
    ) -> Option<&'a AdtDescriptor> {
        self.root_descriptor(name, args_len)
            .or_else(|| self.unique_imported_descriptor(name, args_len, current_module, owner_uses))
    }

    fn fallback_unique_descriptor_for_unqualified_type_path<'a>(
        &'a self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        owner_uses: &[UseDecl],
    ) -> Option<&'a AdtDescriptor> {
        let root = self
            .descriptors_by_source_identity
            .get(&(None, None, name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .filter(|descriptor| Self::descriptor_matches_arity(descriptor, args_len));
        if let Some(root) = Self::unique_candidate(root) {
            return root;
        }
        self.unique_imported_descriptor(name, args_len, current_module, owner_uses)
    }

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
        if qualifier.is_some() {
            if let Some(descriptor) = self.indexed_exact_imported_descriptor_in_owner(
                name,
                Some(args_len),
                current_module,
                current_package,
                true,
            ) {
                return descriptor;
            }
        }
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
        if !name.contains("::") {
            return self.fallback_unique_descriptor_for_unqualified_type_path(
                name,
                Some(args_len),
                current_module,
                &owner_uses,
            );
        }
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
        if qualifier.is_some() {
            if let Some(descriptor) = self.indexed_exact_imported_descriptor_in_owner(
                name,
                None,
                current_module,
                current_package,
                true,
            ) {
                return descriptor;
            }
        }
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
        if !name.contains("::") {
            return self.fallback_unique_descriptor_for_unqualified_type_path(
                name,
                None,
                current_module,
                &owner_uses,
            );
        }
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
        let (qualifier, type_name) = name
            .rsplit_once("::")
            .map_or((None, name), |(qualifier, type_name)| {
                (Some(qualifier), type_name)
            });
        if qualifier.is_some() {
            if let Some(descriptor) = self
                .indexed_exact_imported_descriptor_including_quarantined_in_owner(
                    name,
                    args_len,
                    current_module,
                    current_package,
                )
            {
                return descriptor;
            }
        }
        if qualifier.is_none() || qualifier == current_module {
            let local = self
                .owner_local_descriptors(type_name, current_module, current_package)
                .find(|descriptor| Self::descriptor_matches_arity(descriptor, Some(args_len)));
            if qualifier.is_some() || local.is_some() {
                return local;
            }
        }
        if let Some(descriptor) = self.indexed_imported_descriptor_including_quarantined_in_owner(
            name,
            args_len,
            current_module,
            current_package,
        ) {
            return descriptor;
        }
        let owner_uses = Self::owner_uses(uses, current_module, current_package);
        if qualifier.is_none() {
            return self.fallback_descriptor_for_unqualified_type_path(
                name,
                Some(args_len),
                current_module,
                &owner_uses,
            );
        }
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
        if qualifier.is_some() {
            if let Some(descriptor) = self.indexed_exact_imported_descriptor_in_owner(
                name,
                Some(args_len),
                current_module,
                current_package,
                false,
            ) {
                return descriptor;
            }
        }
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
        if !name.contains("::") {
            return self.fallback_descriptor_for_unqualified_type_path(
                name,
                Some(args_len),
                current_module,
                &owner_uses,
            );
        }
        self.descriptor_for_type_path(name, args_len, current_module, &owner_uses)
    }
}
