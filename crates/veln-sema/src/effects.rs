use veln_ast::Expr;

use veln_core::CoreType;

use crate::adt;
use crate::standard_symbols::{StandardSignature, StandardType, effect_strings, qualified_symbol};
use crate::types::{CallOrigin, Type};

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
        StandardType::Path => Some(path_type()),
        StandardType::FsError => Some(Type::named("FsError", Vec::new())),
        StandardType::ProcessError => Some(Type::named("ProcessError", Vec::new())),
        StandardType::ByteChunk => Some(byte_chunk_type()),
        StandardType::NetListener => Some(net_listener_type()),
        StandardType::NetStream => Some(net_stream_type()),
        StandardType::Deadline => Some(Type::named("Deadline", Vec::new())),
        StandardType::CancelOwner => Some(cancel_owner_type()),
        StandardType::CancelToken => Some(cancel_token_type()),
        StandardType::AcceptOutcome => Some(Type::named("AcceptOutcome", Vec::new())),
        StandardType::StreamReadOutcome => Some(Type::named("StreamReadOutcome", Vec::new())),
        StandardType::StreamWriteOutcome => Some(Type::named("StreamWriteOutcome", Vec::new())),
        StandardType::CancellableWaitOutcome => {
            Some(Type::named("CancellableWaitOutcome", Vec::new()))
        }
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
        params.iter().map(crate::types::core_type).collect(),
        crate::types::core_type(&return_type),
    ))
}

fn path_type() -> Type {
    Type::named("Path", Vec::new())
}

fn byte_chunk_type() -> Type {
    Type::named("ByteChunk", Vec::new())
}

fn net_listener_type() -> Type {
    Type::named("NetListener", Vec::new())
}

fn net_stream_type() -> Type {
    Type::named("NetStream", Vec::new())
}

fn cancel_token_type() -> Type {
    Type::named("CancelToken", Vec::new())
}

fn cancel_owner_type() -> Type {
    Type::named("CancelOwner", Vec::new())
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
            effects: vec!["concurrency".to_string()],
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
                effects: vec!["concurrency".to_string()],
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
            effects: vec!["concurrency".to_string()],
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
                effects: vec!["concurrency".to_string()],
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
mod tests {
    use super::*;

    fn path(module: &str, name: &str) -> Vec<String> {
        vec![module.to_string(), name.to_string()]
    }

    #[test]
    fn stdio_detection_comes_from_descriptor_effect_metadata() {
        assert!(is_stdio_call(&path("stdio", "println")));
        assert!(!is_stdio_call(&path("channel", "send")));
        assert!(!is_stdio_call(&path("stdio", "flush")));
    }

    #[test]
    fn concurrency_detection_comes_from_descriptor_effect_metadata() {
        assert!(is_concurrency_call(&path("task", "spawn")));
        assert!(is_concurrency_call(&path("channel", "send")));
        assert!(!is_concurrency_call(&path("stdio", "println")));
        assert!(!is_concurrency_call(&path("task", "sleep")));
    }

    #[test]
    fn fs_and_process_signatures_come_from_standard_descriptors() {
        let (params, return_type) =
            standard_library_signature(&path("fs", "read_to_string")).expect("fs signature");
        assert_eq!(params, vec![path_type()]);
        assert_eq!(
            return_type,
            Type::result(Type::string(), Type::named("FsError", Vec::new()))
        );

        let (_, return_type) =
            standard_library_signature(&path("process", "args")).expect("process signature");
        assert_eq!(return_type, Type::vec(Type::string()));
    }

    #[test]
    fn net_and_time_signatures_come_from_standard_descriptors() {
        let (params, return_type) =
            standard_library_signature(&path("net", "receive_chunk")).expect("net signature");
        assert!(params.is_empty());
        assert_eq!(return_type, byte_chunk_type());

        let (params, return_type) =
            standard_library_signature(&path("net", "send_chunk")).expect("net signature");
        assert_eq!(params, vec![byte_chunk_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("net", "connect")).expect("net signature");
        assert_eq!(params, vec![Type::string()]);
        assert_eq!(return_type, net_stream_type());

        let (params, return_type) =
            standard_library_signature(&path("net", "accept_until")).expect("net signature");
        assert_eq!(
            params,
            vec![net_listener_type(), Type::named("Deadline", Vec::new())]
        );
        assert_eq!(return_type, adt::option_type(net_stream_type()));

        let (params, return_type) =
            standard_library_signature(&path("net", "accept_until_cancellable"))
                .expect("net signature");
        assert_eq!(
            params,
            vec![
                net_listener_type(),
                Type::named("Deadline", Vec::new()),
                cancel_token_type()
            ]
        );
        assert_eq!(return_type, Type::named("AcceptOutcome", Vec::new()));

        let (params, return_type) =
            standard_library_signature(&path("net", "read_chunk_or_end")).expect("net signature");
        assert_eq!(params, vec![net_stream_type()]);
        assert_eq!(return_type, adt::option_type(byte_chunk_type()));

        let (params, return_type) =
            standard_library_signature(&path("net", "write_chunk_until")).expect("net signature");
        assert_eq!(
            params,
            vec![
                net_stream_type(),
                byte_chunk_type(),
                Type::named("Deadline", Vec::new())
            ]
        );
        assert_eq!(return_type, Type::named("StreamWriteOutcome", Vec::new()));

        let (params, return_type) =
            standard_library_signature(&path("net", "close_stream")).expect("net signature");
        assert_eq!(params, vec![net_stream_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("net", "shutdown_write")).expect("net signature");
        assert_eq!(params, vec![net_stream_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("net", "shutdown_read")).expect("net signature");
        assert_eq!(params, vec![net_stream_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "monotonic_ms")).expect("time signature");
        assert!(params.is_empty());
        assert_eq!(return_type, Type::int());

        let (params, return_type) =
            standard_library_signature(&path("time", "timeout_ms")).expect("time signature");
        assert_eq!(params, vec![Type::int()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "deadline_after_ms")).expect("time signature");
        assert_eq!(params, vec![Type::int()]);
        assert_eq!(return_type, Type::named("Deadline", Vec::new()));

        let (params, return_type) =
            standard_library_signature(&path("time", "deadline_at_ms")).expect("time signature");
        assert_eq!(params, vec![Type::int()]);
        assert_eq!(return_type, Type::named("Deadline", Vec::new()));

        let (params, return_type) =
            standard_library_signature(&path("time", "wait_until")).expect("time signature");
        assert_eq!(params, vec![Type::named("Deadline", Vec::new())]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "cancel_token")).expect("time signature");
        assert!(params.is_empty());
        assert_eq!(return_type, cancel_token_type());

        let (params, return_type) =
            standard_library_signature(&path("time", "cancel_owner")).expect("time signature");
        assert!(params.is_empty());
        assert_eq!(return_type, cancel_owner_type());

        let (params, return_type) =
            standard_library_signature(&path("time", "cancel_token_from")).expect("time signature");
        assert_eq!(params, vec![cancel_owner_type()]);
        assert_eq!(return_type, cancel_token_type());

        let (params, return_type) =
            standard_library_signature(&path("time", "cancel_owned")).expect("time signature");
        assert_eq!(params, vec![cancel_owner_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "cancel")).expect("time signature");
        assert_eq!(params, vec![cancel_token_type()]);
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "is_cancelled")).expect("time signature");
        assert_eq!(params, vec![cancel_token_type()]);
        assert_eq!(return_type, Type::bool());

        let (params, return_type) = standard_library_signature(&path("time", "is_cancelled_owner"))
            .expect("time signature");
        assert_eq!(params, vec![cancel_owner_type()]);
        assert_eq!(return_type, Type::bool());

        let (params, return_type) =
            standard_library_signature(&path("time", "wait_until_cancellable"))
                .expect("time signature");
        assert_eq!(
            params,
            vec![Type::named("Deadline", Vec::new()), cancel_token_type()]
        );
        assert_eq!(return_type, Type::unit());

        let (params, return_type) =
            standard_library_signature(&path("time", "wait_until_cancellable_outcome"))
                .expect("time signature");
        assert_eq!(
            params,
            vec![Type::named("Deadline", Vec::new()), cancel_token_type()]
        );
        assert_eq!(
            return_type,
            Type::named("CancellableWaitOutcome", Vec::new())
        );
    }
}
