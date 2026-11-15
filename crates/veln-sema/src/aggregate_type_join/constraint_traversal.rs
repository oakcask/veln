fn collect_constructor_payload_constraints(
    constructor: AdtConstructor<'_>,
    payload_index: usize,
    actual: &Type,
    adts: &AdtRegistry,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) {
    let Some(field) = constructor.variant.payload_fields.get(payload_index) else {
        return;
    };
    match &field.ty {
        crate::adt::descriptors::AdtPayloadType::TypeParameter(parameter) => {
            contributions.push(ConstructorConstraintContribution {
                parameter: *parameter,
                polarity: Variance::Covariant,
                ty: actual.clone(),
            });
        }
        crate::adt::descriptors::AdtPayloadType::SelfType => {
            let Some(actual_args) = unification::adt_args(actual, constructor.descriptor) else {
                return;
            };
            let variances = checked_parameter_variances(
                &constructor.descriptor.identity(),
                actual_args.len(),
                adts,
            );
            for (parameter, (actual, polarity)) in actual_args.iter().zip(variances).enumerate() {
                contributions.push(ConstructorConstraintContribution {
                    parameter,
                    polarity,
                    ty: actual.clone(),
                });
            }
        }
        crate::adt::descriptors::AdtPayloadType::Concrete(template) => {
            collect_template_constraints(
                template,
                actual,
                Variance::Covariant,
                adts,
                contributions,
            );
        }
    }
}
fn collect_template_constraints(
    template: &Type,
    actual: &Type,
    polarity: Variance,
    adts: &AdtRegistry,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) {
    if collect_parameter_constraint(template, actual, polarity, contributions)
        || collect_named_constraints(template, actual, polarity, adts, contributions)
        || collect_record_constraints(template, actual, polarity, adts, contributions)
    {
        return;
    }
    collect_function_constraints(template, actual, polarity, adts, contributions);
}

fn collect_parameter_constraint(
    template: &Type,
    actual: &Type,
    polarity: Variance,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) -> bool {
    let Type::Named { name, args, .. } = template else {
        return false;
    };
    if !name.starts_with("$param") || !args.is_empty() {
        return false;
    }
    if let Ok(parameter) = name.trim_start_matches("$param").parse::<usize>() {
        contributions.push(ConstructorConstraintContribution {
            parameter,
            polarity,
            ty: actual.clone(),
        });
    }
    true
}

fn named_constraint_parts(ty: &Type) -> Option<(&str, &str, &[Type])> {
    match ty {
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
        } => Some((name, identity, args)),
        _ => None,
    }
}

fn collect_named_constraints(
    template: &Type,
    actual: &Type,
    polarity: Variance,
    adts: &AdtRegistry,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) -> bool {
    let Some((name, identity, args)) = named_constraint_parts(template) else {
        return false;
    };
    let Some((actual_name, actual_identity, actual_args)) = named_constraint_parts(actual) else {
        return true;
    };
    if !same_type_identity(name, identity, actual_name, actual_identity)
        || args.len() != actual_args.len()
    {
        return true;
    }
    let variances = checked_parameter_variances(identity, args.len(), adts);
    for ((template, actual), nested) in args.iter().zip(actual_args).zip(variances) {
        collect_template_constraints(
            template,
            actual,
            polarity.compose(nested),
            adts,
            contributions,
        );
    }
    true
}

fn collect_record_constraints(
    template: &Type,
    actual: &Type,
    polarity: Variance,
    adts: &AdtRegistry,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) -> bool {
    let Type::Record(fields) = template else {
        return false;
    };
    let Type::Record(actual_fields) = actual else {
        return true;
    };
    let mut actual_fields_by_name = HashMap::with_capacity(actual_fields.len());
    for (name, actual) in actual_fields {
        record_constraint_record_field_work();
        actual_fields_by_name.entry(name.as_str()).or_insert(actual);
    }
    for (name, template) in fields {
        record_constraint_record_field_work();
        if let Some(actual) = actual_fields_by_name.get(name.as_str()) {
            collect_template_constraints(template, actual, polarity, adts, contributions);
        }
    }
    true
}

fn collect_function_constraints(
    template: &Type,
    actual: &Type,
    polarity: Variance,
    adts: &AdtRegistry,
    contributions: &mut Vec<ConstructorConstraintContribution>,
) {
    let Type::Function {
        params,
        variadic,
        return_type,
        ..
    } = template
    else {
        return;
    };
    let Type::Function {
        params: actual_params,
        variadic: actual_variadic,
        return_type: actual_return,
        ..
    } = actual
    else {
        return;
    };
    if params.len() != actual_params.len() || variadic.is_some() != actual_variadic.is_some() {
        return;
    }
    let input_polarity = polarity.compose(Variance::Contravariant);
    for (template, actual) in params.iter().zip(actual_params) {
        collect_template_constraints(template, actual, input_polarity, adts, contributions);
    }
    if let (Some(template), Some(actual)) = (variadic.as_deref(), actual_variadic.as_deref()) {
        collect_template_constraints(template, actual, input_polarity, adts, contributions);
    }
    collect_template_constraints(return_type, actual_return, polarity, adts, contributions);
}

fn template_has_non_invariant_parameter(
    template: &Type,
    polarity: Variance,
    adts: &AdtRegistry,
) -> bool {
    match template {
        Type::Named { name, args, .. } if name.starts_with("$param") && args.is_empty() => {
            polarity != Variance::Invariant
        }
        Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
            let variances = checked_parameter_variances(identity, args.len(), adts);
            args.iter().zip(variances).any(|(argument, nested)| {
                template_has_non_invariant_parameter(argument, polarity.compose(nested), adts)
            })
        }
        Type::Record(fields) => fields
            .iter()
            .any(|(_, field)| template_has_non_invariant_parameter(field, polarity, adts)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            let input_polarity = polarity.compose(Variance::Contravariant);
            params.iter().any(|parameter| {
                template_has_non_invariant_parameter(parameter, input_polarity, adts)
            }) || variadic.as_deref().is_some_and(|parameter| {
                template_has_non_invariant_parameter(parameter, input_polarity, adts)
            }) || template_has_non_invariant_parameter(return_type, polarity, adts)
        }
        Type::Unknown => false,
    }
}

fn checked_parameter_variances(identity: &str, arity: usize, adts: &AdtRegistry) -> Vec<Variance> {
    if Type::is_compiler_covariant_identity(identity) {
        return vec![Variance::Covariant; arity];
    }
    adts.parameter_variances_for_identity(identity, arity)
        .filter(|variances| variances.len() == arity)
        .map(<[Variance]>::to_vec)
        .unwrap_or_else(|| vec![Variance::Invariant; arity])
}

fn solve_type_parameter_constraints(
    constraints: &TypeParameterConstraints,
    fallback: Option<&Type>,
    adts: &AdtRegistry,
) -> Result<Option<Type>, unification::TypeParameterContributionConflict> {
    let selected = select_constraint_candidate(constraints)
        .or_else(|| fallback.cloned().map(CanonicalConstraint::new));
    selected
        .map(|selected| validate_constraint_bounds(constraints, selected, adts))
        .transpose()
}

fn select_constraint_candidate(
    constraints: &TypeParameterConstraints,
) -> Option<CanonicalConstraint> {
    if let Some(equality) = &constraints.equality {
        Some(equality.clone())
    } else if constraints.lower.has_constraints() {
        constraints.lower.selected.clone()
    } else if constraints.upper.has_constraints() {
        constraints.upper.selected.clone()
    } else {
        None
    }
}

fn validate_constraint_bounds(
    constraints: &TypeParameterConstraints,
    mut selected: CanonicalConstraint,
    adts: &AdtRegistry,
) -> Result<Type, unification::TypeParameterContributionConflict> {
    for lower in constraints.lower.refs() {
        record_work(1);
        if !is_assignable_nested_with_registry(&selected.ty, lower, adts) {
            return Err(type_parameter_constraint_conflict(&selected.ty, lower));
        }
    }
    for upper in constraints.upper.refs() {
        record_work(1);
        if !is_assignable_nested_with_registry(upper, &selected.ty, adts) {
            return Err(type_parameter_constraint_conflict(upper, &selected.ty));
        }
    }
    reconcile_constraint_presentations(constraints, &mut selected, adts);
    Ok(selected.ty)
}

fn reconcile_constraint_presentations(
    constraints: &TypeParameterConstraints,
    selected: &mut CanonicalConstraint,
    adts: &AdtRegistry,
) {
    let contributions = constraints
        .lower
        .constraints()
        .into_iter()
        .chain(constraints.upper.constraints())
        .chain(constraints.equality.iter());
    for contribution in contributions {
        record_work(1);
        if is_assignable_nested_with_registry(&selected.ty, &contribution.ty, adts)
            && is_assignable_nested_with_registry(&contribution.ty, &selected.ty, adts)
        {
            selected.merge(contribution.clone());
        }
    }
}

fn type_parameter_constraint_conflict(
    expected: &Type,
    actual: &Type,
) -> unification::TypeParameterContributionConflict {
    unification::TypeParameterContributionConflict {
        expected: Box::new(expected.clone()),
        actual: Box::new(actual.clone()),
    }
}

fn type_shape(ty: &Type) -> (usize, bool) {
    type_shape_with_setup_work(ty, false)
}

fn setup_type_shape(ty: &Type) -> (usize, bool) {
    type_shape_with_setup_work(ty, true)
}

fn type_shape_with_setup_work(ty: &Type, setup_work: bool) -> (usize, bool) {
    let mut nodes = 0usize;
    let mut contains_unknown = false;
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        nodes += 1;
        if setup_work {
            record_setup_type_node();
        }
        match ty {
            Type::Unknown => contains_unknown = true,
            Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
                pending.extend(args);
            }
            Type::Record(fields) => pending.extend(fields.iter().map(|(_, field)| field)),
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            } => {
                pending.extend(params);
                pending.extend(variadic.as_deref());
                pending.push(return_type);
            }
        }
    }
    (nodes, contains_unknown)
}
