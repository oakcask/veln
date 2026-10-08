use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use crate::adt::descriptors::AdtConstructor;
use crate::adt::registry::{AdtRegistry, VariantDeclarationOrder};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;
use crate::type_relations::{invariant_args_match, same_type_identity};

pub(crate) struct AggregateTypeJoin {
    name: String,
    canonical_name: String,
    presentation_conflicted: bool,
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
            canonical_name: adts
                .descriptor_for_type(initial.ty)
                .map(|descriptor| adts.canonical_type_name_for_descriptor(descriptor))
                .unwrap_or_else(|| initial.name.to_string()),
            presentation_conflicted: false,
            identity: initial.identity.to_string(),
            args: initial.args.to_vec(),
            declaration_order,
            variants,
            materialized: RefCell::new(None),
        })
    }

    pub(crate) fn new_resolved_refinement(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let Type::VariantRefinement { args, .. } = initial else {
            return None;
        };
        if args.iter().any(type_contains_unknown) {
            return None;
        }
        Self::new(adts, initial)
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
        if !self.presentation_conflicted && right.name != self.canonical_name {
            if self.name == self.canonical_name {
                self.name = right.name.to_string();
                changed = true;
            } else if self.name != right.name {
                self.name.clone_from(&self.canonical_name);
                self.presentation_conflicted = true;
                changed = true;
            }
        }
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

    pub(crate) fn try_join_resolved(&mut self, right: &Type) -> bool {
        let Some(right_parts) = adt_type_parts(right) else {
            return false;
        };
        if right_parts.args.iter().any(type_contains_unknown)
            || !invariant_args_match(&self.args, right_parts.args)
        {
            return false;
        }
        self.try_join(right)
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

pub(crate) fn merge_invariant_payload_type_args(
    inferred: &mut [Type],
    joined: &mut [Option<AggregateTypeJoin>],
    invariant: &mut [bool],
    constructor: AdtConstructor<'_>,
    index: usize,
    actual: &Type,
) -> Result<(), unification::TypeParameterContributionConflict> {
    let mut contributions = Vec::new();
    adt::visit_type_arg_contributions_from_payload(
        constructor,
        index,
        actual,
        |type_index, contribution| {
            record_work(1);
            contributions.push((type_index, contribution.clone()));
        },
    );
    let constraints = unification::merge_type_parameter_contributions_transactionally(
        &contributions,
        |type_index| {
            inferred.get(type_index).map(|inferred| {
                joined[type_index]
                    .as_ref()
                    .map(AggregateTypeJoin::result_type)
                    .unwrap_or_else(|| inferred.clone())
            })
        },
    )?;
    for (type_index, constraint) in constraints {
        record_work(1);
        inferred[type_index] = constraint;
        joined[type_index] = None;
        invariant[type_index] = true;
    }
    Ok(())
}

fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            args.iter().any(type_contains_unknown)
        }
        Type::Record(fields) => fields.iter().any(|(_, field)| type_contains_unknown(field)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_contains_unknown)
                || variadic.as_deref().is_some_and(type_contains_unknown)
                || type_contains_unknown(return_type)
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
