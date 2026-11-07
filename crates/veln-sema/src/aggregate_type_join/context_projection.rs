use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use veln_ast::{Expr, ExprKind, UseDecl};

use crate::adt::descriptors::{AdtConstructor, AdtPayloadType};
use crate::adt::registry::{AdtRegistry, ConstructorLookup};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;

use super::adt_type_parts;
#[cfg(test)]
use super::type_shape;

pub(crate) fn expression_is_immediately_context_free(
    expr: &Expr,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
    local_context: &dyn Fn(&str) -> Option<LocalContext>,
) -> bool {
    match &expr.kind {
        ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => true,
        ExprKind::NamePath { segments, .. } => {
            if let [name] = segments.as_slice()
                && let Some(local) = local_context(name)
            {
                return matches!(local, LocalContext::Complete);
            }
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
    local_context: &dyn Fn(&str) -> Option<LocalContext>,
) -> Type {
    record_context_input_type(admitted_context);
    project_direct_type_parameter_path(
        expr,
        admitted_context,
        adts,
        current_module,
        uses,
        local_context,
    )
    .unwrap_or_else(|| {
        let admitted = Rc::new(clone_context_type(admitted_context));
        let (projected, _, ambiguous) = expression_admitted_context_fallback(
            expr,
            Rc::clone(&admitted),
            adts,
            current_module,
            uses,
            local_context,
            HashMap::new(),
        );
        if ambiguous {
            into_owned_type(admitted)
        } else {
            projected
        }
    })
}

#[derive(Default)]
pub(super) struct ProjectionCache {
    admitted: Option<Rc<Type>>,
    direct_argument_contexts: DirectArgumentContexts,
    input_recorded: bool,
}

impl ProjectionCache {
    pub(super) fn clear(&mut self) {
        self.admitted = None;
        self.direct_argument_contexts.clear();
        self.input_recorded = false;
    }
}

impl super::AggregateTypeJoin {
    pub(crate) fn expression_admitted_context(
        &self,
        expr: &Expr,
        adts: &AdtRegistry,
        current_module: Option<&str>,
        uses: &[UseDecl],
        local_context: &dyn Fn(&str) -> Option<LocalContext>,
    ) -> Type {
        let mut cache = self.projection_cache.borrow_mut();
        let admitted = Rc::clone(
            cache
                .admitted
                .get_or_insert_with(|| Rc::new(self.inference_type())),
        );
        if !cache.input_recorded {
            record_context_input_type(admitted.as_ref());
            cache.input_recorded = true;
        }
        if let Some(projected) = project_direct_type_parameter_path(
            expr,
            admitted.as_ref(),
            adts,
            current_module,
            uses,
            local_context,
        ) {
            return projected;
        }
        let direct_argument_contexts = std::mem::take(&mut cache.direct_argument_contexts);
        let (projected, direct_argument_contexts, ambiguous) = expression_admitted_context_fallback(
            expr,
            Rc::clone(&admitted),
            adts,
            current_module,
            uses,
            local_context,
            direct_argument_contexts,
        );
        cache.direct_argument_contexts = direct_argument_contexts;
        if ambiguous {
            clone_context_type(admitted.as_ref())
        } else {
            projected
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum LocalContext {
    Complete,
    UnknownBearing,
    EmptyRecord,
}

fn project_direct_type_parameter_path(
    mut expr: &Expr,
    mut admitted: &Type,
    adts: &AdtRegistry,
    current_module: Option<&str>,
    uses: &[UseDecl],
    local_context: &dyn Fn(&str) -> Option<LocalContext>,
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
    let projected = expression_admitted_context_fallback(
        expr,
        Rc::new(clone_context_type(admitted)),
        adts,
        current_module,
        uses,
        local_context,
        HashMap::new(),
    )
    .0;
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
    let Some(constructor) = projection_constructor(adts, segments, current_module, uses) else {
        return Some(None);
    };
    let field = constructor.variant.payload_fields.first()?;
    let AdtPayloadType::TypeParameter(type_index) = field.ty else {
        return Some(None);
    };
    let admitted_args = unification::adt_args(admitted, constructor.descriptor)?;
    Some(Some((&args[0], admitted_args.get(type_index)?, type_index)))
}

fn projection_constructor<'registry>(
    adts: &'registry AdtRegistry,
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> Option<AdtConstructor<'registry>> {
    match adts.constructor(segments, current_module, uses) {
        ConstructorLookup::Found(constructor) => Some(constructor),
        ConstructorLookup::Ambiguous | ConstructorLookup::Missing => None,
    }
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
                    clone_context_type(arg)
                }
            })
            .collect();
        projected = replace_adt_args(admitted, args);
    }
    Some(projected)
}

enum ProjectionTask<'expr, 'registry> {
    Evaluate(&'expr Expr, Rc<Type>),
    FinishList(Rc<Type>, usize),
    FinishDict(usize),
    FinishDirectCall {
        admitted: Rc<Type>,
        expected: Rc<Type>,
        type_index: usize,
    },
    FinishCall {
        admitted: Rc<Type>,
        constructor: AdtConstructor<'registry>,
        expected_args: Vec<Type>,
        affected: Vec<Vec<usize>>,
    },
}

type DirectArgumentContexts = HashMap<(*const Type, usize), (Rc<Type>, Rc<Type>)>;

struct ContextProjection<'expr, 'registry> {
    adts: &'registry AdtRegistry,
    current_module: Option<&'registry str>,
    uses: &'registry [UseDecl],
    local_context: &'registry dyn Fn(&str) -> Option<LocalContext>,
    local_contexts: HashMap<String, Option<LocalContext>>,
    direct_argument_contexts: DirectArgumentContexts,
    tasks: Vec<ProjectionTask<'expr, 'registry>>,
    results: Vec<Rc<Type>>,
    ambiguous_constructor: bool,
}

fn expression_admitted_context_fallback<'registry>(
    expr: &Expr,
    admitted_context: Rc<Type>,
    adts: &'registry AdtRegistry,
    current_module: Option<&'registry str>,
    uses: &'registry [UseDecl],
    local_context: &'registry dyn Fn(&str) -> Option<LocalContext>,
    direct_argument_contexts: DirectArgumentContexts,
) -> (Type, DirectArgumentContexts, bool) {
    ContextProjection {
        adts,
        current_module,
        uses,
        local_context,
        local_contexts: HashMap::new(),
        direct_argument_contexts,
        tasks: vec![ProjectionTask::Evaluate(expr, admitted_context)],
        results: Vec::new(),
        ambiguous_constructor: false,
    }
    .run()
}

#[cfg(test)]
fn owned_type_nodes(ty: &Type, seen_args: &mut HashSet<*const Vec<Type>>) -> usize {
    let mut nodes = 0;
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        nodes += 1;
        match ty {
            Type::Unknown => {}
            Type::Named { args, .. } => {
                if seen_args.insert(args.allocation_ptr()) {
                    pending.extend(args);
                }
            }
            Type::VariantRefinement {
                args,
                unresolved_alternatives,
                ..
            } => {
                if seen_args.insert(args.allocation_ptr()) {
                    pending.extend(args);
                }
                for (_, alternatives, _) in unresolved_alternatives {
                    pending.extend(alternatives);
                }
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
    nodes
}

impl<'expr, 'registry> ContextProjection<'expr, 'registry> {
    fn run(mut self) -> (Type, DirectArgumentContexts, bool) {
        while let Some(task) = self.tasks.pop() {
            self.record_retained_state(Some(&task));
            self.perform(task);
        }
        let projected = self
            .results
            .pop()
            .map(into_owned_type)
            .unwrap_or(Type::Unknown);
        (
            projected,
            self.direct_argument_contexts,
            self.ambiguous_constructor,
        )
    }

    #[cfg(test)]
    fn record_retained_state(&self, active: Option<&ProjectionTask<'expr, 'registry>>) {
        fn retain_context(
            context: &Rc<Type>,
            seen: &mut HashSet<*const Type>,
            seen_args: &mut HashSet<*const Vec<Type>>,
            units: &mut usize,
        ) {
            if seen.insert(Rc::as_ptr(context)) {
                *units += owned_type_nodes(context.as_ref(), seen_args);
            } else {
                *units += 1;
            }
        }

        fn retain_task(
            task: &ProjectionTask<'_, '_>,
            seen: &mut HashSet<*const Type>,
            seen_args: &mut HashSet<*const Vec<Type>>,
            units: &mut usize,
        ) {
            match task {
                ProjectionTask::Evaluate(_, admitted) | ProjectionTask::FinishList(admitted, _) => {
                    retain_context(admitted, seen, seen_args, units);
                }
                ProjectionTask::FinishDict(_) => {}
                ProjectionTask::FinishDirectCall {
                    admitted, expected, ..
                } => {
                    retain_context(admitted, seen, seen_args, units);
                    retain_context(expected, seen, seen_args, units);
                }
                ProjectionTask::FinishCall {
                    admitted,
                    expected_args,
                    affected,
                    ..
                } => {
                    retain_context(admitted, seen, seen_args, units);
                    *units += expected_args
                        .iter()
                        .map(|expected| owned_type_nodes(expected, seen_args))
                        .sum::<usize>();
                    *units += affected.len() + affected.iter().map(Vec::len).sum::<usize>();
                }
            }
        }

        let mut seen = HashSet::<*const Type>::new();
        let mut seen_args = HashSet::<*const Vec<Type>>::new();
        let mut units = self.tasks.len()
            + self.results.len()
            + self.local_contexts.len()
            + self.direct_argument_contexts.len()
            + usize::from(active.is_some());
        for task in &self.tasks {
            retain_task(task, &mut seen, &mut seen_args, &mut units);
        }
        if let Some(task) = active {
            retain_task(task, &mut seen, &mut seen_args, &mut units);
        }
        for result in &self.results {
            retain_context(result, &mut seen, &mut seen_args, &mut units);
        }
        for (admitted, argument) in self.direct_argument_contexts.values() {
            retain_context(admitted, &mut seen, &mut seen_args, &mut units);
            retain_context(argument, &mut seen, &mut seen_args, &mut units);
        }
        record_context_retained(units);
    }

    #[cfg(not(test))]
    fn record_retained_state(&self, _active: Option<&ProjectionTask<'expr, 'registry>>) {}

    fn perform(&mut self, task: ProjectionTask<'expr, 'registry>) {
        match task {
            ProjectionTask::Evaluate(expr, admitted) => self.evaluate(expr, admitted),
            ProjectionTask::FinishList(admitted, count) => self.finish_list(admitted, count),
            ProjectionTask::FinishDict(count) => self.finish_dict(count),
            ProjectionTask::FinishDirectCall {
                admitted,
                expected,
                type_index,
            } => self.finish_direct_call(admitted, expected, type_index),
            ProjectionTask::FinishCall {
                admitted,
                constructor,
                expected_args,
                affected,
            } => self.finish_call(admitted, constructor, expected_args, affected),
        }
    }

    fn evaluate(&mut self, expr: &'expr Expr, admitted: Rc<Type>) {
        record_context_classification_work(1);
        match &expr.kind {
            ExprKind::List(items) => self.evaluate_list(items, admitted),
            ExprKind::Dict(entries) => self.evaluate_dict(entries, admitted),
            ExprKind::Record(fields) => self.results.push(if fields.is_empty() {
                admitted
            } else {
                Rc::new(Type::Unknown)
            }),
            ExprKind::NamePath { segments, .. } => self.evaluate_name(segments, admitted),
            ExprKind::Call { callee, args } => self.evaluate_call(callee, args, admitted),
            _ => self.results.push(Rc::new(Type::Unknown)),
        }
    }

    fn evaluate_list(&mut self, items: &'expr [Expr], admitted: Rc<Type>) {
        let Some(item_context) = adt_type_parts(admitted.as_ref())
            .and_then(|parts| parts.args.first())
            .map(|item| Rc::new(clone_context_type(item)))
        else {
            self.results.push(Rc::new(Type::Unknown));
            return;
        };
        if items.is_empty() {
            self.results.push(admitted);
            return;
        }
        self.tasks
            .push(ProjectionTask::FinishList(admitted, items.len()));
        for item in items.iter().rev() {
            self.tasks
                .push(ProjectionTask::Evaluate(item, Rc::clone(&item_context)));
        }
    }

    fn evaluate_dict(&mut self, entries: &'expr [veln_ast::DictEntry], admitted: Rc<Type>) {
        let Some((key_context, value_context)) = admitted.dict_parts() else {
            self.results.push(Rc::new(Type::Unknown));
            return;
        };
        let key_context = Rc::new(clone_context_type(key_context));
        let value_context = Rc::new(clone_context_type(value_context));
        if entries.is_empty() {
            self.results.push(admitted);
            return;
        }
        self.tasks.push(ProjectionTask::FinishDict(entries.len()));
        for entry in entries.iter().rev() {
            self.tasks.push(ProjectionTask::Evaluate(
                &entry.value,
                Rc::clone(&value_context),
            ));
            self.tasks.push(ProjectionTask::Evaluate(
                &entry.key,
                Rc::clone(&key_context),
            ));
        }
    }

    fn evaluate_name(&mut self, segments: &[String], admitted: Rc<Type>) {
        if let [name] = segments {
            let local = match self.local_contexts.get(name) {
                Some(local) => *local,
                None => {
                    let local = (self.local_context)(name);
                    self.local_contexts.insert(name.clone(), local);
                    local
                }
            };
            if let Some(local) = local {
                let requires = match local {
                    LocalContext::Complete => false,
                    LocalContext::UnknownBearing => true,
                    LocalContext::EmptyRecord => {
                        admitted.dict_parts().is_some()
                            && !crate::types::type_has_unknown(admitted.as_ref())
                    }
                };
                self.results.push(if requires {
                    admitted
                } else {
                    Rc::new(Type::Unknown)
                });
                return;
            }
        }
        let requires = match self
            .adts
            .constructor(segments, self.current_module, self.uses)
        {
            ConstructorLookup::Found(constructor) => {
                constructor.variant.payload_fields.is_empty()
                    && !constructor.descriptor.type_parameters.is_empty()
            }
            ConstructorLookup::Ambiguous => {
                self.ambiguous_constructor = true;
                false
            }
            ConstructorLookup::Missing => false,
        };
        self.results.push(if requires {
            admitted
        } else {
            Rc::new(Type::Unknown)
        });
    }

    fn evaluate_call(&mut self, callee: &Expr, args: &'expr [Expr], admitted: Rc<Type>) {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            self.results.push(Rc::new(Type::Unknown));
            return;
        };
        let constructor = match self
            .adts
            .constructor(segments, self.current_module, self.uses)
        {
            ConstructorLookup::Found(constructor) => constructor,
            ConstructorLookup::Ambiguous => {
                self.ambiguous_constructor = true;
                self.results.push(Rc::new(Type::Unknown));
                return;
            }
            ConstructorLookup::Missing => {
                self.results.push(Rc::new(Type::Unknown));
                return;
            }
        };
        if let Some(type_index) = direct_payload_type_parameter(constructor, args.len())
            && let Some(expected) = self.direct_argument_context(&admitted, constructor, type_index)
        {
            self.tasks.push(ProjectionTask::FinishDirectCall {
                admitted,
                expected: Rc::clone(&expected),
                type_index,
            });
            self.tasks
                .push(ProjectionTask::Evaluate(&args[0], expected));
            return;
        }
        let Some(expected_args) = unification::adt_args(admitted.as_ref(), constructor.descriptor)
            .map(|args| args.iter().map(clone_context_type).collect::<Vec<_>>())
        else {
            self.results.push(Rc::new(Type::Unknown));
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
            self.tasks
                .push(ProjectionTask::Evaluate(arg, Rc::new(context)));
        }
    }

    fn direct_argument_context(
        &mut self,
        admitted: &Rc<Type>,
        constructor: AdtConstructor<'_>,
        type_index: usize,
    ) -> Option<Rc<Type>> {
        let key = (Rc::as_ptr(admitted), type_index);
        if let Some((_, context)) = self.direct_argument_contexts.get(&key) {
            record_shared_context_reuse();
            return Some(Rc::clone(context));
        }
        let argument =
            unification::adt_args(admitted.as_ref(), constructor.descriptor)?.get(type_index)?;
        let context = Rc::new(clone_context_type(argument));
        self.direct_argument_contexts
            .insert(key, (Rc::clone(admitted), Rc::clone(&context)));
        Some(context)
    }

    fn finish_list(&mut self, admitted: Rc<Type>, count: usize) {
        let projected = self.results.split_off(self.results.len() - count);
        let mut required = Rc::new(Type::Unknown);
        for projected in projected {
            merge_projected_type_holes(&mut required, &projected);
        }
        self.results.push(if required.as_ref() != &Type::Unknown {
            Rc::new(replace_adt_args(
                admitted.as_ref(),
                vec![into_owned_type(required)],
            ))
        } else {
            Rc::new(Type::Unknown)
        });
    }

    fn finish_dict(&mut self, count: usize) {
        let projected = self.results.split_off(self.results.len() - count * 2);
        let mut required_key = Rc::new(Type::Unknown);
        let mut required_value = Rc::new(Type::Unknown);
        for pair in projected.as_chunks::<2>().0 {
            merge_projected_type_holes(&mut required_key, &pair[0]);
            merge_projected_type_holes(&mut required_value, &pair[1]);
        }
        self.results.push(
            if required_key.as_ref() != &Type::Unknown || required_value.as_ref() != &Type::Unknown
            {
                record_context_materialization(1);
                Rc::new(Type::dict(
                    into_owned_type(required_key),
                    into_owned_type(required_value),
                ))
            } else {
                Rc::new(Type::Unknown)
            },
        );
    }

    fn finish_call(
        &mut self,
        admitted: Rc<Type>,
        constructor: AdtConstructor<'registry>,
        expected_args: Vec<Type>,
        affected: Vec<Vec<usize>>,
    ) {
        let projected = self.results.split_off(self.results.len() - affected.len());
        let mut required_args = vec![Type::Unknown; expected_args.len()];
        let mut provided = vec![false; expected_args.len()];
        for (index, (projected, affected)) in projected.into_iter().zip(affected).enumerate() {
            mark_provided(&mut provided, &affected);
            if projected.as_ref() != &Type::Unknown {
                merge_payload_projection(constructor, index, &projected, &mut required_args);
            }
        }
        restore_unprovided(&mut required_args, &expected_args, provided);
        self.results
            .push(Rc::new(replace_adt_args(admitted.as_ref(), required_args)));
    }

    fn finish_direct_call(&mut self, admitted: Rc<Type>, expected: Rc<Type>, type_index: usize) {
        let projected = self.results.pop().expect("one direct payload projection");
        if Rc::ptr_eq(&projected, &expected) {
            record_shared_context_reuse();
            self.results.push(admitted);
            return;
        }
        let Some(parts) = adt_type_parts(admitted.as_ref()) else {
            self.results.push(Rc::new(Type::Unknown));
            return;
        };
        let mut projected = Some(into_owned_type(projected));
        let args = parts
            .args
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                if index == type_index {
                    projected.take().expect("one direct projected argument")
                } else {
                    clone_context_type(argument)
                }
            })
            .collect();
        self.results
            .push(Rc::new(replace_adt_args(admitted.as_ref(), args)));
    }
}

fn direct_payload_type_parameter(
    constructor: AdtConstructor<'_>,
    payload_count: usize,
) -> Option<usize> {
    if payload_count != 1 || constructor.variant.payload_fields.len() != 1 {
        return None;
    }
    match constructor.variant.payload_fields[0].ty {
        AdtPayloadType::TypeParameter(type_index) => Some(type_index),
        AdtPayloadType::SelfType | AdtPayloadType::Concrete(_) => None,
    }
}

fn into_owned_type(ty: Rc<Type>) -> Type {
    match Rc::try_unwrap(ty) {
        Ok(ty) => ty,
        Err(ty) => clone_context_type(ty.as_ref()),
    }
}

fn merge_projected_type_holes(current: &mut Rc<Type>, projected: &Rc<Type>) {
    if Rc::ptr_eq(current, projected) {
        record_shared_context_reuse();
        return;
    }
    if projected.as_ref() == &Type::Unknown {
        return;
    }
    if current.as_ref() == &Type::Unknown {
        *current = Rc::clone(projected);
        return;
    }
    if Rc::strong_count(current) > 1 {
        *current = Rc::new(clone_context_type(current.as_ref()));
    }
    let current = Rc::get_mut(current).expect("the projected accumulator is uniquely owned");
    merge_projected_type_slots(current, projected.as_ref());
}

fn merge_projected_type_slots(slot: &mut Type, projected: &Type) {
    let mut pending = vec![(slot, projected)];
    while let Some((slot, projected)) = pending.pop() {
        if projected == &Type::Unknown {
            continue;
        }
        match slot {
            slot @ Type::Unknown => *slot = clone_context_type(projected),
            Type::Named {
                name,
                identity,
                args,
            } => {
                let Type::Named {
                    name: projected_name,
                    identity: projected_identity,
                    args: projected_args,
                } = projected
                else {
                    continue;
                };
                if args.len() == projected_args.len()
                    && crate::type_relations::same_type_identity(
                        name,
                        identity,
                        projected_name,
                        projected_identity,
                    )
                {
                    pending.extend(args.iter_mut().zip(projected_args.iter()));
                }
            }
            Type::VariantRefinement {
                name,
                identity,
                args,
                ..
            } => {
                let (projected_name, projected_identity, projected_args) = match projected {
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
                    } => (name, identity, args),
                    _ => continue,
                };
                if args.len() == projected_args.len()
                    && crate::type_relations::same_type_identity(
                        name,
                        identity,
                        projected_name,
                        projected_identity,
                    )
                {
                    pending.extend(args.iter_mut().zip(projected_args.iter()));
                }
            }
            Type::Record(fields) => {
                let Type::Record(projected_fields) = projected else {
                    continue;
                };
                queue_projected_record_slots(fields, projected_fields, &mut pending);
            }
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            } => {
                let Type::Function {
                    params: projected_params,
                    variadic: projected_variadic,
                    return_type: projected_return,
                    ..
                } = projected
                else {
                    continue;
                };
                if params.len() != projected_params.len()
                    || variadic.is_some() != projected_variadic.is_some()
                {
                    continue;
                }
                record_structural_merge_probes(params.len() + usize::from(variadic.is_some()) + 1);
                pending.extend(params.iter_mut().zip(projected_params));
                if let (Some(slot), Some(projected)) =
                    (variadic.as_deref_mut(), projected_variadic.as_deref())
                {
                    pending.push((slot, projected));
                }
                pending.push((return_type, projected_return));
            }
        }
    }
}

fn queue_projected_record_slots<'slot, 'projected>(
    fields: &'slot mut [(String, Type)],
    projected_fields: &'projected [(String, Type)],
    pending: &mut Vec<(&'slot mut Type, &'projected Type)>,
) {
    let mut projected_by_name = HashMap::with_capacity(projected_fields.len());
    for (name, projected) in projected_fields {
        projected_by_name.entry(name.as_str()).or_insert(projected);
    }
    record_structural_merge_probes(projected_fields.len());
    for (name, slot) in fields {
        record_structural_merge_probes(1);
        if let Some(projected) = projected_by_name.get(name.as_str()) {
            pending.push((slot, *projected));
        }
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
            merge_projected_type_slots(&mut required_args[type_index], required);
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
    #[cfg(test)]
    {
        let (cloned, owned_nodes) = crate::semantic_model::measure_type_clone_nodes(|| ty.clone());
        record_context_clone(owned_nodes);
        cloned
    }
    #[cfg(not(test))]
    {
        ty.clone()
    }
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
    pub(crate) structural_merge_probes: usize,
    pub(crate) shared_context_reuses: usize,
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
            structural_merge_probes: 0,
            shared_context_reuses: 0,
            max_retained_state: 0,
        }
    }

    pub(crate) fn total_relevant_work(self) -> usize {
        self.input_type_nodes_scanned
            + self.expression_or_parameter_edges_scanned
            + self.type_nodes_cloned
            + self.materialized_type_nodes
            + self.structural_merge_probes
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
fn record_context_input_type(ty: &Type) {
    let (units, _) = type_shape(ty);
    update_context_work(|work| work.input_type_nodes_scanned += units);
}

#[cfg(not(test))]
fn record_context_input_type(_ty: &Type) {}

#[cfg(test)]
fn record_context_clone(units: usize) {
    update_context_work(|work| work.type_nodes_cloned += units);
}

#[cfg(test)]
fn record_context_materialization(units: usize) {
    update_context_work(|work| work.materialized_type_nodes += units);
}

#[cfg(not(test))]
fn record_context_materialization(_units: usize) {}

#[cfg(test)]
fn record_structural_merge_probes(units: usize) {
    update_context_work(|work| work.structural_merge_probes += units);
}

#[cfg(not(test))]
fn record_structural_merge_probes(_units: usize) {}

#[cfg(test)]
fn record_shared_context_reuse() {
    update_context_work(|work| work.shared_context_reuses += 1);
}

#[cfg(not(test))]
fn record_shared_context_reuse() {}

#[cfg(test)]
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

#[cfg(not(test))]
fn record_payload_template_materialization(
    _constructor: AdtConstructor<'_>,
    _payload_index: usize,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_does_not_select_ambiguous_constructor_owners() {
        let source = veln_source::SourceFile::new(
            "main.veln",
            "type Left<T>\n  Ready\n  Made(T)\nend\n\
             type Right<T>\n  Ready\n  Made(T)\nend\n",
        );
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        let adts = AdtRegistry::from_module(&module);

        for name in ["Ready", "Made"] {
            let segments = vec![name.to_string()];
            assert!(matches!(
                adts.constructor(&segments, None, &[]),
                ConstructorLookup::Ambiguous
            ));
            assert!(projection_constructor(&adts, &segments, None, &[]).is_none());
        }
    }

    fn measured_record_merge(width: usize) -> ContextProjectionWork {
        let fields = (0..width)
            .map(|index| (format!("field_{index}"), Type::Unknown))
            .collect::<Vec<_>>();
        let projected = (0..width)
            .rev()
            .map(|index| (format!("field_{index}"), Type::named("Int", Vec::new())))
            .collect::<Vec<_>>();
        let mut current = Rc::new(Type::Record(fields));
        let projected = Rc::new(Type::Record(projected));
        reset_context_classification_work();
        merge_projected_type_holes(&mut current, &projected);
        take_context_classification_work()
    }

    fn measured_function_merge(width: usize) -> ContextProjectionWork {
        let mut current = Rc::new(Type::function(
            vec![Type::Unknown; width],
            Type::Unknown,
            Vec::new(),
        ));
        let projected = Rc::new(Type::function(
            vec![Type::named("Int", Vec::new()); width],
            Type::named("Int", Vec::new()),
            Vec::new(),
        ));
        reset_context_classification_work();
        merge_projected_type_holes(&mut current, &projected);
        take_context_classification_work()
    }

    #[test]
    fn projection_structural_merge_work_grows_linearly() {
        for measure in [
            measured_record_merge as fn(usize) -> ContextProjectionWork,
            measured_function_merge,
        ] {
            let measurements = [16, 32, 64].map(measure);
            assert!(measurements[0].structural_merge_probes >= 16);
            for adjacent in measurements.windows(2) {
                assert!(
                    adjacent[1].structural_merge_probes
                        <= adjacent[0].structural_merge_probes * 2 + 2,
                    "structural projection merging must be linear: {measurements:?}",
                );
            }
        }
    }

    #[test]
    fn projection_record_merge_preserves_first_matching_field_semantics() {
        let mut current = Rc::new(Type::Record(vec![
            ("field".to_string(), Type::Unknown),
            ("field".to_string(), Type::Unknown),
        ]));
        let projected = Rc::new(Type::Record(vec![
            ("field".to_string(), Type::named("Int", Vec::new())),
            ("field".to_string(), Type::named("String", Vec::new())),
        ]));

        merge_projected_type_holes(&mut current, &projected);

        let Type::Record(fields) = current.as_ref() else {
            panic!("record projection result");
        };
        assert!(fields.iter().all(|(_, ty)| ty.render() == "Int"));
    }
}
