#[cfg(test)]
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

use crate::adt::descriptors::{AdtConstructor, Variance};
use crate::adt::registry::{AdtRegistry, VariantDeclarationOrder};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;
use crate::type_relations::{
    TypePresentationJoin, invariant_args_match, is_assignable_nested_with_registry,
    record_field_match_work, same_type_identity,
};

pub(crate) struct AggregateTypeJoin {
    nodes: Vec<JoinNode>,
    root: usize,
    optimized_contributions: Option<Vec<Type>>,
    bounded: Option<BoundedJoin>,
    projection_cache: RefCell<context_projection::ProjectionCache>,
}

pub(crate) struct ConstructorTypeArgConstraints {
    parameters: Vec<TypeParameterConstraints>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructorLowerBoundAdmission {
    NoPrior,
    Admitted,
    Rejected,
}

#[derive(Default)]
struct TypeParameterConstraints {
    lower: Vec<Type>,
    upper: Vec<Type>,
    equality: Vec<Type>,
    resolved: Option<Type>,
    lower_join: Option<AggregateTypeJoin>,
}

#[derive(Clone)]
struct ConstructorConstraintContribution {
    parameter: usize,
    polarity: Variance,
    ty: Type,
}

struct BoundedJoin {
    contributions: Vec<Type>,
    resolved: Option<Type>,
    fallback: Option<ContributedCandidateState>,
    function_candidates: Option<FunctionCandidateStates>,
    admission_witness: Option<usize>,
}

struct ContributedCandidateState {
    direction: BoundDirection,
    valid: Vec<bool>,
    selected: Option<usize>,
    resolved: Option<Type>,
    presentation: TypePresentationJoin,
}

struct FunctionCandidateStates {
    inputs: Vec<ContributedCandidateState>,
    has_variadic: bool,
    returns: FunctionReturnCandidateState,
}

struct FunctionReturnCandidateState {
    return_keys: HashSet<String>,
    return_join: Option<Box<AggregateTypeJoin>>,
    return_type: Option<Type>,
    return_presentation: TypePresentationJoin,
}

impl BoundedJoin {
    fn from_contributions(adts: &AdtRegistry, contributions: Vec<Type>) -> Self {
        let admission_witness = admission_witness(adts, &contributions);
        let function_candidates = FunctionCandidateStates::new(adts, &contributions);
        let fallback = uses_outer_contributed_fallback(&contributions)
            .then(|| ContributedCandidateState::new(adts, &contributions, BoundDirection::Join));
        let resolved = if let Some(state) = &function_candidates {
            state.result(&contributions)
        } else if let Some(state) = &fallback {
            state.result(&contributions)
        } else {
            bounded_join(adts, &contributions)
        };
        Self {
            contributions,
            resolved,
            fallback,
            function_candidates,
            admission_witness,
        }
    }

    fn admit(&mut self, adts: &AdtRegistry, right: &Type) {
        self.update_admission_witness(adts, right);
        self.contributions.push(right.clone());
        if let Some(function_candidates) = &mut self.function_candidates {
            function_candidates.admit(adts, &self.contributions);
            let resolved = function_candidates.result(&self.contributions);
            self.replace_resolved(resolved);
            return;
        }
        if let Some(fallback) = &mut self.fallback {
            fallback.admit(adts, &self.contributions);
            let resolved = fallback.result(&self.contributions);
            self.replace_resolved(resolved);
            return;
        }
        self.replace_resolved(bounded_join(adts, &self.contributions));
        if uses_outer_contributed_fallback(&self.contributions) {
            self.fallback = Some(ContributedCandidateState::new(
                adts,
                &self.contributions,
                BoundDirection::Join,
            ));
        }
    }

    fn replace_resolved(&mut self, resolved: Option<Type>) {
        let previous = std::mem::replace(&mut self.resolved, resolved);
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
    }

    fn shapes_are_admissible(&self, adts: &AdtRegistry, right: &Type) -> bool {
        if let Some(witness) = self.admission_witness {
            return structurally_compatible_function_shapes(
                adts,
                &self.contributions[witness],
                right,
            );
        }
        bounded_shapes_are_admissible(adts, &self.contributions, right)
    }

    fn update_admission_witness(&mut self, adts: &AdtRegistry, right: &Type) {
        self.admission_witness = if let Some(witness) = self.admission_witness {
            let witness = &self.contributions[witness];
            if structural_paths_cover(adts, right, witness) {
                Some(self.contributions.len())
            } else if structural_paths_cover(adts, witness, right) {
                self.admission_witness
            } else {
                None
            }
        } else if self
            .contributions
            .iter()
            .all(|contribution| structural_paths_cover(adts, right, contribution))
        {
            Some(self.contributions.len())
        } else {
            None
        };
    }
}

impl Drop for BoundedJoin {
    fn drop(&mut self) {
        drop_types_iteratively(&mut self.contributions);
        if let Some(resolved) = self.resolved.take() {
            crate::semantic_model::drop_type_iteratively(resolved);
        }
    }
}

impl ContributedCandidateState {
    fn new(adts: &AdtRegistry, contributions: &[Type], direction: BoundDirection) -> Self {
        let contributions = contributions.iter().collect::<Vec<_>>();
        Self::new_refs(adts, &contributions, direction)
    }

    fn new_refs(adts: &AdtRegistry, contributions: &[&Type], direction: BoundDirection) -> Self {
        let mut state = Self {
            direction,
            valid: Vec::new(),
            selected: None,
            resolved: None,
            presentation: TypePresentationJoin::default(),
        };
        for end in 1..=contributions.len() {
            state.admit_refs(adts, &contributions[..end]);
        }
        state
    }

    fn admit(&mut self, adts: &AdtRegistry, contributions: &[Type]) {
        let contributions = contributions.iter().collect::<Vec<_>>();
        self.admit_refs(adts, &contributions);
    }

    fn admit_refs(&mut self, adts: &AdtRegistry, contributions: &[&Type]) {
        let new_index = contributions.len() - 1;
        debug_assert_eq!(self.valid.len(), new_index);
        let representative = self.valid.iter().position(|valid| *valid);
        let prior_candidates_remain_valid = representative.is_some_and(|index| {
            candidate_relation(
                contributions[index],
                contributions[new_index],
                self.direction,
                adts,
            )
        });
        if representative.is_some() && !prior_candidates_remain_valid {
            self.valid.fill(false);
        }
        let new_valid = if let Some(index) = representative {
            candidate_relation(
                contributions[new_index],
                contributions[index],
                self.direction,
                adts,
            )
        } else {
            contributions[..new_index].iter().all(|contribution| {
                candidate_relation(contributions[new_index], contribution, self.direction, adts)
            })
        };
        self.valid.push(new_valid);

        let selected = if prior_candidates_remain_valid {
            let previous = self
                .selected
                .expect("a valid candidate has a selected representative");
            if new_valid
                && candidate_order_key(contributions[new_index])
                    < candidate_order_key(contributions[previous])
            {
                Some(new_index)
            } else {
                Some(previous)
            }
        } else {
            new_valid.then_some(new_index)
        };
        self.update_resolved(contributions, selected);
    }

    fn result(&self, contributions: &[Type]) -> Option<Type> {
        let contributions = contributions.iter().collect::<Vec<_>>();
        self.result_refs(&contributions)
    }

    fn result_refs(&self, contributions: &[&Type]) -> Option<Type> {
        debug_assert_eq!(self.valid.len(), contributions.len());
        if let Some(resolved) = &self.resolved {
            record_bounded_result_materialization(resolved);
        }
        self.resolved.clone()
    }

    fn update_resolved(&mut self, contributions: &[&Type], selected: Option<usize>) {
        let previous_selected = self.selected;
        if selected == previous_selected {
            if let Some(resolved) = &mut self.resolved {
                self.presentation.merge(
                    resolved,
                    contributions
                        .last()
                        .expect("candidate admission has a contribution"),
                );
            }
            return;
        }

        let previous = self.resolved.take();
        let Some(selected) = selected else {
            if let Some(previous) = previous {
                crate::semantic_model::drop_type_iteratively(previous);
            }
            self.presentation = TypePresentationJoin::default();
            self.selected = None;
            return;
        };

        let candidate = contributions[selected];
        let mut resolved = candidate.clone();
        let can_reuse = previous
            .as_ref()
            .is_some_and(|previous| presentation_layout_is_reusable(previous, candidate));
        if can_reuse {
            self.presentation.merge(
                &mut resolved,
                previous
                    .as_ref()
                    .expect("reusable presentation has a previous result"),
            );
        } else {
            self.presentation = TypePresentationJoin::default();
            for contribution in contributions {
                self.presentation.merge(&mut resolved, contribution);
            }
        }
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
        self.selected = Some(selected);
        self.resolved = Some(resolved);
    }

    #[cfg(test)]
    fn retained_units(&self) -> usize {
        self.valid.len()
            + self.resolved.as_ref().map_or(0, type_retained_units)
            + self.presentation.retained_nodes()
    }
}

impl Drop for ContributedCandidateState {
    fn drop(&mut self) {
        if let Some(resolved) = self.resolved.take() {
            crate::semantic_model::drop_type_iteratively(resolved);
        }
    }
}

impl FunctionCandidateStates {
    fn new(adts: &AdtRegistry, contributions: &[Type]) -> Option<Self> {
        let functions = root_function_parts(contributions)?;
        let first = functions.first()?;
        let mut inputs = (0..first.params.len())
            .map(|index| {
                let candidates = functions
                    .iter()
                    .map(|function| &function.params[index])
                    .collect::<Vec<_>>();
                ContributedCandidateState::new_refs(adts, &candidates, BoundDirection::Meet)
            })
            .collect::<Vec<_>>();
        if first.variadic.is_some() {
            let candidates = functions
                .iter()
                .map(|function| function.variadic.expect("variadic shape was checked"))
                .collect::<Vec<_>>();
            inputs.push(ContributedCandidateState::new_refs(
                adts,
                &candidates,
                BoundDirection::Meet,
            ));
        }
        Some(Self {
            inputs,
            has_variadic: first.variadic.is_some(),
            returns: FunctionReturnCandidateState::new(adts, &functions),
        })
    }

    fn admit(&mut self, adts: &AdtRegistry, contributions: &[Type]) {
        let functions = root_function_parts(contributions)
            .expect("admitted bounded function contributions keep their shape");
        self.admit_inputs(adts, &functions);
        self.returns.admit(adts, &functions);
    }

    fn admit_inputs(&mut self, adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) {
        let parameter_count = functions[0].params.len();
        for (index, state) in self.inputs.iter_mut().enumerate() {
            let candidates = if index < parameter_count {
                functions
                    .iter()
                    .map(|function| &function.params[index])
                    .collect::<Vec<_>>()
            } else {
                functions
                    .iter()
                    .map(|function| function.variadic.expect("variadic shape was checked"))
                    .collect::<Vec<_>>()
            };
            state.admit_refs(adts, &candidates);
        }
    }

    fn result(&self, contributions: &[Type]) -> Option<Type> {
        record_bounded_frame();
        let functions = root_function_parts(contributions)?;
        let contribution_refs = contributions.iter().collect::<Vec<_>>();
        let effects = collect_function_bound_parts(&contribution_refs)?.effects;
        let parameter_count = functions.first()?.params.len();
        let mut params = self.result_params(&functions, parameter_count)?;
        let variadic = if self.has_variadic {
            let candidates = functions
                .iter()
                .map(|function| function.variadic.expect("variadic shape was checked"))
                .collect::<Vec<_>>();
            let Some(variadic) = self.inputs[parameter_count].result_refs(&candidates) else {
                drop_types_iteratively(&mut params);
                return None;
            };
            Some(Box::new(variadic))
        } else {
            None
        };
        let Some(return_type) = self.returns.return_type.clone() else {
            drop_types_iteratively(&mut params);
            if let Some(variadic) = variadic {
                crate::semantic_model::drop_type_iteratively(*variadic);
            }
            return None;
        };
        Some(Type::Function {
            params,
            variadic,
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn result_params(
        &self,
        functions: &[FunctionBoundParts<'_>],
        parameter_count: usize,
    ) -> Option<Vec<Type>> {
        let mut params = Vec::with_capacity(parameter_count);
        for (index, state) in self.inputs.iter().take(parameter_count).enumerate() {
            let candidates = functions
                .iter()
                .map(|function| &function.params[index])
                .collect::<Vec<_>>();
            let Some(param) = state.result_refs(&candidates) else {
                drop_types_iteratively(&mut params);
                return None;
            };
            params.push(param);
        }
        Some(params)
    }
}

impl FunctionReturnCandidateState {
    fn new(adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) -> Self {
        let returns = function_return_types(functions);
        let return_keys = returns
            .iter()
            .map(|return_type| canonical_relation_key(return_type))
            .collect();
        let return_join = incremental_function_return_join(adts, &returns);
        let (return_type, return_presentation) = return_join.as_ref().map_or_else(
            || resolve_function_returns(adts, &returns),
            |joined| {
                let mut resolved = joined.resolved_result_type();
                let mut presentation = TypePresentationJoin::default();
                if let Some(resolved) = &mut resolved {
                    for return_type in &returns {
                        presentation.merge(resolved, return_type);
                    }
                }
                (resolved, presentation)
            },
        );
        Self {
            return_keys,
            return_join,
            return_type,
            return_presentation,
        }
    }

    fn admit(&mut self, adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) {
        let return_type = functions
            .last()
            .expect("admitted bounded function contributions are nonempty")
            .return_type;
        if !self.return_keys.insert(canonical_relation_key(return_type)) {
            if let Some(resolved) = &mut self.return_type {
                self.return_presentation.merge(resolved, return_type);
            }
            return;
        }

        let returns = function_return_types(functions);
        let incrementally_admitted = self
            .return_join
            .as_mut()
            .is_some_and(|joined| joined.try_join(adts, return_type));
        if !incrementally_admitted {
            self.return_join = None;
            let (resolved, presentation) = resolve_function_returns(adts, &returns);
            self.replace_return(resolved, presentation);
            return;
        }

        let mut resolved = self
            .return_join
            .as_ref()
            .and_then(|joined| joined.resolved_result_type());
        if self.return_type.is_some() {
            if let Some(resolved) = &mut resolved {
                self.return_presentation.merge(resolved, return_type);
            } else {
                self.return_presentation = TypePresentationJoin::default();
            }
            self.replace_return_type(resolved);
            return;
        }

        let mut presentation = TypePresentationJoin::default();
        if let Some(resolved) = &mut resolved {
            for return_type in &returns {
                presentation.merge(resolved, return_type);
            }
        }
        self.replace_return(resolved, presentation);
    }

    fn replace_return(&mut self, resolved: Option<Type>, presentation: TypePresentationJoin) {
        self.return_presentation = presentation;
        self.replace_return_type(resolved);
    }

    fn replace_return_type(&mut self, resolved: Option<Type>) {
        let previous = std::mem::replace(&mut self.return_type, resolved);
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
    }

    #[cfg(test)]
    fn retained_units(&self) -> usize {
        self.return_keys.iter().map(String::len).sum::<usize>()
            + self
                .return_join
                .as_ref()
                .map_or(0, |joined| joined.bounded_retained_state())
            + self.return_type.as_ref().map_or(0, type_retained_units)
            + self.return_presentation.retained_nodes()
    }
}

fn function_return_types<'a>(functions: &[FunctionBoundParts<'a>]) -> Vec<&'a Type> {
    functions
        .iter()
        .map(|function| function.return_type)
        .collect()
}

fn incremental_function_return_join(
    adts: &AdtRegistry,
    returns: &[&Type],
) -> Option<Box<AggregateTypeJoin>> {
    // A nested aggregate at every direct function return would retain every
    // remaining suffix. The bounded worklist resolves that spine iteratively.
    if matches!(returns.first(), Some(Type::Function { .. })) {
        return None;
    }
    let mut joined = Box::new(AggregateTypeJoin::new(adts, returns.first()?)?);
    for return_type in &returns[1..] {
        if !joined.try_join(adts, return_type) {
            return None;
        }
    }
    Some(joined)
}

impl Drop for FunctionReturnCandidateState {
    fn drop(&mut self) {
        if let Some(return_type) = self.return_type.take() {
            crate::semantic_model::drop_type_iteratively(return_type);
        }
    }
}

fn resolve_function_returns(
    adts: &AdtRegistry,
    returns: &[&Type],
) -> (Option<Type>, TypePresentationJoin) {
    let mut resolved = bounded_join_refs(adts, returns);
    let mut presentation = TypePresentationJoin::default();
    if let Some(resolved) = &mut resolved {
        for return_type in returns {
            presentation.merge(resolved, return_type);
        }
    }
    (resolved, presentation)
}

fn root_function_parts(contributions: &[Type]) -> Option<Vec<FunctionBoundParts<'_>>> {
    let Type::Function {
        params: first_params,
        variadic: first_variadic,
        ..
    } = contributions.first()?
    else {
        return None;
    };
    contributions
        .iter()
        .map(|contribution| {
            let Type::Function {
                params,
                variadic,
                return_type,
                ..
            } = contribution
            else {
                return None;
            };
            (params.len() == first_params.len() && variadic.is_some() == first_variadic.is_some())
                .then_some(FunctionBoundParts {
                    params,
                    variadic: variadic.as_deref(),
                    return_type,
                })
        })
        .collect()
}

mod context_projection;

#[cfg(test)]
pub(crate) use context_projection::{
    ContextProjectionWork, reset_context_classification_work, take_context_classification_work,
};
pub(crate) use context_projection::{
    LocalContext, expression_admitted_context, expression_has_compiler_covariant_root,
    expression_is_immediately_context_free,
};

struct JoinNode {
    name: String,
    canonical_name: String,
    presentation_conflicted: bool,
    identity: String,
    args: Vec<JoinArg>,
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
    record_field_names: Option<Vec<String>>,
}

type JoinNodeSetup<'a> = (JoinNode, Vec<(usize, &'a Type, bool)>);

enum JoinArg {
    Exact {
        ty: Type,
        presentation: TypePresentationJoin,
        covariant: bool,
    },
    Covariant(usize),
}

struct PreparedContribution<'a> {
    node: usize,
    right: PreparedTypeParts<'a>,
    variant_ranks: Option<Vec<usize>>,
}

enum PreparedTypeParts<'a> {
    Adt(AdtTypeParts<'a>),
    Record(Vec<&'a Type>),
}

struct InitialVariantState {
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
}

impl AggregateTypeJoin {
    pub(crate) fn new(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        Self::new_optimized(adts, initial).or_else(|| {
            matches!(initial, Type::Function { .. }).then(|| Self {
                nodes: Vec::new(),
                root: 0,
                optimized_contributions: None,
                bounded: Some(BoundedJoin::from_contributions(adts, vec![initial.clone()])),
                projection_cache: RefCell::new(context_projection::ProjectionCache::default()),
            })
        })
    }

    fn new_optimized(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let (root, pending) = join_node(adts, initial, false)?;
        let mut nodes = vec![root];
        let mut pending = pending
            .into_iter()
            .map(|(argument, ty, record_subtree_is_complete)| {
                (0, argument, ty, record_subtree_is_complete)
            })
            .collect::<Vec<_>>();
        while let Some((parent, argument, ty, record_subtree_is_complete)) = pending.pop() {
            let Some((node, children)) = join_node(adts, ty, record_subtree_is_complete) else {
                drop_join_nodes_iteratively(&mut nodes);
                return None;
            };
            let child = nodes.len();
            nodes.push(node);
            nodes[parent].args[argument] = JoinArg::Covariant(child);
            pending.extend(children.into_iter().map(
                |(argument, ty, record_subtree_is_complete)| {
                    (child, argument, ty, record_subtree_is_complete)
                },
            ));
        }
        record_retained_nodes(nodes.len());
        Some(Self {
            nodes,
            root: 0,
            optimized_contributions: optimized_join_retains_contributions(adts, initial)
                .then(|| vec![initial.clone()]),
            bounded: None,
            projection_cache: RefCell::new(context_projection::ProjectionCache::default()),
        })
    }

    pub(crate) fn new_resolved_refinement(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        if matches!(initial, Type::Function { .. }) {
            return Self::new(adts, initial);
        }
        if matches!(initial, Type::Record(_)) {
            return Self::new(adts, initial);
        }
        let (identity, args) = match initial {
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                (identity, args)
            }
            _ => return None,
        };
        let compiler_covariant = Type::is_compiler_covariant_identity(identity);
        if (!compiler_covariant && args.iter().any(|argument| type_shape(argument).1))
            || (adts.descriptor_for_type(initial).is_none() && !compiler_covariant)
        {
            return None;
        }
        Self::new(adts, initial)
    }

    pub(crate) fn try_join(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        if right == &Type::Unknown {
            return true;
        }
        if let Some(bounded) = &mut self.bounded {
            if !bounded.shapes_are_admissible(adts, right) {
                return false;
            }
            bounded.admit(adts, right);
            self.projection_cache.get_mut().clear();
            return true;
        }
        let Some(prepared) = self.prepare(adts, right) else {
            let Some(retained) = self.optimized_contributions.as_deref() else {
                return false;
            };
            if !bounded_shapes_are_admissible(adts, retained, right) {
                return false;
            }
            let mut contributions = retained.to_vec();
            contributions.push(right.clone());
            let bounded = BoundedJoin::from_contributions(adts, contributions);
            if bounded.resolved.is_none()
                && !can_retain_unresolved_contributions(adts, &bounded.contributions)
            {
                return false;
            }
            if let Some(mut optimized) = self.optimized_contributions.take() {
                drop_types_iteratively(&mut optimized);
            }
            self.bounded = Some(bounded);
            self.projection_cache.get_mut().clear();
            return true;
        };
        if self.commit(prepared, adts) {
            self.projection_cache.get_mut().clear();
        }
        if let Some(contributions) = &mut self.optimized_contributions {
            contributions.push(right.clone());
        }
        true
    }

    fn can_admit(&self, adts: &AdtRegistry, right: &Type) -> bool {
        if right == &Type::Unknown {
            return true;
        }
        if let Some(bounded) = &self.bounded {
            return bounded.shapes_are_admissible(adts, right);
        }
        if self.prepare(adts, right).is_some() {
            return true;
        }
        let Some(retained) = self.optimized_contributions.as_deref() else {
            return false;
        };
        if !bounded_shapes_are_admissible(adts, retained, right) {
            return false;
        }
        let mut contributions = retained.to_vec();
        contributions.push(right.clone());
        let bounded = BoundedJoin::from_contributions(adts, contributions);
        bounded.resolved.is_some()
            || can_retain_unresolved_contributions(adts, &bounded.contributions)
    }

    #[cfg(test)]
    pub(crate) fn bounded_retained_state(&self) -> usize {
        let Some(bounded) = &self.bounded else {
            return 0;
        };
        let contribution_state = bounded
            .contributions
            .iter()
            .map(type_retained_units)
            .sum::<usize>();
        let resolved_state = bounded.resolved.as_ref().map_or(0, type_retained_units);
        let fallback_state = bounded
            .fallback
            .as_ref()
            .map_or(0, ContributedCandidateState::retained_units);
        let function_candidate_state = bounded.function_candidates.as_ref().map_or(0, |states| {
            states
                .inputs
                .iter()
                .map(ContributedCandidateState::retained_units)
                .sum::<usize>()
                + states.returns.retained_units()
        });
        bounded.contributions.len()
            + contribution_state
            + resolved_state
            + fallback_state
            + function_candidate_state
    }

    fn prepare<'a>(
        &self,
        _adts: &AdtRegistry,
        right: &'a Type,
    ) -> Option<Vec<PreparedContribution<'a>>> {
        let mut pending = vec![(self.root, right)];
        let mut prepared = Vec::new();
        while let Some((node_index, right)) = pending.pop() {
            record_node_visit();
            prepared.push(self.prepare_contribution(node_index, right, &mut pending)?);
        }
        Some(prepared)
    }

    fn prepare_contribution<'a>(
        &self,
        node_index: usize,
        right: &'a Type,
        pending: &mut Vec<(usize, &'a Type)>,
    ) -> Option<PreparedContribution<'a>> {
        if let Some(field_names) = &self.nodes[node_index].record_field_names {
            let Type::Record(fields) = right else {
                return None;
            };
            if fields.len() != field_names.len() {
                return None;
            }
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            if fields
                .iter()
                .zip(field_names)
                .any(|((actual, _), expected)| actual != expected)
            {
                return None;
            }
            let right_fields = fields.into_iter().map(|(_, ty)| ty).collect::<Vec<_>>();
            queue_covariant_contributions(
                &self.nodes[node_index],
                right_fields.iter().copied(),
                pending,
            )?;
            return Some(PreparedContribution {
                node: node_index,
                right: PreparedTypeParts::Record(right_fields),
                variant_ranks: None,
            });
        }
        let right = adt_type_parts(right)?;
        let node = &self.nodes[node_index];
        (same_type_identity(&node.name, &node.identity, right.name, right.identity)
            && node.args.len() == right.args.len())
        .then_some(())?;
        queue_covariant_contributions(node, right.args, pending)?;
        Some(PreparedContribution {
            node: node_index,
            right: PreparedTypeParts::Adt(right),
            variant_ranks: joined_variant_ranks(node, right.variants)?,
        })
    }

    fn commit(&mut self, prepared: Vec<PreparedContribution<'_>>, adts: &AdtRegistry) -> bool {
        let mut changed = false;
        for contribution in prepared {
            let node = &mut self.nodes[contribution.node];
            changed |= commit_prepared_contribution(node, contribution);
        }
        changed | self.expand_covariant_nodes(adts)
    }

    fn expand_covariant_nodes(&mut self, adts: &AdtRegistry) -> bool {
        let original_nodes = self.nodes.len();
        for parent in 0..original_nodes {
            for argument in 0..self.nodes[parent].args.len() {
                let should_expand = matches!(
                    &self.nodes[parent].args[argument],
                    JoinArg::Exact {
                        ty,
                        covariant: true,
                        ..
                    } if join_node(adts, ty, false).is_some()
                );
                if !should_expand {
                    continue;
                }
                let JoinArg::Exact { ty, .. } = &mut self.nodes[parent].args[argument] else {
                    unreachable!();
                };
                let source = std::mem::replace(ty, Type::Unknown);
                let (root, children) = join_node(adts, &source, false)
                    .expect("a concrete covariant source argument has join state");
                let child = self.nodes.len();
                self.nodes.push(root);
                self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                let mut pending = children
                    .into_iter()
                    .map(|(argument, ty, record_subtree_is_complete)| {
                        (child, argument, ty, record_subtree_is_complete)
                    })
                    .collect::<Vec<_>>();
                while let Some((parent, argument, ty, record_subtree_is_complete)) = pending.pop() {
                    let (node, children) = join_node(adts, ty, record_subtree_is_complete)
                        .expect("a nested covariant source argument has join state");
                    let child = self.nodes.len();
                    self.nodes.push(node);
                    self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                    pending.extend(children.into_iter().map(
                        |(argument, ty, record_subtree_is_complete)| {
                            (child, argument, ty, record_subtree_is_complete)
                        },
                    ));
                }
                crate::semantic_model::drop_type_iteratively(source);
            }
        }
        record_retained_nodes(self.nodes.len());
        self.nodes.len() != original_nodes
    }

    pub(crate) fn try_join_resolved(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        self.try_join(adts, right)
    }

    pub(crate) fn has_compiler_covariant_root(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes[self.root].record_field_names.is_none()
            && Type::is_compiler_covariant_identity(&self.nodes[self.root].identity)
    }

    pub(crate) fn preserves_failed_control_flow_contribution(&self, right: &Type) -> bool {
        if self.bounded.is_some() {
            return true;
        }
        (self.has_compiler_covariant_root() && self.has_same_root(right))
            || (self.nodes[self.root].record_field_names.is_some()
                && matches!(right, Type::Record(_)))
    }

    pub(crate) fn has_complete_domain(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes[self.root].variants.is_none()
    }

    pub(crate) fn has_incomplete_nested_refinement_domain(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes.iter().enumerate().any(|(index, node)| {
            index != self.root
                && node.variants.as_ref().is_some_and(|variants| {
                    variants.len()
                        != node
                            .declaration_order
                            .as_ref()
                            .expect("a refinement join retains declaration order")
                            .len()
                })
        })
    }

    pub(crate) fn has_same_root(&self, right: &Type) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        let Some(right) = adt_type_parts(right) else {
            return false;
        };
        let root = &self.nodes[self.root];
        (Type::is_compiler_covariant_identity(&root.identity)
            || right.args.iter().all(|argument| !type_shape(argument).1))
            && same_type_identity(&root.name, &root.identity, right.name, right.identity)
            && root.args.len() == right.args.len()
    }

    pub(crate) fn inference_type(&self) -> Type {
        self.materialize(false)
    }

    pub(crate) fn has_resolved_result(&self) -> bool {
        self.bounded
            .as_ref()
            .is_none_or(|bounded| bounded.resolved.is_some())
    }

    pub(crate) fn resolved_result_type(&self) -> Option<Type> {
        self.has_resolved_result().then(|| self.result_type())
    }

    pub(crate) fn result_type(&self) -> Type {
        self.materialize(true)
    }

    fn materialize(&self, include_root_refinement: bool) -> Type {
        if let Some(bounded) = &self.bounded {
            return bounded.resolved.clone().unwrap_or(Type::Unknown);
        }
        let mut materialized = (0..self.nodes.len()).map(|_| None).collect::<Vec<_>>();
        for (index, node) in self.nodes.iter().enumerate().rev() {
            record_materialization();
            let args = materialize_args(node, &mut materialized);
            materialized[index] =
                Some(self.materialize_node(index, node, args, include_root_refinement));
        }
        materialized[self.root]
            .take()
            .expect("aggregate root is materialized")
    }

    fn materialize_node(
        &self,
        index: usize,
        node: &JoinNode,
        args: Vec<Type>,
        include_root_refinement: bool,
    ) -> Type {
        if let Some(field_names) = &node.record_field_names {
            return Type::Record(field_names.iter().cloned().zip(args).collect());
        }
        let variants = (index != self.root || include_root_refinement)
            .then_some(node.variants.as_ref())
            .flatten();
        let Some(variants) = variants else {
            return materialize_named_node(node, args);
        };
        if variants.len()
            == node
                .declaration_order
                .as_ref()
                .expect("a refinement join retains declaration order")
                .len()
        {
            return materialize_named_node(node, args);
        }
        materialize_refined_node(node, args, variants)
    }
}

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

impl ConstructorTypeArgConstraints {
    pub(crate) fn new(constructor: AdtConstructor<'_>) -> Self {
        let parameter_count = constructor.descriptor.type_parameters.len();
        record_work(parameter_count);
        Self {
            parameters: (0..parameter_count)
                .map(|_| TypeParameterConstraints::default())
                .collect(),
        }
    }

    pub(crate) fn payload_expected(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
    ) -> Type {
        expected
            .and_then(|expected| {
                let expected_args = unification::adt_args(expected, constructor.descriptor)?;
                adt::payload_type_with_resolved_args(constructor, payload_index, |type_index| {
                    expected_args[type_index].clone()
                })
            })
            .or_else(|| {
                adt::payload_type_with_resolved_args(constructor, payload_index, |type_index| {
                    self.result(type_index, None)
                })
            })
            .unwrap_or(Type::Unknown)
    }

    pub(crate) fn payload_can_infer_without_context(
        &self,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
        adts: &AdtRegistry,
    ) -> bool {
        let Some(field) = constructor.variant.payload_fields.get(payload_index) else {
            return false;
        };
        match &field.ty {
            crate::adt::descriptors::AdtPayloadType::TypeParameter(_) => true,
            crate::adt::descriptors::AdtPayloadType::SelfType => checked_parameter_variances(
                &constructor.descriptor.identity(),
                constructor.descriptor.type_parameters.len(),
                adts,
            )
            .iter()
            .any(|variance| *variance != Variance::Invariant),
            crate::adt::descriptors::AdtPayloadType::Concrete(template) => {
                template_has_non_invariant_parameter(template, Variance::Covariant, adts)
            }
        }
    }

    pub(crate) fn lower_bound_admission(
        &self,
        parameter: usize,
        actual: &Type,
        adts: &AdtRegistry,
    ) -> ConstructorLowerBoundAdmission {
        let Some(constraints) = self.parameters.get(parameter) else {
            return ConstructorLowerBoundAdmission::NoPrior;
        };
        if constraints.lower.is_empty() {
            return ConstructorLowerBoundAdmission::NoPrior;
        }
        let first = &constraints.lower[0];
        if matches!(
            (adt_type_parts(first), adt_type_parts(actual)),
            (Some(first), Some(actual))
                if !same_type_identity(
                    first.name,
                    first.identity,
                    actual.name,
                    actual.identity,
                )
        ) {
            return ConstructorLowerBoundAdmission::Admitted;
        }
        let Some(joined) = constraints.lower_join.as_ref() else {
            return ConstructorLowerBoundAdmission::Admitted;
        };
        if joined.can_admit(adts, actual) {
            ConstructorLowerBoundAdmission::Admitted
        } else {
            ConstructorLowerBoundAdmission::Rejected
        }
    }

    pub(crate) fn contribute_payload(
        &mut self,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
        actual: &Type,
        adts: &AdtRegistry,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        let mut contributions = Vec::new();
        collect_constructor_payload_constraints(
            constructor,
            payload_index,
            actual,
            adts,
            &mut contributions,
        );
        if contributions
            .iter()
            .all(|contribution| contribution.polarity == Variance::Covariant)
            && contributions.iter().all(|contribution| {
                self.parameters
                    .get(contribution.parameter)
                    .is_some_and(|parameter| {
                        parameter.upper.is_empty() && parameter.equality.is_empty()
                    })
            })
        {
            for contribution in contributions {
                self.parameters[contribution.parameter].admit_lower_bound(contribution.ty, adts);
            }
            return Ok(());
        }
        let mut trial = HashMap::<usize, TypeParameterConstraints>::new();
        for contribution in contributions {
            let Some(current) = self.parameters.get(contribution.parameter) else {
                continue;
            };
            let parameter = trial
                .entry(contribution.parameter)
                .or_insert_with(|| current.clone_for_trial());
            match contribution.polarity {
                Variance::Covariant => parameter.lower.push(contribution.ty),
                Variance::Contravariant => parameter.upper.push(contribution.ty),
                Variance::Invariant => parameter.equality.push(contribution.ty),
                Variance::Irrelevant => {}
            }
        }
        for parameter in trial.values_mut() {
            parameter.resolved = solve_type_parameter_constraints(parameter, adts)?;
        }
        for (index, parameter) in trial {
            self.parameters[index] = parameter;
        }
        Ok(())
    }

    pub(crate) fn finish(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
    ) -> Vec<Type> {
        let expected_args =
            expected.and_then(|expected| unification::adt_args(expected, constructor.descriptor));
        (0..self.parameters.len())
            .map(|index| self.result(index, expected_args))
            .collect()
    }

    fn result(&self, index: usize, expected_args: Option<&[Type]>) -> Type {
        self.parameters
            .get(index)
            .and_then(TypeParameterConstraints::inferred_result)
            .or_else(|| expected_args.and_then(|args| args.get(index).cloned()))
            .unwrap_or(Type::Unknown)
    }
}

impl TypeParameterConstraints {
    fn clone_for_trial(&self) -> Self {
        Self {
            lower: self.lower.clone(),
            upper: self.upper.clone(),
            equality: self.equality.clone(),
            resolved: self.resolved.clone(),
            lower_join: None,
        }
    }

    fn admit_lower_bound(&mut self, contribution: Type, adts: &AdtRegistry) {
        if self.lower_join.is_none() && self.lower.is_empty() {
            self.lower_join = AggregateTypeJoin::new(adts, &contribution);
            self.resolved = self.lower_join.is_none().then(|| contribution.clone());
            self.lower.push(contribution);
            return;
        }
        self.lower.push(contribution);
        if let Some(joined) = &mut self.lower_join {
            let admitted =
                joined.try_join(adts, self.lower.last().expect("a lower bound was appended"));
            if admitted {
                self.resolved = None;
                return;
            }
        }
        let refs = self.lower.iter().collect::<Vec<_>>();
        self.resolved = bounded_join_refs(adts, &refs);
        self.lower_join = None;
    }

    fn inferred_result(&self) -> Option<Type> {
        self.lower_join
            .as_ref()
            .and_then(AggregateTypeJoin::resolved_result_type)
            .or_else(|| self.resolved.clone())
    }
}

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
    match (template, actual) {
        (Type::Named { name, args, .. }, actual)
            if name.starts_with("$param") && args.is_empty() =>
        {
            if let Ok(parameter) = name.trim_start_matches("$param").parse::<usize>() {
                contributions.push(ConstructorConstraintContribution {
                    parameter,
                    polarity,
                    ty: actual.clone(),
                });
            }
        }
        (
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
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            }
            | Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if same_type_identity(name, identity, actual_name, actual_identity)
            && args.len() == actual_args.len() =>
        {
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
        }
        (Type::Record(fields), Type::Record(actual_fields)) => {
            for (name, template) in fields {
                if let Some((_, actual)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    collect_template_constraints(template, actual, polarity, adts, contributions);
                }
            }
        }
        (
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                ..
            },
        ) if params.len() == actual_params.len()
            && variadic.is_some() == actual_variadic.is_some() =>
        {
            let input_polarity = polarity.compose(Variance::Contravariant);
            for (template, actual) in params.iter().zip(actual_params) {
                collect_template_constraints(template, actual, input_polarity, adts, contributions);
            }
            if let (Some(template), Some(actual)) =
                (variadic.as_deref(), actual_variadic.as_deref())
            {
                collect_template_constraints(template, actual, input_polarity, adts, contributions);
            }
            collect_template_constraints(return_type, actual_return, polarity, adts, contributions);
        }
        _ => {}
    }
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
    adts: &AdtRegistry,
) -> Result<Option<Type>, unification::TypeParameterContributionConflict> {
    let equality = solve_equality_constraints(&constraints.equality)?;
    let selected = select_constraint_candidate(constraints, equality, adts);
    selected
        .map(|selected| validate_constraint_bounds(constraints, selected, adts))
        .transpose()
}

fn solve_equality_constraints(
    equality: &[Type],
) -> Result<Option<Type>, unification::TypeParameterContributionConflict> {
    let Some((first, rest)) = equality.split_first() else {
        return Ok(None);
    };
    for actual in rest {
        if !invariant_args_match(std::slice::from_ref(first), std::slice::from_ref(actual)) {
            return Err(type_parameter_constraint_conflict(first, actual));
        }
    }
    Ok(Some(merge_candidate_presentations(
        equality.iter().collect(),
    )))
}

fn select_constraint_candidate(
    constraints: &TypeParameterConstraints,
    equality: Option<Type>,
    adts: &AdtRegistry,
) -> Option<Type> {
    if let Some(equality) = equality {
        Some(equality)
    } else if !constraints.lower.is_empty() {
        let refs = constraints.lower.iter().collect::<Vec<_>>();
        bounded_join_refs(adts, &refs)
    } else if !constraints.upper.is_empty() {
        let refs = constraints.upper.iter().collect::<Vec<_>>();
        bounded_meet_refs(adts, &refs)
    } else {
        None
    }
}

fn validate_constraint_bounds(
    constraints: &TypeParameterConstraints,
    mut selected: Type,
    adts: &AdtRegistry,
) -> Result<Type, unification::TypeParameterContributionConflict> {
    for lower in &constraints.lower {
        if !is_assignable_nested_with_registry(&selected, lower, adts) {
            return Err(type_parameter_constraint_conflict(&selected, lower));
        }
    }
    for upper in &constraints.upper {
        if !is_assignable_nested_with_registry(upper, &selected, adts) {
            return Err(type_parameter_constraint_conflict(upper, &selected));
        }
    }
    reconcile_constraint_presentations(constraints, &mut selected, adts);
    Ok(selected)
}

fn reconcile_constraint_presentations(
    constraints: &TypeParameterConstraints,
    selected: &mut Type,
    adts: &AdtRegistry,
) {
    let mut presentation = TypePresentationJoin::default();
    for contribution in constraints
        .lower
        .iter()
        .chain(&constraints.upper)
        .chain(&constraints.equality)
    {
        if is_assignable_nested_with_registry(selected, contribution, adts)
            && is_assignable_nested_with_registry(contribution, selected, adts)
        {
            presentation.merge(selected, contribution);
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
        params: children,
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

#[cfg(test)]
thread_local! {
    static FALLBACK_COMPARISONS: Cell<usize> = const { Cell::new(0) };
    static BOUNDED_FRAMES: Cell<usize> = const { Cell::new(0) };
    static FUNCTION_SCAN_NODES: Cell<usize> = const { Cell::new(0) };
    static RELATION_CACHE_CURRENT_KEY_BYTES: Cell<usize> = const { Cell::new(0) };
    static BOUNDED_CANDIDATE_WORK: Cell<BoundedCandidateWork> =
        const { Cell::new(BoundedCandidateWork {
            candidate_iterations: 0,
            relation_key_nodes: 0,
            candidate_order_nodes: 0,
            result_materialization_units: 0,
            relation_cache_peak_entries: 0,
            relation_cache_peak_key_bytes: 0,
        }) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BoundedCandidateWork {
    pub(crate) candidate_iterations: usize,
    pub(crate) relation_key_nodes: usize,
    pub(crate) candidate_order_nodes: usize,
    pub(crate) result_materialization_units: usize,
    pub(crate) relation_cache_peak_entries: usize,
    pub(crate) relation_cache_peak_key_bytes: usize,
}

#[inline(always)]
fn record_bounded_candidate_iteration() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.candidate_iterations += 1;
        work.set(current);
    });
}

#[inline(always)]
fn record_relation_key_node() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.relation_key_nodes += 1;
        work.set(current);
    });
}

#[inline(always)]
fn record_candidate_order_node() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.candidate_order_nodes += 1;
        work.set(current);
    });
}

#[inline(always)]
fn record_bounded_result_materialization(ty: &Type) {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.result_materialization_units += type_retained_units(ty);
        work.set(current);
    });
    #[cfg(not(test))]
    let _ = ty;
}

#[inline(always)]
fn record_relation_cache_insert(entries: usize, key_bytes: usize) {
    #[cfg(test)]
    RELATION_CACHE_CURRENT_KEY_BYTES.with(|current_key_bytes| {
        let key_bytes = current_key_bytes.get() + key_bytes;
        current_key_bytes.set(key_bytes);
        BOUNDED_CANDIDATE_WORK.with(|work| {
            let mut current = work.get();
            current.relation_cache_peak_entries = current.relation_cache_peak_entries.max(entries);
            current.relation_cache_peak_key_bytes =
                current.relation_cache_peak_key_bytes.max(key_bytes);
            work.set(current);
        });
    });
    #[cfg(not(test))]
    let _ = (entries, key_bytes);
}

#[inline(always)]
fn reset_relation_cache_current_key_bytes() {
    #[cfg(test)]
    RELATION_CACHE_CURRENT_KEY_BYTES.with(|current_key_bytes| current_key_bytes.set(0));
}

#[cfg(test)]
pub(crate) fn reset_bounded_candidate_work() {
    BOUNDED_CANDIDATE_WORK.with(|work| work.set(BoundedCandidateWork::default()));
    reset_relation_cache_current_key_bytes();
}

#[cfg(test)]
pub(crate) fn take_bounded_candidate_work() -> BoundedCandidateWork {
    BOUNDED_CANDIDATE_WORK.with(|work| work.replace(BoundedCandidateWork::default()))
}

#[cfg(test)]
fn record_fallback_comparison() {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.set(comparisons.get() + 1));
}

#[cfg(not(test))]
fn record_fallback_comparison() {}

#[cfg(test)]
pub(crate) fn reset_fallback_comparisons() {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.set(0));
}

#[cfg(test)]
pub(crate) fn take_fallback_comparisons() -> usize {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.replace(0))
}

#[cfg(test)]
fn record_bounded_frame() {
    BOUNDED_FRAMES.with(|frames| frames.set(frames.get() + 1));
}

#[cfg(not(test))]
fn record_bounded_frame() {}

#[cfg(test)]
pub(crate) fn reset_bounded_frames() {
    BOUNDED_FRAMES.with(|frames| frames.set(0));
}

#[cfg(test)]
pub(crate) fn take_bounded_frames() -> usize {
    BOUNDED_FRAMES.with(|frames| frames.replace(0))
}

#[cfg(test)]
fn record_function_scan_node() {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.set(nodes.get() + 1));
}

#[cfg(not(test))]
fn record_function_scan_node() {}

#[cfg(test)]
pub(crate) fn reset_function_scan_nodes() {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.set(0));
}

#[cfg(test)]
pub(crate) fn take_function_scan_nodes() -> usize {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.replace(0))
}

#[cfg(test)]
pub(crate) fn record_work(units: usize) {
    crate::inference_work::record(units);
}

#[cfg(not(test))]
pub(crate) fn record_work(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset_work() {
    crate::inference_work::reset();
}

#[cfg(test)]
pub(crate) fn take_work() -> usize {
    crate::inference_work::take()
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeepJoinWork {
    pub(crate) setup_type_nodes: usize,
    pub(crate) node_visits: usize,
    pub(crate) retained_nodes: usize,
    pub(crate) presentation_nodes: usize,
    pub(crate) materialized_nodes: usize,
}

#[cfg(test)]
thread_local! {
    static DEEP_JOIN_WORK: std::cell::Cell<DeepJoinWork> = const {
        std::cell::Cell::new(DeepJoinWork {
            setup_type_nodes: 0,
            node_visits: 0,
            retained_nodes: 0,
            presentation_nodes: 0,
            materialized_nodes: 0,
        })
    };
}

#[cfg(test)]
pub(crate) fn reset_deep_work() {
    DEEP_JOIN_WORK.set(DeepJoinWork::default());
}

#[cfg(test)]
pub(crate) fn take_deep_work() -> DeepJoinWork {
    DEEP_JOIN_WORK.replace(DeepJoinWork::default())
}

#[inline(always)]
fn record_setup_type_node() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.setup_type_nodes += 1;
        work
    });
}

#[inline(always)]
fn record_node_visit() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.node_visits += 1;
        work
    });
}

#[inline(always)]
fn record_retained_nodes(nodes: usize) {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.retained_nodes = work.retained_nodes.max(nodes);
        work
    });
    #[cfg(not(test))]
    let _ = nodes;
}

#[inline(always)]
fn record_presentation_work(nodes: usize) {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.presentation_nodes += nodes;
        work
    });
    #[cfg(not(test))]
    let _ = nodes;
}

#[inline(always)]
fn record_materialization() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.materialized_nodes += 1;
        work
    });
}
