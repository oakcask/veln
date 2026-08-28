use super::*;

pub(super) fn resolve_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "invalid_payload_name",
            format!("repeat payload schema `{payload_name}` is not a valid schema path"),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => {
            resolve_local_schema_repeat_payload_schema(module, schema, field, name, diagnostics)
        }
        [_, .., name] => {
            let Some(use_decl) = normal_imported_use_for_path(
                module,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    payload_name,
                    "unknown_import",
                    format!("repeat payload schema `{payload_name}` is not declared"),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_repeat_payload_schema(
                module,
                schema,
                field,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

pub(super) fn resolve_local_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((index, candidate)) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    }) {
        return repeat_payload_candidate(
            SchemaRepeatPayloadContext { schema, field },
            name,
            current_index,
            index,
            candidate,
            diagnostics,
        );
    }
    if let Some(kind) = codec_schema_wrong_kind(module, schema.module_name.as_deref(), name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "non_schema_payload",
            format!("repeat payload `{name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "unknown_payload_schema",
            format!("repeat payload schema `{name}` is not declared"),
            [],
        ));
    }
    None
}

#[derive(Clone, Copy)]
struct SchemaRepeatPayloadContext<'a> {
    schema: &'a SchemaDecl,
    field: &'a SchemaField,
}

fn repeat_payload_candidate<'a>(
    context: SchemaRepeatPayloadContext<'_>,
    name: &str,
    current_index: usize,
    candidate_index: usize,
    candidate: &'a SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    if candidate_index == current_index {
        diagnostics.push(schema_repeat_payload_diagnostic(
            context.schema,
            context.field,
            name,
            "self_payload_schema",
            format!("repeat payload schema `{name}` cannot reference itself"),
            [],
        ));
        return None;
    }
    if candidate_index > current_index {
        diagnostics.push(schema_repeat_payload_diagnostic(
            context.schema,
            context.field,
            name,
            "forward_payload_schema",
            format!(
                "repeat payload schema `{name}` must be declared before schema `{}`",
                context.schema.name.as_deref().unwrap_or("<missing>")
            ),
            [],
        ));
        return None;
    }
    if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        diagnostics.push(schema_repeat_payload_diagnostic(
            context.schema,
            context.field,
            name,
            "non_binary_payload_schema",
            format!("repeat payload schema `{name}` must use `format binary`"),
            [],
        ));
        return None;
    }
    Some(candidate)
}

pub(super) fn resolve_imported_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        if candidate.visibility != Visibility::Public
            && !companion_private_schema_access_allowed(module, schema, use_decl)
        {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "private_imported_payload_schema",
                format!("imported repeat payload schema `{payload_name}` is private"),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "non_binary_payload_schema",
                format!("repeat payload schema `{payload_name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "non_schema_payload",
            format!("repeat payload `{payload_name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "unknown_payload_schema",
            format!("repeat payload schema `{payload_name}` is not declared"),
            [],
        ));
    }
    None
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

pub(super) fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .chain(module.schemas.iter().filter_map(|schema| {
            companion_access_target(schema.span.file.as_str(), schema.module_name.as_deref())
        }))
        .collect()
}

pub(super) fn companion_access_target(
    path: &str,
    module_name: Option<&str>,
) -> Option<(String, String)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((companion_module, target_module))
}

pub(super) fn schema_repeat_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("count")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
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
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("length")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.byte_view_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
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
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("multiple")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.byte_view_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
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
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("predicate")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.field_predicate_reference",
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
