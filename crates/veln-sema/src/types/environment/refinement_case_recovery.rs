use super::*;
use crate::types::type_case_recovery::recover_type_case;

impl TypeEnvironment {
    pub(super) fn variant_refinement_variant_case_recovery<'a>(
        &'a self,
        descriptor: &'a crate::adt::descriptors::AdtDescriptor,
        variant: &str,
    ) -> Option<crate::adt::registry::VariantLookup<'a>> {
        let recovered = recover_variant_case(variant)?;
        Some(self.adts.variant_for_descriptor(descriptor, &recovered))
    }

    pub(crate) fn recovered_variant_refinement_has_ambiguous_final(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        if segments.len() < 2 {
            return false;
        }
        let Some(recovered) = recover_type_case(&segments[..segments.len() - 1].join("::")) else {
            return false;
        };
        self.variant_refinement_ambiguous_recovery_descriptor(
            &recovered,
            0,
            segments.last().expect("checked final segment"),
            current_module,
        )
        .is_some()
    }

    pub(crate) fn recovered_variant_refinement_candidate(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        if segments.len() < 2 {
            return false;
        }
        let base = segments[..segments.len() - 1].join("::");
        if self
            .adts
            .descriptor_for_type_path_any_arity(&base, current_module, &self.uses)
            .is_some()
        {
            return false;
        }
        let Some(recovered) = recover_type_case(&base) else {
            return false;
        };
        self.variant_refinement_base_name_resolves(&recovered, current_module)
    }

    pub(crate) fn variant_refinement_syntax_candidate_is_selected(
        &self,
        base_segments: &[String],
        type_argument_count: usize,
        variant: &str,
        current_module: Option<&str>,
    ) -> bool {
        if type_argument_count == 0
            && self
                .adts
                .descriptor_for_type_path_any_arity(
                    &base_segments.join("::"),
                    current_module,
                    &self.uses,
                )
                .is_none()
            && self.variant_refinement_base_is_module_qualifier(base_segments, current_module)
        {
            return false;
        }
        let latent_non_generic = type_argument_count == 0
            && base_segments
                .last()
                .and_then(|segment| segment.as_bytes().first())
                .is_some_and(u8::is_ascii_lowercase);
        if !latent_non_generic {
            return true;
        }
        let mut segments = base_segments.to_vec();
        segments.push(variant.to_string());
        self.recovered_variant_refinement_candidate(&segments, current_module)
    }

    pub(crate) fn variant_refinement_base_is_module_qualifier(
        &self,
        base_segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        let [base] = base_segments else {
            return false;
        };
        let recovered = lowercase_initial(base);
        current_module.is_some_and(|module| module == base || module == recovered)
            || self
                .uses
                .iter()
                .chain(&self.quarantined_uses)
                .any(|use_decl| {
                    if use_decl.module_name.as_deref() != current_module {
                        return false;
                    }
                    let leaf = use_decl
                        .name
                        .rsplit("::")
                        .next()
                        .unwrap_or(use_decl.name.as_str());
                    use_decl.alias == *base
                        || use_decl.alias == recovered
                        || leaf == base
                        || leaf == recovered
                })
    }

    pub(crate) fn recovered_variant_refinement_base_resolves(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        recover_type_case(base).is_some_and(|recovered| {
            self.variant_refinement_base_name_resolves(&recovered, current_module)
        })
    }

    pub(crate) fn variant_refinement_base_is_unresolved(
        &self,
        base: &str,
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        if !base.contains("::") && type_parameters.ordinal(base).is_some() {
            return false;
        }
        if self.variant_refinement_base_has_ambiguous_module_qualifier(base, current_module) {
            return false;
        }
        if self
            .adts
            .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
            .is_some()
            || self.variant_refinement_base_name_resolves(base, current_module)
        {
            return false;
        }
        let Some(recovered) = recover_type_case(base) else {
            return true;
        };
        self.adts
            .descriptor_for_type_path_any_arity(&recovered, current_module, &self.uses)
            .is_none()
            && !self.variant_refinement_base_name_resolves(&recovered, current_module)
    }

    pub(crate) fn variant_refinement_base_has_unavailable_declared_identity(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        !base.contains("::")
            && self
                .type_symbols
                .iter()
                .filter(|symbol| {
                    symbol.name == base && symbol.module_name.as_deref() == current_module
                })
                .take(2)
                .count()
                == 1
            && self
                .adts
                .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
                .is_none()
    }

    pub(crate) fn variant_refinement_base_has_ambiguous_module_qualifier(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        let Some((qualifier, _)) = base.rsplit_once("::") else {
            return false;
        };
        self.uses
            .iter()
            .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
            .filter(|use_decl| {
                use_decl.alias == qualifier
                    || use_decl.name == qualifier
                    || use_decl
                        .name
                        .rsplit("::")
                        .next()
                        .is_some_and(|leaf| leaf == qualifier)
            })
            .take(2)
            .count()
            > 1
    }

    fn variant_refinement_base_name_resolves(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        if self
            .adts
            .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
            .is_some()
        {
            return self
                .adts
                .unique_descriptor_for_type_path_any_arity(base, current_module, &self.uses)
                .is_some();
        }
        if !base.contains("::")
            && self
                .type_symbols
                .iter()
                .filter(|symbol| {
                    symbol.name == base && symbol.module_name.as_deref() == current_module
                })
                .take(2)
                .count()
                == 1
        {
            return true;
        }
        crate::source_less_lookup::with_builtin_type_syntax_registry(|registry| {
            registry.arity(base).is_some()
        })
        .unwrap_or(false)
            || matches!(base, "WallTime" | "SourceLocation")
    }
}

fn recover_variant_case(name: &str) -> Option<String> {
    let (first, rest) = name.split_at_checked(1)?;
    let first = first.as_bytes()[0];
    first
        .is_ascii_lowercase()
        .then(|| (first as char).to_ascii_uppercase().to_string() + rest)
}

fn lowercase_initial(name: &str) -> String {
    let Some((_, first)) = name.char_indices().next() else {
        return String::new();
    };
    let rest = &name[first.len_utf8()..];
    first.to_ascii_lowercase().to_string() + rest
}
