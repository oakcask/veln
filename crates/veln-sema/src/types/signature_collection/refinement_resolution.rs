use std::collections::HashSet;

use crate::adt::descriptors::AdtDescriptor;
use crate::adt::registry::VariantDeclarationOrder;
use crate::semantic_model::Type;
use crate::type_relations::TypePresentationJoin;

pub(super) struct RefinementResolution {
    canonical_args: Vec<Type>,
    variants: Vec<String>,
    preferred_alias: Option<String>,
    alias_conflict: bool,
    argument_presentations: Vec<TypePresentationJoin>,
    selected_variants: HashSet<String>,
}

impl RefinementResolution {
    pub(super) fn new(
        descriptor: &AdtDescriptor,
        canonical_args: Vec<Type>,
        variants: Vec<String>,
    ) -> Self {
        let preferred_alias = descriptor
            .nominal_identity
            .is_some()
            .then(|| descriptor.type_name.clone());
        let argument_presentations = (0..canonical_args.len())
            .map(|_| TypePresentationJoin::default())
            .collect();
        let selected_variants = variants.iter().cloned().collect();
        Self {
            canonical_args,
            variants,
            preferred_alias,
            alias_conflict: false,
            argument_presentations,
            selected_variants,
        }
    }

    pub(super) fn merge_alternative(
        &mut self,
        descriptor: &AdtDescriptor,
        alternative_descriptor: &AdtDescriptor,
        alternative_args: Vec<Type>,
        variant: String,
        declaration_order: &VariantDeclarationOrder,
    ) -> bool {
        if alternative_descriptor.identity() != descriptor.identity()
            || !crate::type_relations::invariant_args_match(&self.canonical_args, &alternative_args)
            || declaration_order.rank(&variant).is_none()
        {
            return false;
        }
        for ((canonical_arg, presentation), alternative_arg) in self
            .canonical_args
            .iter_mut()
            .zip(&mut self.argument_presentations)
            .zip(&alternative_args)
        {
            presentation.merge(canonical_arg, alternative_arg);
        }
        self.merge_alias(alternative_descriptor);
        crate::type_relations::record_variant_set_lookup();
        if self.selected_variants.insert(variant.clone()) {
            self.variants.push(variant);
        }
        true
    }

    fn merge_alias(&mut self, descriptor: &AdtDescriptor) {
        let Some(alternative_alias) = descriptor
            .nominal_identity
            .is_some()
            .then_some(&descriptor.type_name)
        else {
            return;
        };
        match self.preferred_alias.as_deref() {
            Some(preferred) if preferred != alternative_alias => {
                self.preferred_alias = None;
                self.alias_conflict = true;
            }
            None if !self.alias_conflict => self.preferred_alias = Some(alternative_alias.clone()),
            _ => {}
        }
    }

    pub(super) fn finish(self, canonical_name: String) -> (Vec<String>, String, Vec<Type>) {
        (
            self.variants,
            self.preferred_alias.unwrap_or(canonical_name),
            self.canonical_args,
        )
    }
}
