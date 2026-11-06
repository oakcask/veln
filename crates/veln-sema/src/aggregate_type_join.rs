use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::adt::descriptors::{AdtConstructor, AdtPayloadType, Variance};
use crate::adt::registry::{AdtRegistry, ConstructorLookup, VariantDeclarationOrder};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;
use crate::type_relations::{TypePresentationJoin, invariant_args_match, same_type_identity};
use veln_ast::{Expr, ExprKind, NodeId, UseDecl};

#[derive(Default)]
pub(crate) struct CompletionContext {
    needs: RefCell<HashMap<NodeId, bool>>,
}

impl CompletionContext {
    pub(crate) fn expression_needs_completion_context(
        &self,
        expr: &Expr,
        adts: &AdtRegistry,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        self.ensure_facts(expr, adts, current_module, uses);
        self.fact(expr.node_id)
    }

    pub(crate) fn completion_context_type(
        &self,
        expr: &Expr,
        expected: &Type,
        adts: &AdtRegistry,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Type {
        self.ensure_facts(expr, adts, current_module, uses);
        let plan = ProjectionPlan::collect(expr, expected, self, adts, current_module, uses);
        plan.apply(expected)
    }

    fn fact(&self, node_id: NodeId) -> bool {
        record_completion_fact_lookup();
        self.needs.borrow().get(&node_id).copied().unwrap_or(false)
    }

    fn ensure_facts(
        &self,
        root: &Expr,
        adts: &AdtRegistry,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) {
        if self.needs.borrow().contains_key(&root.node_id) {
            record_completion_cache_hit();
            return;
        }
        let mut needs = self.needs.borrow_mut();
        let mut pending = vec![(root, false)];
        while let Some((expr, visited)) = pending.pop() {
            record_completion_fact_task();
            if needs.contains_key(&expr.node_id) {
                record_completion_cache_hit();
                continue;
            }
            if !visited {
                pending.push((expr, true));
                queue_completion_children(expr, adts, current_module, uses, |child| {
                    if !needs.contains_key(&child.node_id) {
                        pending.push((child, false));
                    } else {
                        record_completion_cache_hit();
                    }
                });
                continue;
            }
            record_completion_classification();
            let fact = classify_completion_expr(expr, &needs, adts, current_module, uses);
            needs.insert(expr.node_id, fact);
            record_completion_fact_storage();
        }
    }
}

fn queue_completion_children<'a>(
    expr: &'a Expr,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
    mut queue: impl FnMut(&'a Expr),
) {
    match &expr.kind {
        ExprKind::List(items) => {
            for item in items.iter().rev() {
                queue(item);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries.iter().rev() {
                queue(&entry.value);
                queue(&entry.key);
            }
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::NamePath { segments, .. } = &callee.kind else {
                return;
            };
            record_completion_constructor_lookup();
            let Some(constructor) = resolved_constructor(segments, adts, current_module, uses)
            else {
                return;
            };
            for (arg, field) in args.iter().zip(&constructor.variant.payload_fields).rev() {
                if constructor_is_compiler_owned(constructor)
                    || matches!(field.ty, AdtPayloadType::TypeParameter(_))
                {
                    queue(arg);
                }
            }
        }
        _ => {}
    }
}

fn classify_completion_expr(
    expr: &Expr,
    needs: &HashMap<NodeId, bool>,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    match &expr.kind {
        ExprKind::NamePath { segments, .. }
            if segments
                .last()
                .is_some_and(|name| name == "None" || name == "Nil") =>
        {
            compiler_owned_constructor(segments, adts, current_module, uses).is_some()
        }
        ExprKind::List(items) => {
            items.is_empty()
                || items
                    .iter()
                    .all(|item| completion_fact(needs, item.node_id))
        }
        ExprKind::Dict(entries) => {
            entries.is_empty()
                || entries.iter().any(|entry| {
                    completion_fact(needs, entry.key.node_id)
                        || completion_fact(needs, entry.value.node_id)
                })
        }
        ExprKind::Record(fields) => fields.is_empty(),
        ExprKind::Call { callee, args } => {
            call_completion_fact(callee, args, needs, adts, current_module, uses)
        }
        _ => false,
    }
}

fn completion_fact(needs: &HashMap<NodeId, bool>, node_id: NodeId) -> bool {
    record_completion_fact_lookup();
    needs.get(&node_id).copied().unwrap_or(false)
}

fn call_completion_fact(
    callee: &Expr,
    args: &[Expr],
    needs: &HashMap<NodeId, bool>,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return false;
    };
    record_completion_constructor_lookup();
    let Some(constructor) = resolved_constructor(segments, adts, current_module, uses) else {
        return false;
    };
    let argument_needs = args
        .iter()
        .map(|arg| completion_fact(needs, arg.node_id))
        .collect::<Vec<_>>();
    if !constructor_is_compiler_owned(constructor) {
        return argument_needs
            .iter()
            .zip(&constructor.variant.payload_fields)
            .any(|(needs_completion, field)| {
                *needs_completion && matches!(field.ty, AdtPayloadType::TypeParameter(_))
            });
    }
    let mut complete_direct_parameters = vec![false; constructor.descriptor.type_parameters.len()];
    for (field, needs_completion) in constructor
        .variant
        .payload_fields
        .iter()
        .zip(&argument_needs)
    {
        if let AdtPayloadType::TypeParameter(index) = field.ty
            && !needs_completion
            && let Some(complete) = complete_direct_parameters.get_mut(index)
        {
            *complete = true;
        }
    }
    let complete_direct_parameters = complete_direct_parameters
        .into_iter()
        .all(|complete| complete);
    argument_needs
        .iter()
        .enumerate()
        .any(|(index, needs_completion)| {
            *needs_completion
                && !(complete_direct_parameters
                    && constructor
                        .variant
                        .payload_fields
                        .get(index)
                        .is_some_and(|field| matches!(field.ty, AdtPayloadType::SelfType)))
        })
}

#[derive(Default)]
struct ProjectionNode {
    unrefine: bool,
    children: HashMap<usize, usize>,
}

struct ProjectionPlan {
    nodes: Vec<ProjectionNode>,
}

impl ProjectionPlan {
    fn collect(
        root_expr: &Expr,
        root_expected: &Type,
        facts: &CompletionContext,
        adts: &AdtRegistry,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Self {
        let mut plan = Self {
            nodes: vec![ProjectionNode::default()],
        };
        let mut pending = vec![(root_expr, root_expected, 0)];
        while let Some((expr, expected, plan_node)) = pending.pop() {
            record_completion_projection_task();
            match &expr.kind {
                ExprKind::NamePath { segments, .. } => {
                    record_completion_constructor_lookup();
                    if compiler_owned_constructor(segments, adts, current_module, uses).is_some_and(
                        |constructor| constructor_matches_expected(constructor, expected),
                    ) {
                        plan.nodes[plan_node].unrefine = true;
                    }
                }
                ExprKind::List(items) => {
                    let Some(args) =
                        expected_compiler_args(expected, Type::is_compiler_vec_identity, 1)
                    else {
                        continue;
                    };
                    for item in items {
                        if facts.fact(item.node_id) {
                            let child = plan.child(plan_node, 0);
                            pending.push((item, &args[0], child));
                        }
                    }
                }
                ExprKind::Dict(entries) => {
                    let Some(args) =
                        expected_compiler_args(expected, Type::is_compiler_dict_identity, 2)
                    else {
                        continue;
                    };
                    for entry in entries {
                        for (nested, index) in [(&entry.key, 0), (&entry.value, 1)] {
                            if facts.fact(nested.node_id) {
                                let child = plan.child(plan_node, index);
                                pending.push((nested, &args[index], child));
                            }
                        }
                    }
                }
                ExprKind::Call { callee, args } => {
                    let ExprKind::NamePath { segments, .. } = &callee.kind else {
                        continue;
                    };
                    record_completion_constructor_lookup();
                    let Some(constructor) =
                        resolved_constructor(segments, adts, current_module, uses)
                    else {
                        continue;
                    };
                    if !constructor_matches_expected(constructor, expected) {
                        continue;
                    }
                    if constructor_is_compiler_owned(constructor) {
                        plan.nodes[plan_node].unrefine = true;
                    }
                    let Some(type_args) = named_type_args(expected) else {
                        continue;
                    };
                    for (arg, field) in args.iter().zip(&constructor.variant.payload_fields) {
                        let AdtPayloadType::TypeParameter(index) = field.ty else {
                            continue;
                        };
                        if facts.fact(arg.node_id)
                            && let Some(expected_arg) = type_args.get(index)
                        {
                            let child = plan.child(plan_node, index);
                            pending.push((arg, expected_arg, child));
                        }
                    }
                }
                _ => {}
            }
        }
        plan
    }

    fn child(&mut self, parent: usize, argument: usize) -> usize {
        record_completion_plan_edge();
        if let Some(child) = self.nodes[parent].children.get(&argument) {
            return *child;
        }
        let child = self.nodes.len();
        self.nodes.push(ProjectionNode::default());
        self.nodes[parent].children.insert(argument, child);
        child
    }

    fn apply(&self, expected: &Type) -> Type {
        enum Task<'a> {
            Visit(&'a Type, usize),
            Clone(&'a Type),
            FinishNamed(String, String, usize),
            FinishVariant(
                String,
                String,
                usize,
                Arc<[String]>,
                Vec<(String, Vec<Type>, String)>,
            ),
        }
        let mut tasks = vec![Task::Visit(expected, 0)];
        let mut projected = Vec::new();
        while let Some(task) = tasks.pop() {
            record_completion_application_task();
            match task {
                Task::Visit(ty, node_index) => {
                    let node = &self.nodes[node_index];
                    match ty {
                        Type::Named {
                            name,
                            identity,
                            args,
                        } => {
                            tasks.push(Task::FinishNamed(
                                name.clone(),
                                identity.clone(),
                                args.len(),
                            ));
                            for (index, arg) in args.iter().enumerate().rev() {
                                if let Some(child) = node.children.get(&index) {
                                    tasks.push(Task::Visit(arg, *child));
                                } else {
                                    tasks.push(Task::Clone(arg));
                                }
                            }
                        }
                        Type::VariantRefinement {
                            name,
                            identity,
                            args,
                            variants,
                            unresolved_alternatives,
                        } => {
                            if node.unrefine {
                                tasks.push(Task::FinishNamed(
                                    name.clone(),
                                    identity.clone(),
                                    args.len(),
                                ));
                            } else {
                                tasks.push(Task::FinishVariant(
                                    name.clone(),
                                    identity.clone(),
                                    args.len(),
                                    variants.clone(),
                                    unresolved_alternatives.clone(),
                                ));
                            }
                            for (index, arg) in args.iter().enumerate().rev() {
                                if let Some(child) = node.children.get(&index) {
                                    tasks.push(Task::Visit(arg, *child));
                                } else {
                                    tasks.push(Task::Clone(arg));
                                }
                            }
                        }
                        _ => projected.push(clone_completion_type(ty)),
                    }
                }
                Task::Clone(ty) => projected.push(clone_completion_type(ty)),
                Task::FinishNamed(name, identity, count) => {
                    let args = projected.split_off(projected.len() - count);
                    projected.push(Type::resolved_named(name, identity, args));
                }
                Task::FinishVariant(name, identity, count, variants, unresolved_alternatives) => {
                    let args = projected.split_off(projected.len() - count);
                    projected.push(Type::VariantRefinement {
                        name,
                        identity,
                        args,
                        variants,
                        unresolved_alternatives,
                    });
                }
            }
        }
        projected
            .pop()
            .expect("completion projection produces one type")
    }
}

fn named_type_args(ty: &Type) -> Option<&[Type]> {
    match ty {
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => Some(args),
        _ => None,
    }
}

fn expected_compiler_args(
    expected: &Type,
    identity_matches: impl FnOnce(&str) -> bool,
    arity: usize,
) -> Option<&[Type]> {
    match expected {
        Type::Named { identity, args, .. } if identity_matches(identity) && args.len() == arity => {
            Some(args)
        }
        _ => None,
    }
}

fn constructor_matches_expected(constructor: AdtConstructor<'_>, expected: &Type) -> bool {
    let expected_identity = match expected {
        Type::Named { identity, .. } | Type::VariantRefinement { identity, .. } => identity,
        _ => return false,
    };
    constructor.descriptor.identity() == *expected_identity
}

fn resolved_constructor<'a>(
    segments: &[String],
    adts: &'a AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Option<AdtConstructor<'a>> {
    match adts.constructor(segments, current_module, uses) {
        ConstructorLookup::Found(constructor) => Some(constructor),
        ConstructorLookup::Ambiguous | ConstructorLookup::Missing => None,
    }
}

#[cfg(test)]
fn clone_completion_type(ty: &Type) -> Type {
    let (cloned, nodes) = crate::semantic_model::measure_type_clone_nodes(|| ty.clone());
    record_completion_type_clone(nodes);
    cloned
}

#[cfg(not(test))]
fn clone_completion_type(ty: &Type) -> Type {
    ty.clone()
}

fn constructor_is_compiler_owned(constructor: AdtConstructor<'_>) -> bool {
    constructor
        .descriptor
        .nominal_identity
        .as_deref()
        .is_some_and(Type::is_compiler_covariant_identity)
}

fn compiler_owned_constructor<'a>(
    segments: &[String],
    adts: &'a AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Option<AdtConstructor<'a>> {
    match adts.constructor(segments, current_module, uses) {
        ConstructorLookup::Found(constructor)
            if constructor
                .descriptor
                .nominal_identity
                .as_deref()
                .is_some_and(Type::is_compiler_covariant_identity) =>
        {
            Some(constructor)
        }
        ConstructorLookup::Found(_) => None,
        ConstructorLookup::Ambiguous | ConstructorLookup::Missing => match segments {
            [name] if name == "None" || name == "Nil" => {
                let type_name = if name == "None" { "Option" } else { "List" };
                let qualified = [type_name.to_string(), name.clone()];
                match adts.constructor(&qualified, current_module, uses) {
                    ConstructorLookup::Found(constructor)
                        if constructor
                            .descriptor
                            .nominal_identity
                            .as_deref()
                            .is_some_and(Type::is_compiler_covariant_identity) =>
                    {
                        Some(constructor)
                    }
                    ConstructorLookup::Found(_)
                    | ConstructorLookup::Ambiguous
                    | ConstructorLookup::Missing => None,
                }
            }
            _ => None,
        },
    }
}

pub(crate) struct AggregateTypeJoin {
    nodes: Vec<JoinNode>,
    root: usize,
}

struct JoinNode {
    name: String,
    canonical_name: String,
    presentation_conflicted: bool,
    identity: String,
    args: Vec<JoinArg>,
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
}

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
    right: AdtTypeParts<'a>,
    variant_ranks: Option<Vec<usize>>,
}

struct InitialVariantState {
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
}

impl AggregateTypeJoin {
    pub(crate) fn new(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let (root, pending) = join_node(adts, initial)?;
        let mut nodes = vec![root];
        let mut pending = pending
            .into_iter()
            .map(|(argument, ty)| (0, argument, ty))
            .collect::<Vec<_>>();
        while let Some((parent, argument, ty)) = pending.pop() {
            let (node, children) = join_node(adts, ty)?;
            let child = nodes.len();
            nodes.push(node);
            nodes[parent].args[argument] = JoinArg::Covariant(child);
            pending.extend(
                children
                    .into_iter()
                    .map(|(argument, ty)| (child, argument, ty)),
            );
        }
        record_retained_nodes(nodes.len());
        Some(Self { nodes, root: 0 })
    }

    pub(crate) fn new_resolved_refinement(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let (identity, args) = match initial {
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                (identity, args)
            }
            _ => return None,
        };
        if args.iter().any(type_contains_unknown)
            || (adts.descriptor_for_type(initial).is_none()
                && !Type::is_compiler_covariant_identity(identity))
        {
            return None;
        }
        Self::new(adts, initial)
    }

    pub(crate) fn try_join(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        let Some(prepared) = self.prepare(adts, right) else {
            return false;
        };
        self.commit(prepared, adts);
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
            let right = adt_type_parts(right)?;
            let node = &self.nodes[node_index];
            if !same_type_identity(&node.name, &node.identity, right.name, right.identity)
                || node.args.len() != right.args.len()
            {
                return None;
            }
            for (joined, right) in node.args.iter().zip(right.args) {
                match joined {
                    JoinArg::Exact { ty, .. } => {
                        if !invariant_args_match(
                            std::slice::from_ref(ty),
                            std::slice::from_ref(right),
                        ) {
                            return None;
                        }
                    }
                    JoinArg::Covariant(child) => pending.push((*child, right)),
                }
            }
            let variant_ranks = match (&node.variants, right.variants) {
                (Some(_), Some(right_variants)) => {
                    let declaration_order = node
                        .declaration_order
                        .as_ref()
                        .expect("a refinement join retains declaration order");
                    Some(
                        right_variants
                            .iter()
                            .map(|variant| declaration_order.rank(variant))
                            .collect::<Option<Vec<_>>>()?,
                    )
                }
                (Some(_), None) => None,
                (None, _) => Some(Vec::new()),
            };
            prepared.push(PreparedContribution {
                node: node_index,
                right,
                variant_ranks,
            });
        }
        Some(prepared)
    }

    fn commit(&mut self, prepared: Vec<PreparedContribution<'_>>, adts: &AdtRegistry) {
        for contribution in prepared {
            let node = &mut self.nodes[contribution.node];
            record_presentation_work(1);
            if !node.presentation_conflicted && contribution.right.name != node.canonical_name {
                if node.name == node.canonical_name {
                    node.name = contribution.right.name.to_string();
                } else if node.name != contribution.right.name {
                    node.name.clone_from(&node.canonical_name);
                    node.presentation_conflicted = true;
                }
            }
            for (joined, right) in node.args.iter_mut().zip(contribution.right.args) {
                let JoinArg::Exact {
                    ty, presentation, ..
                } = joined
                else {
                    continue;
                };
                unification::merge_type_slot(ty, right);
                presentation.merge(ty, right);
            }
            match (&mut node.variants, contribution.right.variants) {
                (Some(joined), Some(_)) => {
                    let ranks = contribution
                        .variant_ranks
                        .expect("validated refinement ranks");
                    record_work(ranks.len());
                    joined.extend(ranks);
                }
                (variants @ Some(_), None) => *variants = None,
                (None, _) => {}
            }
        }
        self.expand_covariant_nodes(adts);
    }

    fn expand_covariant_nodes(&mut self, adts: &AdtRegistry) {
        let original_nodes = self.nodes.len();
        for parent in 0..original_nodes {
            for argument in 0..self.nodes[parent].args.len() {
                let should_expand = matches!(
                    &self.nodes[parent].args[argument],
                    JoinArg::Exact {
                        ty,
                        covariant: true,
                        ..
                    } if adt_type_parts(ty).is_some()
                );
                if !should_expand {
                    continue;
                }
                let JoinArg::Exact { ty, .. } = &mut self.nodes[parent].args[argument] else {
                    unreachable!();
                };
                let source = std::mem::replace(ty, Type::Unknown);
                let (root, children) = join_node(adts, &source)
                    .expect("a concrete covariant source argument has join state");
                let child = self.nodes.len();
                self.nodes.push(root);
                self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                let mut pending = children
                    .into_iter()
                    .map(|(argument, ty)| (child, argument, ty))
                    .collect::<Vec<_>>();
                while let Some((parent, argument, ty)) = pending.pop() {
                    let (node, children) = join_node(adts, ty)
                        .expect("a nested covariant source argument has join state");
                    let child = self.nodes.len();
                    self.nodes.push(node);
                    self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                    pending.extend(
                        children
                            .into_iter()
                            .map(|(argument, ty)| (child, argument, ty)),
                    );
                }
                crate::semantic_model::drop_type_iteratively(source);
            }
        }
        record_retained_nodes(self.nodes.len());
    }

    pub(crate) fn try_join_resolved(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        let Some(right_parts) = adt_type_parts(right) else {
            return false;
        };
        if right_parts.args.iter().any(type_contains_unknown) {
            return false;
        }
        self.try_join(adts, right)
    }

    pub(crate) fn has_complete_root_domain(&self) -> bool {
        self.nodes[self.root].variants.is_none()
    }

    pub(crate) fn has_compiler_owned_root(&self) -> bool {
        Type::is_compiler_covariant_identity(&self.nodes[self.root].identity)
    }

    pub(crate) fn has_nested_refinement_domain(&self) -> bool {
        self.nodes
            .iter()
            .enumerate()
            .any(|(index, node)| index != self.root && node.variants.is_some())
    }

    pub(crate) fn has_same_root(&self, right: &Type) -> bool {
        let Some(right) = adt_type_parts(right) else {
            return false;
        };
        let root = &self.nodes[self.root];
        right
            .args
            .iter()
            .all(|argument| !type_contains_unknown(argument))
            && same_type_identity(&root.name, &root.identity, right.name, right.identity)
            && root.args.len() == right.args.len()
    }

    pub(crate) fn inference_type(&self) -> Type {
        self.materialize(false)
    }

    pub(crate) fn result_type(&self) -> Type {
        self.materialize(true)
    }

    fn materialize(&self, include_root_refinement: bool) -> Type {
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
) -> Option<(JoinNode, Vec<(usize, &'a Type)>)> {
    let initial = adt_type_parts(initial)?;
    let variant_state = initial_variant_state(adts, initial)?;
    let (args, pending) = initial_argument_state(adts, initial);
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
) -> (Vec<JoinArg>, Vec<(usize, &'a Type)>) {
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
            if variance == Variance::Covariant && adt_type_parts(ty).is_some() {
                pending.push((index, ty));
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

fn type_contains_unknown(ty: &Type) -> bool {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Unknown => return true,
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
    false
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
pub(crate) struct CompletionWork {
    pub(crate) fact_tasks: usize,
    pub(crate) cache_hits: usize,
    pub(crate) fact_lookups: usize,
    pub(crate) fact_snapshot_copies: usize,
    pub(crate) classified_expressions: usize,
    pub(crate) stored_facts: usize,
    pub(crate) constructor_lookups: usize,
    pub(crate) projection_tasks: usize,
    pub(crate) plan_edges: usize,
    pub(crate) application_tasks: usize,
    pub(crate) cloned_type_nodes: usize,
}

#[cfg(test)]
impl CompletionWork {
    pub(crate) fn total(self) -> usize {
        self.fact_tasks
            + self.cache_hits
            + self.fact_lookups
            + self.fact_snapshot_copies
            + self.classified_expressions
            + self.stored_facts
            + self.constructor_lookups
            + self.projection_tasks
            + self.plan_edges
            + self.application_tasks
            + self.cloned_type_nodes
    }
}

#[cfg(test)]
thread_local! {
    static COMPLETION_WORK: std::cell::Cell<CompletionWork> = const {
        std::cell::Cell::new(CompletionWork {
            fact_tasks: 0,
            cache_hits: 0,
            fact_lookups: 0,
            fact_snapshot_copies: 0,
            classified_expressions: 0,
            stored_facts: 0,
            constructor_lookups: 0,
            projection_tasks: 0,
            plan_edges: 0,
            application_tasks: 0,
            cloned_type_nodes: 0,
        })
    };
}

#[cfg(test)]
pub(crate) fn reset_completion_work() {
    COMPLETION_WORK.set(CompletionWork::default());
}

#[cfg(test)]
pub(crate) fn take_completion_work() -> CompletionWork {
    COMPLETION_WORK.replace(CompletionWork::default())
}

macro_rules! completion_work_counter {
    ($name:ident, $field:ident) => {
        #[inline(always)]
        fn $name() {
            #[cfg(test)]
            COMPLETION_WORK.update(|mut work| {
                work.$field += 1;
                work
            });
        }
    };
}

completion_work_counter!(record_completion_fact_task, fact_tasks);
completion_work_counter!(record_completion_cache_hit, cache_hits);
completion_work_counter!(record_completion_fact_lookup, fact_lookups);
completion_work_counter!(record_completion_classification, classified_expressions);
completion_work_counter!(record_completion_fact_storage, stored_facts);
completion_work_counter!(record_completion_constructor_lookup, constructor_lookups);
completion_work_counter!(record_completion_projection_task, projection_tasks);
completion_work_counter!(record_completion_plan_edge, plan_edges);
completion_work_counter!(record_completion_application_task, application_tasks);

#[inline(always)]
#[cfg(test)]
fn record_completion_type_clone(nodes: usize) {
    COMPLETION_WORK.update(|mut work| {
        work.cloned_type_nodes += nodes;
        work
    });
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeepJoinWork {
    pub(crate) node_visits: usize,
    pub(crate) retained_nodes: usize,
    pub(crate) presentation_nodes: usize,
    pub(crate) materialized_nodes: usize,
}

#[cfg(test)]
thread_local! {
    static DEEP_JOIN_WORK: std::cell::Cell<DeepJoinWork> = const {
        std::cell::Cell::new(DeepJoinWork {
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
