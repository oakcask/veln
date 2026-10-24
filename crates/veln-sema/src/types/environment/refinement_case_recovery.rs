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
    ) -> bool {
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
