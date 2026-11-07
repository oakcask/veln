use std::sync::Arc;

use super::{Type, TypeArgs};

#[cfg(test)]
thread_local! {
    static TYPE_CLONE_MEASUREMENT_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MEASURED_TYPE_CLONE_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

enum TypeCloneTask<'a> {
    Visit(&'a Type),
    Named(String, String, TypeArgs),
    Variant(
        String,
        String,
        TypeArgs,
        Arc<[String]>,
        Vec<(String, Vec<Type>, String)>,
    ),
    Record(Vec<String>),
    Function(usize, bool, Vec<String>),
}

impl Clone for Type {
    fn clone(&self) -> Self {
        let mut tasks = vec![TypeCloneTask::Visit(self)];
        let mut cloned = Vec::new();
        while let Some(task) = tasks.pop() {
            process_type_clone_task(task, &mut tasks, &mut cloned);
        }
        cloned.pop().expect("type cloning produces one root")
    }
}

fn process_type_clone_task<'a>(
    task: TypeCloneTask<'a>,
    tasks: &mut Vec<TypeCloneTask<'a>>,
    cloned: &mut Vec<Type>,
) {
    match task {
        TypeCloneTask::Visit(ty) => queue_type_clone(ty, tasks, cloned),
        TypeCloneTask::Named(name, identity, args) => cloned.push(Type::Named {
            name,
            identity,
            args,
        }),
        TypeCloneTask::Variant(name, identity, args, variants, unresolved_alternatives) => cloned
            .push(Type::VariantRefinement {
                name,
                identity,
                args,
                variants,
                unresolved_alternatives,
            }),
        TypeCloneTask::Record(names) => finish_record_type_clone(names, cloned),
        TypeCloneTask::Function(param_count, has_variadic, effects) => {
            finish_function_type_clone(param_count, has_variadic, effects, cloned)
        }
    }
}

fn queue_type_clone<'a>(ty: &'a Type, tasks: &mut Vec<TypeCloneTask<'a>>, cloned: &mut Vec<Type>) {
    #[cfg(test)]
    TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| {
        if depth.get() > 0 {
            MEASURED_TYPE_CLONE_NODES.set(MEASURED_TYPE_CLONE_NODES.get() + 1);
        }
    });
    match ty {
        Type::Unknown => cloned.push(Type::Unknown),
        Type::Named {
            name,
            identity,
            args,
        } => queue_named_type_clone(name, identity, args, tasks),
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        } => queue_variant_type_clone(
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
            tasks,
        ),
        Type::Record(fields) => queue_record_type_clone(fields, tasks),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => queue_function_type_clone(params, variadic, return_type, effects, tasks),
    }
}

fn queue_named_type_clone<'a>(
    name: &str,
    identity: &str,
    args: &TypeArgs,
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Named(
        name.to_owned(),
        identity.to_owned(),
        args.clone(),
    ));
}

fn queue_variant_type_clone<'a>(
    name: &str,
    identity: &str,
    args: &TypeArgs,
    variants: &Arc<[String]>,
    unresolved_alternatives: &[(String, Vec<Type>, String)],
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Variant(
        name.to_owned(),
        identity.to_owned(),
        args.clone(),
        variants.clone(),
        unresolved_alternatives.to_vec(),
    ));
}

fn queue_record_type_clone<'a>(fields: &'a [(String, Type)], tasks: &mut Vec<TypeCloneTask<'a>>) {
    tasks.push(TypeCloneTask::Record(
        fields.iter().map(|(name, _)| name.clone()).collect(),
    ));
    tasks.extend(fields.iter().rev().map(|(_, ty)| TypeCloneTask::Visit(ty)));
}

fn queue_function_type_clone<'a>(
    params: &'a [Type],
    variadic: &'a Option<Box<Type>>,
    return_type: &'a Type,
    effects: &[String],
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Function(
        params.len(),
        variadic.is_some(),
        effects.to_vec(),
    ));
    tasks.push(TypeCloneTask::Visit(return_type));
    if let Some(variadic) = variadic {
        tasks.push(TypeCloneTask::Visit(variadic));
    }
    tasks.extend(params.iter().rev().map(TypeCloneTask::Visit));
}

fn finish_record_type_clone(names: Vec<String>, cloned: &mut Vec<Type>) {
    let fields = cloned
        .split_off(cloned.len() - names.len())
        .into_iter()
        .zip(names)
        .map(|(ty, name)| (name, ty))
        .collect();
    cloned.push(Type::Record(fields));
}

fn finish_function_type_clone(
    param_count: usize,
    has_variadic: bool,
    effects: Vec<String>,
    cloned: &mut Vec<Type>,
) {
    let mut parts = cloned.split_off(cloned.len() - param_count - usize::from(has_variadic) - 1);
    let params = parts.drain(..param_count).collect();
    let variadic = has_variadic.then(|| Box::new(parts.remove(0)));
    let return_type = Box::new(parts.remove(0));
    cloned.push(Type::Function {
        params,
        variadic,
        return_type,
        effects,
    });
}

#[cfg(test)]
pub(crate) fn measure_type_clone_nodes<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    struct MeasurementGuard;

    impl Drop for MeasurementGuard {
        fn drop(&mut self) {
            TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }

    let is_outermost = TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        current == 0
    });
    if is_outermost {
        MEASURED_TYPE_CLONE_NODES.set(0);
    }
    let guard = MeasurementGuard;
    let result = operation();
    drop(guard);
    let measured = if is_outermost {
        MEASURED_TYPE_CLONE_NODES.get()
    } else {
        0
    };
    (result, measured)
}

pub(crate) fn drop_type_iteratively(root: Type) {
    let mut pending = vec![root];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Unknown => {}
            Type::Named { args, .. } => pending.extend(args),
            Type::VariantRefinement {
                args,
                mut unresolved_alternatives,
                ..
            } => {
                pending.extend(args);
                for (_, mut alternatives, _) in unresolved_alternatives.drain(..) {
                    pending.append(&mut alternatives);
                }
            }
            Type::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, ty)| ty));
            }
            Type::Function {
                mut params,
                variadic,
                return_type,
                ..
            } => {
                pending.append(&mut params);
                pending.extend(variadic.map(|ty| *ty));
                pending.push(*return_type);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Type, measure_type_clone_nodes};

    #[test]
    fn type_clone_measurement_counts_owned_nodes_and_skips_shared_type_arguments() {
        let ty = Type::Function {
            params: vec![Type::Record(vec![(
                "field".to_string(),
                Type::named("Vec", vec![Type::Unknown]),
            )])],
            variadic: Some(Box::new(Type::VariantRefinement {
                name: "State".to_string(),
                identity: "State".to_string(),
                args: vec![Type::named("Int", Vec::new())].into(),
                variants: vec!["Ready".to_string()].into(),
                unresolved_alternatives: vec![(
                    "Other".to_string(),
                    vec![Type::Unknown],
                    "Done".to_string(),
                )],
            })),
            return_type: Box::new(Type::result(
                Type::Unknown,
                Type::named("String", Vec::new()),
            )),
            effects: Vec::new(),
        };

        let (cloned, measured_nodes) = measure_type_clone_nodes(|| ty.clone());

        assert_eq!(cloned, ty);
        assert_eq!(measured_nodes, 6);

        let (_, container_nodes) = measure_type_clone_nodes(|| vec![ty.clone()]);
        assert_eq!(container_nodes, 6);

        let (_, temporary_nodes) = measure_type_clone_nodes(|| {
            let _temporary = ty.clone();
        });
        assert_eq!(temporary_nodes, 6);
    }
}
