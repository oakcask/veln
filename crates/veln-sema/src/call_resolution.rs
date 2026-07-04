use veln_ast::{Expr, ExprKind};
use veln_core::{CoreCallTarget, CoreType};

use crate::effects::{
    concurrency_origin, concurrency_signature, core_concurrency_signature,
    core_standard_library_signature, standard_library_origin, standard_library_signature,
    stdio_signature,
};
use crate::prelude::{
    core_prelude_signature, qualified_core_prelude_builtin_signature,
    qualified_core_prelude_signature,
};
use crate::types::{
    CallOrigin, CodecCallBoundary, CodecCallSignature, FunctionSignature,
    SCHEMA_DECODE_STEP_TARGET_PREFIX, SCHEMA_DECODE_TARGET_PREFIX,
    SCHEMA_ENCODE_STEP_TARGET_PREFIX, SCHEMA_ENCODE_TARGET_PREFIX,
    SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX, SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX,
    SCHEMA_VALIDATE_TARGET_PREFIX, Type, TypeEnvironment, core_type, is_assignable,
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
    arg_count: Option<usize>,
    bindings: &'a [TypeBinding<'a>],
    environment: &'a TypeEnvironment,
    current_module: Option<&'a str>,
}

pub(crate) fn type_call_signature(
    callee: &Expr,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    arg_count: Option<usize>,
    bindings: &[TypeBinding<'_>],
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<TypeCallSignature> {
    match &callee.kind {
        ExprKind::NamePath(segments) => type_name_path_call_signature(
            callee,
            segments,
            TypeNamePathCallContext {
                expected,
                handle_type,
                arg_count,
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
    arg_count: Option<usize>,
    bindings: &[CoreBinding<'_>],
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<CoreCallSignature> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    core_name_path_call_signature(
        callee,
        segments,
        expected,
        arg_count,
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
        BindingCallSignature::Missing => {
            function_type_call_signature(segments, context.environment, context.current_module)
                .or_else(|| {
                    codec_type_call_signature(
                        segments,
                        context.expected,
                        context.arg_count,
                        context.environment,
                        context.current_module,
                    )
                })
        }
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
        return Some(TypeCallSignature {
            params,
            variadic: None,
            return_type,
            origin,
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
        .and_then(|type_arg| crate::types::parse_type_annotation(type_arg).ok());
    let explicit_context = type_args
        .get(1)
        .filter(|_| is_task_spawn_with(segments))
        .and_then(|type_arg| crate::types::parse_type_annotation(type_arg).ok());
    let (params, return_type) = concurrency_signature(
        segments,
        expected,
        handle_type,
        explicit_item.as_ref(),
        explicit_context.as_ref(),
    )?;
    Some(TypeCallSignature {
        params,
        variadic: None,
        return_type,
        origin,
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
    arg_count: Option<usize>,
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
    if let Some(signature) = qualified_core_prelude_call_signature(segments, expected) {
        return Some(signature);
    }
    match core_binding_call_signature(segments, bindings) {
        BindingCallSignature::Resolved(signature) => return Some(signature),
        BindingCallSignature::ShadowedNonCallable => return None,
        BindingCallSignature::Missing => {}
    }
    core_function_call_signature(segments, expected, environment, current_module).or_else(|| {
        core_codec_call_signature(segments, expected, arg_count, environment, current_module)
    })
}

fn qualified_core_prelude_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<CoreCallSignature> {
    let signature = qualified_core_prelude_builtin_signature(segments, expected)
        .or_else(|| qualified_core_prelude_signature(segments, expected))?;
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
        return Some(CoreCallSignature {
            target: core_target_from_signature_name(&function.target_name),
            params: function.params.iter().map(core_type).collect(),
            variadic: function.variadic.as_ref().map(core_type),
            return_type: core_type(&function.return_type),
        });
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
    Some(TypeCallSignature {
        params: function.params.clone(),
        variadic: function.variadic.clone(),
        return_type: function.return_type.clone(),
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

fn codec_type_call_signature(
    segments: &[String],
    expected: Option<&Type>,
    arg_count: Option<usize>,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<TypeCallSignature> {
    let (mut codecs, symbol) = match segments {
        [name] => (
            environment.unqualified_codec_calls(name, current_module),
            name.clone(),
        ),
        _ => (
            environment.codec_call_path(segments, current_module),
            segments.join("::"),
        ),
    };
    narrow_codec_candidates_by_arity(&mut codecs, arg_count);
    let codec = select_codec_type_call(codecs, expected)?;
    Some(TypeCallSignature {
        params: codec.params.clone(),
        variadic: None,
        return_type: codec.return_type.clone(),
        origin: CallOrigin {
            node_id: codec.node_id,
            span: codec.span.clone(),
            symbol,
            effects: codec.effects.clone(),
        },
    })
}

fn core_codec_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    arg_count: Option<usize>,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Option<CoreCallSignature> {
    let mut codecs = match segments {
        [name] => environment.unqualified_codec_calls(name, current_module),
        _ => environment.codec_call_path(segments, current_module),
    };
    narrow_codec_candidates_by_arity(&mut codecs, arg_count);
    let codec = select_codec_core_call(codecs, expected)?;
    Some(CoreCallSignature {
        target: core_codec_call_target(codec),
        params: codec.params.iter().map(core_type).collect(),
        variadic: None,
        return_type: core_type(&codec.return_type),
    })
}

fn core_codec_call_target(codec: &CodecCallSignature) -> CoreCallTarget {
    match codec.boundary {
        CodecCallBoundary::Direct => core_target_from_signature_name(&codec.target_name),
        CodecCallBoundary::HandWrittenDecode => CoreCallTarget::CodecDecode {
            function: codec.target_name.clone(),
            codec: codec.name.clone(),
        },
    }
}

fn narrow_codec_candidates_by_arity(
    codecs: &mut Vec<&crate::types::CodecCallSignature>,
    arg_count: Option<usize>,
) {
    let Some(arg_count) = arg_count else {
        return;
    };
    if codecs.iter().any(|codec| codec.params.len() == arg_count) {
        codecs.retain(|codec| codec.params.len() == arg_count);
    }
}

fn select_codec_type_call<'a>(
    codecs: Vec<&'a crate::types::CodecCallSignature>,
    expected: Option<&Type>,
) -> Option<&'a crate::types::CodecCallSignature> {
    if codecs.len() == 1 {
        return codecs.into_iter().next();
    }
    let expected = expected.filter(|expected| expected != &&Type::Unknown)?;
    codecs
        .into_iter()
        .find(|codec| is_assignable(expected, &codec.return_type))
}

fn select_codec_core_call<'a>(
    codecs: Vec<&'a crate::types::CodecCallSignature>,
    expected: Option<&CoreType>,
) -> Option<&'a crate::types::CodecCallSignature> {
    if codecs.len() == 1 {
        return codecs.into_iter().next();
    }
    let expected = expected.filter(|expected| expected != &&CoreType::Unknown)?;
    codecs
        .into_iter()
        .find(|codec| core_type_is_assignable(expected, &core_type(&codec.return_type)))
}

fn core_type_is_assignable(expected: &CoreType, actual: &CoreType) -> bool {
    if expected == &CoreType::Unknown || actual == &CoreType::Unknown || expected == actual {
        return true;
    }
    match (expected, actual) {
        (CoreType::Record(expected_fields), CoreType::Record(actual_fields)) => {
            expected_fields.iter().all(|(expected_name, expected_ty)| {
                actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == expected_name)
                    .is_some_and(|(_, actual_ty)| core_type_is_assignable(expected_ty, actual_ty))
            })
        }
        (
            CoreType::Named {
                name: expected_name,
                args: expected_args,
            },
            CoreType::Named {
                name: actual_name,
                args: actual_args,
            },
        ) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| core_type_is_assignable(expected, actual))
        }
        (
            CoreType::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            CoreType::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| core_type_is_assignable(expected, actual))
                && match (expected_variadic, actual_variadic) {
                    (Some(expected), Some(actual)) => core_type_is_assignable(expected, actual),
                    (None, None) => true,
                    _ => false,
                }
                && core_type_is_assignable(expected_return, actual_return)
                && actual_effects
                    .iter()
                    .all(|effect| expected_effects.iter().any(|expected| expected == effect))
        }
        _ => false,
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
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    Some((segments, type_args))
}
