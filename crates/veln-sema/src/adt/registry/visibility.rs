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
            let owner = import_owner_for_module(uses, current_module);
            if owner
                .as_ref()
                .is_none_or(|owner| descriptor.package_name == owner.package)
            {
                return true;
            }
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
        let Some(import_path) = constructor_import_path(segments) else {
            return false;
        };
        if let Some(target) = self.indexed_import_module(&import_path, current_module, uses) {
            return target.is_some_and(|(package_name, module_name)| {
                descriptor.package_name.as_deref() == package_name
                    && descriptor.module_name.as_deref() == Some(module_name)
            });
        }
        if self.source_import_count != uses.len()
            && let Some(use_decl) = crate::name_recovery::select_use_decl_for_path(
                uses.iter(),
                &import_path,
                current_module,
            )
        {
            let target = use_decl.resolved_target();
            return descriptor.package_name == target.package
                && descriptor.module_name.as_deref() == Some(target.module.as_str());
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
    ) -> Option<Option<(Option<&str>, &str)>> {
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
        let target = candidates
            .next()
            .map(|(package, module)| (package.as_deref(), module.as_str()));
        Some(target.filter(|_| candidates.next().is_none()))
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
                    && {
                        let target = use_decl.resolved_target();
                        descriptor.module_name.as_deref() == Some(target.module.as_str())
                            && descriptor.package_name == target.package
                    }
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
        if descriptor.module_name.is_none()
            || (descriptor.module_name.as_deref() == current_module
                && import_owner_for_module(uses, current_module)
                    .as_ref()
                    .is_none_or(|owner| descriptor.package_name == owner.package))
        {
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
                && use_decl.resolved_target().module == target_module
        })
    }
}

fn constructor_import_path(segments: &[String]) -> Option<String> {
    match segments {
        [_, .., _, _] => Some(segments[..segments.len() - 2].join("::")),
        [first, ..] => Some(first.clone()),
        [] => None,
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
