use super::*;

pub(super) fn resolve_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    resolve_schema_payload(
        module,
        schema,
        field,
        SchemaPayloadKind::Repeat,
        payload_name,
        diagnostics,
    )
}

pub(super) fn companion_private_schema_access_allowed(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    use_decl: &UseDecl,
) -> bool {
    use_decl.package.is_none()
        && schema.module_name.as_deref().is_some_and(|current_module| {
            companion_access_targets(module)
                .get(current_module)
                .is_some_and(|allowed| allowed == use_decl.name.as_str())
        })
}

pub(super) fn companion_schema_access_target(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<String> {
    let current_module = schema.module_name.as_deref()?;
    companion_access_targets(module)
        .get(current_module)
        .cloned()
}

pub(super) fn schema_repeat_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    schema_field_reference_diagnostic(
        schema,
        field,
        reference,
        reason,
        message,
        extra,
        ("schema.repeat_reference", "count"),
    )
}

pub(super) fn schema_byte_view_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    schema_field_reference_diagnostic(
        schema,
        field,
        reference,
        reason,
        message,
        extra,
        ("schema.byte_view_reference", "length"),
    )
}

pub(super) fn schema_byte_view_multiple_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    schema_field_reference_diagnostic(
        schema,
        field,
        reference,
        reason,
        message,
        extra,
        ("schema.byte_view_reference", "multiple"),
    )
}

pub(super) fn schema_validation_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = vec![
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
        ),
        ("reason", JsonValue::string(reason)),
        ("reference", JsonValue::string(reference.to_string())),
    ];
    fields.extend(extra);
    Diagnostic::new(
        "schema.validation_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(validation.span.clone()),
        JsonValue::object(fields),
    )
}

pub(super) fn schema_field_predicate_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    schema_field_reference_diagnostic(
        schema,
        field,
        reference,
        reason,
        message,
        extra,
        ("schema.field_predicate_reference", "predicate"),
    )
}

fn schema_field_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
    identity: (&'static str, &'static str),
) -> Diagnostic {
    let (id, role) = identity;
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string(role)));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

pub(super) fn schema_validation_duplicate_diagnostic(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
) -> Diagnostic {
    Diagnostic::new(
        "schema.validation_duplicate",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema `{}` can declare only one schema-level validation",
            schema.name.as_deref().unwrap_or("<missing>")
        ),
        Some(validation.span.clone()),
        JsonValue::object([
            (
                "schema",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
            ),
            ("reason", JsonValue::string("duplicate_validation")),
        ]),
    )
}

pub(super) fn schema_repeat_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}
