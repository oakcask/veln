fn contributed_candidate_refs(
    adts: &AdtRegistry,
    contributions: &[&Type],
    direction: BoundDirection,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> Option<Type> {
    let valid = contributions
        .iter()
        .copied()
        .filter(|candidate| {
            contributed_candidate_is_valid(
                candidate,
                contributions,
                direction,
                adts,
                relation_cache,
            )
        })
        .collect::<Vec<_>>();
    let mut selected = valid
        .iter()
        .copied()
        .filter(|candidate| {
            contributed_candidate_is_selected(candidate, &valid, direction, adts, relation_cache)
        })
        .collect::<Vec<_>>();
    if !selected_candidates_are_equivalent(&selected, adts, relation_cache) {
        return None;
    }
    selected.sort_unstable_by_key(|candidate| candidate_order_key(candidate));
    let first = selected[0];
    Some(merge_contributed_presentations(first, contributions))
}

fn contributed_candidate_is_valid(
    candidate: &Type,
    contributions: &[&Type],
    direction: BoundDirection,
    adts: &AdtRegistry,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> bool {
    contributions.iter().all(|contribution| match direction {
        BoundDirection::Join => {
            candidate_assignable_cached(candidate, contribution, adts, relation_cache)
        }
        BoundDirection::Meet => {
            candidate_assignable_cached(contribution, candidate, adts, relation_cache)
        }
    })
}

fn contributed_candidate_is_selected(
    candidate: &Type,
    valid: &[&Type],
    direction: BoundDirection,
    adts: &AdtRegistry,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> bool {
    valid.iter().all(|other| match direction {
        BoundDirection::Join => candidate_assignable_cached(other, candidate, adts, relation_cache),
        BoundDirection::Meet => candidate_assignable_cached(candidate, other, adts, relation_cache),
    })
}

fn selected_candidates_are_equivalent(
    selected: &[&Type],
    adts: &AdtRegistry,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> bool {
    let Some(first) = selected.first().copied() else {
        return false;
    };
    selected.iter().skip(1).all(|candidate| {
        candidate_assignable_cached(first, candidate, adts, relation_cache)
            && candidate_assignable_cached(candidate, first, adts, relation_cache)
    })
}

fn merge_candidate_presentations(selected: Vec<&Type>) -> Type {
    let first = selected
        .first()
        .copied()
        .expect("selected candidates are nonempty");
    merge_contributed_presentations(first, &selected)
}

fn merge_contributed_presentations(first: &Type, contributions: &[&Type]) -> Type {
    let mut result = first.clone();
    let mut presentation = TypePresentationJoin::default();
    for contribution in contributions {
        presentation.merge(&mut result, contribution);
    }
    result
}

fn presentation_layout_is_reusable(previous: &Type, next: &Type) -> bool {
    let mut pending = vec![(previous, next)];
    while let Some((previous, next)) = pending.pop() {
        match (previous, next) {
            (
                Type::Named {
                    identity: previous_identity,
                    args: previous_args,
                    ..
                }
                | Type::VariantRefinement {
                    identity: previous_identity,
                    args: previous_args,
                    ..
                },
                Type::Named {
                    identity: next_identity,
                    args: next_args,
                    ..
                }
                | Type::VariantRefinement {
                    identity: next_identity,
                    args: next_args,
                    ..
                },
            ) => {
                if previous_identity != next_identity || previous_args.len() != next_args.len() {
                    return false;
                }
                pending.extend(previous_args.iter().zip(next_args));
            }
            (Type::Record(previous_fields), Type::Record(next_fields)) => {
                let shared = previous_fields.len().min(next_fields.len());
                if previous_fields[..shared]
                    .iter()
                    .zip(&next_fields[..shared])
                    .any(|((previous, _), (next, _))| previous != next)
                {
                    return false;
                }
                pending.extend(
                    previous_fields[..shared]
                        .iter()
                        .zip(&next_fields[..shared])
                        .map(|((_, previous), (_, next))| (previous, next)),
                );
            }
            (
                Type::Function {
                    params: previous_params,
                    variadic: previous_variadic,
                    return_type: previous_return,
                    ..
                },
                Type::Function {
                    params: next_params,
                    variadic: next_variadic,
                    return_type: next_return,
                    ..
                },
            ) => {
                if previous_params.len() != next_params.len()
                    || previous_variadic.is_some() != next_variadic.is_some()
                {
                    return false;
                }
                pending.extend(previous_params.iter().zip(next_params));
                if let (Some(previous), Some(next)) = (previous_variadic, next_variadic) {
                    pending.push((previous, next));
                }
                pending.push((previous_return, next_return));
            }
            (Type::Unknown, Type::Unknown) => {}
            _ => return false,
        }
    }
    true
}
