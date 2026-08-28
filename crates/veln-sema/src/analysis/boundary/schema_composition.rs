use super::*;

pub(super) fn check_format_neutral_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    adts: &AdtRegistry,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let decode_field_type =
        format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty);
    if let Some(field_ty) = decode_field_type.clone() {
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
    } else if schema_composition_quarantine_is_sole_failure(module, schema, &field.ty) {
        return;
    } else if schema_payload_name_path(&field.ty).is_some()
        && !schema_field_has_ordinary_type_target(module, schema, &field.ty)
    {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            unresolved_schema_composition_reason(module, schema, &field.ty),
        ));
    } else {
        diagnostics.push(format_neutral_schema_helper_diagnostic(schema, field));
    }

    let encode_unsupported =
        format_neutral_schema_encode_field_type_for_schema(module, schema, adts, &field.ty)
            .is_none();
    let direct_source_adt_candidate =
        format_neutral_schema_encode_field_is_source_adt_candidate(&field.ty);
    let ordinary_or_non_path = schema_payload_name_path(&field.ty).is_none()
        || schema_field_has_ordinary_type_target(module, schema, &field.ty);
    if encode_unsupported
        && direct_source_adt_candidate
        && decode_field_type.is_none()
        && ordinary_or_non_path
    {
        diagnostics.push(format_neutral_schema_encode_helper_diagnostic(
            schema.name.as_deref().unwrap_or("<missing>"),
            &schema.span,
            field,
        ));
    }
}

pub(super) fn schema_composition_duplicate_binding_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Diagnostic {
    Diagnostic::new(
        "schema.composition_duplicate_binding",
        Severity::Error,
        DiagnosticKind::Name,
        format!("duplicate schema field binding `{}`", field.name),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            (
                "schema",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
            ),
            ("binding", JsonValue::string(field.name.clone())),
            ("reason", JsonValue::string("duplicate_binding")),
        ]),
    )
}

pub(super) fn unresolved_schema_composition_reason(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    text: &str,
) -> &'static str {
    let Some(path) = schema_payload_name_path(text) else {
        return "missing_schema";
    };
    if let Some(reason) = quarantined_schema_composition_reference_reason(
        module,
        schema.module_name.as_deref(),
        &path,
    ) {
        return reason;
    }
    match resolve_schema_alias_check_reference(
        module,
        &path,
        schema.module_name.as_deref(),
        true,
        &mut Vec::new(),
        &mut BTreeMap::new(),
    ) {
        SchemaAliasCheckResolution::Private => "private_schema",
        SchemaAliasCheckResolution::WrongKind(_) => "wrong_kind",
        SchemaAliasCheckResolution::Cyclic => "cyclic_composition",
        SchemaAliasCheckResolution::Resolved | SchemaAliasCheckResolution::Unresolved => {
            "missing_schema"
        }
    }
}

pub(super) fn quarantined_schema_composition_reference_reason(
    module: &SurfaceModule,
    current_module: Option<&str>,
    path: &[String],
) -> Option<&'static str> {
    let [_, .., name] = path else {
        return None;
    };
    let use_decl =
        quarantined_imported_use_for_path(module, &path[..path.len() - 1], current_module)?;
    if module.schemas.iter().any(|candidate| {
        candidate.name.as_deref() == Some(name.as_str())
            && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
            && candidate.visibility != Visibility::Public
    }) {
        return Some("private_schema");
    }
    None
}

pub(super) fn schema_composition_reference_blocker(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Option<&'static str> {
    if schema_field_uses_existing_grammar_at_boundary(schema, &field.ty) {
        return None;
    }
    let target = schema_field_target(module, schema, &field.ty)?;
    if schema_field_has_ordinary_type_target(module, schema, &field.ty) {
        return Some("ambiguous_type_and_schema");
    }
    let containing_format = schema.format.as_ref().map(|format| format.name.as_str());
    let target_format = target.format.as_ref().map(|format| format.name.as_str());
    if containing_format != target_format {
        return Some("format_incompatible");
    }
    schema_composition_reaches(module, target, schema, &mut Vec::new())
        .then_some("cyclic_composition")
}

pub(super) fn schema_composition_quarantine_is_sole_failure(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    text: &str,
) -> bool {
    let Some(path) = schema_payload_name_path(text) else {
        return false;
    };
    let [_, .., name] = path.as_slice() else {
        return false;
    };
    let Some(use_decl) = quarantined_imported_use_for_path(
        module,
        &path[..path.len() - 1],
        schema.module_name.as_deref(),
    ) else {
        return false;
    };
    module.schemas.iter().any(|candidate| {
        candidate.name.as_deref() == Some(name.as_str())
            && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
            && candidate.visibility == Visibility::Public
    }) || module.aliases.iter().any(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name.as_str())
            && alias.module_name.as_deref() == Some(use_decl.name.as_str())
            && !public_alias_has_invalid_target_leaf(module, alias, None)
    })
}

pub(super) fn schema_field_uses_existing_grammar_at_boundary(
    schema: &SchemaDecl,
    text: &str,
) -> bool {
    schema_field_uses_existing_grammar(schema, text)
        || (schema.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
            && (exact_width_binary_primitive_name(text).is_some()
                || reserved_bits_primitive(text).is_some()))
}

pub(super) fn schema_field_has_ordinary_type_target(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    text: &str,
) -> bool {
    let Some(path) = schema_payload_name_path(text) else {
        return false;
    };
    let (module_name, name, imported) = match path.as_slice() {
        [name] => (schema.module_name.as_deref(), name.as_str(), false),
        [_, .., name] => {
            let Some(use_decl) = normal_imported_use_for_path(
                module,
                &path[..path.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                return false;
            };
            (Some(use_decl.name.as_str()), name.as_str(), true)
        }
        _ => return false,
    };
    module.types.iter().any(|ty| {
        ty.name.as_deref() == Some(name)
            && ty.module_name.as_deref() == module_name
            && (!imported || ty.visibility == Visibility::Public)
    }) || module.aliases.iter().any(|alias| {
        alias.kind == PublicAliasKind::Type
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
            && !public_alias_has_invalid_target_leaf(module, alias, Some(NameClass::Type))
    })
}

pub(super) fn schema_composition_reaches(
    module: &SurfaceModule,
    current: &SchemaDecl,
    target: &SchemaDecl,
    visited: &mut Vec<NodeId>,
) -> bool {
    if current.node_id == target.node_id {
        return true;
    }
    if visited.contains(&current.node_id) {
        return false;
    }
    visited.push(current.node_id);
    let reaches = current.fields.iter().any(|field| {
        schema_field_target(module, current, &field.ty)
            .is_some_and(|next| schema_composition_reaches(module, next, target, visited))
    });
    visited.pop();
    reaches
}

pub(super) fn schema_composition_reference_diagnostic(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: &'static str,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let fact = match reason {
        "ambiguous_type_and_schema" => "resolves as both an ordinary type and a schema",
        "format_incompatible" => "does not use the containing schema's format",
        "cyclic_composition" => "creates a schema composition cycle",
        "private_schema" => "resolves to a private imported schema",
        "wrong_kind" => "resolves in a non-schema namespace",
        "decode_ineligible_target" => "does not expose an eligible decode boundary",
        "encode_ineligible_target" => "does not expose an eligible encode boundary",
        _ => "does not resolve to a visible schema or supported binary field family",
    };
    let companion_target = if reason == "private_schema" {
        companion_schema_access_target(module, schema)
    } else {
        None
    };
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        (
            "node_id",
            JsonValue::string(field.node_id.display("schema-field")),
        ),
        ("schema", JsonValue::string(schema_name)),
        ("binding", JsonValue::string(field.name.clone())),
        ("target", JsonValue::string(field.ty.clone())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(target_module) = companion_target.as_deref() {
        if let Some(current_module) = schema.module_name.as_deref() {
            details.push(("companion_module", JsonValue::string(current_module)));
        }
        details.push(("companion_target_module", JsonValue::string(target_module)));
    }
    let mut diagnostic = Diagnostic::new(
        "schema.composition_reference",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema field `{}` cannot compose `{}` because it {fact}",
            field.name, field.ty
        ),
        Some(field.span.clone()),
        JsonValue::object(details),
    );
    if let Some(target_module) = companion_target {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("companion_target")),
            (
                "message",
                JsonValue::string(format!(
                    "This test companion may access private schemas only from target module `{target_module}`."
                )),
            ),
            ("target_module", JsonValue::string(target_module)),
        ]));
    }
    diagnostic
}

pub(super) fn format_neutral_schema_helper_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let supported = "recursive format-neutral visible shape made from scalar leaves, anonymous record fields, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<recursive visible shape, recursive visible shape>, or same-module or public imported source ADTs whose constructor payloads are recursive visible shapes";
    let boundary_message = format!(
        "Generated format-neutral decode helpers for schema `{schema_name}` accept recursive visible shapes made from scalar leaves, anonymous record fields, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<Ok, Err> when both payloads are recursive visible shapes, and same-module or public imported source ADTs whose constructor payloads are recursive visible shapes."
    );
    let mut diagnostic = Diagnostic::new(
        "schema.format_neutral_decode_helper",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "format-neutral schema field `{}` cannot expose a generated decode helper because `{}` is not a {supported}",
            field.name, field.ty,
        ),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("field_type", JsonValue::string(field.ty.clone())),
            (
                "reason",
                JsonValue::string("unsupported_format_neutral_field_type"),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_helper_boundary")),
        ("span", span_json(&schema.span)),
        ("message", JsonValue::string(boundary_message)),
    ]));
    diagnostic
}

pub(in crate::analysis) fn format_neutral_schema_encode_helper_diagnostic(
    schema_name: &str,
    schema_span: &SourceSpan,
    field: &SchemaField,
) -> Diagnostic {
    let supported = "recursive format-neutral visible shape";
    let boundary_message = format!(
        "Generated format-neutral encode helpers for schema `{schema_name}` accept recursive visible shapes made from Int, Bool, Float, and String leaves, anonymous records, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<Ok, Err>, and eligible same-module or public imported source ADTs when every recursively visited child or constructor payload is also eligible."
    );
    let mut diagnostic = Diagnostic::new(
        "schema.format_neutral_encode_helper",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "format-neutral schema field `{}` cannot expose a generated encode helper because `{}` is not a {supported}",
            field.name, field.ty,
        ),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("field_type", JsonValue::string(field.ty.clone())),
            (
                "reason",
                JsonValue::string("unsupported_format_neutral_encode_field_type"),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_helper_boundary")),
        ("span", span_json(schema_span)),
        ("message", JsonValue::string(boundary_message)),
    ]));
    diagnostic
}
