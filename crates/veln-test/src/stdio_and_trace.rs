use super::*;

pub(super) fn collect_stdio_call_spans(
    expr: &Expr,
    spans: &mut BTreeMap<(String, String), SourceSpan>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if is_stdio_callee(callee) {
                spans.insert(
                    (
                        expr.span.file.as_str().to_string(),
                        expr.node_id.display("call"),
                    ),
                    expr.span.clone(),
                );
            }
            collect_stdio_call_spans(callee, spans);
            for arg in args {
                collect_stdio_call_spans(arg, spans);
            }
        }
        ExprKind::TypeApply { callee, .. } => collect_stdio_call_spans(callee, spans),
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_stdio_call_spans(arg, spans);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_stdio_call_spans(body, spans);
            for arg in args {
                collect_stdio_call_spans(arg, spans);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_stdio_call_spans(input, spans);
            collect_stdio_call_spans(base, spans);
        }
        ExprKind::SchemaEncode { value, .. } => collect_stdio_call_spans(value, spans),
        ExprKind::FieldAccess { base, .. } => collect_stdio_call_spans(base, spans),
        ExprKind::Try(inner) => collect_stdio_call_spans(inner, spans),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_stdio_call_spans(&field.expr, spans);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_stdio_call_spans(&entry.key, spans);
                collect_stdio_call_spans(&entry.value, spans);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_stdio_call_spans(item, spans);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_stdio_call_spans(scrutinee, spans);
            for arm in arms {
                collect_stdio_call_spans(&arm.expr, spans);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_stdio_call_spans(condition, spans);
            collect_stdio_call_spans(then_branch, spans);
            for branch in else_if_branches {
                collect_stdio_call_spans(&branch.condition, spans);
                collect_stdio_call_spans(&branch.expr, spans);
            }
            collect_stdio_call_spans(else_branch, spans);
        }
        ExprKind::Prefix { expr, .. } => collect_stdio_call_spans(expr, spans),
        ExprKind::Binary { left, right, .. } => {
            collect_stdio_call_spans(left, spans);
            collect_stdio_call_spans(right, spans);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

pub(super) fn is_stdio_callee(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::NamePath(segments) => matches!(
            segments.as_slice(),
            [module, name]
                if module == "stdio"
                    && matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln")
        ),
        ExprKind::TypeApply { callee, .. } => is_stdio_callee(callee),
        _ => false,
    }
}

pub(super) fn stdio_event_from_trace_line(
    line: &str,
    call_spans: &BTreeMap<(String, String), SourceSpan>,
    fallback_source: &TestCaseSource,
) -> Option<JsonValue> {
    let mut fields = line.splitn(7, '\t');
    let sequence = fields.next()?.parse::<usize>().ok()?;
    let stream = fields.next()?;
    let operation = fields.next()?;
    let terminator = fields.next()?;
    let node_id = fields.next()?;
    let source_file = fields.next()?;
    let text = decode_hex_text(fields.next()?)?;
    let node_id = if node_id.is_empty() {
        fallback_source.node_id.as_str()
    } else {
        node_id
    };
    let source_file = if source_file.is_empty() {
        fallback_source.file.as_str()
    } else {
        source_file
    };
    let span = call_spans
        .get(&(source_file.to_string(), node_id.to_string()))
        .unwrap_or(&fallback_source.span);
    Some(stdio_event(
        stream, operation, &text, terminator, sequence, node_id, span,
    ))
}

pub(super) fn decode_hex_text(hex: &str) -> Option<String> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut chars = hex.chars();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        bytes.push((hex_digit(high)? << 4) | hex_digit(low)?);
    }
    String::from_utf8(bytes).ok()
}

pub(super) fn contract_failure_from_trace_line(line: &str) -> Option<TestFailure> {
    let mut fields = line.split('\t');
    if fields.next()? != "contract" {
        return None;
    }
    let clause = fields.next()?.to_string();
    let predicate = decode_hex_text(fields.next()?)?;
    let function = decode_hex_text(fields.next()?)?;
    let blame = fields.next()?.to_string();
    let node_id = decode_hex_text(fields.next()?)?;
    let source_file = decode_hex_text(fields.next()?)?;
    let start_line = fields.next()?.parse::<usize>().ok()?;
    let start_column = fields.next()?.parse::<usize>().ok()?;
    let end_line = fields.next()?.parse::<usize>().ok()?;
    let end_column = fields.next()?.parse::<usize>().ok()?;
    let span = SourceSpan {
        file: SourcePath::new(source_file),
        start: LineCol {
            line: start_line,
            column: start_column,
            offset: 0,
        },
        end: LineCol {
            line: end_line,
            column: end_column,
            offset: 0,
        },
    };
    let message = format!("contract failure: {clause} `{predicate}` in `{function}` blame {blame}");
    Some(TestFailure::contract(
        message, clause, predicate, function, blame, node_id, span,
    ))
}

pub(super) fn result_failure_from_trace_line(line: &str) -> Option<TestFailure> {
    let mut fields = line.split('\t');
    if fields.next()? != "result" {
        return None;
    }
    let value = decode_hex_text(fields.next()?)?;
    let mut fixture_hex = None;
    let mut byte_diagnostic = None;
    let mut value_diagnostic = None;
    let mut protocol_diagnostic = None;
    while let Some(kind) = fields.next() {
        match kind {
            "fixture_hex" => fixture_hex = Some(fixture_hex_details(&mut fields)?),
            "byte_diagnostic" => byte_diagnostic = Some(byte_diagnostic_details(&mut fields)?),
            "byte_diagnostic_v2" => {
                byte_diagnostic = Some(byte_diagnostic_v2_details(&mut fields)?)
            }
            "value_diagnostic" => value_diagnostic = Some(value_diagnostic_details(&mut fields)?),
            "protocol_diagnostic" => {
                protocol_diagnostic = Some(protocol_diagnostic_details(&mut fields)?)
            }
            _ => return None,
        }
    }
    Some(TestFailure::result_with_extended_details(
        value,
        fixture_hex,
        byte_diagnostic,
        value_diagnostic,
        protocol_diagnostic,
    ))
}

pub(super) fn fixture_hex_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let text_start = fields.next()?.parse::<i64>().ok()?;
    let text_end = fields.next()?.parse::<i64>().ok()?;
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let nibble = fields.next()?.to_string();
    let context_start = fields.next()?.parse::<i64>().ok()?;
    let context_end = fields.next()?.parse::<i64>().ok()?;
    let context = decode_hex_text(fields.next()?)?;
    if fields.next().is_some() {
        return None;
    }
    Some(JsonValue::object([
        ("kind", JsonValue::string("fixture_hex")),
        ("id", JsonValue::string(id)),
        (
            "fixture_text_span",
            JsonValue::object([
                ("start", JsonValue::Number(text_start)),
                ("end", JsonValue::Number(text_end)),
            ]),
        ),
        ("byte_offset", byte_offset_value(byte_offset)),
        ("nibble_position", JsonValue::string(nibble)),
        (
            "nearby_context",
            JsonValue::object([
                ("start", JsonValue::Number(context_start)),
                ("end", JsonValue::Number(context_end)),
                ("text", JsonValue::string(context)),
            ]),
        ),
    ]))
}

pub(super) fn byte_diagnostic_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let (id, byte_offset, field_path) = byte_diagnostic_header(fields)?;
    let expected_count = fields.next()?.parse::<i64>().ok()?;
    let available_count = fields.next()?.parse::<i64>().ok()?;
    let readiness = fields.next()?.to_string();
    Some(JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string(id)),
        ("byte_offset", byte_offset_value(byte_offset)),
        ("field_path", JsonValue::array(field_path)),
        ("expected_count", JsonValue::Number(expected_count)),
        ("available_count", JsonValue::Number(available_count)),
        ("readiness", JsonValue::string(readiness)),
    ]))
}

pub(super) fn byte_diagnostic_v2_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let (id, byte_offset, field_path) = byte_diagnostic_header(fields)?;
    finish_dynamic_diagnostic(
        fields,
        vec![
            ("kind".to_string(), JsonValue::string("byte_diagnostic")),
            ("id".to_string(), JsonValue::string(id)),
            ("byte_offset".to_string(), byte_offset_value(byte_offset)),
            ("field_path".to_string(), JsonValue::array(field_path)),
        ],
    )
}

pub(super) fn byte_preview_value(encoded_hex_text: &str) -> Option<JsonValue> {
    let data = decode_hex_text(encoded_hex_text)?;
    if data.len() % 2 != 0
        || !data
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return None;
    }
    let preview_byte_count = (data.len() / 2) as i64;
    Some(JsonValue::object([
        ("encoding", JsonValue::string("hex")),
        ("data", JsonValue::string(data)),
        ("preview_byte_count", JsonValue::Number(preview_byte_count)),
        ("total_byte_count", JsonValue::Number(preview_byte_count)),
        ("truncated", JsonValue::Bool(false)),
    ]))
}

pub(super) fn byte_preview_v2_value(encoded_preview: &str) -> Option<JsonValue> {
    let mut fields = encoded_preview.split(':');
    let data = decode_hex_text(fields.next()?)?;
    let preview_byte_count = fields.next()?.parse::<i64>().ok()?;
    let total_byte_count = fields.next()?.parse::<i64>().ok()?;
    let truncated = match fields.next()? {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    if fields.next().is_some()
        || data.len() % 2 != 0
        || preview_byte_count < 0
        || total_byte_count < preview_byte_count
        || preview_byte_count != (data.len() / 2) as i64
        || !data
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return None;
    }
    Some(JsonValue::object([
        ("encoding", JsonValue::string("hex")),
        ("data", JsonValue::string(data)),
        ("preview_byte_count", JsonValue::Number(preview_byte_count)),
        ("total_byte_count", JsonValue::Number(total_byte_count)),
        ("truncated", JsonValue::Bool(truncated)),
    ]))
}

pub(super) fn protocol_diagnostic_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    finish_dynamic_diagnostic(
        fields,
        vec![
            ("kind".to_string(), JsonValue::string("protocol_diagnostic")),
            ("id".to_string(), JsonValue::string(id)),
            ("byte_offset".to_string(), byte_offset_value(byte_offset)),
        ],
    )
}

pub(super) fn value_diagnostic_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let field_path_count = fields.next()?.parse::<usize>().ok()?;
    let field_path = diagnostic_field_path(fields, field_path_count)?;
    finish_dynamic_diagnostic(
        fields,
        vec![
            ("kind".to_string(), JsonValue::string("value_diagnostic")),
            ("id".to_string(), JsonValue::string(id)),
            ("field_path".to_string(), JsonValue::array(field_path)),
        ],
    )
}

fn byte_diagnostic_header<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<(String, i64, Vec<JsonValue>)> {
    let id = fields.next()?.to_string();
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let field_path_count = fields.next()?.parse::<usize>().ok()?;
    let field_path = diagnostic_field_path(fields, field_path_count)?;
    Some((id, byte_offset, field_path))
}

fn finish_dynamic_diagnostic<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    mut entries: Vec<(String, JsonValue)>,
) -> Option<JsonValue> {
    let detail_count = fields.next()?.parse::<usize>().ok()?;
    append_diagnostic_details(fields, detail_count, &mut entries)?;
    Some(JsonValue::Object(entries))
}

fn diagnostic_field_path<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    count: usize,
) -> Option<Vec<JsonValue>> {
    let mut field_path = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = fields.next()?.to_string();
        let name = decode_hex_text(fields.next()?)?;
        field_path.push(JsonValue::object([
            ("kind", JsonValue::string(kind)),
            ("name", JsonValue::string(name)),
        ]));
    }
    Some(field_path)
}

fn byte_offset_value(byte_offset: i64) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("ByteOffset")),
        ("value", JsonValue::Number(byte_offset)),
    ])
}

fn append_diagnostic_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    count: usize,
    entries: &mut Vec<(String, JsonValue)>,
) -> Option<()> {
    for _ in 0..count {
        let key = fields.next()?.to_string();
        let value_kind = fields.next()?;
        let value = fields.next()?;
        let json_value = match value_kind {
            "number" => JsonValue::Number(value.parse::<i64>().ok()?),
            "string" => JsonValue::string(decode_hex_text(value)?),
            "byte_preview" => byte_preview_value(value)?,
            "byte_preview_v2" => byte_preview_v2_value(value)?,
            _ => return None,
        };
        entries.push((key, json_value));
    }
    Some(())
}

pub(super) fn hex_digit(character: char) -> Option<u8> {
    match character {
        '0'..='9' => Some(character as u8 - b'0'),
        'a'..='f' => Some(character as u8 - b'a' + 10),
        'A'..='F' => Some(character as u8 - b'A' + 10),
        _ => None,
    }
}

pub(super) fn stdio_event(
    stream: &str,
    operation: &str,
    text: &str,
    terminator: &str,
    sequence: usize,
    node_id: &str,
    span: &SourceSpan,
) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("stdio")),
        ("stream", JsonValue::string(stream)),
        ("operation", JsonValue::string(operation)),
        ("text", JsonValue::string(text)),
        ("terminator", JsonValue::string(terminator)),
        ("sequence", JsonValue::Number(sequence as i64)),
        ("node_id", JsonValue::string(node_id)),
        ("span", source_span_to_json(span)),
    ])
}
