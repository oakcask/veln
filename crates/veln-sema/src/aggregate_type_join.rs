use std::collections::HashSet;

use crate::adt::registry::AdtRegistry;
use crate::adt::unification;
use crate::semantic_model::Type;
use crate::type_relations::{invariant_args_match, same_type_identity};

pub(crate) struct AggregateTypeJoin {
    name: String,
    identity: String,
    args: Vec<Type>,
    declaration_order: Vec<String>,
    variants: Option<HashSet<String>>,
}

impl AggregateTypeJoin {
    pub(crate) fn new(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let initial = adt_type_parts(initial)?;
        let declaration_order = if initial.variants.is_some() {
            adts.descriptor_for_type(initial.ty)?
                .variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        record_work(declaration_order.len());
        let variants = initial.variants.map(|variants| {
            record_work(variants.len());
            variants.iter().cloned().collect()
        });
        Some(Self {
            name: initial.name.to_string(),
            identity: initial.identity.to_string(),
            args: initial.args.to_vec(),
            declaration_order,
            variants,
        })
    }

    pub(crate) fn try_join(&mut self, right: &Type) -> bool {
        let Some(right) = adt_type_parts(right) else {
            return false;
        };
        if !same_type_identity(&self.name, &self.identity, right.name, right.identity)
            || !invariant_args_match(&self.args, right.args)
        {
            return false;
        }
        for (joined, right) in self.args.iter_mut().zip(right.args) {
            unification::merge_type_slot(joined, right);
        }
        match (&mut self.variants, right.variants) {
            (Some(joined), Some(right)) => {
                record_work(right.len());
                joined.extend(right.iter().cloned());
            }
            (variants, None) => *variants = None,
            (None, Some(_)) => {}
        }
        true
    }

    pub(crate) fn inference_type(&self) -> Type {
        Type::resolved_named(self.name.clone(), self.identity.clone(), self.args.clone())
    }

    pub(crate) fn result_type(&self) -> Type {
        let Some(variants) = &self.variants else {
            return self.inference_type();
        };
        record_work(self.declaration_order.len());
        let variants = self
            .declaration_order
            .iter()
            .filter(|variant| variants.contains(*variant))
            .cloned()
            .collect::<Vec<_>>();
        if variants.len() == self.declaration_order.len() {
            self.inference_type()
        } else {
            Type::resolved_variant_refinement(
                self.name.clone(),
                self.identity.clone(),
                self.args.clone(),
                variants,
            )
        }
    }
}

struct AdtTypeParts<'a> {
    ty: &'a Type,
    name: &'a str,
    identity: &'a str,
    args: &'a [Type],
    variants: Option<&'a [String]>,
}

fn adt_type_parts(ty: &Type) -> Option<AdtTypeParts<'_>> {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        } => Some(AdtTypeParts {
            ty,
            name,
            identity,
            args,
            variants: None,
        }),
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            ..
        } => Some(AdtTypeParts {
            ty,
            name,
            identity,
            args,
            variants: Some(variants),
        }),
        _ => None,
    }
}

#[cfg(test)]
thread_local! {
    static AGGREGATE_JOIN_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_work(units: usize) {
    AGGREGATE_JOIN_WORK.with(|work| work.set(work.get() + units));
}

#[cfg(not(test))]
fn record_work(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset_work() {
    AGGREGATE_JOIN_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_work() -> usize {
    AGGREGATE_JOIN_WORK.with(|work| work.replace(0))
}
