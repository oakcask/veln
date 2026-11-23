use std::borrow::Cow;
use std::sync::Arc;

use super::{Type, TypeArgs};

#[cfg(test)]
thread_local! {
    static TYPE_CLONE_MEASUREMENT_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MEASURED_TYPE_CLONE_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

enum TypeCloneTask<'a> {
    Visit(&'a Type),
    Named(Cow<'static, str>, Cow<'static, str>, TypeArgs),
    Variant(
        Cow<'static, str>,
        Cow<'static, str>,
        TypeArgs,
        Arc<[String]>,
        Vec<(String, Vec<Type>, String)>,
    ),
    Record(Vec<String>),
    Function(TypeArgs, bool, Vec<String>),
}

impl Clone for Type {
    fn clone(&self) -> Self {
        let direct = match self {
            Self::Unknown => Some(Self::Unknown),
            Self::Named {
                name,
                identity,
                args,
            } => Some(Self::Named {
                name: name.clone(),
                identity: identity.clone(),
                args: args.clone(),
            }),
            _ => None,
        };
        if let Some(cloned) = direct {
            record_type_clone_node();
            return cloned;
        }
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
        TypeCloneTask::Function(params, has_variadic, effects) => {
            finish_function_type_clone(params, has_variadic, effects, cloned)
        }
    }
}

fn queue_type_clone<'a>(ty: &'a Type, tasks: &mut Vec<TypeCloneTask<'a>>, cloned: &mut Vec<Type>) {
    record_type_clone_node();
    match ty {
        Type::Unknown => cloned.push(Type::Unknown),
        Type::Named {
            name,
            identity,
            args,
        } => queue_named_type_clone(name.clone(), identity.clone(), args, tasks),
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        } => queue_variant_type_clone(
            name.clone(),
            identity.clone(),
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

fn record_type_clone_node() {
    #[cfg(test)]
    TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| {
        if depth.get() > 0 {
            MEASURED_TYPE_CLONE_NODES.set(MEASURED_TYPE_CLONE_NODES.get() + 1);
        }
    });
}

fn queue_named_type_clone<'a>(
    name: Cow<'static, str>,
    identity: Cow<'static, str>,
    args: &TypeArgs,
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Named(name, identity, args.clone()));
}

fn queue_variant_type_clone<'a>(
    name: Cow<'static, str>,
    identity: Cow<'static, str>,
    args: &TypeArgs,
    variants: &Arc<[String]>,
    unresolved_alternatives: &[(String, Vec<Type>, String)],
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Variant(
        name,
        identity,
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
    params: &TypeArgs,
    variadic: &'a Option<Box<Type>>,
    return_type: &'a Type,
    effects: &[String],
    tasks: &mut Vec<TypeCloneTask<'a>>,
) {
    tasks.push(TypeCloneTask::Function(
        params.clone(),
        variadic.is_some(),
        effects.to_vec(),
    ));
    tasks.push(TypeCloneTask::Visit(return_type));
    if let Some(variadic) = variadic {
        tasks.push(TypeCloneTask::Visit(variadic));
    }
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
    params: TypeArgs,
    has_variadic: bool,
    effects: Vec<String>,
    cloned: &mut Vec<Type>,
) {
    let mut parts = cloned.split_off(cloned.len() - usize::from(has_variadic) - 1);
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
                params,
                variadic,
                return_type,
                ..
            } => {
                pending.extend(params);
                pending.extend(variadic.map(|ty| *ty));
                pending.push(*return_type);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Type, TypeArgs, measure_type_clone_nodes};

    #[test]
    fn extending_empty_type_arguments_keeps_other_types_empty() {
        let mut first = TypeArgs::from(Vec::new());
        let second = std::iter::empty::<Type>().collect::<TypeArgs>();
        first.push(Type::int());

        assert_eq!(first.len(), 1);
        assert_eq!(first[0], Type::int());
        assert!(second.into_vec().is_empty());
        assert!(TypeArgs::from(Vec::new()).is_empty());
    }

    #[test]
    fn function_type_clones_detach_parameters_before_mutation() {
        let original = Type::function(vec![Type::option(Type::int())], Type::unit(), Vec::new());
        let mut cloned = original.clone();
        let Type::Function {
            params: original_params,
            ..
        } = &original
        else {
            unreachable!();
        };
        let Type::Function {
            params: cloned_params,
            ..
        } = &mut cloned
        else {
            unreachable!();
        };
        assert!(original_params.ptr_eq(cloned_params));
        let Type::Named { args, .. } = &mut cloned_params[0] else {
            unreachable!();
        };
        args[0] = Type::string();
        assert!(!original_params.ptr_eq(cloned_params));
        assert_eq!(original_params[0], Type::option(Type::int()));
        assert_eq!(cloned_params[0], Type::option(Type::string()));
    }

    #[test]
    fn type_clone_measurement_counts_owned_nodes_and_skips_shared_type_arguments() {
        let ty = Type::Function {
            params: vec![Type::Record(vec![(
                "field".to_string(),
                Type::named("Vec", vec![Type::Unknown]),
            )])]
            .into(),
            variadic: Some(Box::new(Type::VariantRefinement {
                name: "State".into(),
                identity: "State".into(),
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
        assert_eq!(measured_nodes, 4);

        let (_, container_nodes) = measure_type_clone_nodes(|| vec![ty.clone()]);
        assert_eq!(container_nodes, 4);

        let (_, temporary_nodes) = measure_type_clone_nodes(|| {
            let _temporary = ty.clone();
        });
        assert_eq!(temporary_nodes, 4);
    }
}
