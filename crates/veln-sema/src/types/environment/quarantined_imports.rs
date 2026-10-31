use super::*;
use crate::adt::descriptors::{AdtConstructor, AdtDescriptor};
use crate::adt::registry::ConstructorLookup;

impl TypeEnvironment {
    pub(crate) fn quarantined_import_call_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: usize,
    ) -> usize {
        self.quarantined_import_function_recovery_candidate_count(
            segments,
            current_module,
            current_package,
            Some(arg_count),
        )
    }

    fn quarantined_import_function_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: Option<usize>,
    ) -> usize {
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(segments, current_module, current_package)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.functions_named(name)
            .filter(|function| {
                function.module_name.as_deref() == Some(module_name)
                    && function.package_name == use_decl.resolved_target().package
                    && arg_count
                        .is_none_or(|count| function_signature_accepts_arg_count(function, count))
                    && function.visibility == Visibility::Public
            })
            .count()
    }

    pub(crate) fn quarantined_import_value_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> usize {
        self.quarantined_import_function_recovery_candidate_count(
            segments,
            current_module,
            current_package,
            None,
        )
    }

    pub(crate) fn quarantined_import_value_recovery_use(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<UseDecl> {
        let (use_decl, name) =
            self.quarantined_import_for_segments(segments, current_module, current_package)?;
        self.functions_named(name)
            .any(|function| {
                function.module_name.as_deref() == Some(use_decl.name.as_str())
                    && function.package_name == use_decl.resolved_target().package
                    && function.visibility == Visibility::Public
            })
            .then(|| use_decl.clone())
    }

    pub(crate) fn quarantined_import_type_path_lacks_visible_leaf(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        let Some(current_package) = self.unique_quarantined_import_owner_package(current_module)
        else {
            return false;
        };
        self.quarantined_import_type_path_lacks_visible_leaf_in_owner(
            segments,
            current_module,
            current_package,
        )
    }

    pub(crate) fn quarantined_import_type_path_lacks_visible_leaf_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> bool {
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(segments, current_module, current_package)
        else {
            return false;
        };
        let module_name = Some(use_decl.name.as_str());
        let target_package = use_decl.resolved_target().package;
        !self.type_symbols.iter().any(|symbol| {
            symbol.name == name
                && symbol.module_name.as_deref() == module_name
                && symbol.package_name == target_package
                && symbol.visibility() == Visibility::Public
        })
    }

    pub(crate) fn quarantined_import_type_path_uses_nested_alias(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        let Some(current_package) = self.unique_quarantined_import_owner_package(current_module)
        else {
            return false;
        };
        self.quarantined_import_for_segments(segments, current_module, current_package)
            .is_some_and(|(use_decl, _)| use_decl.name.contains("::"))
    }

    fn unique_quarantined_import_owner_package(
        &self,
        current_module: Option<&str>,
    ) -> Option<Option<&str>> {
        let mut selected = None;
        for use_decl in self
            .quarantined_uses
            .iter()
            .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        {
            let candidate = use_decl.importing_package.as_deref();
            match selected {
                None => selected = Some(candidate),
                Some(known) if known == candidate => {}
                Some(_) => return None,
            }
        }
        selected
    }

    pub(super) fn quarantined_import_effect_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> usize {
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(segments, current_module, current_package)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.effects
            .iter()
            .filter(|effect| {
                effect.name == name
                    && effect.module_name.as_deref() == Some(module_name)
                    && effect.package_name == use_decl.resolved_target().package
                    && effect.visibility == Visibility::Public
            })
            .count()
    }

    pub(super) fn quarantined_import_handler_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> usize {
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(segments, current_module, current_package)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.handlers
            .iter()
            .filter(|handler| {
                handler.name == name
                    && handler.module_name.as_deref() == Some(module_name)
                    && handler.package_name == use_decl.resolved_target().package
                    && handler.visibility == Visibility::Public
            })
            .count()
    }

    #[cfg(test)]
    pub(crate) fn quarantined_import_constructor_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> usize {
        self.quarantined_import_constructor_recovery_candidate_count_in_owner(
            segments,
            current_module,
            None,
            arg_count,
        )
    }

    pub(crate) fn quarantined_import_constructor_recovery_candidate_count_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        arg_count: Option<usize>,
    ) -> usize {
        let Some((use_decl, constructor_segments)) = self
            .quarantined_import_for_constructor_segments(segments, current_module, current_package)
        else {
            return 0;
        };
        self.import_constructor_recoveries
            .iter()
            .filter(|(key, _)| {
                key.current_module.as_deref() == current_module
                    && key.current_package == use_decl.resolved_owner().package
                    && key.target_package == use_decl.resolved_target().package
                    && key.target_module == use_decl.resolved_target().module
                    && key.alias == use_decl.alias
                    && key.constructor_segments == constructor_segments
                    && arg_count.is_none_or(|count| key.field_count == count)
            })
            .map(|(_, count)| *count)
            .sum()
    }

    #[cfg(test)]
    pub(crate) fn quarantined_explicit_import_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<AdtConstructor<'_>> {
        self.quarantined_explicit_import_constructor_in_owner(segments, current_module, None)
    }

    pub(crate) fn quarantined_explicit_import_constructor_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<AdtConstructor<'_>> {
        let (use_decl, constructor_segments) = self.quarantined_import_for_constructor_segments(
            segments,
            current_module,
            current_package,
        )?;
        if !use_decl.has_explicit_alias()
            || self.quarantined_import_constructor_recovery_candidate_count_in_owner(
                segments,
                current_module,
                current_package,
                None,
            ) != 1
        {
            return None;
        }
        match self.adts.public_constructor_in_module(
            use_decl.resolved_target().package.as_deref(),
            &use_decl.resolved_target().module,
            constructor_segments,
        ) {
            ConstructorLookup::Found(constructor) => Some(constructor),
            ConstructorLookup::Ambiguous | ConstructorLookup::Missing => None,
        }
    }

    pub(crate) fn quarantined_explicit_import_constructor_for_descriptor_in_owner(
        &self,
        segments: &[String],
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<AdtConstructor<'_>> {
        self.quarantined_explicit_import_constructor_in_owner(
            segments,
            current_module,
            current_package,
        )
        .filter(|constructor| constructor.descriptor.identity() == descriptor.identity())
    }

    #[cfg(test)]
    pub(crate) fn quarantined_import_type_recovery_candidate_count(
        &self,
        type_name: &str,
        current_module: Option<&str>,
        args_len: usize,
    ) -> usize {
        self.quarantined_import_type_recovery_candidate_count_in_owner(
            type_name,
            current_module,
            None,
            args_len,
        )
    }

    #[cfg(test)]
    pub(crate) fn quarantined_import_type_recovery_candidate_count_in_owner(
        &self,
        type_name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
        args_len: usize,
    ) -> usize {
        let segments = type_name
            .split("::")
            .map(str::to_string)
            .collect::<Vec<_>>();
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(&segments, current_module, current_package)
        else {
            return 0;
        };
        let module_name = Some(use_decl.name.as_str());
        let target_package = use_decl.resolved_target().package;
        self.type_symbols
            .iter()
            .filter(|symbol| {
                symbol.name == name
                    && symbol.module_name.as_deref() == module_name
                    && symbol.package_name == target_package
                    && self
                        .adts
                        .unique_public_descriptor_for_type_in_module(
                            name,
                            args_len,
                            target_package.as_deref(),
                            use_decl.name.as_str(),
                        )
                        .is_some()
            })
            .count()
    }

    fn quarantined_import_for_segments<'a>(
        &'a self,
        segments: &'a [String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<(&'a UseDecl, &'a str)> {
        match segments {
            [_, .., name] => imported_use_for_path_in_owner(
                &self.quarantined_uses,
                &segments[..segments.len() - 1],
                current_module,
                current_package,
            )
            .map(|use_decl| (use_decl, name.as_str())),
            _ => None,
        }
    }

    fn quarantined_import_for_constructor_segments<'a>(
        &'a self,
        segments: &'a [String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<(&'a UseDecl, &'a [String])> {
        if segments.len() < 2 {
            return None;
        }
        // Prefer the longest eligible module prefix. Once selected, a missing
        // member must not redirect lookup to a shorter qualifier.
        for constructor_len in [1, 2] {
            let Some(module_len) = segments.len().checked_sub(constructor_len) else {
                continue;
            };
            if module_len == 0 {
                continue;
            }
            if let Some(use_decl) = imported_use_for_path_in_owner(
                &self.quarantined_uses,
                &segments[..module_len],
                current_module,
                current_package,
            ) {
                return Some((use_decl, &segments[module_len..]));
            }
        }
        None
    }
}

fn function_signature_accepts_arg_count(function: &FunctionSignature, arg_count: usize) -> bool {
    if function.variadic.is_some() {
        arg_count >= function.params.len()
    } else {
        arg_count == function.params.len()
    }
}
