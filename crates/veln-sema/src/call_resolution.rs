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
use crate::semantic_model::{CallOrigin, Type, TypeArgs};
use crate::type_lowering::core_type;
use crate::type_syntax::parse_type_annotation;
use crate::types::environment::TypeEnvironment;
use crate::types::signatures::{
    FunctionSignature, SCHEMA_DECODE_STEP_TARGET_PREFIX, SCHEMA_DECODE_TARGET_PREFIX,
    SCHEMA_ENCODE_STEP_TARGET_PREFIX, SCHEMA_ENCODE_TARGET_PREFIX,
    SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX, SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX,
    SCHEMA_VALIDATE_TARGET_PREFIX,
};

mod io_provider_state;

use io_provider_state::{io_provider_state_core_signature, io_provider_state_surface_signature};

pub(crate) struct TypeBinding<'a> {
    pub(crate) name: &'a str,
    pub(crate) ty: &'a Type,
    pub(crate) schema_encode_callable: bool,
}

pub(crate) struct CoreBinding<'a> {
    pub(crate) name: &'a str,
    pub(crate) ty: &'a CoreType,
}

pub(crate) struct TypeCallSignature {
    pub(crate) params: TypeArgs,
    pub(crate) variadic: Option<Type>,
    pub(crate) return_type: Type,
    pub(crate) origin: CallOrigin,
}

pub(crate) struct CoreCallSignature {
    pub(crate) target: CoreCallTarget,
    pub(crate) params: Vec<CoreType>,
    pub(crate) variadic: Option<CoreType>,
    pub(crate) return_type: CoreType,
    pub(crate) callsite: bool,
}

pub(crate) struct CallArgumentEvidence<'a, T> {
    pub(crate) first: Option<&'a T>,
    pub(crate) second: Option<&'a T>,
    pub(crate) third: Option<&'a T>,
}

impl<T> Clone for CallArgumentEvidence<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CallArgumentEvidence<'_, T> {}

impl<T> Default for CallArgumentEvidence<'_, T> {
    fn default() -> Self {
        Self {
            first: None,
            second: None,
            third: None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CallResolutionContext<'a> {
    pub(crate) environment: &'a TypeEnvironment,
    pub(crate) current_module: Option<&'a str>,
    pub(crate) current_package: Option<&'a str>,
}

struct TypeNamePathCallContext<'a> {
    expected: Option<&'a Type>,
    evidence: CallArgumentEvidence<'a, Type>,
    bindings: &'a [TypeBinding<'a>],
    resolution: CallResolutionContext<'a>,
}

pub(crate) fn type_call_signature(
    callee: &Expr,
    expected: Option<&Type>,
    evidence: CallArgumentEvidence<'_, Type>,
    bindings: &[TypeBinding<'_>],
    resolution: CallResolutionContext<'_>,
) -> Option<TypeCallSignature> {
    match &callee.kind {
        ExprKind::NamePath { segments, .. } => type_name_path_call_signature(
            callee,
            segments,
            TypeNamePathCallContext {
                expected,
                evidence,
                bindings,
                resolution,
            },
        ),
        ExprKind::TypeApply { .. } => type_applied_call_signature(callee, expected, evidence.first),
        _ => None,
    }
}

pub(crate) fn core_call_signature(
    callee: &Expr,
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
    bindings: &[CoreBinding<'_>],
    resolution: CallResolutionContext<'_>,
) -> Option<CoreCallSignature> {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return None;
    };
    core_name_path_call_signature(callee, segments, expected, evidence, bindings, resolution)
}

fn type_name_path_call_signature(
    callee: &Expr,
    segments: &[String],
    context: TypeNamePathCallContext<'_>,
) -> Option<TypeCallSignature> {
    if segments.len() > 1
        && let Some(signature) = function_type_call_signature(
            segments,
            context.expected,
            context.evidence,
            context.resolution,
        )
    {
        return Some(signature);
    }
    if let Some(signature) =
        type_effect_call_signature(callee, segments, context.expected, context.evidence.first)
    {
        return Some(signature);
    }
    match type_binding_call_signature(callee, segments, context.bindings) {
        BindingCallSignature::Resolved(signature) => Some(signature),
        BindingCallSignature::ShadowedNonCallable => None,
        BindingCallSignature::Missing => function_type_call_signature(
            segments,
            context.expected,
            context.evidence,
            context.resolution,
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
            params: vec![Type::string()].into(),
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
            params: params.into(),
            variadic: None,
            return_type,
            origin: CallOrigin { effects, ..origin },
        });
    }
    if let Some(origin) = standard_library_origin(segments, callee) {
        let (params, return_type) = standard_library_signature(segments)?;
        return Some(TypeCallSignature {
            params: params.into(),
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
        params: params.into(),
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
        params: params.clone(),
        variadic: variadic.cloned(),
        return_type: return_type.clone(),
        origin: CallOrigin {
            node_id: callee.node_id,
            span: callee.span.clone(),
            declaration_span: None,
            symbol: name.clone(),
            effects,
            schema_encode: binding.schema_encode_callable,
        },
    })
}

fn core_name_path_call_signature(
    callee: &Expr,
    segments: &[String],
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
    bindings: &[CoreBinding<'_>],
    resolution: CallResolutionContext<'_>,
) -> Option<CoreCallSignature> {
    if segments.len() > 1
        && let Some(signature) =
            core_function_call_signature(segments, expected, evidence, resolution)
    {
        return Some(signature);
    }
    if stdio_signature(segments, callee).is_some() {
        return Some(CoreCallSignature {
            target: CoreCallTarget::StdioBuiltin(segments.join("::")),
            params: vec![CoreType::string()],
            variadic: None,
            return_type: CoreType::unit(),
            callsite: false,
        });
    }
    if concurrency_origin(segments, callee).is_some() {
        let (params, return_type) =
            core_concurrency_signature(segments, expected, evidence.first, None, None)?;
        return Some(CoreCallSignature {
            target: CoreCallTarget::ConcurrencyBuiltin(segments.join("::")),
            params,
            variadic: None,
            return_type,
            callsite: false,
        });
    }
    if standard_library_origin(segments, callee).is_some() {
        let (params, return_type) = core_standard_library_signature(segments)?;
        return Some(CoreCallSignature {
            target: CoreCallTarget::StandardLibraryBuiltin(segments.join("::")),
            params,
            variadic: None,
            return_type,
            callsite: false,
        });
    }
    if let Some(signature) = qualified_core_prelude_builtin_call_signature(
        segments,
        expected,
        resolution.current_package,
    ) {
        return Some(signature);
    }
    match core_binding_call_signature(segments, bindings) {
        BindingCallSignature::Resolved(signature) => return Some(signature),
        BindingCallSignature::ShadowedNonCallable => return None,
        BindingCallSignature::Missing => {}
    }
    core_function_call_signature(segments, expected, evidence, resolution)
}

fn qualified_core_prelude_builtin_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    current_package: Option<&str>,
) -> Option<CoreCallSignature> {
    let signature = qualified_core_prelude_builtin_signature(segments, expected, current_package)?;
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
        callsite: false,
    })
}

fn core_function_call_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
    resolution: CallResolutionContext<'_>,
) -> Option<CoreCallSignature> {
    if let Some(function) = resolve_function(
        segments,
        resolution.environment,
        resolution.current_module,
        resolution.current_package,
    ) {
        return Some(resolved_core_function_call_signature(
            function, expected, evidence,
        ));
    }
    if let Some(signature) = qualified_core_prelude_signature(segments, expected) {
        return Some(core_call_signature_from_parts(signature));
    }
    unresolved_core_prelude_signature(segments, expected, evidence.first)
}

fn resolved_core_function_call_signature(
    function: &FunctionSignature,
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
) -> CoreCallSignature {
    let fallback = || lowered_function_types(function);
    let (params, return_type) = if function.module_name.as_deref() == Some("std::prelude") {
        resolved_core_prelude_types(function, expected, evidence.first).unwrap_or_else(fallback)
    } else if is_io_provider_function(function) {
        io_provider_state_core_signature(&function.name, expected, evidence)
            .unwrap_or_else(fallback)
    } else {
        fallback()
    };
    CoreCallSignature {
        target: core_target_from_signature_name(&function.target_name),
        params,
        variadic: function.variadic.as_ref().map(core_type),
        return_type,
        callsite: function.callsite,
    }
}

fn resolved_core_prelude_types(
    function: &FunctionSignature,
    expected: Option<&CoreType>,
    first: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    if function.name == "list_cons"
        && let Some(item) = first
    {
        return Some((
            vec![item.clone(), CoreType::named("List", vec![item.clone()])],
            CoreType::named("List", vec![item.clone()]),
        ));
    }
    core_prelude_signature(&function.name, expected)
        .map(|(_, params, return_type)| (params, return_type))
}

fn unresolved_core_prelude_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    first: Option<&CoreType>,
) -> Option<CoreCallSignature> {
    if matches!(segments, [name] if name == "list_cons")
        && let Some(item) = first
    {
        return Some(core_call_signature_from_parts((
            CoreCallTarget::PreludeBuiltin("list_cons".to_string()),
            vec![item.clone(), CoreType::named("List", vec![item.clone()])],
            CoreType::named("List", vec![item.clone()]),
        )));
    }
    let [name] = segments else {
        return None;
    };
    core_prelude_signature(name, expected).map(core_call_signature_from_parts)
}

fn lowered_function_types(function: &FunctionSignature) -> (Vec<CoreType>, CoreType) {
    (
        function.params.iter().map(core_type).collect(),
        core_type(&function.return_type),
    )
}

fn is_io_provider_function(function: &FunctionSignature) -> bool {
    function.package_name.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        && function.module_name.as_deref() == Some("std::io_provider")
}

fn core_call_signature_from_parts(
    (target, params, return_type): (CoreCallTarget, Vec<CoreType>, CoreType),
) -> CoreCallSignature {
    CoreCallSignature {
        target,
        params,
        variadic: None,
        return_type,
        callsite: false,
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
    evidence: CallArgumentEvidence<'_, Type>,
    resolution: CallResolutionContext<'_>,
) -> Option<TypeCallSignature> {
    let (function, symbol) = resolve_function_and_symbol(segments, resolution)?;
    let (params, return_type) = resolved_surface_function_types(function, expected, evidence);
    Some(TypeCallSignature {
        params,
        variadic: function.variadic.clone(),
        return_type,
        origin: CallOrigin {
            node_id: function.node_id,
            span: function.span.clone(),
            declaration_span: Some(function.span.clone()),
            symbol,
            effects: function.effects.clone(),
            schema_encode: function
                .target_name
                .starts_with(SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX),
        },
    })
}

fn resolve_function_and_symbol<'a>(
    segments: &[String],
    resolution: CallResolutionContext<'a>,
) -> Option<(&'a FunctionSignature, String)> {
    match segments {
        [name] => Some((
            resolution
                .environment
                .unqualified_function_in_owner(
                    name,
                    resolution.current_module,
                    resolution.current_package,
                )
                .found()?,
            name.clone(),
        )),
        _ => Some((
            resolution.environment.function_path_in_owner(
                segments,
                resolution.current_module,
                resolution.current_package,
            )?,
            segments.join("::"),
        )),
    }
}

fn resolved_surface_function_types(
    function: &FunctionSignature,
    expected: Option<&Type>,
    evidence: CallArgumentEvidence<'_, Type>,
) -> (TypeArgs, Type) {
    if function.module_name.as_deref() == Some("std::prelude") {
        prelude_signature(&function.name, expected)
            .map(|(params, result)| (params.into(), result))
            .unwrap_or_else(|| (function.params.clone(), function.return_type.clone()))
    } else if is_io_provider_function(function) {
        io_provider_state_surface_signature(&function.name, expected, evidence)
            .map(|(params, result)| (params.into(), result))
            .unwrap_or_else(|| (function.params.clone(), function.return_type.clone()))
    } else {
        (function.params.clone(), function.return_type.clone())
    }
}

fn resolve_function<'a>(
    segments: &[String],
    environment: &'a TypeEnvironment,
    current_module: Option<&str>,
    current_package: Option<&str>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => environment
            .unqualified_function_in_owner(name, current_module, current_package)
            .found(),
        _ => environment.function_path_in_owner(segments, current_module, current_package),
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
    let ExprKind::TypeApply {
        callee, type_args, ..
    } = &callee.kind
    else {
        return None;
    };
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return None;
    };
    Some((segments, type_args))
}
