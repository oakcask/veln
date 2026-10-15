use super::{AdtDescriptor, AdtRegistry, UseDecl, Visibility};

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
        self.descriptor_in_current_module(name, args_len, current_module)
            .or_else(|| self.root_descriptor(name, args_len))
            .or_else(|| self.unique_imported_descriptor(name, args_len, uses))
    }

    pub(super) fn descriptor_in_current_module(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
    ) -> Option<&AdtDescriptor> {
        self.descriptors_named(name).rev().find(|descriptor| {
            descriptor.module_name.as_deref() == current_module
                && Self::descriptor_matches_arity(descriptor, args_len)
        })
    }

    pub(super) fn root_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
    ) -> Option<&AdtDescriptor> {
        self.descriptors_named(name).find(|descriptor| {
            descriptor.module_name.is_none() && Self::descriptor_matches_arity(descriptor, args_len)
        })
    }

    pub(super) fn unique_imported_descriptor(
        &self,
        name: &str,
        args_len: Option<usize>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let mut imported = self.descriptors_named(name).filter(|descriptor| {
            Self::descriptor_matches_arity(descriptor, args_len)
                && descriptor.visibility == Visibility::Public
                && descriptor
                    .module_name
                    .as_ref()
                    .is_some_and(|module| uses.iter().any(|use_decl| &use_decl.name == module))
        });
        let descriptor = imported.next()?;
        imported.next().is_none().then_some(descriptor)
    }
}
