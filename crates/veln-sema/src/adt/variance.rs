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

const COVARIANT_2: [Variance; 2] = [Variance::Covariant; 2];

fn compiler_owned_variances(identity: &str, arity: usize) -> Option<&'static [Variance]> {
    let expected: &'static [Variance] = match identity {
        Type::COMPILER_VEC_IDENTITY => &[Variance::Covariant],
        Type::COMPILER_DICT_IDENTITY => &COVARIANT_2,
        _ => return None,
    };
    (expected.len() == arity).then_some(expected)
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
        if let Some(variances) = compiler_owned_variances(identity, arity) {
            return variances.to_vec();
        }
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
    factor_path: Option<usize>,
}

struct FactorPathNode {
    parent: Option<usize>,
    factor: VarianceFactor,
}

struct DescriptorConstraints {
    parameters: Vec<ParameterConstraint>,
    factor_paths: Vec<FactorPathNode>,
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
) -> (DescriptorConstraints, BTreeSet<usize>) {
    let descriptor = &descriptors[descriptor_index];
    let mut constraints = Vec::new();
    let mut factor_paths = Vec::new();
    let mut dependencies = BTreeSet::new();
    for variant in &descriptor.variants {
        for field in &variant.payload_fields {
            match &field.ty {
                super::descriptors::AdtPayloadType::TypeParameter(parameter_index) => {
                    constraints.push(ParameterConstraint {
                        parameter_index: *parameter_index,
                        factor_path: None,
                    });
                }
                super::descriptors::AdtPayloadType::SelfType => {
                    for parameter_index in 0..descriptor.type_parameters.len() {
                        dependencies.insert(descriptor_index);
                        let factor_path = push_factor_path(
                            &mut factor_paths,
                            None,
                            VarianceFactor::SourceParameter {
                                descriptor_index,
                                parameter_index,
                            },
                        );
                        constraints.push(ParameterConstraint {
                            parameter_index,
                            factor_path: Some(factor_path),
                        });
                    }
                }
                super::descriptors::AdtPayloadType::Concrete(ty) => collect_type_constraints(
                    ty,
                    source_start,
                    descriptors,
                    metadata,
                    &mut constraints,
                    &mut factor_paths,
                    &mut dependencies,
                ),
            }
        }
    }
    (
        DescriptorConstraints {
            parameters: constraints,
            factor_paths,
        },
        dependencies,
    )
}

fn collect_type_constraints(
    ty: &Type,
    source_start: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
    metadata: &MetadataIndex,
    constraints: &mut Vec<ParameterConstraint>,
    factor_paths: &mut Vec<FactorPathNode>,
    dependencies: &mut BTreeSet<usize>,
) {
    let context = ConstraintMetadata {
        source_start,
        descriptors,
        metadata,
    };
    let mut storage = ConstraintStorage {
        factor_paths,
        dependencies,
    };
    let mut work = vec![(ty, None)];
    while let Some((ty, factor_path)) = work.pop() {
        #[cfg(test)]
        variance_solver_counters::record_collection_visit();
        match ty {
            Type::Named {
                name,
                identity,
                args,
            } if args.is_empty() => {
                if let Some(parameter_index) = parameter_placeholder_index(name)
                    .or_else(|| parameter_placeholder_index(identity))
                {
                    constraints.push(ParameterConstraint {
                        parameter_index,
                        factor_path,
                    });
                }
            }
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                push_named_argument_constraints(
                    identity,
                    args,
                    factor_path,
                    &context,
                    &mut storage,
                    &mut work,
                );
            }
            Type::Record(fields) => {
                for (_, field) in fields.iter().rev() {
                    work.push((field, factor_path));
                }
            }
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            } => {
                work.push((return_type, factor_path));
                let input_path = push_factor_path(
                    storage.factor_paths,
                    factor_path,
                    VarianceFactor::Fixed(Variance::Contravariant),
                );
                if let Some(variadic) = variadic {
                    work.push((variadic, Some(input_path)));
                }
                for parameter in params.iter().rev() {
                    work.push((parameter, Some(input_path)));
                }
            }
            Type::Unknown => {}
        }
    }
}

struct ConstraintMetadata<'a> {
    source_start: usize,
    descriptors: &'a [super::descriptors::AdtDescriptor],
    metadata: &'a MetadataIndex,
}

struct ConstraintStorage<'a> {
    factor_paths: &'a mut Vec<FactorPathNode>,
    dependencies: &'a mut BTreeSet<usize>,
}

fn push_named_argument_constraints<'a>(
    identity: &str,
    args: &'a [Type],
    factor_path: Option<usize>,
    context: &ConstraintMetadata<'_>,
    storage: &mut ConstraintStorage<'_>,
    work: &mut Vec<(&'a Type, Option<usize>)>,
) {
    let fixed = compiler_owned_variances(identity, args.len());
    let resolved = fixed
        .is_none()
        .then(|| context.metadata.resolve(identity, args.len()))
        .flatten();
    let valid_metadata = resolved
        .filter(|index| context.descriptors[*index].parameter_variances.len() == args.len());
    for (parameter_index, argument) in args.iter().enumerate().rev() {
        let factor = match (fixed, valid_metadata) {
            (Some(variances), _) => VarianceFactor::Fixed(variances[parameter_index]),
            (None, Some(descriptor_index)) if descriptor_index >= context.source_start => {
                storage.dependencies.insert(descriptor_index);
                VarianceFactor::SourceParameter {
                    descriptor_index,
                    parameter_index,
                }
            }
            (None, Some(descriptor_index)) => VarianceFactor::Fixed(
                context.descriptors[descriptor_index].parameter_variances[parameter_index],
            ),
            (None, None) => VarianceFactor::Fixed(Variance::Invariant),
        };
        let nested_path = push_factor_path(storage.factor_paths, factor_path, factor);
        work.push((argument, Some(nested_path)));
    }
}

fn push_factor_path(
    factor_paths: &mut Vec<FactorPathNode>,
    parent: Option<usize>,
    factor: VarianceFactor,
) -> usize {
    let index = factor_paths.len();
    factor_paths.push(FactorPathNode { parent, factor });
    #[cfg(test)]
    variance_solver_counters::record_retained_factor();
    index
}

fn evaluate_constraints(
    parameter_count: usize,
    constraints: &DescriptorConstraints,
    descriptors: &[super::descriptors::AdtDescriptor],
) -> Vec<Variance> {
    let mut inferred = vec![Variance::Irrelevant; parameter_count];
    let mut factor_values = Vec::with_capacity(constraints.factor_paths.len());
    for path in &constraints.factor_paths {
        let position = path
            .parent
            .map(|parent| factor_values[parent])
            .unwrap_or(Variance::Covariant);
        let nested = match &path.factor {
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
        factor_values.push(position.compose(nested));
    }
    for constraint in &constraints.parameters {
        let occurrence = constraint
            .factor_path
            .map(|path| factor_values[path])
            .unwrap_or(Variance::Covariant);
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
        static COLLECTION_VISITS: Cell<usize> = const { Cell::new(0) };
        static RETAINED_FACTORS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        EVALUATIONS.set(0);
        COLLECTION_VISITS.set(0);
        RETAINED_FACTORS.set(0);
    }

    pub(super) fn record_evaluation() {
        EVALUATIONS.set(EVALUATIONS.get() + 1);
    }

    pub(crate) fn evaluations() -> usize {
        EVALUATIONS.get()
    }

    pub(super) fn record_collection_visit() {
        COLLECTION_VISITS.set(COLLECTION_VISITS.get() + 1);
    }

    pub(super) fn record_retained_factor() {
        RETAINED_FACTORS.set(RETAINED_FACTORS.get() + 1);
    }

    pub(crate) fn collection_visits() -> usize {
        COLLECTION_VISITS.get()
    }

    pub(crate) fn retained_factors() -> usize {
        RETAINED_FACTORS.get()
    }
}
