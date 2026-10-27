use super::*;

impl AdtRegistry {
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
        if descriptor.module_name.as_deref() == current_module {
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
        if let Some(module_name) = self.indexed_import_module(first, current_module, uses) {
            return module_name
                .is_some_and(|module_name| descriptor.module_name.as_deref() == Some(module_name));
        }
        if self.source_import_count != uses.len()
            && let Some(module_name) =
                crate::name_recovery::select_use_decl_for_path(uses.iter(), first, current_module)
                    .map(|use_decl| use_decl.name.as_str())
        {
            return descriptor.module_name.as_deref() == Some(module_name);
        }
        if imported_descriptor_path_matches(descriptor, segments, current_module, uses) {
            return true;
        }
        self.descriptor_has_bare_import(descriptor, segments, current_module, uses)
    }

    fn indexed_import_module(
        &self,
        qualifier: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<Option<&str>> {
        let owner = import_owner_for_module(uses, current_module)?;
        let modules = self
            .qualified_import_modules
            .get(&(owner, qualifier.to_string()))?;
        let candidates = if modules.exact.is_empty() {
            &modules.implicit
        } else {
            &modules.exact
        };
        let mut candidates = candidates.iter();
        let module_name = candidates.next().map(String::as_str);
        Some(module_name.filter(|_| candidates.next().is_none()))
    }

    fn descriptor_has_bare_import(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        (segments.len() == 1
            || (segments.len() == 2 && segments.first() == Some(&descriptor.type_name)))
            && uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == current_module
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                    && crate::name_recovery::use_decl_contributes_bare_exports(use_decl)
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

    pub(super) fn companion_private_access_allowed(
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

fn imported_descriptor_path_matches(
    descriptor: &AdtDescriptor,
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    let Some(type_index) = descriptor_type_segment_index(descriptor, segments) else {
        return false;
    };
    crate::adt::lookup_validation::imported_module_path_matches(
        descriptor,
        &segments[..type_index],
        uses,
        current_module,
    )
}

fn descriptor_type_segment_index(descriptor: &AdtDescriptor, segments: &[String]) -> Option<usize> {
    segments
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, segment)| (segment == &descriptor.type_name).then_some(index))
        .filter(|index| *index > 0)
}
