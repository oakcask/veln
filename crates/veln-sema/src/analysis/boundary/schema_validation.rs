use crate::schema::primitives::schema_field_reference_type;
use crate::semantic_model::Type;
use std::collections::{BTreeMap, BTreeSet};
use veln_ast::{SchemaDecl, SchemaField, SchemaValidationClause};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{BinaryOp, BodyLine, Expr, ExprKind, PrefixOp, SyntaxItem};

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
    check_schema_predicate_type(
        &validation.predicate,
        &validation.span,
        decoded_fields,
        diagnostics,
    );
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
    check_schema_predicate_type(
        &where_clause.predicate,
        &where_clause.span,
        &visible_fields,
        diagnostics,
    );
}

#[derive(Debug)]
struct SchemaPredicateOperandMismatch {
    operator: &'static str,
    expected: &'static str,
    actual: Box<Type>,
}

fn check_schema_predicate_type(
    predicate: &str,
    span: &SourceSpan,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(expression) = parse_schema_predicate_expression(predicate) else {
        return;
    };
    match schema_predicate_expression_type(&expression, decoded_fields) {
        Ok(Some(actual)) if actual != Type::bool() => diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema predicate has type `{}`, expected `Bool`",
                actual.render()
            ),
            Some(span.clone()),
            JsonValue::object([
                ("expected", JsonValue::string("Bool")),
                ("actual", JsonValue::string(actual.render())),
            ]),
        )),
        Err(mismatch) => diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema predicate operator `{}` expects `{}` operands, but found `{}`",
                mismatch.operator,
                mismatch.expected,
                mismatch.actual.render()
            ),
            Some(span.clone()),
            JsonValue::object([
                ("operator", JsonValue::string(mismatch.operator)),
                ("expected", JsonValue::string(mismatch.expected)),
                ("actual", JsonValue::string(mismatch.actual.render())),
            ]),
        )),
        Ok(_) => {}
    }
}

fn parse_schema_predicate_expression(predicate: &str) -> Option<veln_syntax::Expr> {
    let source = SourceFile::new(
        "<schema-predicate>",
        format!("fn predicate() -> Bool\n  {predicate}\nend\n"),
    );
    let parsed = veln_syntax::parse(&source);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let SyntaxItem::Function(function) = parsed.tree.items.into_iter().next()? else {
        return None;
    };
    let BodyLine::Expr { expr, .. } = function.body.into_iter().next()? else {
        return None;
    };
    Some(expr)
}

fn schema_predicate_expression_type(
    expression: &Expr,
    decoded_fields: &BTreeMap<String, Type>,
) -> Result<Option<Type>, SchemaPredicateOperandMismatch> {
    if let Some(reference) = schema_predicate_reference_path(expression) {
        return Ok(schema_field_reference_type(decoded_fields, &reference).cloned());
    }
    match &expression.kind {
        ExprKind::IntLiteral(_) => Ok(Some(Type::int())),
        ExprKind::BoolLiteral(_) => Ok(Some(Type::bool())),
        ExprKind::Prefix { op, expr } => {
            let expected = match op {
                PrefixOp::Not => Type::bool(),
                PrefixOp::Negate | PrefixOp::BitwiseNot => Type::int(),
            };
            let actual = schema_predicate_expression_type(expr, decoded_fields)?;
            require_schema_predicate_operand(prefix_operator_text(*op), &expected, actual)?;
            Ok(Some(expected))
        }
        ExprKind::Binary { op, left, right } => {
            let left = schema_predicate_expression_type(left, decoded_fields)?;
            let right = schema_predicate_expression_type(right, decoded_fields)?;
            match op {
                BinaryOp::Or | BinaryOp::And => {
                    let expected = Type::bool();
                    require_schema_predicate_operand(binary_operator_text(*op), &expected, left)?;
                    require_schema_predicate_operand(binary_operator_text(*op), &expected, right)?;
                    Ok(Some(Type::bool()))
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if left.is_none() || right.is_none() {
                        Ok(None)
                    } else {
                        Ok(Some(Type::bool()))
                    }
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::BitwiseAnd
                | BinaryOp::ShiftLeft
                | BinaryOp::ShiftRight
                | BinaryOp::ShiftRightLogical
                | BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder => {
                    let expected = Type::int();
                    require_schema_predicate_operand(binary_operator_text(*op), &expected, left)?;
                    require_schema_predicate_operand(binary_operator_text(*op), &expected, right)?;
                    if matches!(
                        op,
                        BinaryOp::Less
                            | BinaryOp::LessEqual
                            | BinaryOp::Greater
                            | BinaryOp::GreaterEqual
                    ) {
                        Ok(Some(Type::bool()))
                    } else {
                        Ok(Some(Type::int()))
                    }
                }
                BinaryOp::PipeGreater => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn schema_predicate_reference_path(expression: &Expr) -> Option<String> {
    match &expression.kind {
        ExprKind::NamePath { segments, .. } if segments.len() == 1 => segments.first().cloned(),
        ExprKind::FieldAccess { base, field, .. } => {
            let mut path = schema_predicate_reference_path(base)?;
            path.push('.');
            path.push_str(field);
            Some(path)
        }
        _ => None,
    }
}

fn require_schema_predicate_operand(
    operator: &'static str,
    expected: &Type,
    actual: Option<Type>,
) -> Result<(), SchemaPredicateOperandMismatch> {
    let Some(actual) = actual else {
        return Ok(());
    };
    if &actual == expected {
        return Ok(());
    }
    Err(SchemaPredicateOperandMismatch {
        operator,
        expected: if expected == &Type::bool() {
            "Bool"
        } else {
            "Int"
        },
        actual: Box::new(actual),
    })
}

fn prefix_operator_text(operator: PrefixOp) -> &'static str {
    match operator {
        PrefixOp::Not => "not",
        PrefixOp::Negate => "-",
        PrefixOp::BitwiseNot => "~",
    }
}

fn binary_operator_text(operator: BinaryOp) -> &'static str {
    match operator {
        BinaryOp::PipeGreater => "|>",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::BitwiseOr => "|",
        BinaryOp::BitwiseXor => "^",
        BinaryOp::BitwiseAnd => "&",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::ShiftRightLogical => ">>>",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
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
