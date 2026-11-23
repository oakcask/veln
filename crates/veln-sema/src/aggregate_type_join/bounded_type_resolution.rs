#[derive(Clone, Copy)]
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

fn join_type_parts(ty: &Type) -> Option<()> {
    matches!(
        ty,
        Type::Named { .. } | Type::VariantRefinement { .. } | Type::Record(_)
    )
    .then_some(())
}

fn bounded_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    let contributions = contributions.iter().collect::<Vec<_>>();
    bounded_join_refs(adts, &contributions)
}

fn bounded_join_refs(adts: &AdtRegistry, contributions: &[&Type]) -> Option<Type> {
    let mut relation_cache = HashMap::new();
    reset_relation_cache_current_key_bytes();
    let resolved = bounded_join_iterative(adts, contributions, &mut relation_cache);
    reset_relation_cache_current_key_bytes();
    resolved
}

enum BoundTask<'a> {
    Ready(Option<Type>),
    Evaluate {
        contributions: Vec<&'a Type>,
        direction: BoundDirection,
    },
    BuildRecord {
        names: Vec<String>,
    },
    BuildNamed {
        name: String,
        identity: String,
        variants: Option<Vec<String>>,
        argument_count: usize,
    },
    BuildFunction {
        parameter_count: usize,
        has_variadic: bool,
        effects: Vec<String>,
    },
}

fn bounded_join_iterative(
    adts: &AdtRegistry,
    contributions: &[&Type],
    relation_cache: &mut HashMap<(String, String), bool>,
) -> Option<Type> {
    let mut tasks = vec![BoundTask::Evaluate {
        contributions: contributions.to_vec(),
        direction: BoundDirection::Join,
    }];
    let mut results = Vec::new();
    let mut function_presence_cache = HashMap::new();
    while let Some(task) = tasks.pop() {
        match task {
            BoundTask::Ready(result) => results.push(result),
            BoundTask::Evaluate {
                contributions,
                direction,
            } => evaluate_bound_task(
                adts,
                &mut tasks,
                &mut results,
                contributions,
                direction,
                relation_cache,
                &mut function_presence_cache,
            ),
            BoundTask::BuildRecord { names } => build_record_bound(&mut results, names),
            BoundTask::BuildNamed {
                name,
                identity,
                variants,
                argument_count,
            } => build_named_bound(&mut results, name, identity, variants, argument_count),
            BoundTask::BuildFunction {
                parameter_count,
                has_variadic,
                effects,
            } => build_function_bound(&mut results, parameter_count, has_variadic, effects),
        }
    }
    debug_assert_eq!(results.len(), 1);
    results.pop().flatten()
}

fn bounded_meet_refs(adts: &AdtRegistry, contributions: &[&Type]) -> Option<Type> {
    let mut relation_cache = HashMap::new();
    reset_relation_cache_current_key_bytes();
    let mut tasks = vec![BoundTask::Evaluate {
        contributions: contributions.to_vec(),
        direction: BoundDirection::Meet,
    }];
    let mut results = Vec::new();
    let mut function_presence_cache = HashMap::new();
    while let Some(task) = tasks.pop() {
        match task {
            BoundTask::Ready(result) => results.push(result),
            BoundTask::Evaluate {
                contributions,
                direction,
            } => evaluate_bound_task(
                adts,
                &mut tasks,
                &mut results,
                contributions,
                direction,
                &mut relation_cache,
                &mut function_presence_cache,
            ),
            BoundTask::BuildRecord { names } => build_record_bound(&mut results, names),
            BoundTask::BuildNamed {
                name,
                identity,
                variants,
                argument_count,
            } => build_named_bound(&mut results, name, identity, variants, argument_count),
            BoundTask::BuildFunction {
                parameter_count,
                has_variadic,
                effects,
            } => build_function_bound(&mut results, parameter_count, has_variadic, effects),
        }
    }
    reset_relation_cache_current_key_bytes();
    debug_assert_eq!(results.len(), 1);
    results.pop().flatten()
}

fn evaluate_bound_task<'a>(
    adts: &AdtRegistry,
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
    direction: BoundDirection,
    relation_cache: &mut HashMap<(String, String), bool>,
    function_presence_cache: &mut HashMap<*const Type, bool>,
) {
    record_bounded_frame();
    if contributions.len() == 1 {
        results.push(contributions.first().map(|ty| (*ty).clone()));
    } else if matches!(direction, BoundDirection::Meet) {
        push_contributed_bound(adts, results, &contributions, direction, relation_cache);
    } else if contributions
        .iter()
        .any(|contribution| matches!(contribution, Type::Function { .. }))
    {
        schedule_function_bound(tasks, results, contributions);
    } else if refs_require_composite_bound(adts, &contributions, function_presence_cache) {
        schedule_composite_bound(adts, tasks, results, contributions, relation_cache);
    } else {
        let mut owned = contributions
            .iter()
            .map(|contribution| (*contribution).clone())
            .collect::<Vec<_>>();
        let result = optimized_join(adts, &owned).or_else(|| {
            contributed_candidate_refs(adts, &contributions, direction, relation_cache)
        });
        drop_types_iteratively(&mut owned);
        results.push(result);
    }
}

fn build_record_bound(results: &mut Vec<Option<Type>>, names: Vec<String>) {
    let children = take_bound_results(results, names.len());
    results.push(children.map(|children| Type::Record(names.into_iter().zip(children).collect())));
}

fn build_named_bound(
    results: &mut Vec<Option<Type>>,
    name: String,
    identity: String,
    variants: Option<Vec<String>>,
    argument_count: usize,
) {
    let children = take_bound_results(results, argument_count);
    results.push(children.map(|children| match variants {
        Some(variants) => Type::resolved_variant_refinement(name, identity, children, variants),
        None => Type::resolved_named(name, identity, children),
    }));
}

fn build_function_bound(
    results: &mut Vec<Option<Type>>,
    parameter_count: usize,
    has_variadic: bool,
    effects: Vec<String>,
) {
    let child_count = parameter_count + usize::from(has_variadic) + 1;
    let Some(mut children) = take_bound_results(results, child_count) else {
        results.push(None);
        return;
    };
    let return_type = Box::new(children.pop().expect("function result child"));
    let variadic = has_variadic.then(|| Box::new(children.pop().expect("variadic child")));
    results.push(Some(Type::Function {
        params: children.into(),
        variadic,
        return_type,
        effects,
    }));
}

fn take_bound_results(results: &mut Vec<Option<Type>>, count: usize) -> Option<Vec<Type>> {
    let start = results.len().checked_sub(count)?;
    let mut children = Vec::with_capacity(count);
    let mut complete = true;
    for result in results.drain(start..) {
        match result {
            Some(ty) if complete => children.push(ty),
            Some(ty) => crate::semantic_model::drop_type_iteratively(ty),
            None => complete = false,
        }
    }
    if complete {
        Some(children)
    } else {
        drop_types_iteratively(&mut children);
        None
    }
}

fn refs_require_composite_bound(
    adts: &AdtRegistry,
    contributions: &[&Type],
    function_presence_cache: &mut HashMap<*const Type, bool>,
) -> bool {
    if contributions
        .iter()
        .any(|ty| type_contains_function(ty, function_presence_cache))
    {
        return true;
    }
    let Some(Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. }) =
        contributions.first().copied()
    else {
        return false;
    };
    !Type::is_compiler_covariant_identity(identity)
        && adts
            .parameter_variances_for_identity(identity, args.len())
            .is_some_and(|variances| {
                variances
                    .iter()
                    .any(|variance| *variance != Variance::Covariant)
            })
}

fn schedule_composite_bound<'a>(
    adts: &AdtRegistry,
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
    relation_cache: &mut HashMap<(String, String), bool>,
) {
    match contributions.first().copied() {
        Some(Type::Record(_)) => {
            schedule_record_bound(adts, tasks, results, contributions, relation_cache)
        }
        Some(Type::Named { .. } | Type::VariantRefinement { .. }) => {
            schedule_named_bound(adts, tasks, results, contributions, relation_cache);
        }
        Some(Type::Function { .. }) => schedule_function_bound(tasks, results, contributions),
        Some(Type::Unknown) | None => results.push(None),
    }
}

fn schedule_record_bound<'a>(
    adts: &AdtRegistry,
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
    relation_cache: &mut HashMap<(String, String), bool>,
) {
    let Some(Type::Record(first_fields)) = contributions.first().copied() else {
        unreachable!("record bound was selected");
    };
    let mut names = first_fields
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    names.sort_unstable();
    if !record_bounds_have_fields(&contributions, &names) {
        push_contributed_bound(
            adts,
            results,
            &contributions,
            BoundDirection::Join,
            relation_cache,
        );
        return;
    }
    tasks.push(BoundTask::BuildRecord {
        names: names.clone(),
    });
    for name in names.into_iter().rev() {
        tasks.push(BoundTask::Evaluate {
            contributions: record_bound_fields(&contributions, &name),
            direction: BoundDirection::Join,
        });
    }
}

fn record_bounds_have_fields(contributions: &[&Type], names: &[String]) -> bool {
    contributions.iter().all(|contribution| {
        let Type::Record(fields) = contribution else {
            return false;
        };
        fields.len() == names.len()
            && names
                .iter()
                .all(|name| fields.iter().any(|(field, _)| field == name))
    })
}

fn record_bound_fields<'a>(contributions: &[&'a Type], name: &str) -> Vec<&'a Type> {
    contributions
        .iter()
        .map(|contribution| {
            let Type::Record(fields) = contribution else {
                unreachable!("record shape was checked");
            };
            fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, ty)| ty)
                .expect("record field was checked")
        })
        .collect()
}

fn schedule_named_bound<'a>(
    adts: &AdtRegistry,
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
    relation_cache: &mut HashMap<(String, String), bool>,
) {
    let Some(first) = contributions.first().and_then(|ty| adt_type_parts(ty)) else {
        results.push(None);
        return;
    };
    let Some(parts) = contributions
        .iter()
        .map(|ty| adt_type_parts(ty))
        .collect::<Option<Vec<_>>>()
    else {
        push_contributed_bound(
            adts,
            results,
            &contributions,
            BoundDirection::Join,
            relation_cache,
        );
        return;
    };
    if parts.iter().any(|part| !same_adt_shape(first, *part)) {
        push_contributed_bound(
            adts,
            results,
            &contributions,
            BoundDirection::Join,
            relation_cache,
        );
        return;
    }
    let name = named_bound_name(adts, first, &parts);
    let Some(variants) = joined_named_variants(adts, first, &parts) else {
        results.push(None);
        return;
    };
    let compiler_covariant = Type::is_compiler_covariant_identity(first.identity);
    let variances = (!compiler_covariant)
        .then(|| adts.parameter_variances_for_identity(first.identity, first.args.len()))
        .flatten();
    tasks.push(BoundTask::BuildNamed {
        name,
        identity: first.identity.to_string(),
        variants,
        argument_count: first.args.len(),
    });
    schedule_named_bound_arguments(tasks, first, &parts, compiler_covariant, variances);
}

fn push_contributed_bound(
    adts: &AdtRegistry,
    results: &mut Vec<Option<Type>>,
    contributions: &[&Type],
    direction: BoundDirection,
    relation_cache: &mut HashMap<(String, String), bool>,
) {
    results.push(contributed_candidate_refs(
        adts,
        contributions,
        direction,
        relation_cache,
    ));
}

fn named_bound_name(
    adts: &AdtRegistry,
    first: AdtTypeParts<'_>,
    parts: &[AdtTypeParts<'_>],
) -> String {
    if parts.iter().all(|part| part.name == first.name) {
        return first.name.to_string();
    }
    Type::compiler_covariant_name(first.identity)
        .map(str::to_string)
        .or_else(|| {
            adts.descriptor_for_type(first.ty)
                .map(|descriptor| adts.canonical_type_name_for_descriptor(descriptor))
        })
        .unwrap_or_else(|| first.name.to_string())
}

fn schedule_named_bound_arguments<'a>(
    tasks: &mut Vec<BoundTask<'a>>,
    first: AdtTypeParts<'a>,
    parts: &[AdtTypeParts<'a>],
    compiler_covariant: bool,
    variances: Option<&[Variance]>,
) {
    for index in (0..first.args.len()).rev() {
        let arguments = parts
            .iter()
            .map(|part| &part.args[index])
            .collect::<Vec<_>>();
        let variance = if compiler_covariant {
            Variance::Covariant
        } else {
            variances
                .and_then(|items| items.get(index).copied())
                .unwrap_or(Variance::Invariant)
        };
        schedule_named_argument(tasks, arguments, variance);
    }
}

fn schedule_named_argument<'a>(
    tasks: &mut Vec<BoundTask<'a>>,
    arguments: Vec<&'a Type>,
    variance: Variance,
) {
    let direction = match variance {
        Variance::Covariant => Some(BoundDirection::Join),
        Variance::Contravariant => Some(BoundDirection::Meet),
        Variance::Invariant | Variance::Irrelevant => None,
    };
    if let Some(direction) = direction {
        tasks.push(BoundTask::Evaluate {
            contributions: arguments,
            direction,
        });
        return;
    }
    let exact = arguments
        .iter()
        .skip(1)
        .all(|argument| {
            invariant_args_match(
                std::slice::from_ref(arguments[0]),
                std::slice::from_ref(argument),
            )
        })
        .then(|| merge_candidate_presentations(arguments));
    tasks.push(BoundTask::Ready(exact));
}

fn joined_named_variants(
    adts: &AdtRegistry,
    first: AdtTypeParts<'_>,
    parts: &[AdtTypeParts<'_>],
) -> Option<Option<Vec<String>>> {
    if parts.iter().any(|part| part.variants.is_none()) {
        return Some(None);
    }
    let order = adts.variant_declaration_order_for_type(first.ty)?;
    let mut ranks = parts
        .iter()
        .flat_map(|part| part.variants.expect("all parts are refined"))
        .map(|variant| order.rank(variant))
        .collect::<Option<HashSet<_>>>()?
        .into_iter()
        .collect::<Vec<_>>();
    ranks.sort_unstable();
    if ranks.len() == order.len() {
        return Some(None);
    }
    ranks
        .into_iter()
        .map(|rank| order.name(rank).map(str::to_string))
        .collect::<Option<Vec<_>>>()
        .map(Some)
}

fn schedule_function_bound<'a>(
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
) {
    let Some(bound) = collect_function_bound_parts(&contributions) else {
        results.push(None);
        return;
    };
    tasks.push(BoundTask::BuildFunction {
        parameter_count: bound.parameter_count,
        has_variadic: bound.has_variadic,
        effects: bound.effects,
    });
    schedule_function_bound_children(tasks, &bound.functions, bound.has_variadic);
}

struct FunctionBoundParts<'a> {
    params: &'a [Type],
    variadic: Option<&'a Type>,
    return_type: &'a Type,
}

struct CollectedFunctionBound<'a> {
    functions: Vec<FunctionBoundParts<'a>>,
    effects: Vec<String>,
    parameter_count: usize,
    has_variadic: bool,
}

fn collect_function_bound_parts<'a>(
    contributions: &[&'a Type],
) -> Option<CollectedFunctionBound<'a>> {
    let Type::Function {
        params: first_params,
        variadic: first_variadic,
        effects: first_effects,
        ..
    } = contributions.first().copied()?
    else {
        return None;
    };
    let mut functions = Vec::with_capacity(contributions.len());
    let mut effects = HashSet::new();
    let has_effect_row = contributions.iter().any(|contribution| {
        matches!(
            contribution,
            Type::Function { effects, .. }
                if effects.iter().any(|effect| effect.starts_with("..."))
        )
    });
    for contribution in contributions {
        let Type::Function {
            params,
            variadic,
            return_type,
            effects: contribution_effects,
        } = contribution
        else {
            return None;
        };
        if params.len() != first_params.len() || variadic.is_some() != first_variadic.is_some() {
            return None;
        }
        if has_effect_row {
            if contribution_effects != first_effects {
                return None;
            }
        } else {
            effects.extend(contribution_effects.iter().cloned());
        }
        functions.push(FunctionBoundParts {
            params,
            variadic: variadic.as_deref(),
            return_type,
        });
    }
    let effects = if has_effect_row {
        first_effects.clone()
    } else {
        let mut effects = effects.into_iter().collect::<Vec<_>>();
        effects.sort_unstable();
        effects
    };
    Some(CollectedFunctionBound {
        functions,
        effects,
        parameter_count: first_params.len(),
        has_variadic: first_variadic.is_some(),
    })
}

fn schedule_function_bound_children<'a>(
    tasks: &mut Vec<BoundTask<'a>>,
    functions: &[FunctionBoundParts<'a>],
    has_variadic: bool,
) {
    tasks.push(BoundTask::Evaluate {
        contributions: functions
            .iter()
            .map(|function| function.return_type)
            .collect(),
        direction: BoundDirection::Join,
    });
    if has_variadic {
        tasks.push(BoundTask::Evaluate {
            contributions: functions
                .iter()
                .map(|function| function.variadic.expect("variadic shape was checked"))
                .collect(),
            direction: BoundDirection::Meet,
        });
    }
    let parameter_count = functions
        .first()
        .map_or(0, |function| function.params.len());
    for index in (0..parameter_count).rev() {
        tasks.push(BoundTask::Evaluate {
            contributions: functions
                .iter()
                .map(|function| &function.params[index])
                .collect(),
            direction: BoundDirection::Meet,
        });
    }
}
