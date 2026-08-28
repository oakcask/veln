use super::*;

pub(super) fn push_schema_type_reference_diagnostics(
    module: &SurfaceModule,
    current_module: Option<&str>,
    annotation: &str,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(ty) = parse_type_annotation(annotation) else {
        return;
    };
    let mut schemas = Vec::new();
    collect_schema_type_references(module, current_module, &ty, &mut schemas);
    for schema in schemas {
        diagnostics.push(schema_type_reference_diagnostic(
            schema,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
    let mut primitives = Vec::new();
    collect_exact_width_schema_primitive_references(&ty, &mut primitives);
    for primitive in primitives {
        diagnostics.push(exact_width_schema_primitive_diagnostic(
            primitive,
            None,
            None,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
    let mut lowercase_primitives = Vec::new();
    collect_lowercase_schema_primitive_references(&ty, &mut lowercase_primitives);
    for primitive in lowercase_primitives {
        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            primitive,
            None,
            None,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
}

pub(super) fn collect_schema_type_references<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    ty: &Type,
    schemas: &mut Vec<&'a SchemaDecl>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(schema) = schema_for_type_name(module, current_module, name) {
                schemas.push(schema);
            }
            for arg in args {
                collect_schema_type_references(module, current_module, arg, schemas);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_schema_type_references(module, current_module, field_ty, schemas);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_schema_type_references(module, current_module, param, schemas);
            }
            collect_schema_type_references(module, current_module, return_type, schemas);
        }
        Type::Unknown => {}
    }
}

pub(super) fn collect_exact_width_schema_primitive_references<'a>(
    ty: &'a Type,
    primitives: &mut Vec<&'a str>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(primitive) = exact_width_binary_primitive_name(name) {
                primitives.push(primitive);
            }
            for arg in args {
                collect_exact_width_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_exact_width_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_exact_width_schema_primitive_references(param, primitives);
            }
            collect_exact_width_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

pub(super) fn collect_lowercase_schema_primitive_references<'a>(
    ty: &'a Type,
    primitives: &mut Vec<&'a str>,
) {
    match ty {
        Type::Named { name, args } => {
            if lowercase_schema_primitive(name).is_some() {
                primitives.push(name);
            }
            for arg in args {
                collect_lowercase_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_lowercase_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_lowercase_schema_primitive_references(param, primitives);
            }
            collect_lowercase_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

pub(in crate::analysis) fn lowercase_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: LowercaseSchemaPrimitiveError,
) -> Diagnostic {
    let reason_text = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => "missing_width",
        LowercaseSchemaPrimitiveError::UnknownEndian => "unknown_endian",
        LowercaseSchemaPrimitiveError::MissingEndian => "missing_endian",
        LowercaseSchemaPrimitiveError::RedundantEndian => "redundant_endian",
        LowercaseSchemaPrimitiveError::UnsupportedWidth => "unsupported_width",
        LowercaseSchemaPrimitiveError::ReservesValue => "reserves_value",
    };
    let message = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => {
            format!("binary schema primitive `{primitive}` must specify a width")
        }
        LowercaseSchemaPrimitiveError::UnknownEndian => {
            format!(
                "binary schema primitive `{primitive}` must end with `be` or `le` when it specifies byte order"
            )
        }
        LowercaseSchemaPrimitiveError::MissingEndian => {
            format!("binary schema primitive `{primitive}` requires byte order suffix `be` or `le`")
        }
        LowercaseSchemaPrimitiveError::RedundantEndian => {
            format!("binary schema primitive `{primitive}` must not specify byte order")
        }
        LowercaseSchemaPrimitiveError::UnsupportedWidth => {
            format!("binary schema primitive `{primitive}` uses an unsupported width")
        }
        LowercaseSchemaPrimitiveError::ReservesValue => {
            format!(
                "binary schema primitive `{primitive}` requires `reserves` value to be a literal non-negative integer"
            )
        }
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive,
        schema,
        field,
        node_id,
        span,
        reason_text,
        message,
    )
}

pub(in crate::analysis) fn lowercase_schema_primitive_position_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let message = match reason {
        "repeat_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in `Repeat` payload positions"
        ),
        "dispatch_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in dispatch payload positions"
        ),
        _ => format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive, schema, field, node_id, span, reason, message,
    )
}

pub(super) fn lowercase_schema_primitive_diagnostic_with_message(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
    message: String,
) -> Diagnostic {
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        ("primitive", JsonValue::string(primitive.to_string())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(schema) = schema {
        details.push((
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ));
    }
    if let Some(field) = field {
        details.push(("field", JsonValue::string(field.name.clone())));
    }
    Diagnostic::new(
        "schema.lowercase_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        JsonValue::object(details),
    )
}

pub(super) fn schema_for_type_name<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => module.schemas.iter().find(|schema| {
            schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == current_module
        }),
        [_, .., name] => {
            let module_name = normal_imported_module_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.schemas.iter().find(|schema| {
                schema.name.as_deref() == Some(name)
                    && schema.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

pub(super) fn schema_type_reference_diagnostic(
    schema: &SchemaDecl,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "type.schema_reference",
        Severity::Error,
        DiagnosticKind::Type,
        format!("schema `{schema_name}` cannot be used as an ordinary type"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("type")),
            ("node_id", JsonValue::string(node_id)),
            ("schema", JsonValue::string(schema_name)),
            ("use_kind", JsonValue::string(use_kind)),
        ]),
    )
}

pub(in crate::analysis) fn exact_width_binary_primitive_name(name: &str) -> Option<&'static str> {
    EXACT_WIDTH_BINARY_PRIMITIVES
        .iter()
        .copied()
        .find(|primitive| *primitive == name)
}

const EXACT_WIDTH_BINARY_PRIMITIVES: &[&str] = &[
    "UInt1", "UInt2", "UInt3", "UInt4", "UInt5", "UInt6", "UInt7", "UInt8", "UInt16be", "UInt16le",
    "UInt24be", "UInt24le", "UInt31be", "UInt31le", "UInt32be", "UInt32le", "UInt40be", "UInt40le",
    "UInt48be", "UInt48le", "UInt56be", "UInt56le", "UInt64be", "UInt64le",
];

pub(in crate::analysis) fn exact_width_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        ("primitive", JsonValue::string(primitive.to_string())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(schema) = schema {
        details.push((
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ));
    }
    if let Some(field) = field {
        details.push(("field", JsonValue::string(field.name.clone())));
    }
    Diagnostic::new(
        "schema.exact_width_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
        Some(span),
        JsonValue::object(details),
    )
}

pub(super) fn reserved_bits_primitive(
    ty: &str,
) -> Option<Result<(i64, i64), ReservedBitsArgumentReason>> {
    let rest = ty.strip_prefix("ReservedBits")?;
    if starts_as_longer_identifier(rest) {
        return None;
    }
    reserved_bits_arguments(rest.trim()).map(|result| {
        result.and_then(|args| {
            parse_reserved_bits_pair(args[0], args[1])
                .map_err(|()| ReservedBitsArgumentReason::Literal)
        })
    })
}

fn starts_as_longer_identifier(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn reserved_bits_arguments(text: &str) -> Option<Result<Vec<&str>, ReservedBitsArgumentReason>> {
    if text.is_empty() {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    if !text.starts_with('(') {
        return None;
    }
    if !text.ends_with(')') {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    let args = text[1..text.len() - 1]
        .trim()
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    if args.len() == 2 {
        Some(Ok(args))
    } else {
        Some(Err(ReservedBitsArgumentReason::Arity))
    }
}

fn parse_reserved_bits_pair(width: &str, value: &str) -> Result<(i64, i64), ()> {
    Ok((
        parse_reserved_bits_integer(width)?,
        parse_reserved_bits_integer(value)?,
    ))
}

pub(super) fn parse_reserved_bits_integer(text: &str) -> Result<i64, ()> {
    parse_integer_literal(text)
        .map(|literal| literal.value)
        .map_err(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReservedBitsArgumentReason {
    Arity,
    Literal,
}

pub(super) fn reserved_bits_format_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        "`ReservedBits` can only be used in a `format binary` schema field",
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string("non_binary_format")),
        ]),
    )
}

pub(super) fn reserved_bits_argument_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: ReservedBitsArgumentReason,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let reason_text = match reason {
        ReservedBitsArgumentReason::Arity => "argument_count",
        ReservedBitsArgumentReason::Literal => "non_literal_argument",
    };
    let message = match reason {
        ReservedBitsArgumentReason::Arity => {
            "`ReservedBits` requires width and value integer arguments"
        }
        ReservedBitsArgumentReason::Literal => {
            "`ReservedBits` arguments must be literal non-negative integers"
        }
    };
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string(reason_text)),
        ]),
    )
}

pub(super) fn reserved_bits_encode_shape_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_index: Option<usize>,
    reserved: (i64, i64),
) -> Diagnostic {
    let layout = reserved_bits_unsupported_layout_context(schema, field_index, reserved.0);
    let mut details = vec![
        (
            "schema",
            JsonValue::string(schema.name.clone().unwrap_or_default()),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("primitive", JsonValue::string("ReservedBits")),
        ("bit_width", JsonValue::Number(reserved.0)),
        ("expected_value", JsonValue::Number(reserved.1)),
        ("reason", JsonValue::string("unsupported_encode_shape")),
        (
            "supported_layout_family",
            JsonValue::string(layout.supported_layout_family),
        ),
    ];
    if let Some(previous_width) = layout.previous_visible_bit_width {
        details.push((
            "previous_visible_bit_width",
            JsonValue::Number(i64::from(previous_width)),
        ));
    }
    if let Some(next_width) = layout.next_visible_bit_width {
        details.push((
            "next_visible_bit_width",
            JsonValue::Number(i64::from(next_width)),
        ));
    }

    let mut diagnostic = Diagnostic::new(
        "schema.reserved_bits_encode",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "`ReservedBits({}, {})` is outside the supported binary schema field layouts",
            reserved.0, reserved.1
        ),
        Some(field.span.clone()),
        JsonValue::object(details),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Schema `{}` field `{}` declares ReservedBits({}, {}).",
            schema.name.clone().unwrap_or_default(),
            field.name,
            reserved.0,
            reserved.1
        )),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(layout.human_adjacent_note),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Supported layout family: {}; {}",
            layout.supported_layout_family, layout.human_supported_note
        )),
    )]));
    diagnostic
}

pub(super) struct ReservedBitsUnsupportedLayoutContext {
    pub(super) supported_layout_family: &'static str,
    pub(super) previous_visible_bit_width: Option<u8>,
    pub(super) next_visible_bit_width: Option<u8>,
    pub(super) human_adjacent_note: String,
    pub(super) human_supported_note: &'static str,
}

pub(super) fn reserved_bits_unsupported_layout_context(
    schema: &SchemaDecl,
    field_index: Option<usize>,
    bit_width: i64,
) -> ReservedBitsUnsupportedLayoutContext {
    let previous_field = field_index
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| schema.fields.get(index));
    let previous_previous_field = field_index
        .and_then(|index| index.checked_sub(2))
        .and_then(|index| schema.fields.get(index));
    let next_field = field_index.and_then(|index| schema.fields.get(index + 1));
    let previous_previous_visible_bit_width =
        previous_previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let previous_visible_bit_width =
        previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let next_visible_bit_width =
        next_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));

    let (supported_layout_family, human_supported_note) = reserved_bits_supported_layout_family(
        bit_width,
        previous_previous_visible_bit_width,
        previous_visible_bit_width,
        next_visible_bit_width,
    );
    ReservedBitsUnsupportedLayoutContext {
        supported_layout_family,
        previous_visible_bit_width,
        next_visible_bit_width,
        human_adjacent_note: reserved_bits_adjacent_width_note(
            previous_field,
            previous_visible_bit_width,
            next_field,
            next_visible_bit_width,
        ),
        human_supported_note,
    }
}

pub(super) fn reserved_bits_supported_layout_family(
    bit_width: i64,
    previous_previous_visible_bit_width: Option<u8>,
    previous_visible_bit_width: Option<u8>,
    next_visible_bit_width: Option<u8>,
) -> (&'static str, &'static str) {
    if previous_visible_bit_width.is_some() && next_visible_bit_width.is_some() {
        return (
            "middle_reserved_bits",
            "visible and reserved widths must complete one supported big-endian storage unit.",
        );
    }
    if previous_visible_bit_width.is_some()
        && suffix_packed_reserved_storage_bit_width(bit_width).is_some()
    {
        if previous_visible_bit_width == Some(8)
            && previous_previous_visible_bit_width
                .is_some_and(|width| i64::from(width) + 8 + bit_width == 16)
            && (1..=7).contains(&bit_width)
        {
            return (
                "suffix_reserved_group",
                "two visible widths plus the reserved width must complete the same two-byte big-endian storage unit.",
            );
        }
        return (
            "packed_reserved_suffix",
            "the previous visible width plus the reserved width must complete one supported big-endian storage unit.",
        );
    }
    if next_visible_bit_width.is_some() && packed_reserved_storage_bit_width(bit_width).is_some() {
        return (
            "packed_reserved_prefix",
            "the reserved width plus the next visible width must complete one supported big-endian storage unit.",
        );
    }
    if bit_width > 0 && bit_width <= 32 && bit_width % 8 == 0 {
        return (
            "byte_aligned_reserved_bits",
            "byte-aligned reserved fields are supported up to four bytes when the value fits the width.",
        );
    }
    (
        "bit_packed_reserved_group",
        "a bit-packed group must contain at least one visible field and complete one supported big-endian storage unit.",
    )
}

pub(super) fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

pub(super) fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else {
            None
        }
    })
}

pub(super) fn reserved_bits_adjacent_width_note(
    previous_field: Option<&SchemaField>,
    previous_visible_bit_width: Option<u8>,
    next_field: Option<&SchemaField>,
    next_visible_bit_width: Option<u8>,
) -> String {
    let mut parts = Vec::new();
    if let (Some(field), Some(width)) = (previous_field, previous_visible_bit_width) {
        parts.push(format!("previous `{}` is {} bit(s)", field.name, width));
    }
    if let (Some(field), Some(width)) = (next_field, next_visible_bit_width) {
        parts.push(format!("next `{}` is {} bit(s)", field.name, width));
    }
    if parts.is_empty() {
        "No adjacent visible exact-width field participates in this unsupported layout.".to_string()
    } else {
        format!("Adjacent visible field widths: {}.", parts.join("; "))
    }
}
