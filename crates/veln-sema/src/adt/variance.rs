use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ResolvedConstructorUse {
    pub(crate) owner_descriptor_index: usize,
    pub(crate) name: String,
    pub(crate) identity: String,
    pub(crate) arity: usize,
}

impl MetadataIndex {
    fn new(
        descriptors: &[super::descriptors::AdtDescriptor],
        alias_targets: &BTreeMap<DescriptorKey, String>,
    ) -> Self {
        let mut by_identity_and_arity = HashMap::<String, HashMap<usize, MetadataEntry>>::new();
        for (index, descriptor) in descriptors.iter().enumerate() {
            if alias_targets.contains_key(&DescriptorKey::new(descriptor)) {
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

struct FactorPathNode {
    parent: Option<usize>,
    children: Vec<usize>,
    factor: VarianceFactor,
    composed: Variance,
    target_parameters: BTreeSet<usize>,
}

struct DescriptorConstraints {
    direct_occurrences: Vec<Variance>,
    factor_paths: Vec<FactorPathNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ParameterNode {
    descriptor_index: usize,
    parameter_index: usize,
}

#[derive(Clone, Copy)]
struct FactorNode {
    descriptor_offset: usize,
    factor_index: usize,
}

pub(super) fn infer_source_parameter_variances(
    descriptors: &mut [super::descriptors::AdtDescriptor],
    source_start: usize,
    alias_targets: &BTreeMap<DescriptorKey, String>,
    resolved_constructor_uses: &BTreeSet<ResolvedConstructorUse>,
) {
    for descriptor in &mut descriptors[source_start..] {
        descriptor.parameter_variances =
            vec![Variance::Irrelevant; descriptor.type_parameters.len()];
    }

    let metadata = MetadataIndex::new(descriptors, alias_targets);
    let source_count = descriptors.len().saturating_sub(source_start);
    let mut constraints = Vec::with_capacity(source_count);
    for descriptor_index in source_start..descriptors.len() {
        constraints.push(collect_constraints(
            descriptor_index,
            source_start,
            descriptors,
            &metadata,
            resolved_constructor_uses,
        ));
    }

    let factor_dependencies = source_factor_dependencies(source_start, descriptors, &constraints);
    initialize_factor_summaries(&mut constraints, descriptors);
    let mut queued = descriptors[source_start..]
        .iter()
        .map(|descriptor| vec![false; descriptor.type_parameters.len()])
        .collect::<Vec<_>>();
    let mut worklist = VecDeque::new();
    seed_parameter_occurrences(
        source_start,
        &constraints,
        descriptors,
        &mut queued,
        &mut worklist,
    );
    while let Some(node) = worklist.pop_front() {
        let offset = node.descriptor_index - source_start;
        queued[offset][node.parameter_index] = false;
        #[cfg(test)]
        variance_solver_counters::record_evaluation();
        for dependent in &factor_dependencies[offset][node.parameter_index] {
            propagate_factor_change(
                *dependent,
                source_start,
                &mut constraints,
                descriptors,
                &mut queued,
                &mut worklist,
            );
        }
    }

    synchronize_alias_variances(descriptors, alias_targets, &metadata);
}

pub(super) fn normalize_parameter_variance_arity(
    descriptors: &mut [super::descriptors::AdtDescriptor],
) {
    for descriptor in descriptors {
        if descriptor.parameter_variances.len() != descriptor.type_parameters.len() {
            descriptor.parameter_variances =
                vec![Variance::Invariant; descriptor.type_parameters.len()];
        }
    }
}

fn collect_constraints(
    descriptor_index: usize,
    source_start: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
    metadata: &MetadataIndex,
    resolved_constructor_uses: &BTreeSet<ResolvedConstructorUse>,
) -> DescriptorConstraints {
    let descriptor = &descriptors[descriptor_index];
    let mut constraints = vec![Variance::Irrelevant; descriptor.type_parameters.len()];
    let mut factor_paths = Vec::new();
    let context = ConstraintMetadata {
        source_start,
        descriptors,
        metadata,
        owner_descriptor_index: descriptor_index,
        resolved_constructor_uses,
    };
    for variant in &descriptor.variants {
        for field in &variant.payload_fields {
            match &field.ty {
                super::descriptors::AdtPayloadType::TypeParameter(parameter_index) => {
                    push_parameter_constraint(
                        &mut constraints,
                        &mut factor_paths,
                        *parameter_index,
                        None,
                    );
                }
                super::descriptors::AdtPayloadType::SelfType => {
                    for parameter_index in 0..descriptor.type_parameters.len() {
                        let factor_path = push_factor_path(
                            &mut factor_paths,
                            None,
                            VarianceFactor::SourceParameter {
                                descriptor_index,
                                parameter_index,
                            },
                        );
                        push_parameter_constraint(
                            &mut constraints,
                            &mut factor_paths,
                            parameter_index,
                            Some(factor_path),
                        );
                    }
                }
                super::descriptors::AdtPayloadType::Concrete(ty) => {
                    collect_type_constraints(ty, &context, &mut constraints, &mut factor_paths)
                }
            }
        }
    }
    DescriptorConstraints {
        direct_occurrences: constraints,
        factor_paths,
    }
}

fn collect_type_constraints(
    ty: &Type,
    context: &ConstraintMetadata<'_>,
    constraints: &mut [Variance],
    factor_paths: &mut Vec<FactorPathNode>,
) {
    let mut storage = ConstraintStorage { factor_paths };
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
                    push_parameter_constraint(
                        constraints,
                        storage.factor_paths,
                        parameter_index,
                        factor_path,
                    );
                }
            }
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
            } => {
                push_named_argument_constraints(
                    name,
                    identity,
                    args,
                    factor_path,
                    context,
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
    owner_descriptor_index: usize,
    resolved_constructor_uses: &'a BTreeSet<ResolvedConstructorUse>,
}

struct ConstraintStorage<'a> {
    factor_paths: &'a mut Vec<FactorPathNode>,
}

fn push_named_argument_constraints<'a>(
    name: &str,
    identity: &str,
    args: &'a [Type],
    factor_path: Option<usize>,
    context: &ConstraintMetadata<'_>,
    storage: &mut ConstraintStorage<'_>,
    work: &mut Vec<(&'a Type, Option<usize>)>,
) {
    let fixed = compiler_owned_variances(identity, args.len());
    let has_resolution = context
        .resolved_constructor_uses
        .contains(&ResolvedConstructorUse {
            owner_descriptor_index: context.owner_descriptor_index,
            name: name.to_string(),
            identity: identity.to_string(),
            arity: args.len(),
        });
    let resolved = (fixed.is_none() && has_resolution)
        .then(|| context.metadata.resolve(identity, args.len()))
        .flatten();
    let valid_metadata = resolved
        .filter(|index| context.descriptors[*index].parameter_variances.len() == args.len());
    for (parameter_index, argument) in args.iter().enumerate().rev() {
        let factor = match (fixed, valid_metadata) {
            (Some(variances), _) => VarianceFactor::Fixed(variances[parameter_index]),
            (None, Some(descriptor_index)) if descriptor_index >= context.source_start => {
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
    factor_paths.push(FactorPathNode {
        parent,
        children: Vec::new(),
        factor,
        composed: Variance::Irrelevant,
        target_parameters: BTreeSet::new(),
    });
    if let Some(parent) = parent {
        factor_paths[parent].children.push(index);
        #[cfg(test)]
        variance_solver_counters::record_dependency_edge();
    }
    #[cfg(test)]
    variance_solver_counters::record_retained_factor();
    index
}

fn factor_value(
    factor: &VarianceFactor,
    descriptors: &[super::descriptors::AdtDescriptor],
) -> Variance {
    match factor {
        VarianceFactor::Fixed(variance) => *variance,
        VarianceFactor::SourceParameter {
            descriptor_index,
            parameter_index,
        } => descriptors
            .get(*descriptor_index)
            .and_then(|descriptor| descriptor.parameter_variances.get(*parameter_index))
            .copied()
            .unwrap_or(Variance::Invariant),
    }
}

fn composed_factor_value(
    factor_index: usize,
    constraints: &DescriptorConstraints,
    descriptors: &[super::descriptors::AdtDescriptor],
) -> Variance {
    let node = &constraints.factor_paths[factor_index];
    let parent = node.parent.map_or(Variance::Covariant, |parent| {
        constraints.factor_paths[parent].composed
    });
    parent.compose(factor_value(&node.factor, descriptors))
}

fn source_factor_dependencies(
    source_start: usize,
    descriptors: &[super::descriptors::AdtDescriptor],
    constraints: &[DescriptorConstraints],
) -> Vec<Vec<Vec<FactorNode>>> {
    let mut reverse = constraints
        .iter()
        .enumerate()
        .map(|(offset, _)| {
            vec![Vec::new(); descriptors[source_start + offset].type_parameters.len()]
        })
        .collect::<Vec<_>>();
    for (descriptor_offset, descriptor) in constraints.iter().enumerate() {
        for (factor_index, factor) in descriptor.factor_paths.iter().enumerate() {
            if let VarianceFactor::SourceParameter {
                descriptor_index,
                parameter_index,
            } = factor.factor
                && descriptor_index >= source_start
            {
                let source_offset = descriptor_index - source_start;
                if let Some(dependents) = reverse
                    .get_mut(source_offset)
                    .and_then(|parameters| parameters.get_mut(parameter_index))
                {
                    dependents.push(FactorNode {
                        descriptor_offset,
                        factor_index,
                    });
                    #[cfg(test)]
                    variance_solver_counters::record_dependency_edge();
                }
            }
        }
    }
    reverse
}

fn initialize_factor_summaries(
    constraints: &mut [DescriptorConstraints],
    descriptors: &[super::descriptors::AdtDescriptor],
) {
    for descriptor in constraints {
        for factor_index in 0..descriptor.factor_paths.len() {
            #[cfg(test)]
            variance_solver_counters::record_factor_evaluation();
            descriptor.factor_paths[factor_index].composed =
                composed_factor_value(factor_index, descriptor, descriptors);
        }
    }
}

fn seed_parameter_occurrences(
    source_start: usize,
    constraints: &[DescriptorConstraints],
    descriptors: &mut [super::descriptors::AdtDescriptor],
    queued: &mut [Vec<bool>],
    worklist: &mut VecDeque<ParameterNode>,
) {
    for (descriptor_offset, descriptor) in constraints.iter().enumerate() {
        for (parameter_index, occurrence) in
            descriptor.direct_occurrences.iter().copied().enumerate()
        {
            join_parameter_occurrence(
                ParameterNode {
                    descriptor_index: source_start + descriptor_offset,
                    parameter_index,
                },
                occurrence,
                source_start,
                descriptors,
                queued,
                worklist,
            );
        }
        for factor in &descriptor.factor_paths {
            for parameter_index in &factor.target_parameters {
                join_parameter_occurrence(
                    ParameterNode {
                        descriptor_index: source_start + descriptor_offset,
                        parameter_index: *parameter_index,
                    },
                    factor.composed,
                    source_start,
                    descriptors,
                    queued,
                    worklist,
                );
            }
        }
    }
}

fn propagate_factor_change(
    factor: FactorNode,
    source_start: usize,
    constraints: &mut [DescriptorConstraints],
    descriptors: &mut [super::descriptors::AdtDescriptor],
    queued: &mut [Vec<bool>],
    worklist: &mut VecDeque<ParameterNode>,
) {
    let mut pending = VecDeque::from([factor]);
    while let Some(factor) = pending.pop_front() {
        #[cfg(test)]
        variance_solver_counters::record_factor_evaluation();
        let composed = composed_factor_value(
            factor.factor_index,
            &constraints[factor.descriptor_offset],
            descriptors,
        );
        let node = &mut constraints[factor.descriptor_offset].factor_paths[factor.factor_index];
        if node.composed == composed {
            continue;
        }
        node.composed = composed;
        let children = node.children.clone();
        let target_parameters = node.target_parameters.clone();
        for parameter_index in target_parameters {
            join_parameter_occurrence(
                ParameterNode {
                    descriptor_index: source_start + factor.descriptor_offset,
                    parameter_index,
                },
                composed,
                source_start,
                descriptors,
                queued,
                worklist,
            );
        }
        pending.extend(children.into_iter().map(|factor_index| FactorNode {
            descriptor_offset: factor.descriptor_offset,
            factor_index,
        }));
    }
}

fn join_parameter_occurrence(
    parameter: ParameterNode,
    occurrence: Variance,
    source_start: usize,
    descriptors: &mut [super::descriptors::AdtDescriptor],
    queued: &mut [Vec<bool>],
    worklist: &mut VecDeque<ParameterNode>,
) {
    #[cfg(test)]
    variance_solver_counters::record_constraint_join();
    let current =
        descriptors[parameter.descriptor_index].parameter_variances[parameter.parameter_index];
    let inferred = current.join(occurrence);
    if current == inferred {
        return;
    }
    descriptors[parameter.descriptor_index].parameter_variances[parameter.parameter_index] =
        inferred;
    let offset = parameter.descriptor_index - source_start;
    if !queued[offset][parameter.parameter_index] {
        queued[offset][parameter.parameter_index] = true;
        worklist.push_back(parameter);
    }
}

fn push_parameter_constraint(
    constraints: &mut [Variance],
    factor_paths: &mut [FactorPathNode],
    parameter_index: usize,
    factor_path: Option<usize>,
) {
    match factor_path {
        None => {
            if let Some(parameter) = constraints.get_mut(parameter_index) {
                *parameter = parameter.join(Variance::Covariant);
            }
        }
        Some(factor_path) => {
            if constraints.get(parameter_index).is_some()
                && factor_paths[factor_path]
                    .target_parameters
                    .insert(parameter_index)
            {
                #[cfg(test)]
                variance_solver_counters::record_dependency_edge();
            }
        }
    }
}

fn synchronize_alias_variances(
    descriptors: &mut [super::descriptors::AdtDescriptor],
    alias_targets: &BTreeMap<DescriptorKey, String>,
    metadata: &MetadataIndex,
) {
    let updates = descriptors
        .iter()
        .enumerate()
        .filter_map(|(index, descriptor)| {
            let target_identity = alias_targets.get(&DescriptorKey::new(descriptor))?;
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

#[cfg(test)]
pub(crate) mod variance_solver_counters {
    use std::cell::Cell;

    thread_local! {
        static EVALUATIONS: Cell<usize> = const { Cell::new(0) };
        static COLLECTION_VISITS: Cell<usize> = const { Cell::new(0) };
        static RETAINED_FACTORS: Cell<usize> = const { Cell::new(0) };
        static FACTOR_EVALUATIONS: Cell<usize> = const { Cell::new(0) };
        static CONSTRAINT_JOINS: Cell<usize> = const { Cell::new(0) };
        static DEPENDENCY_EDGES: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        EVALUATIONS.set(0);
        COLLECTION_VISITS.set(0);
        RETAINED_FACTORS.set(0);
        FACTOR_EVALUATIONS.set(0);
        CONSTRAINT_JOINS.set(0);
        DEPENDENCY_EDGES.set(0);
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

    pub(super) fn record_factor_evaluation() {
        FACTOR_EVALUATIONS.set(FACTOR_EVALUATIONS.get() + 1);
    }

    pub(super) fn record_constraint_join() {
        CONSTRAINT_JOINS.set(CONSTRAINT_JOINS.get() + 1);
    }

    pub(super) fn record_dependency_edge() {
        DEPENDENCY_EDGES.set(DEPENDENCY_EDGES.get() + 1);
    }

    pub(crate) fn collection_visits() -> usize {
        COLLECTION_VISITS.get()
    }

    pub(crate) fn retained_factors() -> usize {
        RETAINED_FACTORS.get()
    }

    pub(crate) fn factor_evaluations() -> usize {
        FACTOR_EVALUATIONS.get()
    }

    pub(crate) fn constraint_joins() -> usize {
        CONSTRAINT_JOINS.get()
    }

    pub(crate) fn dependency_edges() -> usize {
        DEPENDENCY_EDGES.get()
    }
}
