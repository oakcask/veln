use veln_ast::ContractKind;

use crate::types::{Binding, Type};

pub(crate) enum ContractValidation {
    Valid,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
    MissingField { base_type: String, field: String },
}

pub(crate) struct ContractCall {
    pub(crate) callee: String,
    pub(crate) args: Vec<String>,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
    }
}

pub(crate) fn predicate_is_statically_true(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::True
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StaticBooleanValue {
    True,
    False,
    Unknown,
}

fn static_boolean_value(predicate: &str) -> StaticBooleanValue {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return StaticBooleanValue::Unknown;
    }
    if predicate == "true" {
        return StaticBooleanValue::True;
    }
    if predicate == "false" {
        return StaticBooleanValue::False;
    }
    if let Some(rest) = predicate.strip_prefix("not ") {
        return static_boolean_value(rest).negate();
    }
    if let Some(rest) = predicate.strip_prefix("not(") {
        let Some(inner) = rest.strip_suffix(')') else {
            return StaticBooleanValue::Unknown;
        };
        return static_boolean_value(inner).negate();
    }
    if has_complementary_top_level_clauses(predicate, "or") {
        return StaticBooleanValue::True;
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "or") {
        if complementary_predicates(left, right) {
            return StaticBooleanValue::True;
        }
        return static_boolean_value(left).or(static_boolean_value(right));
    }
    if has_complementary_top_level_clauses(predicate, "and") {
        return StaticBooleanValue::False;
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "and") {
        if complementary_predicates(left, right) {
            return StaticBooleanValue::False;
        }
        return static_boolean_value(left).and(static_boolean_value(right));
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            return static_literal_comparison(left, operator, right)
                .or_else(|| static_same_shape_comparison(left, operator, right))
                .map_or(StaticBooleanValue::Unknown, StaticBooleanValue::from);
        }
    }
    StaticBooleanValue::Unknown
}

fn complementary_predicates(left: &str, right: &str) -> bool {
    negated_predicate_shape(left).is_some_and(|left| left == predicate_shape(right))
        || negated_predicate_shape(right).is_some_and(|right| right == predicate_shape(left))
}

fn has_complementary_top_level_clauses(predicate: &str, keyword: &str) -> bool {
    let clauses = split_top_level_keyword(predicate, keyword);
    if clauses.len() <= 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, left)| {
        clauses
            .iter()
            .skip(index + 1)
            .any(|right| complementary_predicates(left, right))
    })
}

fn negated_predicate_shape(predicate: &str) -> Option<String> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(rest) = predicate.strip_prefix("not ") {
        return Some(predicate_shape(rest));
    }
    if let Some(rest) = predicate.strip_prefix("not(") {
        return rest.strip_suffix(')').map(predicate_shape);
    }
    None
}

fn predicate_shape(predicate: &str) -> String {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    let mut shape = String::new();
    let mut chars = predicate.chars();
    let mut in_string = false;
    let mut escaped = false;
    let mut pending_space = false;
    while let Some(ch) = chars.next() {
        if in_string {
            shape.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            if pending_space && !shape.is_empty() {
                shape.push(' ');
            }
            pending_space = false;
            in_string = true;
            shape.push(ch);
            continue;
        }
        if ch.is_whitespace() {
            pending_space = !shape.is_empty();
            continue;
        }
        if pending_space {
            shape.push(' ');
            pending_space = false;
        }
        shape.push(ch);
    }
    shape
}

fn static_same_shape_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    if compact_predicate_text(left) != compact_predicate_text(right) {
        return None;
    }
    match operator {
        "==" | "<=" | ">=" => Some(true),
        "!=" | "<" | ">" => Some(false),
        _ => None,
    }
}

fn compact_predicate_text(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut chars = predicate.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            output.push(ch);
            let mut escaped = false;
            for string_ch in chars.by_ref() {
                output.push(string_ch);
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if !ch.is_whitespace() {
            output.push(ch);
        }
    }
    output
}

impl StaticBooleanValue {
    fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            (Self::False, value) | (value, Self::False) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            (Self::True, value) | (value, Self::True) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }
}

impl From<bool> for StaticBooleanValue {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum StaticLiteral {
    Bool(bool),
    Number(StaticNumber),
    String(String),
}

fn static_literal_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    let left = StaticLiteral::parse(left.trim())?;
    let right = StaticLiteral::parse(right.trim())?;
    match (left, right) {
        (StaticLiteral::Bool(left), StaticLiteral::Bool(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        (StaticLiteral::Number(left), StaticLiteral::Number(right)) => Some(match operator {
            "==" => left == right,
            "!=" => left != right,
            "<" => left < right,
            "<=" => left <= right,
            ">" => left > right,
            ">=" => left >= right,
            _ => return None,
        }),
        (StaticLiteral::String(left), StaticLiteral::String(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        _ => None,
    }
}

impl StaticLiteral {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "true" => return Some(Self::Bool(true)),
            "false" => return Some(Self::Bool(false)),
            _ => {}
        }
        if let Some(number) = StaticNumber::parse(text) {
            return Some(Self::Number(number));
        }
        parse_static_string_literal(text).map(Self::String)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct StaticNumber {
    mantissa: i128,
    scale: u32,
}

impl Ord for StaticNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.mantissa.is_negative(), other.mantissa.is_negative()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        let ordering = self.abs_cmp(other);
        if self.mantissa.is_negative() {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

impl PartialOrd for StaticNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl StaticNumber {
    fn parse(text: &str) -> Option<Self> {
        let (negative, digits) = text
            .strip_prefix('-')
            .map_or((false, text), |digits| (true, digits.trim_start()));
        if digits.is_empty() {
            return None;
        }
        let (integer, fraction) = digits.split_once('.').map_or((digits, ""), |parts| parts);
        if integer.is_empty()
            || !integer.chars().all(|ch| ch.is_ascii_digit())
            || !fraction.chars().all(|ch| ch.is_ascii_digit())
            || (digits.contains('.') && fraction.is_empty())
        {
            return None;
        }
        let mut scale = fraction.len() as u32;
        let signed_digits = if negative {
            format!("-{integer}{fraction}")
        } else {
            format!("{integer}{fraction}")
        };
        let mut mantissa = signed_digits.parse::<i128>().ok()?;
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Some(Self { mantissa, scale })
    }

    fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (left_integer, left_fraction) = self.abs_parts();
        let (right_integer, right_fraction) = other.abs_parts();
        left_integer
            .len()
            .cmp(&right_integer.len())
            .then_with(|| left_integer.cmp(&right_integer))
            .then_with(|| {
                let scale = left_fraction.len().max(right_fraction.len());
                let mut left_fraction = left_fraction;
                let mut right_fraction = right_fraction;
                left_fraction.extend(std::iter::repeat('0').take(scale - left_fraction.len()));
                right_fraction.extend(std::iter::repeat('0').take(scale - right_fraction.len()));
                left_fraction.cmp(&right_fraction)
            })
    }

    fn abs_parts(&self) -> (String, String) {
        let mut digits = self.mantissa.unsigned_abs().to_string();
        if self.scale == 0 {
            return (digits, String::new());
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let padding = "0".repeat(scale + 1 - digits.len());
            digits = format!("{padding}{digits}");
        }
        let split = digits.len() - scale;
        let integer = digits[..split].trim_start_matches('0');
        let integer = if integer.is_empty() { "0" } else { integer };
        (integer.to_string(), digits[split..].to_string())
    }
}

fn parse_static_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            value.push(chars.next()?);
        } else if ch == '"' {
            return None;
        } else {
            value.push(ch);
        }
    }
    Some(value)
}

pub(crate) fn contract_calls(predicate: &str) -> Vec<ContractCall> {
    let bytes = predicate.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        if bytes[index] != b'('
            || index == 0
            || !predicate[..index].trim_end().ends_with_identifier()
        {
            index += 1;
            continue;
        }
        let Some(callee_start) = callee_start(predicate, index) else {
            index += 1;
            continue;
        };
        let Some(close) = matching_close(predicate, index) else {
            index += 1;
            continue;
        };
        calls.push(ContractCall {
            callee: predicate[callee_start..index].trim().to_string(),
            args: split_call_args(&predicate[index + 1..close]),
            start: callee_start,
            end: close + 1,
        });
        index += 1;
    }
    calls
}

trait EndsWithIdentifier {
    fn ends_with_identifier(&self) -> bool;
}

impl EndsWithIdentifier for str {
    fn ends_with_identifier(&self) -> bool {
        self.chars()
            .rev()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }
}

fn callee_start(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut index = open;
    while index > 0 {
        let ch = bytes[index - 1] as char;
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            index -= 1;
        } else {
            break;
        }
    }
    (index < open).then_some(index)
}

fn matching_close(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_call_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg.to_string());
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg.to_string());
    }
    args
}

pub(crate) fn referenced_names(predicate: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = predicate.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let ch = bytes[index] as char;
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            if start >= 2 && &predicate[start - 2..start] == "::" {
                continue;
            }
            if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
                continue;
            }
            if start >= 1 && &predicate[start - 1..start] == "." {
                continue;
            }
            let name = predicate[start..index].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        } else {
            index += 1;
        }
    }
    names
}

pub(crate) fn is_contract_keyword(name: &str) -> bool {
    matches!(name, "and" | "or" | "not")
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
    if matches!(predicate, "true" | "false") {
        return Some(Type::bool());
    }
    if predicate == "()" {
        return Some(Type::unit());
    }
    if is_complete_string_literal(predicate) {
        return Some(Type::string());
    }
    if predicate.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(Type::int());
    }
    if is_float_literal(predicate) {
        return Some(Type::float());
    }
    if let Some(rest) = predicate.strip_prefix("not ") {
        return (predicate_type_with_calls(rest, bindings, call_type)? == Type::bool())
            .then(Type::bool);
    }
    if let Some(rest) = predicate.strip_prefix("not(") {
        let inner = rest.strip_suffix(')')?;
        return (predicate_type_with_calls(inner, bindings, call_type)? == Type::bool())
            .then(Type::bool);
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        let ty = predicate_type_with_calls(rest, bindings, call_type)?;
        return matches!(ty, Type::Named { ref name, ref args } if args.is_empty() && (name == "Int" || name == "Float"))
            .then_some(ty);
    }
    for operator in ["or", "and"] {
        if let Some((left, right)) = split_top_level_keyword_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return (left == Type::bool() && right == Type::bool()).then(Type::bool);
        }
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return comparable_predicate_operands(&left, &right).then(Type::bool);
        }
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return numeric_result_type(&left, &right);
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return numeric_result_type(&left, &right);
        }
    }
    if let Some(access) = split_field_access(predicate) {
        let mut current = predicate_type_with_calls(access.base, bindings, call_type)?;
        for field in access.fields {
            current = current.record_field(field)?.clone();
        }
        return Some(current);
    }
    if let [call] = contract_calls(predicate).as_slice()
        && call.start == 0
        && call.end == predicate.len()
    {
        return call_type(&call.callee);
    }
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

fn comparable_predicate_operands(left: &Type, right: &Type) -> bool {
    left == right || (is_numeric_type(left) && is_numeric_type(right))
}

fn numeric_result_type(left: &Type, right: &Type) -> Option<Type> {
    if !is_numeric_type(left) || !is_numeric_type(right) {
        return None;
    }
    if left == &Type::float() || right == &Type::float() {
        Some(Type::float())
    } else {
        Some(Type::int())
    }
}

fn is_numeric_type(ty: &Type) -> bool {
    ty == &Type::int() || ty == &Type::float()
}

fn is_float_literal(text: &str) -> bool {
    let Some((left, right)) = text.split_once('.') else {
        return false;
    };
    !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|ch| ch.is_ascii_digit())
        && right.chars().all(|ch| ch.is_ascii_digit())
}

fn split_top_level_operator<'a>(predicate: &'a str, operator: &str) -> Option<(&'a str, &'a str)> {
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
            _ if depth == 0 && predicate[index..].starts_with(operator) => {
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

fn split_top_level_keyword_operator<'a>(
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

fn split_top_level_keyword<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
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

fn is_keyword_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.map_or(true, |ch| !is_ident_continue(ch))
        && after.map_or(true, |ch| !is_ident_continue(ch))
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn strip_balanced_outer_parens(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let mut depth = 0usize;
        let mut balanced_outer = true;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index != trimmed.len() - 1 {
                        balanced_outer = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !balanced_outer || depth != 0 {
            return trimmed;
        }
        trimmed = trimmed[1..trimmed.len() - 1].trim();
    }
}

struct FieldAccess {
    base: String,
    fields: Vec<String>,
}

struct FieldAccessRef<'a> {
    base: &'a str,
    fields: Vec<&'a str>,
}

fn split_field_access(predicate: &str) -> Option<FieldAccessRef<'_>> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut first_dot = None;
    let mut fields = Vec::new();
    let mut index = 0usize;
    while index < predicate.len() {
        let ch = predicate[index..].chars().next()?;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                index += ch.len_utf8();
            }
            '(' => {
                depth += 1;
                index += ch.len_utf8();
            }
            ')' => {
                depth = depth.saturating_sub(1);
                index += ch.len_utf8();
            }
            '.' if depth == 0 => {
                let field_start = index + ch.len_utf8();
                let Some(field_first) = predicate[field_start..].chars().next() else {
                    return None;
                };
                if !(field_first.is_ascii_alphabetic() || field_first == '_') {
                    return None;
                }
                let mut field_end = field_start + field_first.len_utf8();
                while field_end < predicate.len() {
                    let next = predicate[field_end..].chars().next()?;
                    if next.is_ascii_alphanumeric() || next == '_' {
                        field_end += next.len_utf8();
                    } else {
                        break;
                    }
                }
                first_dot.get_or_insert(index);
                fields.push(&predicate[field_start..field_end]);
                index = field_end;
                let rest = predicate[index..].trim_start();
                if rest.is_empty() {
                    break;
                }
                if !rest.starts_with('.') {
                    return None;
                }
            }
            _ => index += ch.len_utf8(),
        }
    }
    let dot = first_dot?;
    let base = predicate[..dot].trim();
    (!base.is_empty() && !fields.is_empty()).then_some(FieldAccessRef { base, fields })
}

fn field_accesses(predicate: &str) -> Vec<FieldAccess> {
    let bytes = predicate.as_bytes();
    let mut accesses = Vec::new();
    for call in contract_calls(predicate) {
        if let Some(fields) = field_suffix(&predicate[call.end..]) {
            accesses.push(FieldAccess {
                base: predicate[call.start..call.end].to_string(),
                fields,
            });
        }
    }
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let ch = bytes[index] as char;
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() {
            let ch = bytes[index] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                index += 1;
            } else {
                break;
            }
        }
        if start >= 1 && &predicate[start - 1..start] == "." {
            continue;
        }
        if start >= 2 && &predicate[start - 2..start] == "::" {
            continue;
        }
        if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
            continue;
        }
        let base = predicate[start..index].to_string();
        let mut fields = Vec::new();
        while index < bytes.len() && &predicate[index..index + 1] == "." {
            let field_start = index + 1;
            if field_start >= bytes.len() {
                break;
            }
            let first = bytes[field_start] as char;
            if !(first.is_ascii_alphabetic() || first == '_') {
                break;
            }
            index = field_start + 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            fields.push(predicate[field_start..index].to_string());
        }
        if !fields.is_empty() {
            accesses.push(FieldAccess { base, fields });
        }
    }
    accesses
}

fn field_suffix(text: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut rest = text.trim_start();
    while let Some(after_dot) = rest.strip_prefix('.') {
        let mut chars = after_dot.char_indices();
        let (_, first) = chars.next()?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return None;
        }
        let mut end = first.len_utf8();
        for (index, ch) in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end = index + ch.len_utf8();
            } else {
                break;
            }
        }
        fields.push(after_dot[..end].to_string());
        rest = after_dot[end..].trim_start();
    }
    (!fields.is_empty()).then_some(fields)
}

fn is_complete_string_literal(text: &str) -> bool {
    if !text.starts_with('"') {
        return false;
    }
    string_literal_end(text, 0).is_some_and(|end| end == text.len())
}

fn string_literal_end(text: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut cursor = start + 1;
    while cursor < text.len() {
        let ch = text[cursor..].chars().next()?;
        cursor += ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(cursor);
        }
    }
    None
}
