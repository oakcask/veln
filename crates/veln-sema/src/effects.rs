use veln_ast::Expr;

use veln_core::CoreType;

use crate::standard_symbols::{effect_strings, qualified_symbol};
use crate::types::{CallOrigin, Type};

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
    let module = symbol.module?;
    match (module, symbol.name) {
        ("fs", "read_to_string") => Some((
            vec![path_type()],
            Type::result(Type::string(), Type::named("FsError", Vec::new())),
        )),
        ("fs", "write_string") => Some((
            vec![path_type(), Type::string()],
            Type::result(Type::unit(), Type::named("FsError", Vec::new())),
        )),
        ("fs", "exists") => Some((
            vec![path_type()],
            Type::result(Type::bool(), Type::named("FsError", Vec::new())),
        )),
        ("fs", "read_dir") => Some((
            vec![path_type()],
            Type::result(Type::list(path_type()), Type::named("FsError", Vec::new())),
        )),
        ("process", "args") => Some((Vec::new(), Type::list(Type::string()))),
        ("process", "env") => Some((
            vec![Type::string()],
            Type::named("Option", vec![Type::string()]),
        )),
        ("process", "cwd") => Some((
            Vec::new(),
            Type::result(path_type(), Type::named("ProcessError", Vec::new())),
        )),
        ("process", "exit") => Some((vec![Type::int()], Type::unit())),
        _ => None,
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

pub(crate) fn concurrency_signature(
    segments: &[String],
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    let unknown = Type::Unknown;
    match (module.as_str(), name.as_str()) {
        ("channel", "bounded") => {
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
        ("channel", "clone") => {
            let item = handle_type
                .and_then(|ty| named_type_argument(ty, "Sender"))
                .cloned()
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Sender", vec![item.clone()])],
                Type::named("Sender", vec![item]),
            ))
        }
        ("channel", "send") => {
            let item = handle_type
                .and_then(|ty| named_type_argument(ty, "Sender"))
                .cloned()
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Sender", vec![item.clone()]), item],
                Type::result(Type::unit(), Type::named("SendError", Vec::new())),
            ))
        }
        ("channel", "recv") => {
            let item = expected
                .and_then(Type::option_part)
                .cloned()
                .or_else(|| {
                    handle_type
                        .and_then(|ty| named_type_argument(ty, "Receiver"))
                        .cloned()
                })
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Receiver", vec![item.clone()])],
                Type::named("Option", vec![item]),
            ))
        }
        (
            "channel",
            "select"
            | "select_priority"
            | "select_timeout"
            | "select_result"
            | "select_priority_result"
            | "select_timeout_result",
        ) => {
            let reports_interrupt = name.ends_with("_result");
            let item = select_item_type(expected, reports_interrupt)
                .or_else(|| {
                    handle_type
                        .and_then(|ty| named_type_argument(ty, "Receiver"))
                        .cloned()
                })
                .unwrap_or(Type::Unknown);
            let mut params = vec![
                Type::named("Receiver", vec![item.clone()]),
                Type::named("Receiver", vec![item.clone()]),
            ];
            if matches!(name.as_str(), "select_timeout" | "select_timeout_result") {
                params.push(Type::int());
            }
            let output = Type::named("Option", vec![select_output_record(item)]);
            let return_type = if reports_interrupt {
                Type::result(output, Type::named("SelectError", Vec::new()))
            } else {
                output
            };
            Some((params, return_type))
        }
        ("channel", "close") => Some((vec![Type::named("Sender", vec![unknown])], Type::unit())),
        ("task", "spawn") => {
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
                    return_type: Box::new(item.clone()),
                    effects: vec!["concurrency".to_string()],
                }],
                Type::named("Task", vec![item]),
            ))
        }
        ("task", "join") => {
            let item = handle_type
                .and_then(|ty| named_type_argument(ty, "Task"))
                .cloned()
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Task", vec![item.clone()])],
                Type::result(item, Type::named("JoinError", Vec::new())),
            ))
        }
        ("task", "cancel") => Some((vec![Type::named("Task", vec![unknown])], Type::unit())),
        _ => None,
    }
}

fn function_return_type(ty: &Type) -> Option<&Type> {
    let (_, return_type) = ty.function_parts()?;
    Some(return_type)
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
) -> Option<(Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    let unknown = CoreType::Unknown;
    match (module.as_str(), name.as_str()) {
        ("channel", "bounded") => {
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
        ("channel", "clone") => {
            let item = handle_type
                .and_then(|ty| core_named_type_argument(ty, "Sender"))
                .cloned()
                .unwrap_or(CoreType::Unknown);
            Some((
                vec![CoreType::named("Sender", vec![item.clone()])],
                CoreType::named("Sender", vec![item]),
            ))
        }
        ("channel", "send") => Some((
            vec![
                CoreType::named("Sender", vec![unknown.clone()]),
                unknown.clone(),
            ],
            CoreType::result(CoreType::unit(), CoreType::named("SendError", Vec::new())),
        )),
        ("channel", "recv") => {
            let item = expected
                .and_then(CoreType::option_part)
                .cloned()
                .or_else(|| {
                    handle_type
                        .and_then(|ty| core_named_type_argument(ty, "Receiver"))
                        .cloned()
                })
                .unwrap_or(CoreType::Unknown);
            Some((
                vec![CoreType::named("Receiver", vec![item.clone()])],
                CoreType::option(item),
            ))
        }
        (
            "channel",
            "select"
            | "select_priority"
            | "select_timeout"
            | "select_result"
            | "select_priority_result"
            | "select_timeout_result",
        ) => {
            let reports_interrupt = name.ends_with("_result");
            let item = core_select_item_type(expected, reports_interrupt)
                .or_else(|| {
                    handle_type
                        .and_then(|ty| core_named_type_argument(ty, "Receiver"))
                        .cloned()
                })
                .unwrap_or(CoreType::Unknown);
            let mut params = vec![
                CoreType::named("Receiver", vec![item.clone()]),
                CoreType::named("Receiver", vec![item.clone()]),
            ];
            if matches!(name.as_str(), "select_timeout" | "select_timeout_result") {
                params.push(CoreType::int());
            }
            let output = CoreType::option(core_select_output_record(item));
            let return_type = if reports_interrupt {
                CoreType::result(output, CoreType::named("SelectError", Vec::new()))
            } else {
                output
            };
            Some((params, return_type))
        }
        ("channel", "close") => Some((
            vec![CoreType::named("Sender", vec![unknown])],
            CoreType::unit(),
        )),
        ("task", "spawn") => {
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
                    return_type: Box::new(item.clone()),
                    effects: vec!["concurrency".to_string()],
                }],
                CoreType::named("Task", vec![item]),
            ))
        }
        ("task", "join") => {
            let item = handle_type
                .and_then(|ty| core_named_type_argument(ty, "Task"))
                .cloned()
                .unwrap_or(CoreType::Unknown);
            Some((
                vec![CoreType::named("Task", vec![item.clone()])],
                CoreType::result(item, CoreType::named("JoinError", Vec::new())),
            ))
        }
        ("task", "cancel") => Some((
            vec![CoreType::named("Task", vec![unknown])],
            CoreType::unit(),
        )),
        _ => None,
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
        assert_eq!(return_type, Type::list(Type::string()));
    }
}

fn core_function_return_type(ty: &CoreType) -> Option<&CoreType> {
    match ty {
        CoreType::Function { return_type, .. } => Some(return_type),
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
            .and_then(Type::result_parts)
            .map(|(value, _)| value)
            .and_then(Type::option_part)
            .and_then(select_result_value_type)
            .cloned()
    } else {
        expected
            .and_then(Type::option_part)
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
            .and_then(CoreType::result_parts)
            .map(|(value, _)| value)
            .and_then(CoreType::option_part)
            .and_then(core_select_result_value_type)
            .cloned()
    } else {
        expected
            .and_then(CoreType::option_part)
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
