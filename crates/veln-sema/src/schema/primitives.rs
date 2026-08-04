use std::collections::BTreeMap;

use veln_literals::parse_integer_literal;

use crate::semantic_model::Type;

pub(crate) fn schema_payload_name_path(text: &str) -> Option<Vec<String>> {
    let segments = text.split("::").map(str::trim).collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !is_schema_identifier(segment))
    {
        return None;
    }
    Some(segments.into_iter().map(str::to_string).collect())
}

pub(crate) fn schema_payload_name_is_path(text: &str) -> bool {
    schema_payload_name_path(text).is_some()
}

pub(crate) fn schema_payload_name_last_segment(text: &str) -> &str {
    text.rsplit("::").next().unwrap_or(text)
}

pub(crate) fn exact_width_schema_primitive(ty: &str) -> Option<u8> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive(name),
        None => match ty.trim() {
            "UInt1" | "UInt2" | "UInt3" | "UInt4" | "UInt5" | "UInt6" | "UInt7" => Some(1),
            "UInt8" => Some(1),
            "UInt16be" | "UInt16le" => Some(2),
            "UInt24be" | "UInt24le" => Some(3),
            "UInt31be" | "UInt31le" | "UInt32be" | "UInt32le" => Some(4),
            "UInt40be" | "UInt40le" => Some(5),
            "UInt48be" | "UInt48le" => Some(6),
            "UInt56be" | "UInt56le" => Some(7),
            "UInt64be" | "UInt64le" => Some(8),
            _ => None,
        },
    }
}

pub(crate) fn exact_width_schema_primitive_little_endian(ty: &str) -> bool {
    let canonical = canonical_schema_primitive_name(ty);
    let name = canonical.as_deref().unwrap_or_else(|| ty.trim());
    matches!(
        name,
        "UInt16le"
            | "UInt24le"
            | "UInt31le"
            | "UInt32le"
            | "UInt40le"
            | "UInt48le"
            | "UInt56le"
            | "UInt64le"
    )
}

pub(crate) fn exact_width_schema_primitive_bit_width(ty: &str) -> Option<u8> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive_bit_width(name),
        None => match ty.trim() {
            "UInt1" => Some(1),
            "UInt2" => Some(2),
            "UInt3" => Some(3),
            "UInt4" => Some(4),
            "UInt5" => Some(5),
            "UInt6" => Some(6),
            "UInt7" => Some(7),
            "UInt8" => Some(8),
            "UInt16be" | "UInt16le" => Some(16),
            "UInt24be" | "UInt24le" => Some(24),
            "UInt31be" | "UInt31le" => Some(31),
            "UInt32be" | "UInt32le" => Some(32),
            "UInt40be" | "UInt40le" => Some(40),
            "UInt48be" | "UInt48le" => Some(48),
            "UInt56be" | "UInt56le" => Some(56),
            "UInt64be" | "UInt64le" => Some(64),
            _ => None,
        },
    }
}

pub(crate) fn exact_width_schema_primitive_max_value(ty: &str) -> Option<i64> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive_max_value(name),
        None => match ty.trim() {
            "UInt1" => Some(0x1),
            "UInt2" => Some(0x3),
            "UInt3" => Some(0x7),
            "UInt4" => Some(0xf),
            "UInt5" => Some(0x1f),
            "UInt6" => Some(0x3f),
            "UInt7" => Some(0x7f),
            "UInt8" => Some(0xff),
            "UInt16be" | "UInt16le" => Some(0xffff),
            "UInt24be" | "UInt24le" => Some(0xffffff),
            "UInt31be" | "UInt31le" => Some(0x7fffffff),
            "UInt32be" | "UInt32le" => Some(0xffffffff),
            "UInt40be" | "UInt40le" => Some(0xffffffffff),
            "UInt48be" | "UInt48le" => Some(0xffffffffffff),
            "UInt56be" | "UInt56le" => Some(0xffffffffffffff),
            "UInt64be" | "UInt64le" => Some(i64::MAX),
            _ => None,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LowercaseSchemaPrimitive {
    pub(crate) spelling: String,
    pub(crate) family: &'static str,
    pub(crate) width_bits: u16,
    pub(crate) endian: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LowercaseSchemaPrimitiveError {
    MissingWidth,
    UnknownEndian,
    MissingEndian,
    RedundantEndian,
    UnsupportedWidth,
    ReservesValue,
}

impl LowercaseSchemaPrimitive {
    pub(crate) fn canonical_name(&self) -> String {
        let family = match self.family {
            "uint" => "UInt",
            _ => unreachable!("schema primitive family is fixed"),
        };
        match self.endian {
            Some(endian) => format!("{family}{}{endian}", self.width_bits),
            None => format!("{family}{}", self.width_bits),
        }
    }
}

pub(crate) fn lowercase_schema_primitive(
    text: &str,
) -> Option<Result<LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError>> {
    let spelling = text.trim();
    let rest = spelling.strip_prefix("uint")?;
    let family = "uint";
    if rest.is_empty() {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingWidth));
    }
    if !rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return match rest {
            "be" | "le" => Some(Err(LowercaseSchemaPrimitiveError::MissingWidth)),
            _ => None,
        };
    }
    let width_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if width_len == 0 {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingWidth));
    }
    let width_text = &rest[..width_len];
    let suffix = &rest[width_len..];
    let Ok(width_bits) = width_text.parse::<u16>() else {
        return Some(Err(LowercaseSchemaPrimitiveError::UnsupportedWidth));
    };
    let endian = match suffix {
        "" => None,
        "be" => Some("be"),
        "le" => Some("le"),
        _ => return Some(Err(LowercaseSchemaPrimitiveError::UnknownEndian)),
    };
    let supported_width = matches!(
        width_bits,
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 16 | 24 | 31 | 32 | 40 | 48 | 56 | 64
    );
    if !supported_width {
        return Some(Err(LowercaseSchemaPrimitiveError::UnsupportedWidth));
    }
    if width_bits <= 8 && endian.is_some() {
        return Some(Err(LowercaseSchemaPrimitiveError::RedundantEndian));
    }
    if width_bits > 8 && endian.is_none() {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingEndian));
    }
    Some(Ok(LowercaseSchemaPrimitive {
        spelling: spelling.to_string(),
        family,
        width_bits,
        endian,
    }))
}

pub(crate) fn lowercase_reserved_bits_schema_primitive(
    text: &str,
) -> Option<Result<(i64, i64), LowercaseSchemaPrimitiveError>> {
    let spelling = text.trim();
    let mut parts = spelling.split_whitespace();
    let primitive_text = parts.next()?;
    if parts.next()? != "reserves" {
        return None;
    }
    let Some(value_text) = parts.next() else {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    };
    if parts.next().is_some() {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    }
    let primitive = match lowercase_schema_primitive(primitive_text)? {
        Ok(primitive) => primitive,
        Err(reason) => return Some(Err(reason)),
    };
    let Some(value) = parse_reserved_bits_integer(value_text) else {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    };
    Some(Ok((i64::from(primitive.width_bits), value)))
}

pub(crate) fn canonical_schema_primitive_name(text: &str) -> Option<String> {
    match lowercase_schema_primitive(text)? {
        Ok(primitive) => Some(primitive.canonical_name()),
        Err(_) => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewLengthExpr {
    Field(String),
    Sum { left: String, right: String },
    Difference { left: String, right: String },
    Product { left: String, right: String },
    Quotient { left: String, right: String },
}

impl ByteViewLengthExpr {
    pub(crate) fn references(&self) -> Vec<&str> {
        match self {
            Self::Field(field) => vec![field.as_str()],
            Self::Sum { left, right } => vec![left.as_str(), right.as_str()],
            Self::Difference { left, right } => vec![left.as_str(), right.as_str()],
            Self::Product { left, right } => vec![left.as_str(), right.as_str()],
            Self::Quotient { left, right } => vec![left.as_str(), right.as_str()],
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Sum { left, right } => format!("{left} + {right}"),
            Self::Difference { left, right } => format!("{left} - {right}"),
            Self::Product { left, right } => format!("{left} * {right}"),
            Self::Quotient { left, right } => format!("{left} / {right}"),
        }
    }
}

pub(crate) fn schema_length_expression(text: &str) -> Option<ByteViewLengthExpr> {
    schema_length_expression_with_product(text, true)
}

fn schema_length_expression_with_product(
    text: &str,
    allow_product: bool,
) -> Option<ByteViewLengthExpr> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(ByteViewLengthExpr::Field(text.to_string()));
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(ByteViewLengthExpr::Sum {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(ByteViewLengthExpr::Difference {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(ByteViewLengthExpr::Quotient {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if !allow_product {
        return None;
    }
    let (left, right) = schema_length_binary_expression_operands(text, '*')?;
    Some(ByteViewLengthExpr::Product {
        left: left.to_string(),
        right: right.to_string(),
    })
}

pub(crate) fn schema_length_expression_references(text: &str) -> Option<Vec<&str>> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(vec![text]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '*') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(vec![left, right]);
    }
    None
}

fn schema_length_binary_expression_operands(text: &str, op: char) -> Option<(&str, &str)> {
    for other_op in ['+', '-', '*', '/'] {
        if other_op != op && text.contains(other_op) {
            return None;
        }
    }
    let (left, right) = text.split_once(op)?;
    if right.contains(op) {
        return None;
    }
    let left = left.trim();
    let right = right.trim();
    if is_simple_schema_field_reference(left) && is_simple_schema_field_reference(right) {
        Some((left, right))
    } else {
        None
    }
}

pub(crate) fn byte_view_schema_primitive(ty: &str) -> Option<ByteViewLengthExpr> {
    let text = ty.trim();
    let inner = text.strip_prefix("ByteView(")?.strip_suffix(')')?.trim();
    schema_length_expression_with_product(inner, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewMultipleConstraint {
    Field(String),
    Literal(i64),
}

impl ByteViewMultipleConstraint {
    pub(crate) fn reference(&self) -> Option<&str> {
        match self {
            Self::Field(field) => Some(field.as_str()),
            Self::Literal(_) => None,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Literal(value) => value.to_string(),
        }
    }
}

pub(crate) fn byte_view_multiple_constraint(predicate: &str) -> Option<ByteViewMultipleConstraint> {
    let divisor = predicate
        .trim()
        .strip_prefix("payload_count multiple of ")?
        .trim();
    if divisor.is_empty() || divisor.contains(char::is_whitespace) {
        return None;
    }
    if let Ok(literal) = parse_integer_literal(divisor) {
        return (literal.value > 0).then_some(ByteViewMultipleConstraint::Literal(literal.value));
    }
    is_simple_schema_field_reference(divisor)
        .then(|| ByteViewMultipleConstraint::Field(divisor.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaRepeatSpec {
    pub(crate) count_field: String,
    pub(crate) payload: SchemaRepeatPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaRepeatPayload {
    Primitive {
        width: u8,
        max_value: i64,
        little_endian: bool,
    },
    ReservedBits {
        bit_width: u8,
        expected_value: i64,
    },
    ByteView {
        length_field: String,
    },
    Schema {
        schema_name: String,
    },
}

pub(crate) fn repeat_schema_primitive(ty: &str) -> Option<SchemaRepeatSpec> {
    if let Some((payload, count_field)) = canonical_repeat_schema_primitive_parts(ty) {
        return repeat_schema_primitive_from_parts(count_field, payload);
    }
    let inner = schema_call_inner(ty, "Repeat")?;
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [count_field, primitive] = args.as_slice() else {
        return None;
    };
    repeat_schema_primitive_from_parts(count_field, primitive)
}

fn repeat_schema_primitive_from_parts(
    count_field: &str,
    primitive: &str,
) -> Option<SchemaRepeatSpec> {
    let count_expr = schema_length_expression(count_field)?;
    let payload = if let Some(width) = exact_width_schema_primitive(primitive) {
        if exact_width_schema_primitive_bit_width(primitive)? < 8 {
            return None;
        }
        SchemaRepeatPayload::Primitive {
            width,
            max_value: exact_width_schema_primitive_max_value(primitive)?,
            little_endian: exact_width_schema_primitive_little_endian(primitive),
        }
    } else if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(primitive) {
        SchemaRepeatPayload::ReservedBits {
            bit_width: dispatch_reserved_bits_width(bit_width, expected_value)?,
            expected_value,
        }
    } else if let Some(length_expr) = byte_view_schema_primitive(primitive) {
        match length_expr {
            ByteViewLengthExpr::Field(_)
            | ByteViewLengthExpr::Sum { .. }
            | ByteViewLengthExpr::Difference { .. } => SchemaRepeatPayload::ByteView {
                length_field: length_expr.render(),
            },
            ByteViewLengthExpr::Product { .. } | ByteViewLengthExpr::Quotient { .. } => {
                return None;
            }
        }
    } else if schema_payload_name_path(primitive).is_some() {
        SchemaRepeatPayload::Schema {
            schema_name: (*primitive).to_string(),
        }
    } else {
        return None;
    };
    Some(SchemaRepeatSpec {
        count_field: count_expr.render(),
        payload,
    })
}

pub(crate) fn schema_repeat_payload_accepts_lowercase_primitive(text: &str) -> bool {
    (lowercase_schema_primitive(text).is_some()
        || lowercase_reserved_bits_schema_primitive(text).is_some())
        && repeat_schema_primitive_from_parts("count", text).is_some()
}

fn canonical_repeat_schema_primitive_parts(ty: &str) -> Option<(&str, &str)> {
    let text = ty.trim();
    let inner = text.strip_prefix('[')?.strip_suffix(']')?.trim();
    let (payload, count) = split_top_level_once(inner, ';')?;
    if count.contains(';') {
        return None;
    }
    let payload = payload.trim();
    let count = count.trim();
    if payload.is_empty() || count.is_empty() {
        return None;
    }
    Some((payload, count))
}

fn split_top_level_once(text: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => {
                return Some((&text[..index], &text[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn is_simple_schema_field_reference(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(is_schema_identifier)
}

pub(crate) fn schema_field_reference_type<'a>(
    fields: &'a BTreeMap<String, Type>,
    reference: &str,
) -> Option<&'a Type> {
    let mut segments = reference.split('.');
    let mut ty = fields.get(segments.next()?)?;
    for segment in segments {
        let Type::Record(record_fields) = ty else {
            return None;
        };
        ty = record_fields
            .iter()
            .find_map(|(name, ty)| (name == segment).then_some(ty))?;
    }
    Some(ty)
}

pub(crate) fn reserved_bits_schema_primitive(ty: &str) -> Option<(i64, i64)> {
    if let Some(reserved) = lowercase_reserved_bits_schema_primitive(ty) {
        return reserved.ok();
    }
    let rest = ty.strip_prefix("ReservedBits")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let inner = rest[1..rest.len() - 1].trim();
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [width, value] = args.as_slice() else {
        return None;
    };
    let width = parse_reserved_bits_integer(width)?;
    let value = parse_reserved_bits_integer(value)?;
    Some((width, value))
}

pub(super) fn canonical_schema_primitive_is(ty: &str, expected: &str) -> bool {
    canonical_schema_primitive_name(ty)
        .as_deref()
        .unwrap_or_else(|| ty.trim())
        == expected
}

pub(super) fn parse_reserved_bits_integer(text: &str) -> Option<i64> {
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}

pub(super) fn dispatch_reserved_bits_width(bit_width: i64, expected_value: i64) -> Option<u8> {
    if bit_width <= 0 || bit_width > 32 {
        return None;
    }
    if !(1..=7).contains(&bit_width) && bit_width % 8 != 0 {
        return None;
    }
    let max_value = if bit_width == 32 {
        0xffffffff
    } else {
        (1_i64 << bit_width) - 1
    };
    (expected_value <= max_value).then_some(bit_width as u8)
}

pub(super) fn schema_call_inner<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let rest = ty.trim().strip_prefix(name)?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    rest.strip_prefix('(')?.strip_suffix(')')
}

pub(super) fn is_schema_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
