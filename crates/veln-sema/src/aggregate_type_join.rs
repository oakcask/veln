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
    TypePresentationJoin, invariant_args_match, is_assignable_with_registry, same_type_identity,
};

pub(crate) struct AggregateTypeJoin {
    nodes: Vec<JoinNode>,
    root: usize,
    optimized_contributions: Option<Vec<Type>>,
    bounded: Option<BoundedJoin>,
    projection_cache: RefCell<context_projection::ProjectionCache>,
}

struct BoundedJoin {
    contributions: Vec<Type>,
    resolved: Option<Type>,
    fallback: Option<ContributedCandidateState>,
    relation_cache: HashMap<(String, String), bool>,
}

struct ContributedCandidateState {
    direction: BoundDirection,
    assignable: Vec<Vec<bool>>,
    valid: Vec<bool>,
    selection_blockers: Vec<usize>,
}

impl BoundedJoin {
    fn from_contributions(adts: &AdtRegistry, contributions: Vec<Type>) -> Self {
        let fallback = uses_outer_contributed_fallback(&contributions)
            .then(|| ContributedCandidateState::new(adts, &contributions, BoundDirection::Join));
        let mut relation_cache = HashMap::new();
        let resolved = fallback
            .as_ref()
            .map(|state| state.result(&contributions))
            .unwrap_or_else(|| bounded_join_with_cache(adts, &contributions, &mut relation_cache));
        Self {
            contributions,
            resolved,
            fallback,
            relation_cache,
        }
    }

    fn admit(&mut self, adts: &AdtRegistry, right: &Type) {
        self.contributions.push(right.clone());
        if let Some(fallback) = &mut self.fallback {
            fallback.admit(adts, &self.contributions);
            self.resolved = fallback.result(&self.contributions);
            return;
        }
        self.resolved =
            bounded_join_with_cache(adts, &self.contributions, &mut self.relation_cache);
        if uses_outer_contributed_fallback(&self.contributions) {
            self.fallback = Some(ContributedCandidateState::new(
                adts,
                &self.contributions,
                BoundDirection::Join,
            ));
        }
    }
}

impl ContributedCandidateState {
    fn new(adts: &AdtRegistry, contributions: &[Type], direction: BoundDirection) -> Self {
        let mut state = Self {
            direction,
            assignable: Vec::new(),
            valid: Vec::new(),
            selection_blockers: Vec::new(),
        };
        for end in 1..=contributions.len() {
            state.admit(adts, &contributions[..end]);
        }
        state
    }

    fn admit(&mut self, adts: &AdtRegistry, contributions: &[Type]) {
        let new_index = contributions.len() - 1;
        debug_assert_eq!(self.assignable.len(), new_index);
        for row in &mut self.assignable {
            row.push(false);
        }
        let mut new_row = vec![false; contributions.len()];
        for index in 0..new_index {
            self.assignable[index][new_index] =
                candidate_assignable(&contributions[index], &contributions[new_index], adts);
            new_row[index] =
                candidate_assignable(&contributions[new_index], &contributions[index], adts);
        }
        new_row[new_index] = true;
        self.assignable.push(new_row);

        let previously_valid = self.valid.clone();
        for index in 0..new_index {
            if self.valid[index] && !self.valid_relation(index, new_index) {
                self.valid[index] = false;
            }
        }
        let new_valid = (0..=new_index).all(|other| self.valid_relation(new_index, other));
        self.valid.push(new_valid);
        self.selection_blockers.push(0);

        for (removed, was_valid) in previously_valid.iter().enumerate().take(new_index) {
            if *was_valid && !self.valid[removed] {
                for candidate in 0..new_index {
                    if self.valid[candidate] && !self.selection_relation(candidate, removed) {
                        self.selection_blockers[candidate] -= 1;
                    }
                }
            }
        }
        if new_valid {
            for candidate in 0..new_index {
                if self.valid[candidate] {
                    if !self.selection_relation(candidate, new_index) {
                        self.selection_blockers[candidate] += 1;
                    }
                    if !self.selection_relation(new_index, candidate) {
                        self.selection_blockers[new_index] += 1;
                    }
                }
            }
        }
    }

    fn valid_relation(&self, candidate: usize, contribution: usize) -> bool {
        match self.direction {
            BoundDirection::Join => self.assignable[candidate][contribution],
            BoundDirection::Meet => self.assignable[contribution][candidate],
        }
    }

    fn selection_relation(&self, candidate: usize, other: usize) -> bool {
        match self.direction {
            BoundDirection::Join => self.assignable[other][candidate],
            BoundDirection::Meet => self.assignable[candidate][other],
        }
    }

    fn result(&self, contributions: &[Type]) -> Option<Type> {
        let mut selected = self
            .valid
            .iter()
            .zip(&self.selection_blockers)
            .enumerate()
            .filter_map(|(index, (valid, blockers))| (*valid && *blockers == 0).then_some(index))
            .collect::<Vec<_>>();
        selected.sort_unstable_by_key(|index| contributions[*index].render());
        let first = *selected.first()?;
        let mut result = contributions[first].clone();
        let mut presentation = TypePresentationJoin::default();
        for index in selected.into_iter().skip(1) {
            presentation.merge(&mut result, &contributions[index]);
        }
        Some(result)
    }
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
                bounded: Some(BoundedJoin {
                    contributions: vec![initial.clone()],
                    resolved: Some(initial.clone()),
                    fallback: None,
                    relation_cache: HashMap::new(),
                }),
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
            let (node, children) = join_node(adts, ty, record_subtree_is_complete)?;
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
            if !bounded_shapes_are_admissible(adts, &bounded.contributions, right) {
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
            self.optimized_contributions = None;
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

    pub(crate) fn preserves_failed_control_flow_contribution(
        &self,
        adts: &AdtRegistry,
        right: &Type,
    ) -> bool {
        if let Some(bounded) = &self.bounded {
            return bounded_shapes_are_admissible(adts, &bounded.contributions, right);
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
        for node in &mut self.nodes {
            for argument in &mut node.args {
                if let JoinArg::Exact { ty, .. } = argument {
                    crate::semantic_model::drop_type_iteratively(std::mem::replace(
                        ty,
                        Type::Unknown,
                    ));
                }
            }
        }
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

pub(crate) fn merge_invariant_payload_type_args(
    inferred: &mut [Type],
    joined: &mut [Option<AggregateTypeJoin>],
    invariant: &mut [bool],
    presentations: &mut [TypePresentationJoin],
    constructor: AdtConstructor<'_>,
    index: usize,
    actual: &Type,
) -> Result<(), unification::TypeParameterContributionConflict> {
    let mut contributions = Vec::new();
    adt::visit_type_arg_contributions_from_payload(
        constructor,
        index,
        actual,
        |type_index, contribution| {
            record_work(1);
            contributions.push((type_index, contribution.clone()));
        },
    );
    let constraints = unification::merge_type_parameter_contributions_transactionally(
        &contributions,
        |type_index| {
            inferred.get(type_index).map(|inferred| {
                (
                    joined[type_index]
                        .as_ref()
                        .map(AggregateTypeJoin::result_type)
                        .unwrap_or_else(|| inferred.clone()),
                    presentations[type_index].clone(),
                )
            })
        },
    )?;
    for (type_index, constraint, presentation) in constraints {
        record_work(1);
        inferred[type_index] = constraint;
        joined[type_index] = None;
        invariant[type_index] = true;
        presentations[type_index] = presentation;
    }
    Ok(())
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

fn bounded_join_with_cache(
    adts: &AdtRegistry,
    contributions: &[Type],
    relation_cache: &mut HashMap<(String, String), bool>,
) -> Option<Type> {
    bounded_join_iterative(adts, contributions, relation_cache)
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
    contributions: &[Type],
    relation_cache: &mut HashMap<(String, String), bool>,
) -> Option<Type> {
    let mut tasks = vec![BoundTask::Evaluate {
        contributions: contributions.iter().collect(),
        direction: BoundDirection::Join,
    }];
    let mut results = Vec::new();
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

fn evaluate_bound_task<'a>(
    adts: &AdtRegistry,
    tasks: &mut Vec<BoundTask<'a>>,
    results: &mut Vec<Option<Type>>,
    contributions: Vec<&'a Type>,
    direction: BoundDirection,
    relation_cache: &mut HashMap<(String, String), bool>,
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
    } else if refs_require_composite_bound(adts, &contributions) {
        schedule_composite_bound(adts, tasks, results, contributions, relation_cache);
    } else {
        let owned = contributions
            .iter()
            .map(|contribution| (*contribution).clone())
            .collect::<Vec<_>>();
        results.push(optimized_join(adts, &owned).or_else(|| {
            contributed_candidate_refs(adts, &contributions, direction, relation_cache)
        }));
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
    results.drain(start..).collect::<Option<Vec<_>>>()
}

fn refs_require_composite_bound(adts: &AdtRegistry, contributions: &[&Type]) -> bool {
    if contributions.iter().any(|ty| type_contains_function(ty)) {
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
        .then(|| arguments[0].clone());
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
    selected.sort_unstable_by_key(|candidate| candidate.render());
    Some(merge_candidate_presentations(selected))
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
    let mut candidates = selected.into_iter();
    let mut result = candidates
        .next()
        .expect("selected candidates are nonempty")
        .clone();
    let mut presentation = TypePresentationJoin::default();
    for candidate in candidates {
        presentation.merge(&mut result, candidate);
    }
    result
}

fn bounded_shapes_are_admissible(adts: &AdtRegistry, contributions: &[Type], right: &Type) -> bool {
    if contributions.is_empty() {
        return false;
    }
    contributions
        .iter()
        .all(|contribution| structurally_compatible_function_shapes(adts, contribution, right))
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
    if left.len() != right.len()
        || left
            .iter()
            .any(|(name, _)| !right.iter().any(|(other, _)| other == name))
    {
        return true;
    }
    for (name, left) in left {
        let Some((_, right)) = right.iter().find(|(right_name, _)| right_name == name) else {
            return false;
        };
        pending.push((left, right));
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

fn type_contains_function(ty: &Type) -> bool {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Function { .. } => return true,
            Type::Named { args, .. } | Type::VariantRefinement { args, .. } => pending.extend(args),
            Type::Record(fields) => pending.extend(fields.iter().map(|(_, field)| field)),
            Type::Unknown => {}
        }
    }
    false
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
    is_assignable_with_registry(expected, actual, adts)
}

fn candidate_assignable_cached(
    expected: &Type,
    actual: &Type,
    adts: &AdtRegistry,
    relation_cache: &mut HashMap<(String, String), bool>,
) -> bool {
    let key = (
        canonical_relation_key(expected),
        canonical_relation_key(actual),
    );
    if let Some(assignable) = relation_cache.get(&key) {
        return *assignable;
    }
    let assignable = candidate_assignable(expected, actual, adts);
    relation_cache.insert(key, assignable);
    assignable
}

enum RelationKeyPart<'a> {
    Type(&'a Type),
    Text(&'a str),
    End,
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
