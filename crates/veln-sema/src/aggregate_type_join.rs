use std::collections::HashSet;
use std::sync::Arc;

use veln_ast::{Expr, ExprKind, UseDecl};

use crate::adt::descriptors::{AdtConstructor, Variance};
use crate::adt::registry::{AdtRegistry, ConstructorLookup, VariantDeclarationOrder};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;
use crate::type_relations::{TypePresentationJoin, invariant_args_match, same_type_identity};

pub(crate) struct AggregateTypeJoin {
    nodes: Vec<JoinNode>,
    root: usize,
}

pub(crate) fn expression_is_immediately_context_free(
    expr: &Expr,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    match &expr.kind {
        ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => true,
        ExprKind::NamePath { segments, .. } => {
            matches!(
                adts.constructor(segments, current_module, uses),
                ConstructorLookup::Found(constructor)
                    if constructor.descriptor.type_parameters.is_empty()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath { segments, .. } = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                ConstructorLookup::Missing
            )
        }
        _ => false,
    }
}

pub(crate) fn expression_has_compiler_covariant_root(
    expr: &Expr,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    match &expr.kind {
        ExprKind::List(_) | ExprKind::Dict(_) => true,
        ExprKind::NamePath { segments, .. } => {
            matches!(
                adts.constructor(segments, current_module, uses),
                ConstructorLookup::Found(constructor)
                    if Type::is_compiler_covariant_identity(&constructor.descriptor.identity())
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath { segments, .. } = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                ConstructorLookup::Found(constructor)
                    if Type::is_compiler_covariant_identity(&constructor.descriptor.identity())
            )
        }
        _ => false,
    }
}

pub(crate) fn expression_admitted_context(
    expr: &Expr,
    admitted_context: &Type,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Type {
    let (input_nodes, _) = type_shape(admitted_context);
    record_context_input_scan(input_nodes);
    record_context_retained(input_nodes);
    if let Some(projected) =
        project_direct_type_parameter_path(expr, admitted_context, adts, current_module, uses)
    {
        return projected;
    }
    expression_admitted_context_fallback(expr, admitted_context, adts, current_module, uses)
}

fn project_direct_type_parameter_path(
    mut expr: &Expr,
    mut admitted: &Type,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Option<Type> {
    let mut frames = Vec::new();
    while let ExprKind::Call { callee, args } = &expr.kind {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            break;
        };
        if args.len() != 1 {
            break;
        }
        let constructor = match adts.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor) => constructor,
            ConstructorLookup::Ambiguous => {
                adts.descriptor_for_type(admitted).and_then(|descriptor| {
                    adts.constructor_for_descriptor(segments, descriptor, current_module, uses)
                })?
            }
            ConstructorLookup::Missing => break,
        };
        let field = constructor.variant.payload_fields.first()?;
        let crate::adt::descriptors::AdtPayloadType::TypeParameter(type_index) = field.ty else {
            break;
        };
        let admitted_args = unification::adt_args(admitted, constructor.descriptor)?;
        let next = admitted_args.get(type_index)?;
        frames.push((admitted, type_index));
        expr = &args[0];
        admitted = next;
    }
    if frames.len() < 2 {
        return None;
    }

    record_context_retained(frames.len());
    let mut projected =
        expression_admitted_context_fallback(expr, admitted, adts, current_module, uses);
    for (admitted, projected_index) in frames.into_iter().rev() {
        let parts = adt_type_parts(admitted)?;
        let mut child = Some(projected);
        let args = parts
            .args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                if index == projected_index {
                    child.take().expect("one projected argument")
                } else {
                    let (nodes, _) = type_shape(arg);
                    record_context_clone(nodes);
                    arg.clone()
                }
            })
            .collect();
        projected = replace_adt_args(admitted, args);
    }
    Some(projected)
}

fn expression_admitted_context_fallback(
    expr: &Expr,
    admitted_context: &Type,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Type {
    enum ProjectionTask<'expr, 'registry> {
        Evaluate(&'expr Expr, Type),
        FinishList(Type, usize),
        FinishDict(usize),
        FinishCall {
            admitted: Type,
            constructor: AdtConstructor<'registry>,
            expected_args: Vec<Type>,
            affected: Vec<Vec<usize>>,
        },
    }

    let mut tasks = vec![ProjectionTask::Evaluate(
        expr,
        clone_context_type(admitted_context),
    )];
    let mut results = Vec::new();
    while let Some(task) = tasks.pop() {
        record_context_retained(tasks.len() + results.len() + 1);
        match task {
            ProjectionTask::Evaluate(expr, admitted) => {
                record_context_classification_work(1);
                match &expr.kind {
                    ExprKind::List(items) => {
                        let Some(item_context) = adt_type_parts(&admitted)
                            .and_then(|parts| parts.args.first())
                            .map(clone_context_type)
                        else {
                            results.push(Type::Unknown);
                            continue;
                        };
                        if items.is_empty() {
                            results.push(admitted);
                            continue;
                        }
                        tasks.push(ProjectionTask::FinishList(admitted, items.len()));
                        for item in items.iter().rev() {
                            tasks.push(ProjectionTask::Evaluate(
                                item,
                                clone_context_type(&item_context),
                            ));
                        }
                    }
                    ExprKind::Dict(entries) => {
                        let Some((key_context, value_context)) = admitted.dict_parts() else {
                            results.push(Type::Unknown);
                            continue;
                        };
                        let key_context = clone_context_type(key_context);
                        let value_context = clone_context_type(value_context);
                        if entries.is_empty() {
                            results.push(admitted);
                            continue;
                        }
                        tasks.push(ProjectionTask::FinishDict(entries.len()));
                        for entry in entries.iter().rev() {
                            tasks.push(ProjectionTask::Evaluate(
                                &entry.value,
                                clone_context_type(&value_context),
                            ));
                            tasks.push(ProjectionTask::Evaluate(
                                &entry.key,
                                clone_context_type(&key_context),
                            ));
                        }
                    }
                    ExprKind::Record(fields) => results.push(if fields.is_empty() {
                        admitted
                    } else {
                        Type::Unknown
                    }),
                    ExprKind::NamePath { segments, .. } => {
                        let requires = match adts.constructor(segments, current_module, uses) {
                            ConstructorLookup::Found(constructor) => {
                                constructor.variant.payload_fields.is_empty()
                                    && !constructor.descriptor.type_parameters.is_empty()
                            }
                            ConstructorLookup::Ambiguous => adts
                                .descriptor_for_type(&admitted)
                                .and_then(|descriptor| {
                                    adts.constructor_for_descriptor(
                                        segments,
                                        descriptor,
                                        current_module,
                                        uses,
                                    )
                                })
                                .is_some_and(|constructor| {
                                    constructor.variant.payload_fields.is_empty()
                                }),
                            ConstructorLookup::Missing => false,
                        };
                        results.push(if requires { admitted } else { Type::Unknown });
                    }
                    ExprKind::Call { callee, args } => {
                        let ExprKind::NamePath { segments, .. } = &callee.kind else {
                            results.push(Type::Unknown);
                            continue;
                        };
                        let constructor = match adts.constructor(segments, current_module, uses) {
                            ConstructorLookup::Found(constructor) => constructor,
                            ConstructorLookup::Ambiguous => {
                                let Some(constructor) =
                                    adts.descriptor_for_type(&admitted).and_then(|descriptor| {
                                        adts.constructor_for_descriptor(
                                            segments,
                                            descriptor,
                                            current_module,
                                            uses,
                                        )
                                    })
                                else {
                                    results.push(Type::Unknown);
                                    continue;
                                };
                                constructor
                            }
                            ConstructorLookup::Missing => {
                                results.push(Type::Unknown);
                                continue;
                            }
                        };
                        let Some(expected_args) =
                            unification::adt_args(&admitted, constructor.descriptor).map(|args| {
                                args.iter().map(clone_context_type).collect::<Vec<Type>>()
                            })
                        else {
                            results.push(Type::Unknown);
                            continue;
                        };
                        let mut affected = Vec::with_capacity(args.len());
                        let mut payload_contexts = Vec::with_capacity(args.len());
                        for index in 0..args.len() {
                            let mut indices = HashSet::new();
                            adt::visit_type_arg_indices_from_payload(
                                constructor,
                                index,
                                |type_index| {
                                    record_context_classification_work(1);
                                    indices.insert(type_index);
                                },
                            );
                            let mut indices = indices.into_iter().collect::<Vec<_>>();
                            indices.sort_unstable();
                            affected.push(indices);
                            payload_contexts.push(
                                adt::payload_type_with_resolved_args(
                                    constructor,
                                    index,
                                    |type_index| clone_context_type(&expected_args[type_index]),
                                )
                                .unwrap_or(Type::Unknown),
                            );
                        }
                        tasks.push(ProjectionTask::FinishCall {
                            admitted,
                            constructor,
                            expected_args,
                            affected,
                        });
                        for (arg, context) in args.iter().zip(payload_contexts).rev() {
                            tasks.push(ProjectionTask::Evaluate(arg, context));
                        }
                    }
                    _ => results.push(Type::Unknown),
                }
            }
            ProjectionTask::FinishList(admitted, count) => {
                let projected = results.split_off(results.len() - count);
                let mut required = Type::Unknown;
                for projected in projected {
                    adt::merge_type_holes(&mut required, &projected);
                }
                results.push(if required != Type::Unknown {
                    replace_adt_args(&admitted, vec![required])
                } else {
                    Type::Unknown
                });
            }
            ProjectionTask::FinishDict(count) => {
                let projected = results.split_off(results.len() - count * 2);
                let mut required_key = Type::Unknown;
                let mut required_value = Type::Unknown;
                for pair in projected.as_chunks::<2>().0 {
                    adt::merge_type_holes(&mut required_key, &pair[0]);
                    adt::merge_type_holes(&mut required_value, &pair[1]);
                }
                results.push(
                    if required_key != Type::Unknown || required_value != Type::Unknown {
                        record_context_materialization(1);
                        Type::dict(required_key, required_value)
                    } else {
                        Type::Unknown
                    },
                );
            }
            ProjectionTask::FinishCall {
                admitted,
                constructor,
                expected_args,
                affected,
            } => {
                let projected = results.split_off(results.len() - affected.len());
                let mut required_args = expected_args
                    .iter()
                    .map(clone_context_type)
                    .collect::<Vec<_>>();
                let mut provided = vec![false; expected_args.len()];
                for (index, (projected, affected)) in
                    projected.into_iter().zip(affected).enumerate()
                {
                    for type_index in &affected {
                        provided[*type_index] = true;
                    }
                    if projected == Type::Unknown {
                        for type_index in affected {
                            required_args[type_index] = Type::Unknown;
                        }
                        continue;
                    }
                    adt::visit_type_arg_contributions_from_payload(
                        constructor,
                        index,
                        &projected,
                        |type_index, required| {
                            if required_args[type_index] != Type::Unknown {
                                required_args[type_index] = clone_context_type(required);
                            }
                        },
                    );
                }
                for (index, was_provided) in provided.into_iter().enumerate() {
                    if !was_provided {
                        required_args[index] = clone_context_type(&expected_args[index]);
                    }
                }
                results.push(replace_adt_args(&admitted, required_args));
            }
        }
    }
    results.pop().unwrap_or(Type::Unknown)
}

fn replace_adt_args(ty: &Type, args: Vec<Type>) -> Type {
    record_context_materialization(1);
    match ty {
        Type::Named { name, identity, .. } => {
            Type::resolved_named(name.clone(), identity.clone(), args)
        }
        Type::VariantRefinement {
            name,
            identity,
            variants,
            unresolved_alternatives,
            ..
        } => Type::VariantRefinement {
            name: name.clone(),
            identity: identity.clone(),
            args: args.into(),
            variants: variants.clone(),
            unresolved_alternatives: unresolved_alternatives.clone(),
        },
        _ => Type::Unknown,
    }
}

fn clone_context_type(ty: &Type) -> Type {
    // Named type arguments are copy-on-write shared. Count the retained root
    // handle here; walking the shared suffix would make accounting itself
    // quadratic and would no longer describe owned state.
    record_context_clone(1);
    ty.clone()
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
        let compiler_covariant = Type::is_compiler_covariant_identity(identity);
        if (!compiler_covariant && args.iter().any(|argument| type_shape(argument).1))
            || (adts.descriptor_for_type(initial).is_none() && !compiler_covariant)
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
                let compiler_covariant = Type::is_compiler_covariant_identity(&node.identity);
                match joined {
                    JoinArg::Exact {
                        ty,
                        covariant: true,
                        ..
                    } if compiler_covariant
                        && (ty == &Type::Unknown || right == &Type::Unknown) => {}
                    JoinArg::Exact { ty, .. } => {
                        if !invariant_args_match(
                            std::slice::from_ref(ty),
                            std::slice::from_ref(right),
                        ) {
                            return None;
                        }
                    }
                    JoinArg::Covariant(_) if compiler_covariant && right == &Type::Unknown => {}
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
        if adt_type_parts(right).is_none() {
            return false;
        }
        self.try_join(adts, right)
    }

    pub(crate) fn has_compiler_covariant_root(&self) -> bool {
        Type::is_compiler_covariant_identity(&self.nodes[self.root].identity)
    }

    pub(crate) fn has_complete_domain(&self) -> bool {
        self.nodes[self.root].variants.is_none()
    }

    pub(crate) fn has_incomplete_nested_refinement_domain(&self) -> bool {
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

fn type_shape(ty: &Type) -> (usize, bool) {
    let mut nodes = 0usize;
    let mut contains_unknown = false;
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        nodes += 1;
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
thread_local! {
    static CONTEXT_CLASSIFICATION_WORK: std::cell::Cell<ContextProjectionWork> =
        const { std::cell::Cell::new(ContextProjectionWork::new()) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContextProjectionWork {
    pub(crate) input_type_nodes_scanned: usize,
    pub(crate) expression_or_parameter_edges_scanned: usize,
    pub(crate) type_nodes_cloned: usize,
    pub(crate) materialized_type_nodes: usize,
    pub(crate) max_retained_state: usize,
}

#[cfg(test)]
impl ContextProjectionWork {
    const fn new() -> Self {
        Self {
            input_type_nodes_scanned: 0,
            expression_or_parameter_edges_scanned: 0,
            type_nodes_cloned: 0,
            materialized_type_nodes: 0,
            max_retained_state: 0,
        }
    }

    pub(crate) fn total_relevant_work(self) -> usize {
        self.input_type_nodes_scanned
            + self.expression_or_parameter_edges_scanned
            + self.type_nodes_cloned
            + self.materialized_type_nodes
    }
}

#[cfg(test)]
fn update_context_work(update: impl FnOnce(&mut ContextProjectionWork)) {
    CONTEXT_CLASSIFICATION_WORK.with(|work| {
        let mut current = work.get();
        update(&mut current);
        work.set(current);
    });
}

#[cfg(test)]
fn record_context_classification_work(units: usize) {
    update_context_work(|work| work.expression_or_parameter_edges_scanned += units);
}

#[cfg(not(test))]
fn record_context_classification_work(_units: usize) {}

#[cfg(test)]
fn record_context_input_scan(units: usize) {
    update_context_work(|work| work.input_type_nodes_scanned += units);
}

#[cfg(not(test))]
fn record_context_input_scan(_units: usize) {}

#[cfg(test)]
fn record_context_clone(units: usize) {
    update_context_work(|work| work.type_nodes_cloned += units);
}

#[cfg(not(test))]
fn record_context_clone(_units: usize) {}

#[cfg(test)]
fn record_context_materialization(units: usize) {
    update_context_work(|work| work.materialized_type_nodes += units);
}

#[cfg(not(test))]
fn record_context_materialization(_units: usize) {}

#[cfg(test)]
pub(crate) fn record_context_template_materialization(ty: &Type) {
    let (nodes, _) = type_shape(ty);
    record_context_materialization(nodes);
}

#[cfg(not(test))]
pub(crate) fn record_context_template_materialization(_ty: &Type) {}

#[cfg(test)]
fn record_context_retained(units: usize) {
    update_context_work(|work| work.max_retained_state = work.max_retained_state.max(units));
}

#[cfg(not(test))]
fn record_context_retained(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset_context_classification_work() {
    CONTEXT_CLASSIFICATION_WORK.with(|work| work.set(ContextProjectionWork::new()));
}

#[cfg(test)]
pub(crate) fn take_context_classification_work() -> ContextProjectionWork {
    CONTEXT_CLASSIFICATION_WORK.with(|work| work.replace(ContextProjectionWork::new()))
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
