#[cfg(test)]
fn type_retained_units(ty: &Type) -> usize {
    let mut units = 0;
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        units += 1;
        match ty {
            Type::Unknown => {}
            Type::Named {
                name,
                identity,
                args,
            } => {
                units += name.len() + identity.len();
                pending.extend(args);
            }
            Type::VariantRefinement {
                name,
                identity,
                args,
                variants,
                ..
            } => {
                units +=
                    name.len() + identity.len() + variants.iter().map(String::len).sum::<usize>();
                pending.extend(args);
            }
            Type::Record(fields) => {
                units += fields.iter().map(|(name, _)| name.len()).sum::<usize>();
                pending.extend(fields.iter().map(|(_, field)| field));
            }
            Type::Function {
                params,
                variadic,
                return_type,
                effects,
            } => {
                units += effects.iter().map(String::len).sum::<usize>();
                pending.extend(params);
                pending.extend(variadic.as_deref());
                pending.push(return_type);
            }
        }
    }
    units
}

fn commit_prepared_contribution(
    node: &mut JoinNode,
    contribution: PreparedContribution<'_>,
) -> bool {
    let mut changed = update_node_presentation(node, &contribution.right);
    changed |= merge_prepared_arguments(node, &contribution.right);
    let right_variants = match contribution.right {
        PreparedTypeParts::Adt(right) => right.variants,
        PreparedTypeParts::Record(_) => None,
    };
    changed | merge_prepared_variants(node, right_variants, contribution.variant_ranks)
}

fn update_node_presentation(node: &mut JoinNode, right: &PreparedTypeParts<'_>) -> bool {
    let PreparedTypeParts::Adt(right) = right else {
        return false;
    };
    record_presentation_work(1);
    if node.presentation_conflicted || right.name == node.canonical_name {
        return false;
    }
    if node.name == node.canonical_name {
        node.name = right.name.to_string();
    } else if node.name != right.name {
        node.name.clone_from(&node.canonical_name);
        node.presentation_conflicted = true;
    } else {
        return false;
    }
    true
}

fn merge_prepared_arguments(node: &mut JoinNode, right: &PreparedTypeParts<'_>) -> bool {
    match right {
        PreparedTypeParts::Adt(right) => merge_join_arguments(node, right.args.iter()),
        PreparedTypeParts::Record(fields) => merge_join_arguments(node, fields.iter().copied()),
    }
}

fn merge_join_arguments<'a>(
    node: &mut JoinNode,
    right_args: impl IntoIterator<Item = &'a Type>,
) -> bool {
    let compiler_covariant = Type::is_compiler_covariant_identity(&node.identity);
    let mut changed = false;
    for (joined, right) in node.args.iter_mut().zip(right_args) {
        let JoinArg::Exact {
            ty,
            presentation,
            covariant,
        } = joined
        else {
            continue;
        };
        if *covariant && compiler_covariant && right == &Type::Unknown {
            continue;
        }
        changed |= unification::merge_type_slot(ty, right);
        changed |= presentation.merge(ty, right);
    }
    changed
}

fn merge_prepared_variants(
    node: &mut JoinNode,
    right_variants: Option<&[String]>,
    variant_ranks: Option<Vec<usize>>,
) -> bool {
    match (&mut node.variants, right_variants) {
        (Some(joined), Some(_)) => {
            let ranks = variant_ranks.expect("validated refinement ranks");
            record_work(ranks.len());
            let previous_len = joined.len();
            joined.extend(ranks);
            joined.len() != previous_len
        }
        (variants @ Some(_), None) => {
            *variants = None;
            true
        }
        (None, _) => false,
    }
}

fn optimized_join_retains_contributions(adts: &AdtRegistry, initial: &Type) -> bool {
    let mut pending = vec![initial];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Record(_) | Type::Function { .. } => return true,
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                if Type::is_compiler_covariant_identity(identity) {
                    pending.extend(args);
                    continue;
                }
                let Some(variances) = adts.parameter_variances_for_identity(identity, args.len())
                else {
                    continue;
                };
                for (argument, variance) in args.iter().zip(variances) {
                    if *variance == Variance::Covariant {
                        pending.push(argument);
                    } else {
                        return true;
                    }
                }
            }
            Type::Unknown => {}
        }
    }
    false
}

fn can_retain_unresolved_contributions(adts: &AdtRegistry, contributions: &[Type]) -> bool {
    let mut pending = vec![contributions.iter().collect::<Vec<_>>()];
    let mut retainable = false;
    while let Some(group) = pending.pop() {
        let Some(group_retainable) = queue_retainable_contribution_group(adts, group, &mut pending)
        else {
            return false;
        };
        retainable |= group_retainable;
    }
    retainable
}

fn queue_retainable_contribution_group<'a>(
    adts: &AdtRegistry,
    group: Vec<&'a Type>,
    pending: &mut Vec<Vec<&'a Type>>,
) -> Option<bool> {
    match group.first().copied()? {
        Type::Function { .. } => Some(true),
        Type::Record(first_fields) => queue_retainable_record_group(first_fields, group, pending),
        Type::Named { .. } | Type::VariantRefinement { .. } => {
            queue_retainable_named_group(adts, group, pending)
        }
        Type::Unknown => None,
    }
}

fn queue_retainable_record_group<'a>(
    first_fields: &[(String, Type)],
    group: Vec<&'a Type>,
    pending: &mut Vec<Vec<&'a Type>>,
) -> Option<bool> {
    let records = group
        .into_iter()
        .map(|contribution| match contribution {
            Type::Record(fields) => Some(fields.as_slice()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if records.iter().skip(1).any(|fields| {
        fields.len() != first_fields.len()
            || first_fields
                .iter()
                .any(|(name, _)| !fields.iter().any(|(other, _)| other == name))
    }) {
        return Some(true);
    }
    for (name, _) in first_fields {
        pending.push(
            records
                .iter()
                .map(|fields| {
                    &fields
                        .iter()
                        .find(|(other, _)| other == name)
                        .expect("validated record field set")
                        .1
                })
                .collect(),
        );
    }
    Some(false)
}

fn queue_retainable_named_group<'a>(
    adts: &AdtRegistry,
    group: Vec<&'a Type>,
    pending: &mut Vec<Vec<&'a Type>>,
) -> Option<bool> {
    let first = adt_type_parts(group.first().copied()?).expect("named contribution parts");
    let parts = group
        .into_iter()
        .map(adt_type_parts)
        .collect::<Option<Vec<_>>>()?;
    if parts.iter().any(|part| !same_adt_shape(first, *part)) {
        return None;
    }
    if Type::is_compiler_covariant_identity(first.identity) {
        queue_named_argument_groups(&parts, 0..first.args.len(), pending);
        return Some(false);
    }
    let Some(variances) = adts.parameter_variances_for_identity(first.identity, first.args.len())
    else {
        return parts
            .iter()
            .skip(1)
            .all(|part| invariant_args_match(first.args, part.args))
            .then_some(false);
    };
    if !named_exact_arguments_match(&parts, variances) {
        return None;
    }
    let covariant = variances
        .iter()
        .enumerate()
        .filter_map(|(argument, variance)| (*variance == Variance::Covariant).then_some(argument));
    queue_named_argument_groups(&parts, covariant, pending);
    Some(variances.contains(&Variance::Contravariant))
}

fn named_exact_arguments_match(parts: &[AdtTypeParts<'_>], variances: &[Variance]) -> bool {
    variances.iter().enumerate().all(|(argument, variance)| {
        !matches!(variance, Variance::Invariant | Variance::Irrelevant)
            || parts.iter().skip(1).all(|part| {
                invariant_args_match(
                    std::slice::from_ref(&parts[0].args[argument]),
                    std::slice::from_ref(&part.args[argument]),
                )
            })
    })
}

fn same_adt_shape(first: AdtTypeParts<'_>, other: AdtTypeParts<'_>) -> bool {
    same_type_identity(first.name, first.identity, other.name, other.identity)
        && first.args.len() == other.args.len()
}

fn queue_named_argument_groups<'a>(
    parts: &[AdtTypeParts<'a>],
    arguments: impl IntoIterator<Item = usize>,
    pending: &mut Vec<Vec<&'a Type>>,
) {
    for argument in arguments {
        pending.push(parts.iter().map(|part| &part.args[argument]).collect());
    }
}

fn queue_covariant_contributions<'a>(
    node: &JoinNode,
    right_args: impl IntoIterator<Item = &'a Type>,
    pending: &mut Vec<(usize, &'a Type)>,
) -> Option<()> {
    let compiler_covariant = Type::is_compiler_covariant_identity(&node.identity);
    for (joined, right) in node.args.iter().zip(right_args) {
        match joined {
            JoinArg::Exact {
                ty,
                covariant: true,
                ..
            } if compiler_covariant && (ty == &Type::Unknown || right == &Type::Unknown) => {}
            JoinArg::Exact { ty, .. } => {
                invariant_args_match(std::slice::from_ref(ty), std::slice::from_ref(right))
                    .then_some(())?
            }
            JoinArg::Covariant(_) if compiler_covariant && right == &Type::Unknown => {}
            JoinArg::Covariant(child) => pending.push((*child, right)),
        }
    }
    Some(())
}

fn joined_variant_ranks(
    node: &JoinNode,
    right_variants: Option<&[String]>,
) -> Option<Option<Vec<usize>>> {
    match (&node.variants, right_variants) {
        (Some(_), Some(right_variants)) => {
            let declaration_order = node
                .declaration_order
                .as_ref()
                .expect("a refinement join retains declaration order");
            Some(Some(
                right_variants
                    .iter()
                    .map(|variant| declaration_order.rank(variant))
                    .collect::<Option<Vec<_>>>()?,
            ))
        }
        (Some(_), None) => Some(None),
        (None, _) => Some(Some(Vec::new())),
    }
}

fn materialize_args(node: &JoinNode, materialized: &mut [Option<Type>]) -> Vec<Type> {
    node.args
        .iter()
        .map(|argument| match argument {
            JoinArg::Exact { ty, .. } => ty.clone(),
            JoinArg::Covariant(child) => materialized[*child]
                .take()
                .expect("covariant child is materialized before its parent"),
        })
        .collect()
}

fn materialize_named_node(node: &JoinNode, args: Vec<Type>) -> Type {
    Type::resolved_named(node.name.clone(), node.identity.clone(), args)
}

fn materialize_refined_node(
    node: &JoinNode,
    args: Vec<Type>,
    joined_variants: &HashSet<usize>,
) -> Type {
    record_work(joined_variants.len());
    let declaration_order = node
        .declaration_order
        .as_ref()
        .expect("a refinement join retains declaration order");
    let mut ranks = joined_variants.iter().copied().collect::<Vec<_>>();
    ranks.sort_unstable();
    let variants = ranks
        .into_iter()
        .map(|rank| {
            declaration_order
                .name(rank)
                .expect("joined variant rank belongs to its declaration")
                .to_string()
        })
        .collect();
    Type::resolved_variant_refinement(node.name.clone(), node.identity.clone(), args, variants)
}

impl Drop for AggregateTypeJoin {
    fn drop(&mut self) {
        if let Some(contributions) = &mut self.optimized_contributions {
            drop_types_iteratively(contributions);
        }
        drop_join_nodes_iteratively(&mut self.nodes);
    }
}

fn drop_join_nodes_iteratively(nodes: &mut [JoinNode]) {
    for node in nodes {
        for argument in &mut node.args {
            if let JoinArg::Exact { ty, .. } = argument {
                crate::semantic_model::drop_type_iteratively(std::mem::replace(ty, Type::Unknown));
            }
        }
    }
}

fn drop_types_iteratively(types: &mut Vec<Type>) {
    for ty in types.drain(..) {
        crate::semantic_model::drop_type_iteratively(ty);
    }
}

fn join_node<'a>(
    adts: &AdtRegistry,
    initial: &'a Type,
    record_subtree_is_complete: bool,
) -> Option<JoinNodeSetup<'a>> {
    if let Type::Record(fields) = initial {
        if !record_subtree_is_complete && setup_type_shape(initial).1 {
            return None;
        }
        return record_join_node(fields);
    }
    let initial = adt_type_parts(initial)?;
    let variant_state = initial_variant_state(adts, initial)?;
    let (args, pending) = initial_argument_state(adts, initial, record_subtree_is_complete);
    Some((
        JoinNode {
            name: initial.name.to_string(),
            canonical_name: Type::compiler_covariant_name(initial.identity)
                .map(str::to_string)
                .or_else(|| {
                    adts.descriptor_for_type(initial.ty)
                        .map(|descriptor| adts.canonical_type_name_for_descriptor(descriptor))
                })
                .unwrap_or_else(|| initial.name.to_string()),
            presentation_conflicted: false,
            identity: initial.identity.to_string(),
            args,
            declaration_order: variant_state.declaration_order,
            variants: variant_state.variants,
            record_field_names: None,
        },
        pending,
    ))
}

fn record_join_node<'a>(record_fields: &'a [(String, Type)]) -> Option<JoinNodeSetup<'a>> {
    let mut fields = record_fields.iter().collect::<Vec<_>>();
    fields.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if fields.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return None;
    }
    let record_field_names = fields
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let mut pending = Vec::new();
    let args = fields
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let ty: &'a Type = &fields[index].1;
            if join_type_parts(ty).is_some() {
                pending.push((index, ty, true));
                JoinArg::Covariant(usize::MAX)
            } else {
                JoinArg::Exact {
                    ty: ty.clone(),
                    presentation: TypePresentationJoin::default(),
                    covariant: false,
                }
            }
        })
        .collect();
    Some((
        JoinNode {
            name: String::new(),
            canonical_name: String::new(),
            presentation_conflicted: false,
            identity: String::new(),
            args,
            declaration_order: None,
            variants: None,
            record_field_names: Some(record_field_names),
        },
        pending,
    ))
}

fn initial_variant_state(
    adts: &AdtRegistry,
    initial: AdtTypeParts<'_>,
) -> Option<InitialVariantState> {
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
    Some(InitialVariantState {
        declaration_order,
        variants,
    })
}

fn initial_argument_state<'a>(
    adts: &AdtRegistry,
    initial: AdtTypeParts<'a>,
    record_subtree_is_complete: bool,
) -> (Vec<JoinArg>, Vec<(usize, &'a Type, bool)>) {
    let compiler_covariant = Type::is_compiler_covariant_identity(initial.identity);
    let variances = (!compiler_covariant)
        .then(|| adts.parameter_variances_for_identity(initial.identity, initial.args.len()))
        .flatten();
    let mut pending = Vec::new();
    let args = initial
        .args
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let variance = if compiler_covariant {
                Variance::Covariant
            } else {
                variances
                    .and_then(|variances| variances.get(index).copied())
                    .unwrap_or(Variance::Invariant)
            };
            if variance == Variance::Covariant && join_type_parts(ty).is_some() {
                pending.push((index, ty, record_subtree_is_complete));
                JoinArg::Covariant(usize::MAX)
            } else {
                JoinArg::Exact {
                    ty: ty.clone(),
                    presentation: TypePresentationJoin::default(),
                    covariant: variance == Variance::Covariant,
                }
            }
        })
        .collect();
    (args, pending)
}
