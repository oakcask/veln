use crate::adt::registry::AdtRegistry;
use crate::adt::unification;
use crate::semantic_model::Type;
use crate::type_relations::{invariant_args_match, same_type_identity};

pub(crate) fn join_same_adt_types(adts: &AdtRegistry, left: &Type, right: &Type) -> Option<Type> {
    let left = adt_type_parts(left)?;
    let right = adt_type_parts(right)?;
    if !same_type_identity(left.name, left.identity, right.name, right.identity)
        || !invariant_args_match(left.args, right.args)
    {
        return None;
    }
    let mut joined_args = left.args.to_vec();
    for (joined, right) in joined_args.iter_mut().zip(right.args) {
        unification::merge_type_slot(joined, right);
    }

    if left.variants.is_none() || right.variants.is_none() {
        return Some(Type::resolved_named(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
        ));
    }

    let descriptor = adts.descriptor_for_type(left.ty)?;
    let left_variants = left.variants.expect("checked above");
    let right_variants = right.variants.expect("checked above");
    let variants = descriptor
        .variants
        .iter()
        .filter(|variant| {
            left_variants.contains(&variant.name) || right_variants.contains(&variant.name)
        })
        .map(|variant| variant.name.clone())
        .collect::<Vec<_>>();
    if variants.len() == descriptor.variants.len() {
        Some(Type::resolved_named(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
        ))
    } else {
        Some(Type::resolved_variant_refinement(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
            variants,
        ))
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
