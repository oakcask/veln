use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use crate::adt::registry::{AdtRegistry, VariantDeclarationOrder};
use crate::adt::unification;
use crate::semantic_model::Type;
use crate::type_relations::{invariant_args_match, same_type_identity};

pub(crate) struct AggregateTypeJoin {
    name: String,
    identity: String,
    args: Vec<Type>,
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
    materialized: RefCell<Option<Type>>,
}

impl AggregateTypeJoin {
    pub(crate) fn new(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let initial = adt_type_parts(initial)?;
        let (declaration_order, variants) = match initial.variants {
            None => (None, None),
            Some(variants) => {
                let declaration_order = adts.variant_declaration_order_for_type(initial.ty)?;
                record_work(variants.len());
                let variants = Some(
                    variants
                        .iter()
                        .map(|variant| declaration_order.rank(variant))
                        .collect::<Option<HashSet<_>>>()?,
                );
                (Some(declaration_order), variants)
            }
        };
        Some(Self {
            name: initial.name.to_string(),
            identity: initial.identity.to_string(),
            args: initial.args.to_vec(),
            declaration_order,
            variants,
            materialized: RefCell::new(None),
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
        let mut changed = false;
        for (joined, right) in self.args.iter_mut().zip(right.args) {
            changed |= unification::merge_type_slot(joined, right);
        }
        match (&mut self.variants, right.variants) {
            (Some(joined), Some(right)) => {
                record_work(right.len());
                let declaration_order = self
                    .declaration_order
                    .as_ref()
                    .expect("a refinement join retains declaration order");
                for variant in right {
                    let Some(rank) = declaration_order.rank(variant) else {
                        return false;
                    };
                    changed |= joined.insert(rank);
                }
            }
            (variants @ Some(_), None) => {
                *variants = None;
                changed = true;
            }
            (None, Some(_)) => {}
            (None, None) => {}
        }
        if changed {
            *self.materialized.borrow_mut() = None;
        }
        true
    }

    pub(crate) fn inference_type(&self) -> Type {
        Type::resolved_named(self.name.clone(), self.identity.clone(), self.args.clone())
    }

    pub(crate) fn result_type(&self) -> Type {
        if let Some(materialized) = self.materialized.borrow().clone() {
            return materialized;
        }
        let Some(variants) = &self.variants else {
            return self.inference_type();
        };
        let declaration_order = self
            .declaration_order
            .as_ref()
            .expect("a refinement join retains declaration order");
        let materialized = if variants.len() == declaration_order.len() {
            self.inference_type()
        } else {
            record_work(variants.len());
            let mut ranks = variants.iter().copied().collect::<Vec<_>>();
            ranks.sort_unstable_by(|left, right| {
                record_work(1);
                left.cmp(right)
            });
            let variants = ranks
                .into_iter()
                .map(|rank| {
                    declaration_order
                        .name(rank)
                        .expect("joined variant rank belongs to its declaration")
                        .to_string()
                })
                .collect();
            Type::resolved_variant_refinement(
                self.name.clone(),
                self.identity.clone(),
                self.args.clone(),
                variants,
            )
        };
        *self.materialized.borrow_mut() = Some(materialized.clone());
        materialized
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
pub(crate) fn record_work(units: usize) {
    crate::inference_work::record(units);
}

#[cfg(not(test))]
pub(crate) fn record_work(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset_work() {
    crate::inference_work::reset();
}

#[cfg(test)]
pub(crate) fn take_work() -> usize {
    crate::inference_work::take()
}
