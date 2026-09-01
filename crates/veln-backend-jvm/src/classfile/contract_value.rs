use super::*;

pub(super) enum ContractValue<'a> {
    Not(&'a str),
    Binary {
        left: &'a str,
        right: &'a str,
        op: BinaryOp,
    },
    BitwiseNot(&'a str),
    Call {
        callee: &'a str,
        args: Vec<&'a str>,
    },
    Field {
        base: &'a str,
        field: &'a str,
    },
    Scalar(ContractScalar<'a>),
}

pub(super) enum ContractScalar<'a> {
    Bool(bool),
    Unit,
    String(&'a str),
    Integer(i64),
    Float(f64),
    Symbol(&'a str),
}

pub(super) fn parse_contract_value(text: &str) -> ContractValue<'_> {
    let text = strip_contract_outer_parens(text.trim());
    if let Some(rest) = text.strip_prefix("not ") {
        return ContractValue::Not(rest);
    }
    for (operator, op) in contract_binary_operators() {
        if let Some((left, right)) = split_contract_binary(text, operator) {
            return ContractValue::Binary {
                left,
                right,
                op: *op,
            };
        }
    }
    if let Some(rest) = text.strip_prefix('~') {
        return ContractValue::BitwiseNot(rest);
    }
    if let Some((callee, args)) = parse_contract_call(text) {
        return ContractValue::Call { callee, args };
    }
    if let Some((base, field)) = text.split_once('.') {
        return ContractValue::Field { base, field };
    }
    match text {
        "true" => ContractValue::Scalar(ContractScalar::Bool(true)),
        "false" => ContractValue::Scalar(ContractScalar::Bool(false)),
        "()" => ContractValue::Scalar(ContractScalar::Unit),
        _ if text.starts_with('"') && text.ends_with('"') => {
            ContractValue::Scalar(ContractScalar::String(text))
        }
        _ => ContractValue::Scalar(parse_contract_scalar(text)),
    }
}

fn contract_binary_operators() -> &'static [(&'static str, BinaryOp)] {
    &[
        ("|", BinaryOp::BitwiseOr),
        ("^", BinaryOp::BitwiseXor),
        ("&", BinaryOp::BitwiseAnd),
        ("==", BinaryOp::Equal),
        ("!=", BinaryOp::NotEqual),
        (">=", BinaryOp::GreaterEqual),
        ("<=", BinaryOp::LessEqual),
        (">", BinaryOp::Greater),
        ("<", BinaryOp::Less),
        (">>>", BinaryOp::ShiftRightLogical),
        (">>", BinaryOp::ShiftRight),
        ("<<", BinaryOp::ShiftLeft),
        ("+", BinaryOp::Add),
        ("-", BinaryOp::Subtract),
        ("*", BinaryOp::Multiply),
        ("/", BinaryOp::Divide),
    ]
}

fn parse_contract_scalar(text: &str) -> ContractScalar<'_> {
    if let Some(value) = contract_integer_value(text) {
        ContractScalar::Integer(value)
    } else if let Ok(value) = text.parse::<f64>() {
        ContractScalar::Float(value)
    } else {
        ContractScalar::Symbol(text)
    }
}

fn contract_integer_value(text: &str) -> Option<i64> {
    if let Some(magnitude) = text.strip_prefix('-') {
        return parse_integer_literal(magnitude.trim())
            .ok()
            .and_then(|literal| literal.value.checked_neg());
    }
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}

pub(crate) fn split_contract_binary<'a>(text: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let bytes = text.as_bytes();
    let mut split = None;
    for_each_contract_top_level_character(text, |index, _| {
        if index + op.len() <= text.len() && contract_operator_at(bytes, index, op.as_bytes()) {
            let left = text[..index].trim();
            let right = text[index + op.len()..].trim();
            if !left.is_empty() && !right.is_empty() {
                split = Some((left, right));
            }
        }
    });
    split
}

fn for_each_contract_top_level_character(text: &str, mut visit: impl FnMut(usize, char)) {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => visit(index, character),
            _ => {}
        }
    }
}

fn contract_operator_at(text: &[u8], index: usize, operator: &[u8]) -> bool {
    if !text[index..].starts_with(operator) {
        return false;
    }
    let previous = index
        .checked_sub(1)
        .and_then(|index| text.get(index))
        .copied();
    let next = text.get(index + operator.len()).copied();
    match operator {
        b">" => previous != Some(b'>') && !matches!(next, Some(b'>' | b'=')),
        b">>" | b">>>" => previous != Some(b'>') && next != Some(b'>'),
        b"<" => previous != Some(b'<') && !matches!(next, Some(b'<' | b'=')),
        b"<<" => previous != Some(b'<') && next != Some(b'<'),
        b"|" => next != Some(b'>'),
        _ => true,
    }
}

fn strip_contract_outer_parens(mut text: &str) -> &str {
    loop {
        let Some(inner) = text
            .strip_prefix('(')
            .and_then(|text| text.strip_suffix(')'))
        else {
            return text;
        };
        let mut depth = 0usize;
        let mut closes_at_end = false;
        for (index, ch) in text.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        closes_at_end = index + ch.len_utf8() == text.len();
                        if !closes_at_end {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            return text;
        }
        text = inner.trim();
    }
}

fn parse_contract_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let callee = text[..open].trim();
    if callee.is_empty()
        || !callee
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ':')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    Some((callee, split_contract_args(inner)))
}

fn split_contract_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    for_each_contract_top_level_character(text, |index, character| {
        if character == ',' {
            let arg = text[start..index].trim();
            if !arg.is_empty() {
                args.push(arg);
            }
            start = index + 1;
        }
    });
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}
