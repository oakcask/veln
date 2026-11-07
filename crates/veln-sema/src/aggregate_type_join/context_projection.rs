use std::collections::HashSet;

use veln_ast::{Expr, ExprKind, UseDecl};

use crate::adt::descriptors::{AdtConstructor, AdtPayloadType};
use crate::adt::registry::{AdtRegistry, ConstructorLookup};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;

use super::{adt_type_parts, type_shape};

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
        ExprKind::NamePath { segments, .. } => matches!(
            adts.constructor(segments, current_module, uses),
            ConstructorLookup::Found(constructor)
                if constructor.descriptor.type_parameters.is_empty()
        ),
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
    let segments = match &expr.kind {
        ExprKind::List(_) | ExprKind::Dict(_) => return true,
        ExprKind::NamePath { segments, .. } => segments,
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath { segments, .. } = &callee.kind else {
                return false;
            };
            segments
        }
        _ => return false,
    };
    matches!(
        adts.constructor(segments, current_module, uses),
        ConstructorLookup::Found(constructor)
            if Type::is_compiler_covariant_identity(&constructor.descriptor.identity())
    )
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
    project_direct_type_parameter_path(expr, admitted_context, adts, current_module, uses)
        .unwrap_or_else(|| {
            expression_admitted_context_fallback(expr, admitted_context, adts, current_module, uses)
        })
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
        let Some((next_expr, next_admitted, type_index)) =
            direct_projection_step(callee, args, admitted, adts, current_module, uses)?
        else {
            break;
        };
        frames.push((admitted, type_index));
        expr = next_expr;
        admitted = next_admitted;
    }
    if frames.len() < 2 {
        return None;
    }
    record_context_retained(frames.len());
    let projected =
        expression_admitted_context_fallback(expr, admitted, adts, current_module, uses);
    rebuild_direct_projection(projected, frames)
}

fn direct_projection_step<'expr, 'ty>(
    callee: &'expr Expr,
    args: &'expr [Expr],
    admitted: &'ty Type,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Option<Option<(&'expr Expr, &'ty Type, usize)>> {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return Some(None);
    };
    if args.len() != 1 {
        return Some(None);
    }
    let constructor = match adts.constructor(segments, current_module, uses) {
        ConstructorLookup::Found(constructor) => constructor,
        ConstructorLookup::Ambiguous => {
            adts.descriptor_for_type(admitted).and_then(|descriptor| {
                adts.constructor_for_descriptor(segments, descriptor, current_module, uses)
            })?
        }
        ConstructorLookup::Missing => return Some(None),
    };
    let field = constructor.variant.payload_fields.first()?;
    let AdtPayloadType::TypeParameter(type_index) = field.ty else {
        return Some(None);
    };
    let admitted_args = unification::adt_args(admitted, constructor.descriptor)?;
    Some(Some((&args[0], admitted_args.get(type_index)?, type_index)))
}

fn rebuild_direct_projection(mut projected: Type, frames: Vec<(&Type, usize)>) -> Option<Type> {
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

struct ContextProjection<'expr, 'registry> {
    adts: &'registry AdtRegistry,
    current_module: Option<&'registry str>,
    uses: &'registry [UseDecl],
    tasks: Vec<ProjectionTask<'expr, 'registry>>,
    results: Vec<Type>,
}

fn expression_admitted_context_fallback<'expr, 'registry>(
    expr: &'expr Expr,
    admitted_context: &Type,
    adts: &'registry AdtRegistry,
    current_module: Option<&'registry str>,
    uses: &'registry [UseDecl],
) -> Type {
    ContextProjection {
        adts,
        current_module,
        uses,
        tasks: vec![ProjectionTask::Evaluate(
            expr,
            clone_context_type(admitted_context),
        )],
        results: Vec::new(),
    }
    .run()
}

impl<'expr, 'registry> ContextProjection<'expr, 'registry> {
    fn run(mut self) -> Type {
        while let Some(task) = self.tasks.pop() {
            record_context_retained(self.tasks.len() + self.results.len() + 1);
            self.perform(task);
        }
        self.results.pop().unwrap_or(Type::Unknown)
    }

    fn perform(&mut self, task: ProjectionTask<'expr, 'registry>) {
        match task {
            ProjectionTask::Evaluate(expr, admitted) => self.evaluate(expr, admitted),
            ProjectionTask::FinishList(admitted, count) => self.finish_list(admitted, count),
            ProjectionTask::FinishDict(count) => self.finish_dict(count),
            ProjectionTask::FinishCall {
                admitted,
                constructor,
                expected_args,
                affected,
            } => self.finish_call(admitted, constructor, expected_args, affected),
        }
    }

    fn evaluate(&mut self, expr: &'expr Expr, admitted: Type) {
        record_context_classification_work(1);
        match &expr.kind {
            ExprKind::List(items) => self.evaluate_list(items, admitted),
            ExprKind::Dict(entries) => self.evaluate_dict(entries, admitted),
            ExprKind::Record(fields) => self.results.push(if fields.is_empty() {
                admitted
            } else {
                Type::Unknown
            }),
            ExprKind::NamePath { segments, .. } => self.evaluate_name(segments, admitted),
            ExprKind::Call { callee, args } => self.evaluate_call(callee, args, admitted),
            _ => self.results.push(Type::Unknown),
        }
    }

    fn evaluate_list(&mut self, items: &'expr [Expr], admitted: Type) {
        let Some(item_context) = adt_type_parts(&admitted)
            .and_then(|parts| parts.args.first())
            .map(clone_context_type)
        else {
            self.results.push(Type::Unknown);
            return;
        };
        if items.is_empty() {
            self.results.push(admitted);
            return;
        }
        self.tasks
            .push(ProjectionTask::FinishList(admitted, items.len()));
        for item in items.iter().rev() {
            self.tasks.push(ProjectionTask::Evaluate(
                item,
                clone_context_type(&item_context),
            ));
        }
    }

    fn evaluate_dict(&mut self, entries: &'expr [veln_ast::DictEntry], admitted: Type) {
        let Some((key_context, value_context)) = admitted.dict_parts() else {
            self.results.push(Type::Unknown);
            return;
        };
        let key_context = clone_context_type(key_context);
        let value_context = clone_context_type(value_context);
        if entries.is_empty() {
            self.results.push(admitted);
            return;
        }
        self.tasks.push(ProjectionTask::FinishDict(entries.len()));
        for entry in entries.iter().rev() {
            self.tasks.push(ProjectionTask::Evaluate(
                &entry.value,
                clone_context_type(&value_context),
            ));
            self.tasks.push(ProjectionTask::Evaluate(
                &entry.key,
                clone_context_type(&key_context),
            ));
        }
    }

    fn evaluate_name(&mut self, segments: &[String], admitted: Type) {
        let requires = match self
            .adts
            .constructor(segments, self.current_module, self.uses)
        {
            ConstructorLookup::Found(constructor) => {
                constructor.variant.payload_fields.is_empty()
                    && !constructor.descriptor.type_parameters.is_empty()
            }
            ConstructorLookup::Ambiguous => self
                .constructor_for_admitted(segments, &admitted)
                .is_some_and(|constructor| constructor.variant.payload_fields.is_empty()),
            ConstructorLookup::Missing => false,
        };
        self.results
            .push(if requires { admitted } else { Type::Unknown });
    }

    fn evaluate_call(&mut self, callee: &Expr, args: &'expr [Expr], admitted: Type) {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            self.results.push(Type::Unknown);
            return;
        };
        let Some(constructor) = self.resolve_constructor(segments, &admitted) else {
            self.results.push(Type::Unknown);
            return;
        };
        let Some(expected_args) = unification::adt_args(&admitted, constructor.descriptor)
            .map(|args| args.iter().map(clone_context_type).collect::<Vec<_>>())
        else {
            self.results.push(Type::Unknown);
            return;
        };
        let (affected, payload_contexts) =
            payload_projection_contexts(constructor, args.len(), &expected_args);
        self.tasks.push(ProjectionTask::FinishCall {
            admitted,
            constructor,
            expected_args,
            affected,
        });
        for (arg, context) in args.iter().zip(payload_contexts).rev() {
            self.tasks.push(ProjectionTask::Evaluate(arg, context));
        }
    }

    fn resolve_constructor(
        &self,
        segments: &[String],
        admitted: &Type,
    ) -> Option<AdtConstructor<'registry>> {
        match self
            .adts
            .constructor(segments, self.current_module, self.uses)
        {
            ConstructorLookup::Found(constructor) => Some(constructor),
            ConstructorLookup::Ambiguous => self.constructor_for_admitted(segments, admitted),
            ConstructorLookup::Missing => None,
        }
    }

    fn constructor_for_admitted(
        &self,
        segments: &[String],
        admitted: &Type,
    ) -> Option<AdtConstructor<'registry>> {
        self.adts
            .descriptor_for_type(admitted)
            .and_then(|descriptor| {
                self.adts.constructor_for_descriptor(
                    segments,
                    descriptor,
                    self.current_module,
                    self.uses,
                )
            })
    }

    fn finish_list(&mut self, admitted: Type, count: usize) {
        let projected = self.results.split_off(self.results.len() - count);
        let mut required = Type::Unknown;
        for projected in projected {
            adt::merge_type_holes(&mut required, &projected);
        }
        self.results.push(if required != Type::Unknown {
            replace_adt_args(&admitted, vec![required])
        } else {
            Type::Unknown
        });
    }

    fn finish_dict(&mut self, count: usize) {
        let projected = self.results.split_off(self.results.len() - count * 2);
        let mut required_key = Type::Unknown;
        let mut required_value = Type::Unknown;
        for pair in projected.as_chunks::<2>().0 {
            adt::merge_type_holes(&mut required_key, &pair[0]);
            adt::merge_type_holes(&mut required_value, &pair[1]);
        }
        self.results.push(
            if required_key != Type::Unknown || required_value != Type::Unknown {
                record_context_materialization(1);
                Type::dict(required_key, required_value)
            } else {
                Type::Unknown
            },
        );
    }

    fn finish_call(
        &mut self,
        admitted: Type,
        constructor: AdtConstructor<'registry>,
        expected_args: Vec<Type>,
        affected: Vec<Vec<usize>>,
    ) {
        let projected = self.results.split_off(self.results.len() - affected.len());
        let mut required_args = expected_args
            .iter()
            .map(clone_context_type)
            .collect::<Vec<_>>();
        let mut provided = vec![false; expected_args.len()];
        for (index, (projected, affected)) in projected.into_iter().zip(affected).enumerate() {
            mark_provided(&mut provided, &affected);
            if projected == Type::Unknown {
                clear_affected(&mut required_args, affected);
            } else {
                merge_payload_projection(constructor, index, &projected, &mut required_args);
            }
        }
        restore_unprovided(&mut required_args, &expected_args, provided);
        self.results
            .push(replace_adt_args(&admitted, required_args));
    }
}

fn payload_projection_contexts(
    constructor: AdtConstructor<'_>,
    payload_count: usize,
    expected_args: &[Type],
) -> (Vec<Vec<usize>>, Vec<Type>) {
    let mut affected = Vec::with_capacity(payload_count);
    let mut payload_contexts = Vec::with_capacity(payload_count);
    for index in 0..payload_count {
        let mut indices = HashSet::new();
        adt::visit_type_arg_indices_from_payload(constructor, index, |type_index| {
            record_context_classification_work(1);
            indices.insert(type_index);
        });
        let mut indices = indices.into_iter().collect::<Vec<_>>();
        indices.sort_unstable();
        affected.push(indices);
        record_payload_template_materialization(constructor, index);
        payload_contexts.push(
            adt::payload_type_with_resolved_args(constructor, index, |type_index| {
                clone_context_type(&expected_args[type_index])
            })
            .unwrap_or(Type::Unknown),
        );
    }
    (affected, payload_contexts)
}

fn mark_provided(provided: &mut [bool], affected: &[usize]) {
    for index in affected {
        provided[*index] = true;
    }
}

fn clear_affected(required_args: &mut [Type], affected: Vec<usize>) {
    for index in affected {
        required_args[index] = Type::Unknown;
    }
}

fn merge_payload_projection(
    constructor: AdtConstructor<'_>,
    payload_index: usize,
    projected: &Type,
    required_args: &mut [Type],
) {
    adt::visit_type_arg_contributions_from_payload(
        constructor,
        payload_index,
        projected,
        |type_index, required| {
            if required_args[type_index] != Type::Unknown {
                required_args[type_index] = clone_context_type(required);
            }
        },
    );
}

fn restore_unprovided(required_args: &mut [Type], expected_args: &[Type], provided: Vec<bool>) {
    for (index, was_provided) in provided.into_iter().enumerate() {
        if !was_provided {
            required_args[index] = clone_context_type(&expected_args[index]);
        }
    }
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

fn record_payload_template_materialization(constructor: AdtConstructor<'_>, payload_index: usize) {
    let Some(AdtPayloadType::Concrete(template)) = constructor
        .variant
        .payload_fields
        .get(payload_index)
        .map(|field| &field.ty)
    else {
        return;
    };
    let (nodes, _) = type_shape(template);
    record_context_materialization(nodes);
}

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
