use crate::semantic_model::Type;

pub(crate) use super::descriptors::Variance;

impl Variance {
    pub(crate) const fn compose(self, nested: Self) -> Self {
        use Variance::{Contravariant, Covariant, Invariant, Irrelevant};
        match (self, nested) {
            (Covariant, variance) => variance,
            (Contravariant, Covariant) => Contravariant,
            (Contravariant, Contravariant) => Covariant,
            (Contravariant, Invariant) => Invariant,
            (Contravariant, Irrelevant) => Irrelevant,
            (Invariant, _) => Invariant,
            (Irrelevant, _) => Irrelevant,
        }
    }

    pub(crate) const fn join(self, occurrence: Self) -> Self {
        use Variance::{Contravariant, Covariant, Invariant, Irrelevant};
        match (self, occurrence) {
            (Irrelevant, variance) | (variance, Irrelevant) => variance,
            (Covariant, Covariant) => Covariant,
            (Contravariant, Contravariant) => Contravariant,
            (Invariant, _)
            | (_, Invariant)
            | (Covariant, Contravariant)
            | (Contravariant, Covariant) => Invariant,
        }
    }

    const fn reverse(self) -> Self {
        self.compose(Self::Contravariant)
    }
}

pub(super) fn infer_source_parameter_variances(
    descriptors: &mut [super::descriptors::AdtDescriptor],
    source_start: usize,
) {
    for descriptor in &mut descriptors[source_start..] {
        descriptor.parameter_variances =
            vec![Variance::Irrelevant; descriptor.type_parameters.len()];
    }

    loop {
        let snapshot = descriptors.to_vec();
        let mut changed = false;
        for (offset, descriptor) in snapshot[source_start..].iter().enumerate() {
            let mut inferred = vec![Variance::Irrelevant; descriptor.type_parameters.len()];
            for variant in &descriptor.variants {
                for field in &variant.payload_fields {
                    match &field.ty {
                        super::descriptors::AdtPayloadType::TypeParameter(index) => {
                            join_parameter(&mut inferred, *index, Variance::Covariant);
                        }
                        super::descriptors::AdtPayloadType::SelfType => {
                            for (index, variance) in
                                descriptor.parameter_variances.iter().enumerate()
                            {
                                join_parameter(&mut inferred, index, *variance);
                            }
                        }
                        super::descriptors::AdtPayloadType::Concrete(ty) => {
                            collect_type_occurrences(
                                ty,
                                Variance::Covariant,
                                &snapshot,
                                &mut inferred,
                            );
                        }
                    }
                }
            }
            let target = &mut descriptors[source_start + offset].parameter_variances;
            if *target != inferred {
                *target = inferred;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    synchronize_alias_variances(descriptors);
}

fn collect_type_occurrences(
    ty: &Type,
    position: Variance,
    descriptors: &[super::descriptors::AdtDescriptor],
    inferred: &mut [Variance],
) {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        } if args.is_empty() => {
            if let Some(index) = parameter_placeholder_index(name) {
                join_parameter(inferred, index, position);
            } else if let Some(index) = parameter_placeholder_index(identity) {
                join_parameter(inferred, index, position);
            }
        }
        Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
            let nested = checked_variances(identity, args.len(), descriptors);
            for (argument, variance) in args.iter().zip(nested) {
                collect_type_occurrences(
                    argument,
                    position.compose(variance),
                    descriptors,
                    inferred,
                );
            }
        }
        Type::Record(fields) => {
            for (_, field) in fields {
                collect_type_occurrences(field, position, descriptors, inferred);
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            let input_position = position.reverse();
            for parameter in params {
                collect_type_occurrences(parameter, input_position, descriptors, inferred);
            }
            if let Some(variadic) = variadic {
                collect_type_occurrences(variadic, input_position, descriptors, inferred);
            }
            collect_type_occurrences(return_type, position, descriptors, inferred);
        }
        Type::Unknown => {}
    }
}

fn checked_variances(
    identity: &str,
    arity: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
) -> Vec<Variance> {
    if Type::is_compiler_covariant_identity(identity) {
        return vec![Variance::Covariant; arity];
    }
    let matching = descriptors
        .iter()
        .filter(|descriptor| {
            descriptor.identity() == identity && descriptor.type_parameters.len() == arity
        })
        .collect::<Vec<_>>();
    let canonical = matching
        .iter()
        .copied()
        .filter(|descriptor| descriptor.nominal_identity.is_none())
        .collect::<Vec<_>>();
    let descriptor = match canonical.as_slice() {
        [descriptor] => *descriptor,
        [] if matching.len() == 1 => matching[0],
        _ => return vec![Variance::Invariant; arity],
    };
    if descriptor.parameter_variances.len() != arity {
        return vec![Variance::Invariant; arity];
    }
    descriptor.parameter_variances.clone()
}

fn synchronize_alias_variances(descriptors: &mut [super::descriptors::AdtDescriptor]) {
    let snapshot = descriptors.to_vec();
    for descriptor in descriptors {
        let Some(target_identity) = descriptor.nominal_identity.as_deref() else {
            continue;
        };
        let mut targets = snapshot.iter().filter(|target| {
            target.nominal_identity.is_none()
                && target.identity() == target_identity
                && target.type_parameters.len() == descriptor.type_parameters.len()
        });
        let Some(target) = targets.next() else {
            continue;
        };
        if targets.next().is_none()
            && target.parameter_variances.len() == descriptor.type_parameters.len()
        {
            descriptor
                .parameter_variances
                .clone_from(&target.parameter_variances);
        }
    }
}

fn parameter_placeholder_index(name: &str) -> Option<usize> {
    name.strip_prefix("$param")?.parse().ok()
}

fn join_parameter(inferred: &mut [Variance], index: usize, occurrence: Variance) {
    if let Some(current) = inferred.get_mut(index) {
        *current = current.join(occurrence);
    }
}
