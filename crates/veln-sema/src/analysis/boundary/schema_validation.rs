use crate::schema::primitives::schema_field_reference_type;
use crate::semantic_model::Type;
use std::collections::{BTreeMap, BTreeSet};
use veln_ast::{SchemaDecl, SchemaField, SchemaValidationClause};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use super::schema_dispatch_resolution::schema_field_declared_after;
use super::schema_repeat_resolution::{
    schema_field_predicate_reference_diagnostic, schema_validation_reference_diagnostic,
};

pub(super) fn check_schema_validation_clause(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_schema_predicate_shift_count(&validation.predicate, &validation.span, diagnostics);
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
    check_schema_predicate_shift_count(&where_clause.predicate, &where_clause.span, diagnostics);
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

fn check_schema_predicate_shift_count(
    predicate: &str,
    span: &SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((operator, count)) =
        crate::contracts::predicate_invalid_literal_shift_count(predicate)
    else {
        return;
    };
    diagnostics.push(Diagnostic::new(
        "type.invalid_shift_count",
        Severity::Error,
        DiagnosticKind::Type,
        format!("shift count {count} is outside the permitted range 0 through 63"),
        Some(span.clone()),
        JsonValue::object([
            ("operator", JsonValue::string(operator)),
            ("actual_count", JsonValue::Number(count)),
            ("minimum_count", JsonValue::Number(0)),
            ("maximum_count", JsonValue::Number(63)),
        ]),
    ));
}

pub(super) fn schema_validation_references(predicate: &str) -> Vec<String> {
    let mut references = BTreeSet::new();
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if start > 0 && predicate.as_bytes()[start - 1] == b'0' && matches!(ch, 'b' | 'x') {
            while let Some((_, next)) = chars.peek().copied() {
                let is_digit = if ch == 'b' {
                    matches!(next, '0' | '1')
                } else {
                    next.is_ascii_hexdigit()
                };
                if !is_digit {
                    break;
                }
                chars.next();
            }
            continue;
        }
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
