use veln_ast::{Expr, ExprKind};
use veln_core::{CoreCallTarget, CoreType};

use crate::effects::{
    concurrency_call_effects, concurrency_origin, concurrency_signature,
    core_concurrency_signature, core_standard_library_signature, standard_library_origin,
    standard_library_signature, stdio_signature,
};
use crate::prelude::{
    core_prelude_signature, prelude_signature, qualified_core_prelude_builtin_signature,
    qualified_core_prelude_signature,
};
use crate::semantic_model::{CallOrigin, Type};
use crate::type_lowering::core_type;
use crate::type_syntax::parse_type_annotation;
use crate::types::environment::TypeEnvironment;
use crate::types::signatures::{
    FunctionSignature, SCHEMA_DECODE_STEP_TARGET_PREFIX, SCHEMA_DECODE_TARGET_PREFIX,
    SCHEMA_ENCODE_STEP_TARGET_PREFIX, SCHEMA_ENCODE_TARGET_PREFIX,
    SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX, SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX,
    SCHEMA_VALIDATE_TARGET_PREFIX,
};

pub(crate) struct TypeBinding<'a> {
    pub(crate) name: &'a str,
    pub(crate) ty: &'a Type,
}

pub(crate) struct CoreBinding<'a> {
    pub(crate) name: &'a str,
    pub(crate) ty: &'a CoreType,
}

pub(crate) struct TypeCallSignature {
    pub(crate) params: Vec<Type>,
    pub(crate) variadic: Option<Type>,
    pub(crate) return_type: Type,
    pub(crate) origin: CallOrigin,
}

pub(crate) struct CoreCallSignature {
    pub(crate) target: CoreCallTarget,
    pub(crate) params: Vec<CoreType>,
    pub(crate) variadic: Option<CoreType>,
    pub(crate) return_type: CoreType,
}

struct TypeNamePathCallContext<'a> {
    expected: Option<&'a Type>,
    handle_type: Option<&'a Type>,
    bindings: &'a [TypeBinding<'a>],
    environment: &'a TypeEnvironment,
    current_module: Option<&'a str>,
}

pub(crate) fn type_call_signature(
    callee: &Expr,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    bindings: &[TypeBinding<'_>],
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<TypeCallSignature> {
    match &callee.kind {
        ExprKind::NamePath { segments, .. } => type_name_path_call_signature(
            callee,
            segments,
            TypeNamePathCallContext {
                expected,
                handle_type,
                bindings,
                environment,
                current_module,
            },
        ),
        ExprKind::TypeApply { .. } => type_applied_call_signature(callee, expected, handle_type),
        _ => None,
    }
}

pub(crate) fn core_call_signature(
    callee: &Expr,
    expected: Option<&CoreType>,
    bindings: &[CoreBinding<'_>],
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<CoreCallSignature> {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return None;
    };
    core_name_path_call_signature(
        callee,
        segments,
        expected,
        bindings,
        environment,
        current_module,
    )
}

fn type_name_path_call_signature(
    callee: &Expr,
    segments: &[String],
    context: TypeNamePathCallContext<'_>,
) -> Option<TypeCallSignature> {
    if let Some(signature) =
        type_effect_call_signature(callee, segments, context.expected, context.handle_type)
    {
        return Some(signature);
    }
    match type_binding_call_signature(callee, segments, context.bindings) {
        BindingCallSignature::Resolved(signature) => Some(signature),
        BindingCallSignature::ShadowedNonCallable => None,
        BindingCallSignature::Missing => function_type_call_signature(
            segments,
            context.expected,
            context.environment,
            context.current_module,
        ),
    }
}

fn type_effect_call_signature(
    callee: &Expr,
    segments: &[String],
    expected: Option<&Type>,
    handle_type: Option<&Type>,
) -> Option<TypeCallSignature> {
    if let Some(origin) = stdio_signature(segments, callee) {
        return Some(TypeCallSignature {
            params: vec![Type::string()],
            variadic: None,
            return_type: Type::unit(),
            origin,
        });
    }
    if let Some(origin) = concurrency_origin(segments, callee) {
        let (params, return_type) =
            concurrency_signature(segments, expected, handle_type, None, None)?;
        let effects = concurrency_call_effects(segments, handle_type)?;
        return Some(TypeCallSignature {
            params,
            variadic: None,
            return_type,
            origin: CallOrigin { effects, ..origin },
        });
    }
    if let Some(origin) = standard_library_origin(segments, callee) {
        let (params, return_type) = standard_library_signature(segments)?;
        return Some(TypeCallSignature {
            params,
            variadic: None,
            return_type,
            origin,
        });
    }
    None
}

fn type_applied_call_signature(
    callee: &Expr,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
) -> Option<TypeCallSignature> {
    let (segments, type_args) = type_applied_name_path(callee)?;
    let origin = concurrency_origin(segments, callee)?;
    if type_args.len() > concurrency_type_arg_limit(segments)? {
        return None;
    }
    let explicit_item = type_args
        .first()
        .and_then(|type_arg| parse_type_annotation(type_arg).ok());
    let explicit_context = type_args
        .get(1)
        .filter(|_| is_task_spawn_with(segments))
        .and_then(|type_arg| parse_type_annotation(type_arg).ok());
    let (params, return_type) = concurrency_signature(
        segments,
        expected,
        handle_type,
        explicit_item.as_ref(),
        explicit_context.as_ref(),
    )?;
    let effects = concurrency_call_effects(segments, handle_type)?;
    Some(TypeCallSignature {
        params,
        variadic: None,
        return_type,
        origin: CallOrigin { effects, ..origin },
    })
}

fn concurrency_type_arg_limit(segments: &[String]) -> Option<usize> {
    match segments {
        [module, name] if module == "task" && name == "spawn_with" => Some(2),
        [module, _] if module == "channel" || module == "task" => Some(1),
        _ => None,
    }
}

fn is_task_spawn_with(segments: &[String]) -> bool {
    matches!(segments, [module, name] if module == "task" && name == "spawn_with")
}

fn type_binding_call_signature(
    callee: &Expr,
    segments: &[String],
    bindings: &[TypeBinding<'_>],
) -> BindingCallSignature<TypeCallSignature> {
    let [name] = segments else {
        return BindingCallSignature::Missing;
    };
    let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name) else {
        return BindingCallSignature::Missing;
    };
    let Some((params, variadic, return_type)) = binding.ty.callable_parts() else {
        return BindingCallSignature::ShadowedNonCallable;
    };
    let effects = binding.ty.function_effects().unwrap_or_default().to_vec();
    BindingCallSignature::Resolved(TypeCallSignature {
        params: params.to_vec(),
        variadic: variadic.cloned(),
        return_type: return_type.clone(),
        origin: CallOrigin {
            node_id: callee.node_id,
            span: callee.span.clone(),
            symbol: name.clone(),
            effects,
        },
    })
}

fn core_name_path_call_signature(
    callee: &Expr,
    segments: &[String],
    expected: Option<&CoreType>,
    bindings: &[CoreBinding<'_>],
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<CoreCallSignature> {
    if stdio_signature(segments, callee).is_some() {
        return Some(CoreCallSignature {
            target: CoreCallTarget::StdioBuiltin(segments.join("::")),
            params: vec![CoreType::string()],
            variadic: None,
            return_type: CoreType::unit(),
        });
    }
    if concurrency_origin(segments, callee).is_some() {
        let (params, return_type) =
            core_concurrency_signature(segments, expected, None, None, None)?;
        return Some(CoreCallSignature {
            target: CoreCallTarget::ConcurrencyBuiltin(segments.join("::")),
            params,
            variadic: None,
            return_type,
        });
    }
    if standard_library_origin(segments, callee).is_some() {
        let (params, return_type) = core_standard_library_signature(segments)?;
        return Some(CoreCallSignature {
            target: CoreCallTarget::StandardLibraryBuiltin(segments.join("::")),
            params,
            variadic: None,
            return_type,
        });
    }
    if let Some(signature) = qualified_core_prelude_builtin_call_signature(segments, expected) {
        return Some(signature);
    }
    match core_binding_call_signature(segments, bindings) {
        BindingCallSignature::Resolved(signature) => return Some(signature),
        BindingCallSignature::ShadowedNonCallable => return None,
        BindingCallSignature::Missing => {}
    }
    core_function_call_signature(segments, expected, environment, current_module)
}

fn qualified_core_prelude_builtin_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<CoreCallSignature> {
    let signature = qualified_core_prelude_builtin_signature(segments, expected)?;
    Some(core_call_signature_from_parts(signature))
}

fn core_binding_call_signature(
    segments: &[String],
    bindings: &[CoreBinding<'_>],
) -> BindingCallSignature<CoreCallSignature> {
    let [name] = segments else {
        return BindingCallSignature::Missing;
    };
    let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name) else {
        return BindingCallSignature::Missing;
    };
    let CoreType::Function {
        params,
        variadic,
        return_type,
        ..
    } = binding.ty
    else {
        return BindingCallSignature::ShadowedNonCallable;
    };
    BindingCallSignature::Resolved(CoreCallSignature {
        target: CoreCallTarget::Value(name.clone()),
        params: params.clone(),
        variadic: variadic.as_deref().cloned(),
        return_type: return_type.as_ref().clone(),
    })
}

fn core_function_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<CoreCallSignature> {
    if let Some(function) = resolve_function(segments, environment, current_module) {
        let (params, return_type) = if function.module_name.as_deref() == Some("std::prelude") {
            core_prelude_signature(&function.name, expected)
                .map(|(_, params, return_type)| (params, return_type))
                .unwrap_or_else(|| {
                    (
                        function.params.iter().map(core_type).collect(),
                        core_type(&function.return_type),
                    )
                })
        } else {
            (
                function.params.iter().map(core_type).collect(),
                core_type(&function.return_type),
            )
        };
        return Some(CoreCallSignature {
            target: core_target_from_signature_name(&function.target_name),
            params,
            variadic: function.variadic.as_ref().map(core_type),
            return_type,
        });
    }
    if let Some(signature) = qualified_core_prelude_signature(segments, expected) {
        return Some(core_call_signature_from_parts(signature));
    }
    if let [name] = segments
        && let Some((target, params, return_type)) = core_prelude_signature(name, expected)
    {
        return Some(CoreCallSignature {
            target,
            params,
            variadic: None,
            return_type,
        });
    }
    None
}

fn core_call_signature_from_parts(
    (target, params, return_type): (CoreCallTarget, Vec<CoreType>, CoreType),
) -> CoreCallSignature {
    CoreCallSignature {
        target,
        params,
        variadic: None,
        return_type,
    }
}

enum BindingCallSignature<T> {
    Resolved(T),
    ShadowedNonCallable,
    Missing,
}

fn function_type_call_signature(
    segments: &[String],
    expected: Option<&Type>,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<TypeCallSignature> {
    let (function, symbol) = match segments {
        [name] => (
            environment
                .unqualified_function(name, current_module)
                .found(),
            name.clone(),
        ),
        _ => (
            environment.function_path(segments, current_module),
            segments.join("::"),
        ),
    };
    let function = function?;
    let (params, return_type) = if function.module_name.as_deref() == Some("std::prelude") {
        prelude_signature(&function.name, expected)
            .unwrap_or_else(|| (function.params.clone(), function.return_type.clone()))
    } else {
        (function.params.clone(), function.return_type.clone())
    };
    Some(TypeCallSignature {
        params,
        variadic: function.variadic.clone(),
        return_type,
        origin: CallOrigin {
            node_id: function.node_id,
            span: function.span.clone(),
            symbol,
            effects: function.effects.clone(),
        },
    })
}

fn resolve_function<'a>(
    segments: &[String],
    environment: &'a TypeEnvironment,
    current_module: Option<&str>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => environment
            .unqualified_function(name, current_module)
            .found(),
        _ => environment.function_path(segments, current_module),
    }
}

fn core_target_from_signature_name(target_name: &str) -> CoreCallTarget {
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_DECODE_TARGET_PREFIX) {
        return CoreCallTarget::SchemaDecode(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_DECODE_STEP_TARGET_PREFIX) {
        return CoreCallTarget::SchemaDecodeStep(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX) {
        return CoreCallTarget::SchemaNeutralDecode(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX) {
        return CoreCallTarget::SchemaNeutralEncode(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_ENCODE_TARGET_PREFIX) {
        return CoreCallTarget::SchemaEncode(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_ENCODE_STEP_TARGET_PREFIX) {
        return CoreCallTarget::SchemaEncodeStep(schema_name.to_string());
    }
    if let Some(schema_name) = target_name.strip_prefix(SCHEMA_VALIDATE_TARGET_PREFIX) {
        return CoreCallTarget::SchemaValidate(schema_name.to_string());
    }
    CoreCallTarget::Function(target_name.to_string())
}

fn type_applied_name_path(callee: &Expr) -> Option<(&[String], &[String])> {
    let ExprKind::TypeApply { callee, type_args } = &callee.kind else {
        return None;
    };
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return None;
    };
    Some((segments, type_args))
}
