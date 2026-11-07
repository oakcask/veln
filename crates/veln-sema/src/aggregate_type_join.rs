use std::cell::RefCell;
use std::collections::HashSet;
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
    contributions: Vec<Type>,
    bounded: Option<BoundedJoin>,
    projection_cache: RefCell<context_projection::ProjectionCache>,
}

struct BoundedJoin {
    contributions: Vec<Type>,
    resolved: Option<Type>,
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
                contributions: Vec::new(),
                bounded: Some(BoundedJoin {
                    contributions: vec![initial.clone()],
                    resolved: Some(initial.clone()),
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
            contributions: vec![initial.clone()],
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
            if !bounded_shapes_are_admissible(&bounded.contributions, right) {
                return false;
            }
            bounded.contributions.push(right.clone());
            bounded.resolved = bounded_join(adts, &bounded.contributions);
            self.projection_cache.get_mut().clear();
            return true;
        }
        let Some(prepared) = self.prepare(adts, right) else {
            if !bounded_shapes_are_admissible(&self.contributions, right) {
                return false;
            }
            let mut contributions = self.contributions.clone();
            contributions.push(right.clone());
            if bounded_join(adts, &contributions).is_none()
                && !can_retain_unresolved_contributions(adts, &contributions)
            {
                return false;
            }
            self.bounded = Some(BoundedJoin {
                resolved: bounded_join(adts, &contributions),
                contributions,
            });
            self.projection_cache.get_mut().clear();
            return true;
        };
        if self.commit(prepared, adts) {
            self.projection_cache.get_mut().clear();
        }
        self.contributions.push(right.clone());
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
            let right_args = match &contribution.right {
                PreparedTypeParts::Adt(right) => {
                    record_presentation_work(1);
                    if !node.presentation_conflicted && right.name != node.canonical_name {
                        if node.name == node.canonical_name {
                            node.name = right.name.to_string();
                            changed = true;
                        } else if node.name != right.name {
                            node.name.clone_from(&node.canonical_name);
                            node.presentation_conflicted = true;
                            changed = true;
                        }
                    }
                    right.args.iter().collect::<Vec<_>>()
                }
                PreparedTypeParts::Record(fields) => fields.clone(),
            };
            for (joined, right) in node.args.iter_mut().zip(right_args) {
                let JoinArg::Exact {
                    ty,
                    presentation,
                    covariant,
                } = joined
                else {
                    continue;
                };
                if *covariant
                    && Type::is_compiler_covariant_identity(&node.identity)
                    && right == &Type::Unknown
                {
                    continue;
                }
                changed |= unification::merge_type_slot(ty, right);
                changed |= presentation.merge(ty, right);
            }
            let right_variants = match contribution.right {
                PreparedTypeParts::Adt(right) => right.variants,
                PreparedTypeParts::Record(_) => None,
            };
            match (&mut node.variants, right_variants) {
                (Some(joined), Some(_)) => {
                    let ranks = contribution
                        .variant_ranks
                        .expect("validated refinement ranks");
                    record_work(ranks.len());
                    let previous_len = joined.len();
                    joined.extend(ranks);
                    changed |= joined.len() != previous_len;
                }
                (variants @ Some(_), None) => {
                    *variants = None;
                    changed = true;
                }
                (None, _) => {}
            }
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

fn can_retain_unresolved_contributions(adts: &AdtRegistry, contributions: &[Type]) -> bool {
    if contributions.iter().any(type_contains_function) {
        return true;
    }
    match contributions.first() {
        Some(Type::Record(first)) => contributions.iter().any(|contribution| {
            let Type::Record(fields) = contribution else {
                return false;
            };
            fields.len() != first.len()
                || first
                    .iter()
                    .any(|(name, _)| !fields.iter().any(|(other, _)| other == name))
        }),
        Some(Type::Named { identity, args, .. })
        | Some(Type::VariantRefinement { identity, args, .. }) => adts
            .parameter_variances_for_identity(identity, args.len())
            .is_some_and(|variances| {
                variances
                    .iter()
                    .any(|variance| *variance != Variance::Covariant)
            }),
        _ => false,
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

fn bounded_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    if contributions.len() == 1 {
        return contributions.first().cloned();
    }
    if contributions
        .iter()
        .any(|contribution| matches!(contribution, Type::Function { .. }))
    {
        return bounded_function_join(adts, contributions);
    }
    if requires_composite_bound(adts, contributions) {
        return bounded_composite_join(adts, contributions);
    }
    if let Some(joined) = optimized_join(adts, contributions) {
        return Some(joined);
    }
    contributed_candidate(adts, contributions, BoundDirection::Join)
}

fn requires_composite_bound(adts: &AdtRegistry, contributions: &[Type]) -> bool {
    if contributions.iter().any(type_contains_function) {
        return true;
    }
    let Some(Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. }) =
        contributions.first()
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

fn bounded_shapes_are_admissible(contributions: &[Type], right: &Type) -> bool {
    let Some(first) = contributions.first() else {
        return false;
    };
    structurally_compatible_function_shapes(first, right)
}

fn structurally_compatible_function_shapes(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (
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
        ) => {
            left_params.len() == right_params.len()
                && left_variadic.is_some() == right_variadic.is_some()
                && left_params
                    .iter()
                    .zip(right_params)
                    .all(|(left, right)| structurally_compatible_function_shapes(left, right))
                && match (left_variadic, right_variadic) {
                    (Some(left), Some(right)) => {
                        structurally_compatible_function_shapes(left, right)
                    }
                    (None, None) => true,
                    _ => false,
                }
                && structurally_compatible_function_shapes(left_return, right_return)
        }
        (Type::Function { .. }, _) | (_, Type::Function { .. }) => false,
        (Type::Record(left), Type::Record(right)) if left.len() == right.len() => {
            left.iter().all(|(name, left)| {
                right
                    .iter()
                    .find(|(right_name, _)| right_name == name)
                    .is_some_and(|(_, right)| structurally_compatible_function_shapes(left, right))
            })
        }
        (
            Type::Named {
                name: left_name,
                identity: left_identity,
                args: left_args,
                ..
            }
            | Type::VariantRefinement {
                name: left_name,
                identity: left_identity,
                args: left_args,
                ..
            },
            Type::Named {
                name: right_name,
                identity: right_identity,
                args: right_args,
                ..
            }
            | Type::VariantRefinement {
                name: right_name,
                identity: right_identity,
                args: right_args,
                ..
            },
        ) => {
            same_type_identity(left_name, left_identity, right_name, right_identity)
                && left_args.len() == right_args.len()
                && left_args
                    .iter()
                    .zip(right_args)
                    .all(|(left, right)| structurally_compatible_function_shapes(left, right))
        }
        (Type::Record(_), Type::Record(_)) => true,
        (Type::Record(_), _) | (_, Type::Record(_)) => false,
        _ => true,
    }
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

fn bounded_composite_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    match contributions.first()? {
        Type::Record(first_fields) => {
            let mut names = first_fields
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            names.sort_unstable();
            if contributions.iter().any(|contribution| {
                let Type::Record(fields) = contribution else {
                    return true;
                };
                fields.len() != names.len()
                    || names
                        .iter()
                        .any(|name| !fields.iter().any(|(field_name, _)| field_name == name))
            }) {
                return None;
            }
            let fields = names
                .into_iter()
                .map(|name| {
                    let field_contributions = contributions
                        .iter()
                        .map(|contribution| {
                            let Type::Record(fields) = contribution else {
                                return None;
                            };
                            fields
                                .iter()
                                .find(|(field_name, _)| field_name == &name)
                                .map(|(_, ty)| ty.clone())
                        })
                        .collect::<Option<Vec<_>>>()?;
                    (field_contributions.len() == contributions.len())
                        .then(|| bounded_join(adts, &field_contributions))
                        .flatten()
                        .map(|ty| (name, ty))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Type::Record(fields))
        }
        Type::Named { .. } | Type::VariantRefinement { .. } => {
            bounded_named_join(adts, contributions)
        }
        Type::Function { .. } => bounded_function_join(adts, contributions),
        Type::Unknown => None,
    }
}

fn bounded_named_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    let first = adt_type_parts(contributions.first()?)?;
    let parts = contributions
        .iter()
        .map(adt_type_parts)
        .collect::<Option<Vec<_>>>()?;
    if parts.iter().any(|part| {
        !same_type_identity(first.name, first.identity, part.name, part.identity)
            || first.args.len() != part.args.len()
    }) {
        return None;
    }
    let compiler_covariant = Type::is_compiler_covariant_identity(first.identity);
    let variances = (!compiler_covariant)
        .then(|| adts.parameter_variances_for_identity(first.identity, first.args.len()))
        .flatten();
    let args = (0..first.args.len())
        .map(|index| {
            let arguments = parts
                .iter()
                .map(|part| part.args[index].clone())
                .collect::<Vec<_>>();
            let variance = if compiler_covariant {
                Variance::Covariant
            } else {
                variances
                    .and_then(|variances| variances.get(index).copied())
                    .unwrap_or(Variance::Invariant)
            };
            match variance {
                Variance::Covariant => bounded_join(adts, &arguments),
                Variance::Contravariant => bounded_meet(adts, &arguments),
                Variance::Invariant | Variance::Irrelevant => arguments
                    .iter()
                    .skip(1)
                    .all(|argument| {
                        invariant_args_match(
                            std::slice::from_ref(&arguments[0]),
                            std::slice::from_ref(argument),
                        )
                    })
                    .then(|| arguments[0].clone()),
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let all_refined = parts.iter().all(|part| part.variants.is_some());
    let mut joined = if all_refined {
        let order = adts.variant_declaration_order_for_type(first.ty)?;
        let mut ranks = parts
            .iter()
            .flat_map(|part| part.variants.expect("all parts are refined"))
            .map(|variant| order.rank(variant))
            .collect::<Option<HashSet<_>>>()?
            .into_iter()
            .collect::<Vec<_>>();
        ranks.sort_unstable();
        let variants = ranks
            .into_iter()
            .map(|rank| order.name(rank).map(str::to_string))
            .collect::<Option<Vec<_>>>()?;
        if variants.len() == order.len() {
            Type::resolved_named(first.name, first.identity, args)
        } else {
            Type::resolved_variant_refinement(first.name, first.identity, args, variants)
        }
    } else {
        Type::resolved_named(first.name, first.identity, args)
    };
    let mut ordered = contributions.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|contribution| contribution.render());
    let mut presentation = TypePresentationJoin::default();
    for contribution in ordered {
        presentation.merge(&mut joined, contribution);
    }
    Some(joined)
}

fn bounded_meet(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    contributed_candidate(adts, contributions, BoundDirection::Meet)
}

fn optimized_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    let mut joined = AggregateTypeJoin::new_optimized(adts, contributions.first()?)?;
    for contribution in &contributions[1..] {
        let prepared = joined.prepare(adts, contribution)?;
        joined.commit(prepared, adts);
    }
    Some(joined.result_type())
}

fn bounded_function_join(adts: &AdtRegistry, contributions: &[Type]) -> Option<Type> {
    let Type::Function {
        params: first_params,
        variadic: first_variadic,
        ..
    } = contributions.first()?
    else {
        return None;
    };
    let mut parameter_contributions = (0..first_params.len())
        .map(|_| Vec::with_capacity(contributions.len()))
        .collect::<Vec<_>>();
    let mut variadic_contributions = first_variadic
        .as_ref()
        .map(|_| Vec::with_capacity(contributions.len()));
    let mut result_contributions = Vec::with_capacity(contributions.len());
    let mut effects = HashSet::new();
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
        for (slot, parameter) in parameter_contributions.iter_mut().zip(params) {
            slot.push(parameter.clone());
        }
        if let (Some(slot), Some(variadic)) = (&mut variadic_contributions, variadic) {
            slot.push(variadic.as_ref().clone());
        }
        result_contributions.push(return_type.as_ref().clone());
        effects.extend(contribution_effects.iter().cloned());
    }
    let params = parameter_contributions
        .iter()
        .map(|contributions| bounded_meet(adts, contributions))
        .collect::<Option<Vec<_>>>()?;
    let variadic = match &variadic_contributions {
        Some(contributions) => Some(Box::new(bounded_meet(adts, contributions)?)),
        None => None,
    };
    let return_type = Box::new(bounded_join(adts, &result_contributions)?);
    let mut effects = effects.into_iter().collect::<Vec<_>>();
    effects.sort_unstable();
    Some(Type::Function {
        params,
        variadic,
        return_type,
        effects,
    })
}

#[derive(Clone, Copy)]
enum BoundDirection {
    Join,
    Meet,
}

fn contributed_candidate(
    adts: &AdtRegistry,
    contributions: &[Type],
    direction: BoundDirection,
) -> Option<Type> {
    let valid = contributions
        .iter()
        .filter(|candidate| {
            contributions.iter().all(|contribution| match direction {
                BoundDirection::Join => is_assignable_with_registry(candidate, contribution, adts),
                BoundDirection::Meet => is_assignable_with_registry(contribution, candidate, adts),
            })
        })
        .collect::<Vec<_>>();
    let selected = valid
        .iter()
        .copied()
        .filter(|candidate| {
            valid.iter().all(|other| match direction {
                BoundDirection::Join => is_assignable_with_registry(other, candidate, adts),
                BoundDirection::Meet => is_assignable_with_registry(candidate, other, adts),
            })
        })
        .collect::<Vec<_>>();
    let first = *selected.first()?;
    if selected.iter().skip(1).any(|candidate| {
        !is_assignable_with_registry(first, candidate, adts)
            || !is_assignable_with_registry(candidate, first, adts)
    }) {
        return None;
    }
    let mut ordered = selected;
    ordered.sort_unstable_by_key(|candidate| candidate.render());
    let mut result = (*ordered[0]).clone();
    let mut presentation = TypePresentationJoin::default();
    for candidate in ordered.into_iter().skip(1) {
        presentation.merge(&mut result, candidate);
    }
    Some(result)
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
