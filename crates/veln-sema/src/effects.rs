use veln_ast::Expr;

use veln_core::CoreType;

use crate::types::{CallOrigin, Type};

pub(crate) fn stdio_signature(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let [module, name] = segments else {
        return None;
    };
    if module != "stdio" || !matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln") {
        return None;
    }
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{name}"),
        effects: vec!["stdio".to_string()],
    })
}

pub(crate) fn concurrency_origin(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let [module, name] = segments else {
        return None;
    };
    if module != "channel" || !matches!(name.as_str(), "send" | "recv" | "close") {
        return None;
    }
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{name}"),
        effects: vec!["concurrency".to_string()],
    })
}

pub(crate) fn concurrency_signature(
    segments: &[String],
    expected: Option<&Type>,
    handle_type: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != "channel" {
        return None;
    }
    let unknown = Type::Unknown;
    match name.as_str() {
        "send" => {
            let item = handle_type
                .and_then(|ty| named_type_argument(ty, "Sender"))
                .cloned()
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Sender", vec![item.clone()]), item],
                Type::result(Type::unit(), Type::named("SendError", Vec::new())),
            ))
        }
        "recv" => {
            let item = expected
                .and_then(Type::option_part)
                .cloned()
                .unwrap_or(Type::Unknown);
            Some((
                vec![Type::named("Receiver", vec![item.clone()])],
                Type::named("Option", vec![item]),
            ))
        }
        "close" => Some((vec![Type::named("Sender", vec![unknown])], Type::unit())),
        _ => None,
    }
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
) -> Option<(Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != "channel" {
        return None;
    }
    let unknown = CoreType::Unknown;
    match name.as_str() {
        "send" => Some((
            vec![
                CoreType::named("Sender", vec![unknown.clone()]),
                unknown.clone(),
            ],
            CoreType::result(CoreType::unit(), CoreType::named("SendError", Vec::new())),
        )),
        "recv" => {
            let item = expected
                .and_then(CoreType::option_part)
                .cloned()
                .unwrap_or(CoreType::Unknown);
            Some((
                vec![CoreType::named("Receiver", vec![item.clone()])],
                CoreType::option(item),
            ))
        }
        "close" => Some((
            vec![CoreType::named("Sender", vec![unknown])],
            CoreType::unit(),
        )),
        _ => None,
    }
}

pub(crate) fn is_concurrency_call(segments: &[String]) -> bool {
    matches!(
        segments,
        [module, name]
            if module == "channel" && matches!(name.as_str(), "send" | "recv" | "close")
    )
}
