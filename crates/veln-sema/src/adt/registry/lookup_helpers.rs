use super::*;

impl AdtRegistry {
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
        if let Some(owner) = import_owner_for_module(uses, current_module) {
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
            let owner = import_owner_for_module(uses, current_module)?;
            let index = if let Some(args_len) = args_len {
                self.imported_descriptors_by_type_arity
                    .get(&(owner, name.to_string(), args_len))
            } else {
                self.imported_descriptors_by_type
                    .get(&(owner, name.to_string()))
            }
            .copied()
            .flatten()?;
            return Some(self.descriptor_lookup_candidate(index));
        }
        let mut imported = self.descriptors_named(name).filter(|descriptor| {
            Self::descriptor_matches_arity(descriptor, args_len)
                && Self::descriptor_is_unqualified_import(descriptor, current_module, uses)
        });
        let descriptor = imported.next()?;
        imported.next().is_none().then_some(descriptor)
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
