use super::*;

pub(super) fn unsupported_dispatch_payload_helper_blocker<'a>(
    _module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    _helper_availability: SchemaHelperAvailability,
) -> Option<UnsupportedDispatchPayloadHelperBlocker<'a>> {
    unsupported_dispatch_payload_helper_field(schema)
        .map(UnsupportedDispatchPayloadHelperBlocker::Field)
}

pub(super) fn unsupported_dispatch_payload_helper_field(
    schema: &SchemaDecl,
) -> Option<UnsupportedDispatchPayloadHelperField<'_>> {
    let schema_name = schema.name.clone().unwrap_or_default();
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            if let Some(unsupported) = unsupported_reserved_bits_payload_field(
                schema,
                field,
                index,
                reserved,
                &schema_name,
            ) {
                return Some(unsupported);
            };
            continue;
        }
        if let Some(width) = exact_width_schema_primitive(&field.ty) {
            decoded_fields.insert(field.name.clone(), Type::int());
            if width == 0 {
                return None;
            }
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if let Some(unsupported) = unsupported_byte_view_payload_field(
                schema,
                field,
                &length_expr,
                &decoded_fields,
                &schema_name,
            ) {
                return Some(unsupported);
            }
            decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
            continue;
        }
    }
    None
}

fn unsupported_reserved_bits_payload_field<'a>(
    schema: &SchemaDecl,
    field: &'a SchemaField,
    index: usize,
    reserved: (i64, i64),
    schema_name: &str,
) -> Option<UnsupportedDispatchPayloadHelperField<'a>> {
    if schema_field_uses_generalized_reserved_byte_prefix(&schema.fields, index, reserved) {
        return Some(UnsupportedDispatchPayloadHelperField {
            field,
            field_path_display: format!("{schema_name}.{}", field.name),
            layout_fact: format!(
                "`ReservedBits({}, {})` uses the general direct byte-prefix rule, which is outside dispatch and repeat payload helpers",
                reserved.0, reserved.1
            ),
            reason: "unsupported_reserved_bits_layout",
            schema_name: schema_name.to_string(),
        });
    }
    if supported_encode_reserved_bits(&schema.fields, index, reserved).is_some() {
        return None;
    }
    let layout = reserved_bits_unsupported_layout_context(schema, Some(index), reserved.0);
    Some(UnsupportedDispatchPayloadHelperField {
        field,
        field_path_display: format!("{schema_name}.{}", field.name),
        layout_fact: format!(
            "`ReservedBits({}, {})` is outside the supported `{}` layout: {}",
            reserved.0, reserved.1, layout.supported_layout_family, layout.human_supported_note
        ),
        reason: "unsupported_reserved_bits_layout",
        schema_name: schema_name.to_string(),
    })
}

fn unsupported_byte_view_payload_field<'a>(
    schema: &SchemaDecl,
    field: &'a SchemaField,
    length_expr: &ByteViewLengthExpr,
    decoded_fields: &BTreeMap<String, Type>,
    schema_name: &str,
) -> Option<UnsupportedDispatchPayloadHelperField<'a>> {
    let reference = length_expr.references().into_iter().find(|reference| {
        schema_field_reference_type(decoded_fields, reference) != Some(&Type::int())
    })?;
    let reference_fact = byte_view_ineligible_length_fact(schema, field, decoded_fields, reference);
    Some(UnsupportedDispatchPayloadHelperField {
        field,
        field_path_display: format!("{schema_name}.{}", field.name),
        layout_fact: format!(
            "`ByteView({})` requires {reference_fact}",
            length_expr.render()
        ),
        reason: "ineligible_byte_view_length_reference",
        schema_name: schema_name.to_string(),
    })
}

pub(super) fn byte_view_ineligible_length_fact(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
) -> String {
    if let Some(actual) = schema_field_reference_type(decoded_fields, reference) {
        format!(
            "length reference `{reference}` to decode as `Int`; it decodes as `{}`",
            actual.render()
        )
    } else if schema_field_declared_after(schema, field, reference) {
        format!(
            "length reference `{reference}` to be declared before field `{}`",
            field.name
        )
    } else {
        format!("length reference `{reference}` to name an earlier decoded `Int` field")
    }
}

pub(super) fn add_dispatch_payload_helper_unavailable_details(
    details: JsonValue,
    helper_availability: SchemaHelperAvailability,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unavailable_helper_directions".to_string(),
        JsonValue::array(dispatch_payload_unavailable_helper_directions(
            helper_availability,
        )),
    ));
    JsonValue::Object(fields)
}

pub(super) fn dispatch_payload_unavailable_helper_directions(
    helper_availability: SchemaHelperAvailability,
) -> Vec<JsonValue> {
    let mut directions = Vec::new();
    if !helper_availability.decode {
        directions.push(JsonValue::string("decode"));
    }
    if !helper_availability.encode {
        directions.push(JsonValue::string("encode"));
    }
    directions
}

pub(super) fn add_dispatch_payload_unsupported_blocker_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            add_dispatch_payload_unsupported_field_details(details, field)
        }
    }
}

pub(super) fn add_dispatch_payload_unsupported_field_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unsupported_nested_schema".to_string(),
        JsonValue::string(unsupported.schema_name.clone()),
    ));
    fields.push((
        "unsupported_nested_field".to_string(),
        JsonValue::string(unsupported.field.name.clone()),
    ));
    fields.push((
        "unsupported_nested_field_path".to_string(),
        dispatch_payload_unsupported_field_path(unsupported),
    ));
    fields.push((
        "unsupported_nested_layout_reason".to_string(),
        JsonValue::string(unsupported.reason),
    ));
    fields.push((
        "unsupported_nested_layout_fact".to_string(),
        JsonValue::string(unsupported.layout_fact.clone()),
    ));
    JsonValue::Object(fields)
}

pub(super) fn dispatch_payload_unsupported_blocker_related(
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            dispatch_payload_unsupported_field_related(field)
        }
    }
}

pub(super) fn dispatch_payload_unsupported_field_related(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let layout_fact = unsupported.layout_fact.trim_end_matches('.');
    JsonValue::object([
        ("kind", JsonValue::string("unsupported_nested_field")),
        ("span", span_json(&unsupported.field.span)),
        (
            "field_path",
            dispatch_payload_unsupported_field_path(unsupported),
        ),
        (
            "message",
            JsonValue::string(format!(
                "Nested dispatch payload field `{}` prevents generated decode and encode helpers: {}.",
                unsupported.field_path_display, layout_fact
            )),
        ),
    ])
}

pub(super) fn dispatch_payload_unsupported_field_path(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            ("name", JsonValue::string(unsupported.schema_name.clone())),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(unsupported.field.name.clone())),
        ]),
    ])
}

pub(super) fn schema_dispatch_details(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: &'static str,
) -> Vec<(&'static str, JsonValue)> {
    vec![
        ("phase", JsonValue::string("schema")),
        (
            "node_id",
            JsonValue::string(field.node_id.display("schema-field")),
        ),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("field_path", schema_dispatch_field_path(schema, field)),
        ("reason", JsonValue::string(reason)),
    ]
}

pub(super) fn schema_dispatch_field_path(schema: &SchemaDecl, field: &SchemaField) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            (
                "name",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
            ),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(field.name.clone())),
        ]),
    ])
}
