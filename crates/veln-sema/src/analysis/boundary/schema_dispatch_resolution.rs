use super::*;

pub(super) fn resolve_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    resolve_schema_payload(
        module,
        schema,
        field,
        SchemaPayloadKind::Dispatch { tag },
        payload_name,
        diagnostics,
    )
}

pub(super) fn schema_dispatch_case_payload_name(payload: &SchemaDispatchCasePayload) -> &str {
    match payload {
        SchemaDispatchCasePayload::Primitive { .. } => "<primitive>",
        SchemaDispatchCasePayload::ReservedBits { .. } => "<reserved>",
        SchemaDispatchCasePayload::Schema { schema_name } => schema_name,
    }
}

pub(super) fn schema_field_declared_after(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
) -> bool {
    let reference = reference.split('.').next().unwrap_or(reference);
    let current_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.node_id == field.node_id);
    let reference_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.name == reference);
    matches!((current_index, reference_index), (Some(current), Some(reference)) if reference > current)
}

pub(super) fn add_compatible_prior_int_field_related(
    diagnostic: &mut Diagnostic,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    role: &str,
) {
    let expected_type = Type::int();
    let int_field = |field: &&SchemaField| decoded_fields.get(&field.name) == Some(&expected_type);
    let Some(candidate_field) = schema
        .fields
        .iter()
        .find(|field| int_field(field) && field.name.contains(role))
        .or_else(|| schema.fields.iter().find(int_field))
    else {
        return;
    };
    let candidate_name = &candidate_field.name;
    diagnostic.related.push(JsonValue::object([
        ("span", span_json(&candidate_field.span)),
        (
            "message",
            JsonValue::string(format!(
                "Compatible earlier {role} field `{candidate_name}` is declared here."
            )),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    (
                        "name",
                        JsonValue::string(
                            schema.name.as_deref().unwrap_or("<missing>").to_string(),
                        ),
                    ),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string(candidate_name.clone())),
                ]),
            ]),
        ),
    ]));
}

pub(super) fn schema_dispatch_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    role: &'static str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string(role)));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

pub(super) fn schema_dispatch_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("case_tag", JsonValue::Number(tag)));
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

pub(super) fn incompatible_schema_dispatch_payload_diagnostic(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    helper_availability: SchemaHelperAvailability,
) -> Diagnostic {
    let payload_schema_name = schema_payload_name_last_segment(payload_name);
    let decode_helper = schema_decode_step_function_name(payload_schema_name);
    let encode_helper = schema_encode_function_name(payload_schema_name);
    let unsupported_blocker =
        unsupported_dispatch_payload_helper_blocker(module, payload_schema, helper_availability);
    let mut diagnostic = schema_dispatch_payload_diagnostic(
        schema,
        field,
        tag,
        payload_name,
        "incompatible_payload_schema",
        format!(
            "dispatch payload schema `{payload_schema_name}` is outside the generated binary schema helper slice"
        ),
        [
            (
                "expected_decode_helper",
                JsonValue::string(decode_helper.clone()),
            ),
            (
                "decode_helper_boundary",
                JsonValue::string("generated_binary_schema_decode_step"),
            ),
            (
                "expected_encode_helper",
                JsonValue::string(encode_helper.clone()),
            ),
            (
                "encode_helper_boundary",
                JsonValue::string("generated_binary_schema_encode"),
            ),
        ],
    );
    diagnostic.details =
        add_dispatch_payload_helper_unavailable_details(diagnostic.details, helper_availability);
    if let Some(unsupported_blocker) = &unsupported_blocker {
        diagnostic.details = add_dispatch_payload_unsupported_blocker_details(
            diagnostic.details,
            unsupported_blocker,
        );
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_declaration")),
        (
            "message",
            JsonValue::string(dispatch_payload_schema_declaration_message(
                payload_schema_name,
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
        ("span", span_json(&payload_schema.span)),
    ]));
    if let Some(unsupported_blocker) = unsupported_blocker {
        diagnostic
            .related
            .push(dispatch_payload_unsupported_blocker_related(
                &unsupported_blocker,
            ));
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("helper_boundary")),
        (
            "message",
            JsonValue::string(dispatch_payload_helper_boundary_message(
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
    ]));
    diagnostic
}

#[derive(Clone, Copy)]
pub(super) struct SchemaHelperAvailability {
    pub(super) decode: bool,
    pub(super) encode: bool,
}

pub(super) fn schema_dispatch_payload_helper_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema_payload_has_generalized_reserved_byte_prefix(payload_schema) {
        diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
            module,
            schema,
            field,
            tag,
            payload_name,
            payload_schema,
            SchemaHelperAvailability {
                decode: false,
                encode: false,
            },
        ));
        return None;
    }
    let decode_type = schema_decode_value_type(module, payload_schema);
    match decode_type {
        Some(payload_ty) => Some(payload_ty),
        None => {
            let encode_type = schema_encode_value_type(module, payload_schema);
            diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
                module,
                schema,
                field,
                tag,
                payload_name,
                payload_schema,
                SchemaHelperAvailability {
                    decode: false,
                    encode: encode_type.is_some(),
                },
            ));
            None
        }
    }
}

pub(super) fn dispatch_payload_schema_declaration_message(
    payload_schema_name: &str,
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (false, true) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (true, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{encode_helper}` helper required for dispatch payload encoding."
        ),
        (true, true) => format!(
            "Schema `{payload_schema_name}` is declared here and exposes the generated dispatch payload helpers."
        ),
    }
}

pub(super) fn dispatch_payload_helper_boundary_message(
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) | (true, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them, and generated encode helpers before parent encode helpers can use them; expected `{decode_helper}` and `{encode_helper}`."
        ),
        (false, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them; expected `{decode_helper}`."
        ),
        (true, false) => format!(
            "Dispatch payload schemas must expose generated encode helpers before parent encode helpers can use them; expected `{encode_helper}`."
        ),
    }
}

pub(super) struct UnsupportedDispatchPayloadHelperField<'a> {
    pub(super) schema_name: String,
    pub(super) field: &'a SchemaField,
    pub(super) field_path_display: String,
    pub(super) layout_fact: String,
    pub(super) reason: &'static str,
}

pub(super) enum UnsupportedDispatchPayloadHelperBlocker<'a> {
    Field(UnsupportedDispatchPayloadHelperField<'a>),
}
