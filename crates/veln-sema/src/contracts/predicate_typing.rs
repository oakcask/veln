use super::*;

#[cfg(test)]
thread_local! {
    static PREDICATE_TYPING_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_predicate_typing_work(predicate: &str) {
    PREDICATE_TYPING_WORK.with(|work| work.set(work.get() + predicate.len()));
}

#[cfg(not(test))]
fn record_predicate_typing_work(_predicate: &str) {}

#[cfg(test)]
pub(crate) fn reset_predicate_typing_work() {
    PREDICATE_TYPING_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_predicate_typing_work() -> usize {
    PREDICATE_TYPING_WORK.with(|work| work.replace(0))
}

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
        |ty| matches!(ty, Type::Named { name, args, .. } if name == "Bool" && args.is_empty()),
    )
}

pub(crate) fn predicate_is_boolean_with_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<bool> {
    (!remainders.is_empty()).then(|| {
        predicate_type_with_remainders(predicate, remainders, bindings, call_type)
            == Some(Type::bool())
    })
}

pub(crate) fn predicate_type_with_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let chains = flat_remainder_chain_texts(remainders);
    predicate_type_with_flat_remainder_chains(predicate, &chains, bindings, call_type)
}

fn predicate_type_with_flat_remainder_chains(
    predicate: &str,
    chains: &[String],
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    record_predicate_typing_work(predicate);
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if flat_remainder_chain_text_matches(predicate, chains) {
        return Some(Type::int());
    }
    if let Some(ty) = predicate_literal_type(predicate) {
        return Some(ty);
    }
    if let Some(rest) = predicate.strip_prefix('-')
        && let Some(ty) =
            predicate_type_with_flat_remainder_chains(rest, chains, bindings, call_type)
        && matches!(ty, Type::Named { ref name, ref args, .. } if args.is_empty() && (name == "Int" || name == "Float"))
    {
        return Some(ty);
    }
    if let Some(rest) = predicate.strip_prefix('~')
        && predicate_type_with_flat_remainder_chains(rest, chains, bindings, call_type)
            == Some(Type::int())
    {
        return Some(Type::int());
    }
    if let Some(rest) = predicate.strip_prefix("not ")
        && predicate_type_with_flat_remainder_chains(rest, chains, bindings, call_type)
            == Some(Type::bool())
    {
        return Some(Type::bool());
    }
    if let Some(inner) = predicate
        .strip_prefix("not(")
        .and_then(|inner| inner.strip_suffix(')'))
        && predicate_type_with_flat_remainder_chains(inner, chains, bindings, call_type)
            == Some(Type::bool())
    {
        return Some(Type::bool());
    }
    for operator in ["or", "and"] {
        let clauses = split_top_level_keyword(predicate, operator);
        if clauses.len() > 1 {
            return clauses
                .into_iter()
                .all(|clause| {
                    predicate_type_with_flat_remainder_chains(clause, chains, bindings, call_type)
                        == Some(Type::bool())
                })
                .then(Type::bool);
        }
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left =
                predicate_type_with_flat_remainder_chains(left, chains, bindings, call_type)?;
            let right =
                predicate_type_with_flat_remainder_chains(right, chains, bindings, call_type)?;
            return comparable_predicate_operands(&left, &right).then(Type::bool);
        }
    }
    if contains_binary_bitwise_operator(predicate) {
        for operators in [&["|"][..], &["^"][..], &["&"][..], &[">>>", ">>", "<<"][..]] {
            for operator in operators {
                if let Some((left, right)) = split_top_level_operator(predicate, operator) {
                    let left = predicate_type_with_flat_remainder_chains(
                        left, chains, bindings, call_type,
                    )?;
                    let right = predicate_type_with_flat_remainder_chains(
                        right, chains, bindings, call_type,
                    )?;
                    return (left == Type::int() && right == Type::int()).then(Type::int);
                }
            }
        }
    }
    for operators in [&["+", "-"][..], &["*", "/", "%"][..]] {
        if let Some((left, operator, right)) = split_top_level_operator_group(predicate, operators)
        {
            let left =
                predicate_type_with_flat_remainder_chains(left, chains, bindings, call_type)?;
            let right =
                predicate_type_with_flat_remainder_chains(right, chains, bindings, call_type)?;
            if operator == "%" {
                return (left == Type::int() && right == Type::int()).then(Type::int);
            }
            return numeric_result_type(&left, &right);
        }
    }
    predicate_field_access_type(predicate, bindings, call_type)
        .or_else(|| predicate_contract_call_type(predicate, call_type))
        .or_else(|| predicate_binding_type(predicate, bindings))
}

fn flat_remainder_chain_texts(remainders: &[PredicateRemainder]) -> Vec<String> {
    let mut chains = Vec::new();
    let mut current = String::new();
    for remainder in remainders {
        if !remainder.left_is_prior_remainder {
            if !current.is_empty() {
                chains.push(current);
            }
            current = remainder
                .left_text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
        } else if current.is_empty() {
            continue;
        }
        current.push('%');
        current.extend(
            remainder
                .right_text
                .chars()
                .filter(|character| !character.is_whitespace()),
        );
    }
    if !current.is_empty() {
        chains.push(current);
    }
    chains
}

fn flat_remainder_chain_text_matches(predicate: &str, chains: &[String]) -> bool {
    let compact = strip_balanced_outer_parens(predicate.trim())
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    chains.iter().any(|chain| chain == &compact)
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
    record_predicate_typing_work(predicate);
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
        return matches!(ty, Type::Named { ref name, ref args, .. } if args.is_empty() && (name == "Int" || name == "Float"))
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
    for operators in [&["+", "-"][..], &["*", "/", "%"][..]] {
        if let Some((left, operator, right)) = split_top_level_operator_group(predicate, operators)
        {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            if operator == "%" {
                return Some(Type::int());
            }
            return numeric_result_type(&left, &right);
        }
    }
    None
}

pub(super) fn split_top_level_operator_group<'a, 'b>(
    predicate: &'a str,
    operators: &'b [&'b str],
) -> Option<(&'a str, &'b str, &'a str)> {
    operators
        .iter()
        .filter_map(|operator| {
            split_top_level_operator(predicate, operator)
                .map(|(left, right)| (left, *operator, right))
        })
        .max_by_key(|(left, _, _)| left.len())
}

pub(crate) fn predicate_literal_is_zero(text: &str) -> bool {
    let text = strip_balanced_outer_parens(text.trim());
    if let Ok(literal) = parse_integer_literal(text) {
        return literal.value == 0;
    }
    text.strip_prefix('-')
        .is_some_and(predicate_literal_is_zero)
}

pub(crate) fn predicate_has_invalid_remainder_operands(
    remainders: &[PredicateRemainder],
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> bool {
    remainders.iter().any(|remainder| {
        let left_invalid = !remainder.left_is_prior_remainder
            && predicate_remainder_operand_type(&remainder.left_text, bindings, call_type)
                .is_some_and(|ty| ty != Type::int() && ty != Type::Unknown);
        let right_invalid =
            predicate_remainder_operand_type(&remainder.right_text, bindings, call_type)
                .is_some_and(|ty| ty != Type::int() && ty != Type::Unknown);
        left_invalid || right_invalid
    })
}

pub(crate) fn predicate_remainder_operand_type(
    operand: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    predicate_type_with_calls(operand, bindings, call_type).or_else(|| {
        let operand = strip_balanced_outer_parens(operand.trim());
        let inner = operand
            .strip_prefix('-')
            .or_else(|| operand.strip_prefix('~'))?;
        predicate_type_with_calls(inner, bindings, call_type)
    })
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
    crate::predicate_text::split_top_level_operator_where(
        predicate,
        operator,
        contract_operator_at,
        |_, _| true,
    )
}

pub(super) fn contract_operator_at(text: &str, index: usize, operator: &str) -> bool {
    if !text[index..].starts_with(operator) {
        return false;
    }
    let next = text[index + operator.len()..].chars().next();
    let previous = text[..index].chars().next_back();
    match operator {
        "-" => text[..index]
            .chars()
            .rev()
            .find(|character| !character.is_whitespace())
            .is_some_and(|character| {
                !matches!(
                    character,
                    '(' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '&' | '|' | '^'
                )
            }),
        ">" => previous != Some('>') && !matches!(next, Some('>' | '=')),
        ">>" | ">>>" => previous != Some('>') && next != Some('>'),
        "<" => previous != Some('<') && !matches!(next, Some('<' | '=')),
        "<<" => previous != Some('<') && next != Some('<'),
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
    crate::predicate_text::split_top_level_keyword_raw(predicate, keyword)
        .into_iter()
        .map(str::trim)
        .collect()
}

pub(super) fn is_keyword_boundary(text: &str, start: usize, end: usize) -> bool {
    crate::predicate_text::is_identifier_word_boundary(text, start, end)
}
