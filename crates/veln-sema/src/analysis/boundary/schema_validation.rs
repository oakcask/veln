use super::*;

pub(super) fn check_schema_validation_clause(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for reference in schema_validation_references(&validation.predicate) {
        let Some(ty) = schema_field_reference_type(decoded_fields, &reference) else {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "unknown_field_reference",
                format!("schema validation reference `{reference}` is not a decoded schema field"),
                [],
            ));
            continue;
        };
        if ty != &Type::int() {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema validation reference `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            ));
        }
    }
}

pub(super) fn record_decoded_schema_field(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_ty: Type,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_schema_field_predicate_references(schema, field, &field_ty, decoded_fields, diagnostics);
    decoded_fields.insert(field.name.clone(), field_ty);
}

pub(super) fn check_schema_field_predicate_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_ty: &Type,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(where_clause) = &field.where_clause else {
        return;
    };
    let mut visible_fields = decoded_fields.clone();
    visible_fields.insert(field.name.clone(), field_ty.clone());
    for reference in schema_validation_references(&where_clause.predicate) {
        let Some(ty) = schema_field_reference_type(&visible_fields, &reference) else {
            let reason = if schema_field_declared_after(schema, field, &reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            diagnostics.push(schema_field_predicate_reference_diagnostic(
                schema,
                field,
                &reference,
                reason,
                format!(
                    "schema field predicate reference `{reference}` must name the field being checked or an earlier decoded schema field"
                ),
                [],
            ));
            continue;
        };
        if reference.contains('.') && ty != &Type::int() {
            diagnostics.push(schema_field_predicate_reference_diagnostic(
                schema,
                field,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema field predicate reference `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            ));
        }
    }
}

pub(super) fn schema_validation_references(predicate: &str) -> Vec<String> {
    let mut references = BTreeSet::new();
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            continue;
        }
        let mut end = start + ch.len_utf8();
        while let Some((index, next)) = chars.peek().copied() {
            if next.is_ascii_alphanumeric() || next == '_' || next == '.' {
                chars.next();
                end = index + next.len_utf8();
            } else {
                break;
            }
        }
        let ident = &predicate[start..end];
        if !matches!(ident, "true" | "false" | "and" | "or" | "not") {
            references.insert(ident.to_string());
        }
    }
    references.into_iter().collect()
}
