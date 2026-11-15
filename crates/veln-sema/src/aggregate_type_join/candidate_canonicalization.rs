fn optimized_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    let mut joined = AggregateTypeJoin::new_optimized(adts, contributions.first()?)?;
    for contribution in &contributions[1..] {
        let prepared = joined.prepare(adts, contribution)?;
        joined.commit(prepared, adts);
    }
    Some(joined.result_type())
}

#[derive(Clone, Copy)]
enum BoundDirection {
    Join,
    Meet,
}

fn uses_outer_contributed_fallback(contributions: &[Type]) -> bool {
    let Some(first) = contributions.first() else {
        return false;
    };
    match first {
        Type::Function { .. } | Type::Unknown => false,
        Type::Record(first_fields) => contributions.iter().skip(1).any(|contribution| {
            let Type::Record(fields) = contribution else {
                return true;
            };
            fields.len() != first_fields.len()
                || first_fields
                    .iter()
                    .any(|(name, _)| !fields.iter().any(|(other, _)| other == name))
        }),
        Type::Named {
            name,
            identity,
            args,
        }
        | Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => contributions.iter().skip(1).any(|contribution| {
            adt_type_parts(contribution).is_none_or(|other| {
                !same_type_identity(name, identity, other.name, other.identity)
                    || args.len() != other.args.len()
            })
        }),
    }
}

fn candidate_assignable(expected: &Type, actual: &Type, adts: &AdtRegistry) -> bool {
    record_fallback_comparison();
    is_assignable_nested_with_registry(expected, actual, adts)
}

fn candidate_relation(
    candidate: &Type,
    contribution: &Type,
    direction: BoundDirection,
    adts: &AdtRegistry,
) -> bool {
    record_bounded_candidate_iteration();
    match direction {
        BoundDirection::Join => candidate_assignable(candidate, contribution, adts),
        BoundDirection::Meet => candidate_assignable(contribution, candidate, adts),
    }
}

fn constraint_pair_bound(
    candidate: &Type,
    contribution: &Type,
    direction: BoundDirection,
    adts: &AdtRegistry,
) -> Option<Type> {
    if candidate_relation(candidate, contribution, direction, adts) {
        return Some(match direction {
            BoundDirection::Join => candidate.clone(),
            BoundDirection::Meet => contribution.clone(),
        });
    }
    let pair = [candidate, contribution];
    match direction {
        BoundDirection::Join => bounded_join_refs(adts, &pair),
        BoundDirection::Meet => bounded_meet_refs(adts, &pair),
    }
}

fn candidate_assignable_cached(
    expected: &Type,
    actual: &Type,
    adts: &AdtRegistry,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> bool {
    record_bounded_candidate_iteration();
    let key = (
        canonical_relation_key(expected),
        canonical_relation_key(actual),
    );
    if let Some(assignable) = relation_cache.get(&key) {
        return *assignable;
    }
    let assignable = candidate_assignable(expected, actual, adts);
    let key_bytes = key.0.len() + key.1.len();
    relation_cache.insert(key, assignable);
    record_relation_cache_insert(relation_cache.len(), key_bytes);
    assignable
}

enum RelationKeyPart<'a> {
    Type(&'a Type),
    Text(&'a str),
    End,
}

enum CandidateOrderPart<'a> {
    Type(&'a Type),
    Named(&'a str, &'a [Type]),
    Text(&'a str),
}

fn candidate_order_key(ty: &Type) -> String {
    let mut key = String::new();
    let mut pending = vec![CandidateOrderPart::Type(ty)];
    while let Some(part) = pending.pop() {
        match part {
            CandidateOrderPart::Text(text) => key.push_str(text),
            CandidateOrderPart::Named(name, args) => {
                key.push_str(name);
                if !args.is_empty() {
                    pending.push(CandidateOrderPart::Text(">"));
                    for (index, argument) in args.iter().enumerate().rev() {
                        pending.push(CandidateOrderPart::Type(argument));
                        if index > 0 {
                            pending.push(CandidateOrderPart::Text(", "));
                        }
                    }
                    pending.push(CandidateOrderPart::Text("<"));
                }
            }
            CandidateOrderPart::Type(ty) => append_candidate_order_type(&mut key, &mut pending, ty),
        }
    }
    key
}

fn append_candidate_order_type<'a>(
    key: &mut String,
    pending: &mut Vec<CandidateOrderPart<'a>>,
    ty: &'a Type,
) {
    record_candidate_order_node();
    match ty {
        Type::Unknown => key.push_str("unknown"),
        Type::Named { name, args, .. } if name == "Unit" && args.is_empty() => key.push_str("()"),
        Type::Named { name, args, .. } => {
            pending.push(CandidateOrderPart::Named(name, args));
        }
        Type::VariantRefinement {
            name,
            args,
            variants,
            ..
        } => schedule_candidate_order_variants(pending, name, args, variants),
        Type::Record(fields) => schedule_candidate_order_record(pending, fields),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => schedule_candidate_order_function(
            pending,
            params,
            variadic.as_deref(),
            return_type,
            effects,
        ),
    }
}

fn schedule_candidate_order_variants<'a>(
    pending: &mut Vec<CandidateOrderPart<'a>>,
    name: &'a str,
    args: &'a [Type],
    variants: &'a [String],
) {
    for (index, variant) in variants.iter().enumerate().rev() {
        pending.push(CandidateOrderPart::Text(variant));
        pending.push(CandidateOrderPart::Text("::"));
        pending.push(CandidateOrderPart::Named(name, args));
        if index > 0 {
            pending.push(CandidateOrderPart::Text(" | "));
        }
    }
}

fn schedule_candidate_order_record<'a>(
    pending: &mut Vec<CandidateOrderPart<'a>>,
    fields: &'a [(String, Type)],
) {
    pending.push(CandidateOrderPart::Text("}"));
    for (index, (name, field)) in fields.iter().enumerate().rev() {
        pending.push(CandidateOrderPart::Type(field));
        pending.push(CandidateOrderPart::Text(": "));
        pending.push(CandidateOrderPart::Text(name));
        if index > 0 {
            pending.push(CandidateOrderPart::Text(", "));
        }
    }
    pending.push(CandidateOrderPart::Text("{"));
}

fn schedule_candidate_order_function<'a>(
    pending: &mut Vec<CandidateOrderPart<'a>>,
    params: &'a [Type],
    variadic: Option<&'a Type>,
    return_type: &'a Type,
    effects: &'a [String],
) {
    schedule_candidate_order_effects(pending, effects);
    pending.push(CandidateOrderPart::Type(return_type));
    pending.push(CandidateOrderPart::Text(") -> "));
    if let Some(variadic) = variadic {
        pending.push(CandidateOrderPart::Type(variadic));
        pending.push(CandidateOrderPart::Text("..."));
        if !params.is_empty() {
            pending.push(CandidateOrderPart::Text(", "));
        }
    }
    for (index, param) in params.iter().enumerate().rev() {
        pending.push(CandidateOrderPart::Type(param));
        if index > 0 {
            pending.push(CandidateOrderPart::Text(", "));
        }
    }
    pending.push(CandidateOrderPart::Text("fn("));
}

fn schedule_candidate_order_effects<'a>(
    pending: &mut Vec<CandidateOrderPart<'a>>,
    effects: &'a [String],
) {
    if effects.is_empty() {
        return;
    }
    pending.push(CandidateOrderPart::Text("]"));
    for (index, effect) in effects.iter().enumerate().rev() {
        pending.push(CandidateOrderPart::Text(effect));
        if index > 0 {
            pending.push(CandidateOrderPart::Text(", "));
        }
    }
    pending.push(CandidateOrderPart::Text(" effects ["));
}

fn canonical_relation_key(ty: &Type) -> String {
    let mut key = String::new();
    let mut pending = vec![RelationKeyPart::Type(ty)];
    while let Some(part) = pending.pop() {
        match part {
            RelationKeyPart::Text(text) => append_relation_key_text(&mut key, text),
            RelationKeyPart::End => key.push(']'),
            RelationKeyPart::Type(ty) => append_relation_type_key(&mut key, &mut pending, ty),
        }
    }
    key
}

fn append_relation_key_text(key: &mut String, text: &str) {
    let _ = write!(key, "{}:{text};", text.len());
}

fn append_relation_type_key<'a>(
    key: &mut String,
    pending: &mut Vec<RelationKeyPart<'a>>,
    ty: &'a Type,
) {
    record_relation_key_node();
    match ty {
        Type::Unknown => key.push('U'),
        Type::Named { identity, args, .. } => {
            append_relation_named_key(key, pending, identity, args)
        }
        Type::VariantRefinement {
            identity,
            args,
            variants,
            ..
        } => append_relation_variant_key(key, pending, identity, args, variants),
        Type::Record(fields) => append_relation_record_key(key, pending, fields),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => append_relation_function_key(
            key,
            pending,
            params,
            variadic.as_deref(),
            return_type,
            effects,
        ),
    }
}

fn append_relation_named_key<'a>(
    key: &mut String,
    pending: &mut Vec<RelationKeyPart<'a>>,
    identity: &'a str,
    args: &'a [Type],
) {
    let _ = write!(key, "N{}[", args.len());
    pending.push(RelationKeyPart::End);
    pending.extend(args.iter().rev().map(RelationKeyPart::Type));
    pending.push(RelationKeyPart::Text(identity));
}

fn append_relation_variant_key<'a>(
    key: &mut String,
    pending: &mut Vec<RelationKeyPart<'a>>,
    identity: &'a str,
    args: &'a [Type],
    variants: &'a [String],
) {
    let _ = write!(key, "V{}:{}[", args.len(), variants.len());
    pending.push(RelationKeyPart::End);
    pending.extend(
        variants
            .iter()
            .rev()
            .map(|variant| RelationKeyPart::Text(variant)),
    );
    pending.extend(args.iter().rev().map(RelationKeyPart::Type));
    pending.push(RelationKeyPart::Text(identity));
}

fn append_relation_record_key<'a>(
    key: &mut String,
    pending: &mut Vec<RelationKeyPart<'a>>,
    fields: &'a [(String, Type)],
) {
    let _ = write!(key, "R{}[", fields.len());
    pending.push(RelationKeyPart::End);
    for (name, field) in fields.iter().rev() {
        pending.push(RelationKeyPart::Type(field));
        pending.push(RelationKeyPart::Text(name));
    }
}

fn append_relation_function_key<'a>(
    key: &mut String,
    pending: &mut Vec<RelationKeyPart<'a>>,
    params: &'a [Type],
    variadic: Option<&'a Type>,
    return_type: &'a Type,
    effects: &'a [String],
) {
    let _ = write!(
        key,
        "F{}:{}:{}[",
        params.len(),
        usize::from(variadic.is_some()),
        effects.len()
    );
    pending.push(RelationKeyPart::End);
    pending.extend(
        effects
            .iter()
            .rev()
            .map(|effect| RelationKeyPart::Text(effect)),
    );
    pending.push(RelationKeyPart::Type(return_type));
    pending.push(match variadic {
        Some(variadic) => RelationKeyPart::Type(variadic),
        None => RelationKeyPart::Text("no-variadic"),
    });
    pending.extend(params.iter().rev().map(RelationKeyPart::Type));
}
