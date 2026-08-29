use super::*;

pub(crate) fn missing_contract_field(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<(String, String)> {
    for access in field_accesses(predicate) {
        let Some(mut current) = predicate_type_with_calls(&access.base, bindings, call_type) else {
            continue;
        };
        for field in access.fields {
            let Some(next) = current.record_field(&field) else {
                return Some((current.render(), field));
            };
            current = next.clone();
        }
    }
    None
}

pub(crate) fn predicate_is_boolean_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> bool {
    let trimmed = predicate.trim();
    predicate_type_with_calls(trimmed, bindings, call_type).is_some_and(
        |ty| matches!(ty, Type::Named { name, args } if name == "Bool" && args.is_empty()),
    )
}

pub(crate) fn predicate_rendered_type_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> String {
    let trimmed = predicate.trim();
    predicate_type_with_calls(trimmed, bindings, call_type)
        .map_or_else(|| "unknown".to_string(), |ty| ty.render())
}

pub(crate) fn predicate_type_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    predicate_literal_type(predicate)
        .or_else(|| predicate_unary_type(predicate, bindings, call_type))
        .or_else(|| predicate_boolean_type(predicate, bindings, call_type))
        .or_else(|| predicate_comparison_type(predicate, bindings, call_type))
        .or_else(|| predicate_bitwise_type(predicate, bindings, call_type))
        .or_else(|| predicate_arithmetic_type(predicate, bindings, call_type))
        .or_else(|| predicate_field_access_type(predicate, bindings, call_type))
        .or_else(|| predicate_contract_call_type(predicate, call_type))
        .or_else(|| predicate_binding_type(predicate, bindings))
}

pub(super) fn predicate_literal_type(predicate: &str) -> Option<Type> {
    if matches!(predicate, "true" | "false") {
        return Some(Type::bool());
    }
    if predicate == "()" {
        return Some(Type::unit());
    }
    if is_complete_string_literal(predicate) {
        return Some(Type::string());
    }
    if parse_integer_literal(predicate).is_ok() {
        return Some(Type::int());
    }
    is_float_literal(predicate).then(Type::float)
}

pub(super) fn predicate_unary_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    if let Some(rest) = predicate.strip_prefix('-') {
        let ty = predicate_type_with_calls(rest, bindings, call_type)?;
        return matches!(ty, Type::Named { ref name, ref args } if args.is_empty() && (name == "Int" || name == "Float"))
            .then_some(ty);
    }
    if let Some(rest) = predicate.strip_prefix('~') {
        return (predicate_type_with_calls(rest, bindings, call_type)? == Type::int())
            .then(Type::int);
    }
    if let Some(rest) = predicate.strip_prefix("not ") {
        return boolean_unary_type(rest, bindings, call_type);
    }
    let inner = predicate.strip_prefix("not(")?.strip_suffix(')')?;
    boolean_unary_type(inner, bindings, call_type)
}

pub(super) fn boolean_unary_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    (predicate_type_with_calls(predicate, bindings, call_type)? == Type::bool()).then(Type::bool)
}

pub(super) fn predicate_boolean_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["or", "and"] {
        let clauses = split_top_level_keyword(predicate, operator);
        if clauses.len() > 1 {
            return clauses
                .into_iter()
                .all(|clause| {
                    predicate_type_with_calls(clause, bindings, call_type) == Some(Type::bool())
                })
                .then(Type::bool);
        }
    }
    None
}

pub(super) fn predicate_comparison_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return comparable_predicate_operands(&left, &right).then(Type::bool);
        }
    }
    None
}

pub(super) fn predicate_arithmetic_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["+", "-", "*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return numeric_result_type(&left, &right);
        }
    }
    None
}

pub(super) fn predicate_bitwise_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    if !contains_binary_bitwise_operator(predicate) {
        return None;
    }
    for operators in [&["|"][..], &["^"][..], &["&"][..], &[">>>", ">>", "<<"][..]] {
        for operator in operators {
            if let Some((left, right)) = split_top_level_operator(predicate, operator) {
                let left = predicate_type_with_calls(left, bindings, call_type)?;
                let right = predicate_type_with_calls(right, bindings, call_type)?;
                return (left == Type::int() && right == Type::int()).then(Type::int);
            }
        }
    }
    None
}

pub(super) fn contains_binary_bitwise_operator(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| match byte {
        b'|' | b'^' | b'&' => true,
        b'<' | b'>' => bytes.get(index + 1) == Some(byte),
        _ => false,
    })
}

pub(super) fn predicate_field_access_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let access = split_field_access(predicate)?;
    let mut current = predicate_type_with_calls(access.base, bindings, call_type)?;
    for field in access.fields {
        current = current.record_field(field)?.clone();
    }
    Some(current)
}

pub(super) fn predicate_contract_call_type(
    predicate: &str,
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let calls = contract_calls(predicate);
    let [call] = calls.as_slice() else {
        return None;
    };
    (call.start == 0 && call.end == predicate.len())
        .then(|| call_type(&call.callee))
        .flatten()
}

pub(super) fn predicate_binding_type(predicate: &str, bindings: &[Binding]) -> Option<Type> {
    if let Some(binding) = bindings.iter().find(|binding| binding.name == predicate) {
        return Some(binding.ty.clone());
    }
    let mut parts = predicate.split('.');
    let base = parts.next()?;
    let binding = bindings.iter().find(|binding| binding.name == base)?;
    let mut current = binding.ty.clone();
    for field in parts {
        current = current.record_field(field)?.clone();
    }
    Some(current)
}

pub(super) fn comparable_predicate_operands(left: &Type, right: &Type) -> bool {
    left == right || (is_numeric_type(left) && is_numeric_type(right))
}

pub(super) fn numeric_result_type(left: &Type, right: &Type) -> Option<Type> {
    if !is_numeric_type(left) || !is_numeric_type(right) {
        return None;
    }
    if left == &Type::float() || right == &Type::float() {
        Some(Type::float())
    } else {
        Some(Type::int())
    }
}

pub(super) fn is_numeric_type(ty: &Type) -> bool {
    ty == &Type::int() || ty == &Type::float()
}

pub(super) fn is_float_literal(text: &str) -> bool {
    let Some((left, right)) = text.split_once('.') else {
        return false;
    };
    !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|ch| ch.is_ascii_digit())
        && right.chars().all(|ch| ch.is_ascii_digit())
}

pub(super) fn split_top_level_operator<'a>(
    predicate: &'a str,
    operator: &str,
) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && contract_operator_at(predicate, index, operator) => {
                let left = predicate[..index].trim();
                let right = predicate[index + operator.len()..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn contract_operator_at(text: &str, index: usize, operator: &str) -> bool {
    if !text[index..].starts_with(operator) {
        return false;
    }
    let next = text[index + operator.len()..].chars().next();
    match operator {
        ">" => !matches!(next, Some('>' | '=')),
        ">>" => next != Some('>'),
        "<" => !matches!(next, Some('<' | '=')),
        "|" => next != Some('>'),
        _ => true,
    }
}

pub(super) fn split_top_level_keyword_operator<'a>(
    predicate: &'a str,
    keyword: &str,
) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[index..].starts_with(keyword) => {
                let end = index + keyword.len();
                if !is_keyword_boundary(predicate, index, end) {
                    continue;
                }
                let left = predicate[..index].trim();
                let right = predicate[end..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn split_top_level_keyword<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = 0;

    while cursor < predicate.len() {
        let ch = predicate[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        let end = cursor + ch.len_utf8();
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor = end;
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[cursor..].starts_with(keyword) => {
                let keyword_end = cursor + keyword.len();
                if is_keyword_boundary(predicate, cursor, keyword_end) {
                    clauses.push(predicate[start..cursor].trim());
                    start = keyword_end;
                    cursor = keyword_end;
                    continue;
                }
            }
            _ => {}
        }
        cursor = end;
    }

    clauses.push(predicate[start..].trim());
    clauses
}

pub(super) fn is_keyword_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_ident_continue(ch)) && after.is_none_or(|ch| !is_ident_continue(ch))
}

pub(super) fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
