fn bounded_shapes_are_admissible(adts: &AdtRegistry, contributions: &[Type], right: &Type) -> bool {
    if contributions.is_empty() {
        return false;
    }
    contributions
        .iter()
        .all(|contribution| structurally_compatible_function_shapes(adts, contribution, right))
}

fn admission_witness(adts: &AdtRegistry, contributions: &[Type]) -> Option<usize> {
    let mut witness = None;
    for (index, contribution) in contributions.iter().enumerate() {
        witness = if let Some(witness_index) = witness {
            let current = &contributions[witness_index];
            if structural_paths_cover(adts, contribution, current) {
                Some(index)
            } else if structural_paths_cover(adts, current, contribution) {
                Some(witness_index)
            } else {
                None
            }
        } else if contributions[..index]
            .iter()
            .all(|previous| structural_paths_cover(adts, contribution, previous))
        {
            Some(index)
        } else {
            None
        };
    }
    witness
}

fn structural_paths_cover(adts: &AdtRegistry, cover: &Type, covered: &Type) -> bool {
    let mut pending = vec![(cover, covered)];
    while let Some((cover, covered)) = pending.pop() {
        if !queue_structural_cover_pair(adts, cover, covered, &mut pending) {
            return false;
        }
    }
    true
}

fn queue_structural_cover_pair<'a>(
    adts: &AdtRegistry,
    cover: &'a Type,
    covered: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    match (cover, covered) {
        (Type::Function { .. }, Type::Function { .. }) => {
            queue_compatible_function_shape(cover, covered, pending)
        }
        (Type::Function { .. }, _) | (_, Type::Function { .. }) => false,
        (Type::Record(cover), Type::Record(covered)) => {
            queue_structural_record_cover(cover, covered, pending)
        }
        (Type::Record(_), _) | (_, Type::Record(_)) => false,
        (
            cover @ (Type::Named { .. } | Type::VariantRefinement { .. }),
            covered @ (Type::Named { .. } | Type::VariantRefinement { .. }),
        ) => queue_structural_named_cover(adts, cover, covered, pending),
        (Type::Named { .. } | Type::VariantRefinement { .. }, _)
        | (_, Type::Named { .. } | Type::VariantRefinement { .. }) => false,
        (Type::Unknown, Type::Unknown) => true,
    }
}

fn queue_structural_record_cover<'a>(
    cover: &'a [(String, Type)],
    covered: &'a [(String, Type)],
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    for _ in cover {
        record_field_match_work();
    }
    let cover_by_name = cover
        .iter()
        .map(|(name, ty)| (name.as_str(), ty))
        .collect::<HashMap<_, _>>();
    for (name, covered) in covered {
        record_field_match_work();
        let Some(cover) = cover_by_name.get(name.as_str()) else {
            return false;
        };
        pending.push((*cover, covered));
    }
    true
}

fn queue_structural_named_cover<'a>(
    adts: &AdtRegistry,
    cover: &'a Type,
    covered: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    let first = adt_type_parts(cover).expect("named coverage was selected");
    let Some(other) = adt_type_parts(covered) else {
        return false;
    };
    if !same_adt_shape(first, other) {
        return false;
    }
    let compiler_covariant = Type::is_compiler_covariant_identity(first.identity);
    let variances = (!compiler_covariant)
        .then(|| adts.parameter_variances_for_identity(first.identity, first.args.len()))
        .flatten();
    for (index, (cover, covered)) in first.args.iter().zip(other.args).enumerate() {
        let variance = if compiler_covariant {
            Variance::Covariant
        } else {
            variances
                .and_then(|items| items.get(index).copied())
                .unwrap_or(Variance::Invariant)
        };
        if matches!(variance, Variance::Covariant | Variance::Contravariant) {
            pending.push((cover, covered));
        }
    }
    true
}

fn structurally_compatible_function_shapes(adts: &AdtRegistry, left: &Type, right: &Type) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        if !queue_compatible_shape_pair(adts, left, right, &mut pending) {
            return false;
        }
    }
    true
}

fn queue_compatible_shape_pair<'a>(
    adts: &AdtRegistry,
    left: &'a Type,
    right: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    match (left, right) {
        (Type::Function { .. }, Type::Function { .. }) => {
            queue_compatible_function_shape(left, right, pending)
        }
        (Type::Function { .. }, _) | (_, Type::Function { .. }) => false,
        (Type::Record(left), Type::Record(right)) => {
            queue_compatible_record_shape(left, right, pending)
        }
        (Type::Record(_), _) | (_, Type::Record(_)) => false,
        (
            left @ (Type::Named { .. } | Type::VariantRefinement { .. }),
            right @ (Type::Named { .. } | Type::VariantRefinement { .. }),
        ) => queue_compatible_named_shape(adts, left, right, pending),
        _ => true,
    }
}

fn queue_compatible_function_shape<'a>(
    left: &'a Type,
    right: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    let (
        Type::Function {
            params: left_params,
            variadic: left_variadic,
            return_type: left_return,
            ..
        },
        Type::Function {
            params: right_params,
            variadic: right_variadic,
            return_type: right_return,
            ..
        },
    ) = (left, right)
    else {
        unreachable!("function shapes were selected");
    };
    if left_params.len() != right_params.len()
        || left_variadic.is_some() != right_variadic.is_some()
    {
        return false;
    }
    pending.extend(left_params.iter().zip(right_params));
    if let (Some(left), Some(right)) = (left_variadic, right_variadic) {
        pending.push((left, right));
    }
    pending.push((left_return, right_return));
    true
}

fn queue_compatible_record_shape<'a>(
    left: &'a [(String, Type)],
    right: &'a [(String, Type)],
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    for _ in right {
        record_field_match_work();
    }
    let right_by_name = right
        .iter()
        .map(|(name, ty)| (name.as_str(), ty))
        .collect::<HashMap<_, _>>();
    for (name, left) in left.iter().rev() {
        record_field_match_work();
        if let Some(right) = right_by_name.get(name.as_str()) {
            pending.push((left, *right));
        }
    }
    true
}

fn queue_compatible_named_shape<'a>(
    adts: &AdtRegistry,
    left: &'a Type,
    right: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    let first = adt_type_parts(left).expect("named shape was selected");
    let Some(other) = adt_type_parts(right) else {
        return false;
    };
    if !same_adt_shape(first, other) {
        return false;
    }
    let compiler_covariant = Type::is_compiler_covariant_identity(first.identity);
    let variances = (!compiler_covariant)
        .then(|| adts.parameter_variances_for_identity(first.identity, first.args.len()))
        .flatten();
    for (index, (left, right)) in first.args.iter().zip(other.args).enumerate() {
        let variance = if compiler_covariant {
            Variance::Covariant
        } else {
            variances
                .and_then(|items| items.get(index).copied())
                .unwrap_or(Variance::Invariant)
        };
        if matches!(variance, Variance::Invariant | Variance::Irrelevant) {
            if !invariant_args_match(std::slice::from_ref(left), std::slice::from_ref(right)) {
                return false;
            }
        } else {
            pending.push((left, right));
        }
    }
    true
}

fn type_contains_function(ty: &Type, cache: &mut HashMap<*const Type, bool>) -> bool {
    let mut pending = vec![(ty, false)];
    while let Some((current, expanded)) = pending.pop() {
        let key = std::ptr::from_ref(current);
        if cache.contains_key(&key) {
            continue;
        }
        if expanded {
            let contains = match current {
                Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
                    args.iter().any(|child| cache[&std::ptr::from_ref(child)])
                }
                Type::Record(fields) => fields
                    .iter()
                    .any(|(_, child)| cache[&std::ptr::from_ref(child)]),
                Type::Function { .. } => true,
                Type::Unknown => false,
            };
            cache.insert(key, contains);
            continue;
        }
        record_function_scan_node();
        match current {
            Type::Function { .. } => {
                cache.insert(key, true);
            }
            Type::Unknown => {
                cache.insert(key, false);
            }
            Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
                pending.push((current, true));
                pending.extend(args.iter().map(|child| (child, false)));
            }
            Type::Record(fields) => {
                pending.push((current, true));
                pending.extend(fields.iter().map(|(_, child)| (child, false)));
            }
        }
    }
    cache[&std::ptr::from_ref(ty)]
}
