use veln_ast::Expr;

use veln_core::CoreType;

use crate::adt;
use crate::semantic_model::{CallOrigin, Type};
use crate::source_less_lookup::qualified_symbol;
use crate::standard_symbols::{StandardSignature, StandardType, effect_strings};
use crate::type_lowering::core_type;

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
    let symbol = qualified_symbol(segments)?;
    if !symbol.effects.contains(&"stdio") {
        return None;
    }
    let module = symbol.module?;
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{}", symbol.name),
        effects: effect_strings(symbol),
    })
}

pub(crate) fn concurrency_origin(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let symbol = qualified_symbol(segments)?;
    if !symbol.effects.contains(&"concurrency") {
        return None;
    }
    let module = symbol.module?;
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{}", symbol.name),
        effects: effect_strings(symbol),
    })
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

pub(crate) fn core_standard_library_signature(
    segments: &[String],
) -> Option<(Vec<CoreType>, CoreType)> {
    let (params, return_type) = standard_library_signature(segments)?;
    Some((
        params.iter().map(core_type).collect(),
        core_type(&return_type),
    ))
}

fn cancel_token_type() -> Type {
    Type::named("CancelToken", Vec::new())
}

pub(crate) fn concurrency_signature(
    segments: &[String],
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
    explicit_context: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    match module.as_str() {
        "channel" => channel_signature(name, expected, handle_type, explicit_item),
        "task" => task_signature(name, expected, handle_type, explicit_item, explicit_context),
        _ => None,
    }
}

fn channel_signature(
    name: &str,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let unknown = Type::Unknown;
    match name {
        "bounded" => {
            let item = explicit_item
                .cloned()
                .or_else(|| channel_pair_item_type(expected))
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::int()],
                Type::Record(vec![
                    ("tx".to_string(), Type::named("Sender", vec![item.clone()])),
                    ("rx".to_string(), Type::named("Receiver", vec![item])),
                ]),
            ))
        }
        "clone" => sender_clone_signature(handle_type),
        "send" => sender_send_signature(handle_type),
        "recv" => receiver_recv_signature(expected, handle_type),
        "select"
        | "select_priority"
        | "select_many_priority"
        | "select_many_timeout"
        | "select_many_timeout_result"
        | "select_many_timeout_cancellable"
        | "select_timeout"
        | "select_timeout_cancellable"
        | "select_result"
        | "select_priority_result"
        | "select_timeout_result" => select_signature(name, expected, handle_type),
        "close" => Some((vec![Type::named("Sender", vec![unknown])], Type::unit())),
        _ => None,
    }
}

fn sender_clone_signature(handle_type: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let item = sender_item_type(handle_type);
    Some((
        vec![Type::named("Sender", vec![item.clone()])],
        Type::named("Sender", vec![item]),
    ))
}

fn sender_send_signature(handle_type: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let item = sender_item_type(handle_type);
    Some((
        vec![Type::named("Sender", vec![item.clone()]), item],
        adt::result_type(Type::unit(), Type::named("SendError", Vec::new())),
    ))
}

fn receiver_recv_signature(
    expected: Option<&Type>,
    handle_type: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let item = expected
        .and_then(adt::option_part)
        .cloned()
        .or_else(|| receiver_item_type(handle_type))
        .unwrap_or(Type::Unknown);
    Some((
        vec![Type::named("Receiver", vec![item.clone()])],
        adt::option_type(item),
    ))
}

fn select_signature(
    name: &str,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let reports_interrupt = name.ends_with("_result")
        || matches!(
            name,
            "select_many_timeout_cancellable" | "select_timeout_cancellable"
        );
    let item = select_item_type(expected, reports_interrupt)
        .or_else(|| select_receiver_item_type(name, handle_type))
        .unwrap_or(Type::Unknown);
    let mut params = if matches!(
        name,
        "select_many_priority"
            | "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
    ) {
        vec![Type::named(
            "List",
            vec![Type::named("Receiver", vec![item.clone()])],
        )]
    } else {
        vec![
            Type::named("Receiver", vec![item.clone()]),
            Type::named("Receiver", vec![item.clone()]),
        ]
    };
    if matches!(
        name,
        "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
            | "select_timeout"
            | "select_timeout_cancellable"
            | "select_timeout_result"
    ) {
        params.push(Type::int());
    }
    if matches!(
        name,
        "select_many_timeout_cancellable" | "select_timeout_cancellable"
    ) {
        params.push(cancel_token_type());
    }
    let output = adt::option_type(select_output_record(item));
    let return_type = if reports_interrupt {
        adt::result_type(output, Type::named("SelectError", Vec::new()))
    } else {
        output
    };
    Some((params, return_type))
}

fn task_signature(
    name: &str,
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
    explicit_context: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let unknown = Type::Unknown;
    match name {
        "spawn" => task_spawn_signature(expected, handle_type, explicit_item),
        "spawn_with" => {
            task_spawn_with_signature(expected, handle_type, explicit_item, explicit_context)
        }
        "join" => task_join_signature(handle_type),
        "cancel" => Some((vec![Type::named("Task", vec![unknown])], Type::unit())),
        _ => None,
    }
}

fn task_spawn_signature(
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let item = explicit_item
        .cloned()
        .or_else(|| {
            expected
                .and_then(|ty| named_type_argument(ty, "Task"))
                .cloned()
        })
        .or_else(|| handle_type.and_then(function_return_type).cloned())
        .unwrap_or(Type::Unknown);
    Some((
        vec![Type::Function {
            params: Vec::new(),
            variadic: None,
            return_type: Box::new(item.clone()),
            effects: function_effects(handle_type),
        }],
        Type::named("Task", vec![item]),
    ))
}

fn task_spawn_with_signature(
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
    explicit_context: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let arg = explicit_context
        .cloned()
        .or_else(|| {
            handle_type
                .and_then(function_params)
                .and_then(|params| params.first())
                .cloned()
        })
        .unwrap_or(Type::Unknown);
    let item = explicit_item
        .cloned()
        .or_else(|| {
            expected
                .and_then(|ty| named_type_argument(ty, "Task"))
                .cloned()
        })
        .or_else(|| handle_type.and_then(function_return_type).cloned())
        .unwrap_or(Type::Unknown);
    Some((
        vec![
            Type::Function {
                params: vec![arg.clone()],
                variadic: None,
                return_type: Box::new(item.clone()),
                effects: function_effects(handle_type),
            },
            arg,
        ],
        Type::named("Task", vec![item]),
    ))
}

fn task_join_signature(handle_type: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let item = handle_type
        .and_then(|ty| named_type_argument(ty, "Task"))
        .cloned()
        .unwrap_or(Type::Unknown);
    Some((
        vec![Type::named("Task", vec![item.clone()])],
        adt::result_type(item, Type::named("JoinError", Vec::new())),
    ))
}

fn sender_item_type(handle_type: Option<&Type>) -> Type {
    handle_type
        .and_then(|ty| named_type_argument(ty, "Sender"))
        .cloned()
        .unwrap_or(Type::Unknown)
}

fn receiver_item_type(handle_type: Option<&Type>) -> Option<Type> {
    handle_type
        .and_then(|ty| named_type_argument(ty, "Receiver"))
        .cloned()
}

fn select_receiver_item_type(name: &str, handle_type: Option<&Type>) -> Option<Type> {
    if matches!(
        name,
        "select_many_priority"
            | "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
    ) {
        handle_type
            .and_then(|ty| named_type_argument(ty, "List"))
            .and_then(|ty| named_type_argument(ty, "Receiver"))
            .cloned()
    } else {
        receiver_item_type(handle_type)
    }
}

fn function_return_type(ty: &Type) -> Option<&Type> {
    let (_, return_type) = ty.function_parts()?;
    Some(return_type)
}

fn function_params(ty: &Type) -> Option<&[Type]> {
    let (params, _) = ty.function_parts()?;
    Some(params)
}

fn function_effects(ty: Option<&Type>) -> Vec<String> {
    ty.and_then(Type::function_effects)
        .map_or_else(Vec::new, <[String]>::to_vec)
}

fn named_type_argument<'a>(ty: &'a Type, expected_name: &str) -> Option<&'a Type> {
    match ty {
        Type::Named { name, args } if name == expected_name && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}

pub(crate) fn core_concurrency_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
    explicit_context: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    match module.as_str() {
        "channel" => core_channel_signature(name, expected, handle_type, explicit_item),
        "task" => core_task_signature(name, expected, handle_type, explicit_item, explicit_context),
        _ => None,
    }
}

fn core_channel_signature(
    name: &str,
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let unknown = CoreType::Unknown;
    match name {
        "bounded" => {
            let item = explicit_item
                .cloned()
                .or_else(|| core_channel_pair_item_type(expected))
                .unwrap_or(CoreType::Unknown);
            Some((
                vec![CoreType::int()],
                CoreType::Record(vec![
                    (
                        "tx".to_string(),
                        CoreType::named("Sender", vec![item.clone()]),
                    ),
                    ("rx".to_string(), CoreType::named("Receiver", vec![item])),
                ]),
            ))
        }
        "clone" => core_sender_clone_signature(handle_type),
        "send" => Some((
            vec![
                CoreType::named("Sender", vec![unknown.clone()]),
                unknown.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::named("SendError", Vec::new())),
        )),
        "recv" => core_receiver_recv_signature(expected, handle_type),
        "select"
        | "select_priority"
        | "select_many_priority"
        | "select_many_timeout"
        | "select_many_timeout_result"
        | "select_many_timeout_cancellable"
        | "select_timeout"
        | "select_timeout_cancellable"
        | "select_result"
        | "select_priority_result"
        | "select_timeout_result" => core_select_signature(name, expected, handle_type),
        "close" => Some((
            vec![CoreType::named("Sender", vec![unknown])],
            CoreType::unit(),
        )),
        _ => None,
    }
}

fn core_sender_clone_signature(
    handle_type: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let item = handle_type
        .and_then(|ty| core_named_type_argument(ty, "Sender"))
        .cloned()
        .unwrap_or(CoreType::Unknown);
    Some((
        vec![CoreType::named("Sender", vec![item.clone()])],
        CoreType::named("Sender", vec![item]),
    ))
}

fn core_receiver_recv_signature(
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let item = expected
        .and_then(adt::core_option_part)
        .cloned()
        .or_else(|| core_receiver_item_type(handle_type))
        .unwrap_or(CoreType::Unknown);
    Some((
        vec![CoreType::named("Receiver", vec![item.clone()])],
        adt::core_option_type(item),
    ))
}

fn core_select_signature(
    name: &str,
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let reports_interrupt = name.ends_with("_result")
        || matches!(
            name,
            "select_many_timeout_cancellable" | "select_timeout_cancellable"
        );
    let item = core_select_item_type(expected, reports_interrupt)
        .or_else(|| core_select_receiver_item_type(name, handle_type))
        .unwrap_or(CoreType::Unknown);
    let mut params = if matches!(
        name,
        "select_many_priority"
            | "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
    ) {
        vec![CoreType::named(
            "List",
            vec![CoreType::named("Receiver", vec![item.clone()])],
        )]
    } else {
        vec![
            CoreType::named("Receiver", vec![item.clone()]),
            CoreType::named("Receiver", vec![item.clone()]),
        ]
    };
    if matches!(
        name,
        "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
            | "select_timeout"
            | "select_timeout_cancellable"
            | "select_timeout_result"
    ) {
        params.push(CoreType::int());
    }
    if matches!(
        name,
        "select_many_timeout_cancellable" | "select_timeout_cancellable"
    ) {
        params.push(CoreType::named("CancelToken", Vec::new()));
    }
    let output = adt::core_option_type(core_select_output_record(item));
    let return_type = if reports_interrupt {
        adt::core_result_type(output, CoreType::named("SelectError", Vec::new()))
    } else {
        output
    };
    Some((params, return_type))
}

fn core_task_signature(
    name: &str,
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
    explicit_context: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let unknown = CoreType::Unknown;
    match name {
        "spawn" => core_task_spawn_signature(expected, handle_type, explicit_item),
        "spawn_with" => {
            core_task_spawn_with_signature(expected, handle_type, explicit_item, explicit_context)
        }
        "join" => core_task_join_signature(handle_type),
        "cancel" => Some((
            vec![CoreType::named("Task", vec![unknown])],
            CoreType::unit(),
        )),
        _ => None,
    }
}

fn core_task_spawn_signature(
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let item = explicit_item
        .cloned()
        .or_else(|| {
            expected
                .and_then(|ty| core_named_type_argument(ty, "Task"))
                .cloned()
        })
        .or_else(|| handle_type.and_then(core_function_return_type).cloned())
        .unwrap_or(CoreType::Unknown);
    Some((
        vec![CoreType::Function {
            params: Vec::new(),
            variadic: None,
            return_type: Box::new(item.clone()),
            effects: core_function_effects(handle_type),
        }],
        CoreType::named("Task", vec![item]),
    ))
}

fn core_task_spawn_with_signature(
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
    explicit_context: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    let arg = explicit_context
        .cloned()
        .or_else(|| {
            handle_type
                .and_then(core_function_params)
                .and_then(|params| params.first())
                .cloned()
        })
        .unwrap_or(CoreType::Unknown);
    let item = explicit_item
        .cloned()
        .or_else(|| {
            expected
                .and_then(|ty| core_named_type_argument(ty, "Task"))
                .cloned()
        })
        .or_else(|| handle_type.and_then(core_function_return_type).cloned())
        .unwrap_or(CoreType::Unknown);
    Some((
        vec![
            CoreType::Function {
                params: vec![arg.clone()],
                variadic: None,
                return_type: Box::new(item.clone()),
                effects: core_function_effects(handle_type),
            },
            arg,
        ],
        CoreType::named("Task", vec![item]),
    ))
}

fn core_task_join_signature(handle_type: Option<&CoreType>) -> Option<(Vec<CoreType>, CoreType)> {
    let item = handle_type
        .and_then(|ty| core_named_type_argument(ty, "Task"))
        .cloned()
        .unwrap_or(CoreType::Unknown);
    Some((
        vec![CoreType::named("Task", vec![item.clone()])],
        adt::core_result_type(item, CoreType::named("JoinError", Vec::new())),
    ))
}

fn core_function_effects(ty: Option<&CoreType>) -> Vec<String> {
    match ty {
        Some(CoreType::Function { effects, .. }) => effects.clone(),
        _ => Vec::new(),
    }
}

fn core_receiver_item_type(handle_type: Option<&CoreType>) -> Option<CoreType> {
    handle_type
        .and_then(|ty| core_named_type_argument(ty, "Receiver"))
        .cloned()
}

fn core_select_receiver_item_type(name: &str, handle_type: Option<&CoreType>) -> Option<CoreType> {
    if matches!(
        name,
        "select_many_priority"
            | "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
    ) {
        handle_type
            .and_then(|ty| core_named_type_argument(ty, "List"))
            .and_then(|ty| core_named_type_argument(ty, "Receiver"))
            .cloned()
    } else {
        core_receiver_item_type(handle_type)
    }
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

fn core_function_return_type(ty: &CoreType) -> Option<&CoreType> {
    match ty {
        CoreType::Function { return_type, .. } => Some(return_type),
        _ => None,
    }
}

fn core_function_params(ty: &CoreType) -> Option<&[CoreType]> {
    match ty {
        CoreType::Function { params, .. } => Some(params),
        _ => None,
    }
}

fn channel_pair_item_type(expected: Option<&Type>) -> Option<Type> {
    let tx = expected?.record_field("tx")?;
    let rx = expected?.record_field("rx")?;
    let tx_item = named_type_argument(tx, "Sender")?;
    let rx_item = named_type_argument(rx, "Receiver")?;
    if tx_item == rx_item {
        Some(tx_item.clone())
    } else {
        None
    }
}

fn select_item_type(expected: Option<&Type>, reports_interrupt: bool) -> Option<Type> {
    if reports_interrupt {
        expected
            .and_then(adt::result_parts)
            .map(|(value, _)| value)
            .and_then(adt::option_part)
            .and_then(select_result_value_type)
            .cloned()
    } else {
        expected
            .and_then(adt::option_part)
            .and_then(select_result_value_type)
            .cloned()
    }
}

fn select_result_value_type(ty: &Type) -> Option<&Type> {
    ty.record_field("value")
}

fn select_output_record(item: Type) -> Type {
    Type::Record(vec![
        ("index".to_string(), Type::int()),
        ("value".to_string(), item),
    ])
}

fn core_channel_pair_item_type(expected: Option<&CoreType>) -> Option<CoreType> {
    let tx = expected?.record_field("tx")?;
    let rx = expected?.record_field("rx")?;
    let tx_item = core_named_type_argument(tx, "Sender")?;
    let rx_item = core_named_type_argument(rx, "Receiver")?;
    if tx_item == rx_item {
        Some(tx_item.clone())
    } else {
        None
    }
}

fn core_select_item_type(expected: Option<&CoreType>, reports_interrupt: bool) -> Option<CoreType> {
    if reports_interrupt {
        expected
            .and_then(adt::core_result_parts)
            .map(|(value, _)| value)
            .and_then(adt::core_option_part)
            .and_then(core_select_result_value_type)
            .cloned()
    } else {
        expected
            .and_then(adt::core_option_part)
            .and_then(core_select_result_value_type)
            .cloned()
    }
}

fn core_select_result_value_type(ty: &CoreType) -> Option<&CoreType> {
    ty.record_field("value")
}

fn core_select_output_record(item: CoreType) -> CoreType {
    CoreType::Record(vec![
        ("index".to_string(), CoreType::int()),
        ("value".to_string(), item),
    ])
}

fn core_named_type_argument<'a>(ty: &'a CoreType, expected_name: &str) -> Option<&'a CoreType> {
    match ty {
        CoreType::Named { name, args } if name == expected_name && args.len() == 1 => {
            Some(&args[0])
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "effects/tests.rs"]
mod tests;
