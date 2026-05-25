use std::collections::BTreeSet;

use veln_ast::BinaryOp;

pub(crate) fn stdio_method(name: &str) -> &'static str {
    match name {
        "stdio::print" => "stdioPrint",
        "stdio::println" => "stdioPrintln",
        "stdio::eprint" => "stdioEprint",
        "stdio::eprintln" => "stdioEprintln",
        _ => "stdioPrintln",
    }
}

pub(crate) fn prelude_method(name: &str) -> &'static str {
    match name {
        "float_negate" => "floatNegate",
        "float_add" => "floatAdd",
        "float_subtract" => "floatSubtract",
        "float_multiply" => "floatMultiply",
        "float_divide" => "floatDivide",
        "float_less" => "floatLess",
        "float_less_equal" => "floatLessEqual",
        "float_greater" => "floatGreater",
        "float_greater_equal" => "floatGreaterEqual",
        "list_len" => "listLen",
        "list_is_empty" => "listIsEmpty",
        "list_push" => "listPush",
        "list_concat" => "listConcat",
        "list_map" => "listMap",
        "list_filter" => "listFilter",
        "list_fold" => "listFold",
        "list_try_map" => "listTryMap",
        "dict_get" => "dictGet",
        "dict_contains" => "dictContains",
        "dict_insert" => "dictInsert",
        "dict_remove" => "dictRemove",
        "option_map" => "optionMap",
        "option_and_then" => "optionAndThen",
        "option_unwrap_or" => "optionUnwrapOr",
        "result_map" => "resultMap",
        "result_map_err" => "resultMapErr",
        "result_and_then" => "resultAndThen",
        _ => "listLen",
    }
}

pub(crate) fn concurrency_method(name: &str) -> &'static str {
    match name {
        "channel::bounded" => "channelBounded",
        "channel::send" => "channelSend",
        "channel::recv" => "channelRecv",
        "channel::close" => "channelClose",
        _ => "channelRecv",
    }
}

pub(crate) fn binary_method(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "pipe",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "equal",
        BinaryOp::NotEqual => "notEqual",
        BinaryOp::Less => "less",
        BinaryOp::LessEqual => "lessEqual",
        BinaryOp::Greater => "greater",
        BinaryOp::GreaterEqual => "greaterEqual",
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "subtract",
        BinaryOp::Multiply => "multiply",
        BinaryOp::Divide => "divide",
    }
}

pub(crate) fn java_type_identifier(name: &str) -> String {
    let sanitized = sanitize_identifier_text(name);
    if sanitized.is_empty() || is_java_keyword(&sanitized) {
        "VelnGenerated".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn sanitize_identifier_text(text: &str) -> String {
    let mut output = String::new();
    for (index, character) in text.chars().enumerate() {
        let valid = character == '_' || character == '$' || character.is_ascii_alphanumeric();
        if !valid {
            output.push('_');
            continue;
        }
        if index == 0 && character.is_ascii_digit() {
            output.push('_');
        }
        output.push(character);
    }
    output
}

pub(crate) fn unique_java_identifier(base: &str, used_names: &mut BTreeSet<String>) -> String {
    let mut candidate = if base.is_empty() || is_java_keyword(base) {
        format!("_{base}")
    } else {
        base.to_string()
    };
    if candidate == "_" {
        candidate = "_value".to_string();
    }
    let original = candidate.clone();
    let mut suffix = 1;
    while used_names.contains(&candidate) || is_java_keyword(&candidate) {
        candidate = format!("{original}_{suffix}");
        suffix += 1;
    }
    used_names.insert(candidate.clone());
    candidate
}

fn is_java_keyword(value: &str) -> bool {
    java_keywords().iter().any(|keyword| *keyword == value)
}

fn java_keywords() -> &'static [&'static str] {
    &[
        "abstract",
        "assert",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "float",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "int",
        "interface",
        "long",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "void",
        "volatile",
        "while",
    ]
}

pub(crate) fn java_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(crate) fn veln_string_literal_value(raw: &str) -> String {
    let Some(inner) = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return raw.to_string();
    };
    let mut output = String::new();
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('"') => output.push('"'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}
