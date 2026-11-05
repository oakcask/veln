use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::semantic_model::Type;

use super::registry::DescriptorKey;

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
}

#[derive(Clone, Copy)]
enum MetadataEntry {
    Unique(usize),
    Ambiguous,
}

struct MetadataIndex {
    by_identity_and_arity: HashMap<String, HashMap<usize, MetadataEntry>>,
}

impl MetadataIndex {
    fn new(
        descriptors: &[super::descriptors::AdtDescriptor],
        alias_identities: &BTreeSet<DescriptorKey>,
    ) -> Self {
        let mut by_identity_and_arity = HashMap::<String, HashMap<usize, MetadataEntry>>::new();
        for (index, descriptor) in descriptors.iter().enumerate() {
            if alias_identities.contains(&DescriptorKey::new(descriptor)) {
                continue;
            }
            by_identity_and_arity
                .entry(descriptor.identity())
                .or_default()
                .entry(descriptor.type_parameters.len())
                .and_modify(|entry| *entry = MetadataEntry::Ambiguous)
                .or_insert(MetadataEntry::Unique(index));
        }
        Self {
            by_identity_and_arity,
        }
    }

    fn resolve(&self, identity: &str, arity: usize) -> Option<usize> {
        match self.by_identity_and_arity.get(identity)?.get(&arity)? {
            MetadataEntry::Unique(index) => Some(*index),
            MetadataEntry::Ambiguous => None,
        }
    }

    fn checked_variances(
        &self,
        identity: &str,
        arity: usize,
        descriptors: &[super::descriptors::AdtDescriptor],
    ) -> Vec<Variance> {
        let Some(descriptor) = self
            .resolve(identity, arity)
            .and_then(|index| descriptors.get(index))
        else {
            return vec![Variance::Invariant; arity];
        };
        if descriptor.parameter_variances.len() != arity {
            return vec![Variance::Invariant; arity];
        }
        descriptor.parameter_variances.clone()
    }
}

#[derive(Clone)]
enum VarianceFactor {
    Fixed(Variance),
    SourceParameter {
        descriptor_index: usize,
        parameter_index: usize,
    },
}

struct ParameterConstraint {
    parameter_index: usize,
    factors: Vec<VarianceFactor>,
}

pub(super) fn infer_source_parameter_variances(
    descriptors: &mut [super::descriptors::AdtDescriptor],
    source_start: usize,
    alias_identities: &BTreeSet<DescriptorKey>,
) {
    for descriptor in &mut descriptors[source_start..] {
        descriptor.parameter_variances =
            vec![Variance::Irrelevant; descriptor.type_parameters.len()];
    }

    let metadata = MetadataIndex::new(descriptors, alias_identities);
    let source_count = descriptors.len().saturating_sub(source_start);
    let mut constraints = Vec::with_capacity(source_count);
    let mut reverse_dependencies = vec![BTreeSet::new(); source_count];
    for descriptor_index in source_start..descriptors.len() {
        let (descriptor_constraints, dependencies) =
            collect_constraints(descriptor_index, source_start, descriptors, &metadata);
        let dependent_offset = descriptor_index - source_start;
        for dependency in dependencies {
            reverse_dependencies[dependency - source_start].insert(dependent_offset);
        }
        constraints.push(descriptor_constraints);
    }

    let mut queued = vec![true; source_count];
    let mut worklist = (0..source_count).collect::<VecDeque<_>>();
    while let Some(offset) = worklist.pop_front() {
        queued[offset] = false;
        #[cfg(test)]
        variance_solver_counters::record_evaluation();
        let inferred = evaluate_constraints(
            descriptors[source_start + offset].type_parameters.len(),
            &constraints[offset],
            descriptors,
        );
        if descriptors[source_start + offset].parameter_variances == inferred {
            continue;
        }
        descriptors[source_start + offset].parameter_variances = inferred;
        for dependent in &reverse_dependencies[offset] {
            if !queued[*dependent] {
                queued[*dependent] = true;
                worklist.push_back(*dependent);
            }
        }
    }

    synchronize_alias_variances(descriptors, alias_identities, &metadata);
}

fn collect_constraints(
    descriptor_index: usize,
    source_start: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
    metadata: &MetadataIndex,
) -> (Vec<ParameterConstraint>, BTreeSet<usize>) {
    let descriptor = &descriptors[descriptor_index];
    let mut constraints = Vec::new();
    let mut dependencies = BTreeSet::new();
    for variant in &descriptor.variants {
        for field in &variant.payload_fields {
            match &field.ty {
                super::descriptors::AdtPayloadType::TypeParameter(parameter_index) => {
                    constraints.push(ParameterConstraint {
                        parameter_index: *parameter_index,
                        factors: Vec::new(),
                    });
                }
                super::descriptors::AdtPayloadType::SelfType => {
                    for parameter_index in 0..descriptor.type_parameters.len() {
                        dependencies.insert(descriptor_index);
                        constraints.push(ParameterConstraint {
                            parameter_index,
                            factors: vec![VarianceFactor::SourceParameter {
                                descriptor_index,
                                parameter_index,
                            }],
                        });
                    }
                }
                super::descriptors::AdtPayloadType::Concrete(ty) => collect_type_constraints(
                    ty,
                    &[],
                    source_start,
                    descriptors,
                    metadata,
                    &mut constraints,
                    &mut dependencies,
                ),
            }
        }
    }
    (constraints, dependencies)
}

fn collect_type_constraints(
    ty: &Type,
    factors: &[VarianceFactor],
    source_start: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
    metadata: &MetadataIndex,
    constraints: &mut Vec<ParameterConstraint>,
    dependencies: &mut BTreeSet<usize>,
) {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        } if args.is_empty() => {
            if let Some(parameter_index) =
                parameter_placeholder_index(name).or_else(|| parameter_placeholder_index(identity))
            {
                constraints.push(ParameterConstraint {
                    parameter_index,
                    factors: factors.to_vec(),
                });
            }
        }
        Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
            let resolved = metadata.resolve(identity, args.len());
            let valid_metadata = resolved
                .filter(|index| descriptors[*index].parameter_variances.len() == args.len());
            for (parameter_index, argument) in args.iter().enumerate() {
                let factor = match valid_metadata {
                    Some(descriptor_index) if descriptor_index >= source_start => {
                        dependencies.insert(descriptor_index);
                        VarianceFactor::SourceParameter {
                            descriptor_index,
                            parameter_index,
                        }
                    }
                    Some(descriptor_index) => VarianceFactor::Fixed(
                        descriptors[descriptor_index].parameter_variances[parameter_index],
                    ),
                    None => VarianceFactor::Fixed(Variance::Invariant),
                };
                let mut nested_factors = factors.to_vec();
                nested_factors.push(factor);
                collect_type_constraints(
                    argument,
                    &nested_factors,
                    source_start,
                    descriptors,
                    metadata,
                    constraints,
                    dependencies,
                );
            }
        }
        Type::Record(fields) => {
            for (_, field) in fields {
                collect_type_constraints(
                    field,
                    factors,
                    source_start,
                    descriptors,
                    metadata,
                    constraints,
                    dependencies,
                );
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            let mut input_factors = factors.to_vec();
            input_factors.push(VarianceFactor::Fixed(Variance::Contravariant));
            for parameter in params {
                collect_type_constraints(
                    parameter,
                    &input_factors,
                    source_start,
                    descriptors,
                    metadata,
                    constraints,
                    dependencies,
                );
            }
            if let Some(variadic) = variadic {
                collect_type_constraints(
                    variadic,
                    &input_factors,
                    source_start,
                    descriptors,
                    metadata,
                    constraints,
                    dependencies,
                );
            }
            collect_type_constraints(
                return_type,
                factors,
                source_start,
                descriptors,
                metadata,
                constraints,
                dependencies,
            );
        }
        Type::Unknown => {}
    }
}

fn evaluate_constraints(
    parameter_count: usize,
    constraints: &[ParameterConstraint],
    descriptors: &[super::descriptors::AdtDescriptor],
) -> Vec<Variance> {
    let mut inferred = vec![Variance::Irrelevant; parameter_count];
    for constraint in constraints {
        let occurrence = constraint
            .factors
            .iter()
            .fold(Variance::Covariant, |position, factor| {
                let nested = match factor {
                    VarianceFactor::Fixed(variance) => *variance,
                    VarianceFactor::SourceParameter {
                        descriptor_index,
                        parameter_index,
                    } => descriptors
                        .get(*descriptor_index)
                        .and_then(|descriptor| descriptor.parameter_variances.get(*parameter_index))
                        .copied()
                        .unwrap_or(Variance::Invariant),
                };
                position.compose(nested)
            });
        join_parameter(&mut inferred, constraint.parameter_index, occurrence);
    }
    inferred
}

fn synchronize_alias_variances(
    descriptors: &mut [super::descriptors::AdtDescriptor],
    alias_identities: &BTreeSet<DescriptorKey>,
    metadata: &MetadataIndex,
) {
    let updates = descriptors
        .iter()
        .enumerate()
        .filter(|(_, descriptor)| alias_identities.contains(&DescriptorKey::new(descriptor)))
        .filter_map(|(index, descriptor)| {
            let target_identity = descriptor.nominal_identity.as_deref()?;
            Some((
                index,
                metadata.checked_variances(
                    target_identity,
                    descriptor.type_parameters.len(),
                    descriptors,
                ),
            ))
        })
        .collect::<Vec<_>>();
    for (index, variances) in updates {
        descriptors[index].parameter_variances = variances;
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

#[cfg(test)]
pub(crate) mod variance_solver_counters {
    use std::cell::Cell;

    thread_local! {
        static EVALUATIONS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        EVALUATIONS.set(0);
    }

    pub(super) fn record_evaluation() {
        EVALUATIONS.set(EVALUATIONS.get() + 1);
    }

    pub(crate) fn evaluations() -> usize {
        EVALUATIONS.get()
    }
}
