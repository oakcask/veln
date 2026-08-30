use veln_ast::Expr;

use veln_core::CoreType;

use crate::adt::type_operations as adt;
use crate::semantic_model::{CallOrigin, Type};
use crate::source_less_lookup::qualified_symbol;
use crate::standard_symbols::{
    StandardSignature, StandardSymbolDescriptor, StandardType, effect_strings,
};
use crate::type_lowering::core_type;

mod concurrency;

pub(crate) use concurrency::{concurrency_signature, core_concurrency_signature};

pub(crate) const KNOWN_EFFECT_LABELS: &[&str] = &[
    "stdio",
    "fs",
    "net",
    "db",
    "time",
    "random",
    "process",
    "concurrency",
];

const NET_CONCURRENCY_EFFECTS: &[&str] = &["net", "concurrency"];
const NET_TIME_CONCURRENCY_EFFECTS: &[&str] = &["net", "time", "concurrency"];

pub(crate) fn stdio_signature(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    qualified_effect_origin(segments, callee, "stdio")
}

pub(crate) fn concurrency_origin(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    qualified_effect_origin(segments, callee, "concurrency")
}

fn qualified_effect_origin(
    segments: &[String],
    callee: &Expr,
    required_effect: &str,
) -> Option<CallOrigin> {
    let symbol = qualified_symbol(segments)?;
    if !symbol.effects.contains(&required_effect) {
        return None;
    }
    call_origin(symbol, callee)
}

pub(crate) fn concurrency_effects(segments: &[String]) -> Option<&'static [&'static str]> {
    let symbol = qualified_symbol(segments)?;
    if !symbol.effects.contains(&"concurrency") {
        return None;
    }
    Some(symbol.effects)
}

pub(crate) fn concurrency_call_effects(
    segments: &[String],
    handle_type: Option<&Type>,
) -> Option<Vec<String>> {
    let symbol = qualified_symbol(segments)?;
    if !symbol.effects.contains(&"concurrency") {
        return None;
    }
    let mut effects = effect_strings(symbol);
    if task_creation_call(segments) {
        append_function_effects(&mut effects, handle_type.and_then(Type::function_effects));
    }
    Some(effects)
}

pub(crate) fn standard_library_origin(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let symbol = qualified_symbol(segments)?;
    if symbol.effects.is_empty()
        || symbol.effects.contains(&"stdio")
        || symbol.effects.contains(&"concurrency")
    {
        return None;
    }
    call_origin(symbol, callee)
}

fn call_origin(symbol: &StandardSymbolDescriptor, callee: &Expr) -> Option<CallOrigin> {
    let module = symbol.module?;
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{}", symbol.name),
        effects: effect_strings(symbol),
    })
}

pub(crate) fn standard_library_signature(segments: &[String]) -> Option<(Vec<Type>, Type)> {
    let symbol = qualified_symbol(segments)?;
    standard_signature_types(symbol.signature?)
}

fn standard_signature_types(signature: StandardSignature) -> Option<(Vec<Type>, Type)> {
    Some((
        signature
            .params
            .iter()
            .map(standard_type)
            .collect::<Option<Vec<_>>>()?,
        standard_type(&signature.return_type)?,
    ))
}

fn standard_type(spec: &StandardType) -> Option<Type> {
    match spec {
        StandardType::Bool => Some(Type::bool()),
        StandardType::Int => Some(Type::int()),
        StandardType::String => Some(Type::string()),
        StandardType::Unit => Some(Type::unit()),
        StandardType::Named(name) => Some(Type::named(*name, Vec::new())),
        StandardType::Vec(item) => Some(Type::vec(standard_type(item)?)),
        StandardType::List(item) => Some(adt::list_type(standard_type(item)?)),
        StandardType::Option(value) => Some(adt::option_type(standard_type(value)?)),
        StandardType::Result(value, error) => Some(adt::result_type(
            standard_type(value)?,
            standard_type(error)?,
        )),
    }
}

#[cfg(test)]
fn cancel_token_type() -> Type {
    Type::named("CancelToken", Vec::new())
}

pub(crate) fn core_standard_library_signature(
    segments: &[String],
) -> Option<(Vec<CoreType>, CoreType)> {
    let (params, return_type) = standard_library_signature(segments)?;
    Some((
        params.iter().map(core_type).collect(),
        core_type(&return_type),
    ))
}

pub(crate) fn is_concurrency_call(segments: &[String]) -> bool {
    qualified_symbol(segments).is_some_and(|symbol| symbol.effects.contains(&"concurrency"))
}

fn task_creation_call(segments: &[String]) -> bool {
    matches!(segments, [module, name] if module == "task" && matches!(name.as_str(), "spawn" | "spawn_with"))
}

fn append_function_effects(effects: &mut Vec<String>, function_effects: Option<&[String]>) {
    let Some(function_effects) = function_effects else {
        return;
    };
    for effect in function_effects {
        if !effects.iter().any(|existing| existing == effect) {
            effects.push(effect.clone());
        }
    }
}

pub(crate) fn is_stdio_call(segments: &[String]) -> bool {
    qualified_symbol(segments).is_some_and(|symbol| symbol.effects.contains(&"stdio"))
}

pub(crate) fn standard_library_effects(segments: &[String]) -> Option<&'static [&'static str]> {
    let symbol = qualified_symbol(segments)?;
    if symbol.effects.is_empty()
        || symbol.effects.contains(&"stdio")
        || symbol.effects.contains(&"concurrency")
    {
        return None;
    }
    Some(symbol.effects)
}

pub(crate) fn prelude_effect_origin(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let effects = prelude_effects(segments)?;
    let symbol = prelude_effect_symbol(segments)?;
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol,
        effects: effects.iter().map(|effect| (*effect).to_string()).collect(),
    })
}

pub(crate) fn prelude_effects(segments: &[String]) -> Option<&'static [&'static str]> {
    match segments {
        [name] if name == "stream_adapter_drain_actions" => Some(NET_CONCURRENCY_EFFECTS),
        [name] if name == "stream_adapter_accept_loop" => Some(NET_CONCURRENCY_EFFECTS),
        [name] if name == "stream_adapter_drain_actions_until_cancellable" => {
            Some(NET_TIME_CONCURRENCY_EFFECTS)
        }
        [module, name]
            if crate::source_less_lookup::is_reserved_source_less_module(module)
                && name == "stream_adapter_drain_actions" =>
        {
            Some(NET_CONCURRENCY_EFFECTS)
        }
        [module, name]
            if crate::source_less_lookup::is_reserved_source_less_module(module)
                && name == "stream_adapter_accept_loop" =>
        {
            Some(NET_CONCURRENCY_EFFECTS)
        }
        [module, name]
            if crate::source_less_lookup::is_reserved_source_less_module(module)
                && name == "stream_adapter_drain_actions_until_cancellable" =>
        {
            Some(NET_TIME_CONCURRENCY_EFFECTS)
        }
        _ => None,
    }
}

fn prelude_effect_symbol(segments: &[String]) -> Option<String> {
    match segments {
        [name] => Some(name.clone()),
        [module, name] => Some(format!("{module}::{name}")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "effects/tests.rs"]
mod tests;
