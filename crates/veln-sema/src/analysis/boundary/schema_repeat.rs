use super::*;

pub(super) fn check_schema_repeat_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    repeat: &SchemaRepeatSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if !check_schema_repeat_references(
        schema,
        field,
        decoded_fields,
        &repeat.count_field,
        diagnostics,
    ) {
        return None;
    }
    let element_ty = match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Type::int(),
        SchemaRepeatPayload::ReservedBits { .. } => return None,
        SchemaRepeatPayload::ByteView { length_field } => {
            if !check_schema_repeat_byte_view_reference(
                schema,
                field,
                length_field,
                decoded_fields,
                diagnostics,
            ) {
                return None;
            }
            Type::named("ByteView", Vec::new())
        }
        SchemaRepeatPayload::Schema { schema_name } => {
            let payload_schema = resolve_schema_repeat_payload_schema(
                module,
                schema,
                field,
                schema_name,
                diagnostics,
            )?;
            if schema_payload_has_generalized_reserved_byte_prefix(payload_schema) {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    schema_name,
                    "incompatible_payload_schema",
                    format!(
                        "repeat payload schema `{}` uses a reserved-byte-prefix layout outside repeat payload helpers",
                        schema_payload_name_last_segment(schema_name)
                    ),
                    [],
                ));
                return None;
            }
            schema_decode_value_type(module, payload_schema).or_else(|| {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    schema_name,
                    "incompatible_payload_schema",
                    format!(
                        "repeat payload schema `{}` is not a supported decoded binary schema",
                        schema_payload_name_last_segment(schema_name)
                    ),
                    [],
                ));
                None
            })?
        }
    };
    Some(Type::named("List", vec![element_ty]))
}

pub(super) fn check_schema_repeat_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_expr: &str,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(length_expr) else {
        return false;
    };
    let mut valid = true;
    for reference in references {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!(
                    "repeat ByteView length operand `{reference}` must be an earlier decoded `Int` field"
                ),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "repeat ByteView length operand `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

pub(super) fn check_schema_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_expr: &ByteViewLengthExpr,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut valid = true;
    for reference in length_expr.references() {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!(
                    "ByteView length operand `{reference}` must be an earlier decoded `Int` field"
                ),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "ByteView length operand `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

pub(super) fn check_schema_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(where_clause) = &field.where_clause else {
        return true;
    };
    let Some(constraint) = byte_view_multiple_constraint(&where_clause.predicate) else {
        diagnostics.push(schema_byte_view_multiple_diagnostic(
            schema,
            field,
            &where_clause.predicate,
            "unsupported_multiple_predicate",
            "ByteView field validation must use `payload_count multiple of <field-or-positive-integer>`".to_string(),
            [],
        ));
        return false;
    };
    let Some(reference) = constraint.reference() else {
        return true;
    };
    let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            reason,
            format!(
                "ByteView multiple operand `{reference}` must be an earlier decoded `Int` field"
            ),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            "incompatible_field_reference",
            format!(
                "ByteView multiple operand `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

pub(super) fn check_schema_non_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(where_clause) = &field.where_clause else {
        return;
    };
    if !where_clause
        .predicate
        .trim()
        .starts_with("payload_count multiple of ")
    {
        return;
    }
    diagnostics.push(schema_byte_view_multiple_diagnostic(
        schema,
        field,
        &where_clause.predicate,
        "invalid_field_kind",
        "ByteView multiple validation can only be used on length-bounded `ByteView` fields"
            .to_string(),
        [],
    ));
}

pub(super) fn check_schema_repeat_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    count_expr: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(count_expr) else {
        return false;
    };
    let label = if references.len() == 1 {
        "repeat count field"
    } else {
        "repeat count operand"
    };
    let mut valid = true;
    for reference in references {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!("{label} `{reference}` must be an earlier decoded `Int` field"),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "{label} `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}
