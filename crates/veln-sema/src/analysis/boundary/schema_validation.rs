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
        if ty != &Type::int() && ty != &Type::bool() {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema validation reference `{reference}` decodes as `{}`, not `Int` or `Bool`",
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
        if ty != &Type::int() && ty != &Type::bool() {
            diagnostics.push(schema_field_predicate_reference_diagnostic(
                schema,
                field,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema field predicate reference `{reference}` decodes as `{}`, not `Int` or `Bool`",
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

#[derive(Debug)]
struct SchemaPredicateExpressionInfo {
    ty: Option<Type>,
    literal_int: Option<i64>,
    invalid_shift: Option<(&'static str, i64)>,
}

#[derive(Debug)]
struct SchemaPredicateAnalysis {
    actual: Option<Type>,
    invalid_shift: Option<(&'static str, i64)>,
}

enum SchemaPredicateAnalysisTask {
    Visit(Box<Expr>),
    FinishPrefix(PrefixOp),
    FinishBinary(BinaryOp),
}

#[cfg(test)]
thread_local! {
    static SCHEMA_PREDICATE_VISITED_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_schema_predicate_visited_nodes() {
    SCHEMA_PREDICATE_VISITED_NODES.set(0);
}

#[cfg(test)]
pub(crate) fn schema_predicate_visited_nodes() -> usize {
    SCHEMA_PREDICATE_VISITED_NODES.get()
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
    let analysis = match analyze_schema_predicate_expression(expression, decoded_fields) {
        Ok(analysis) => analysis,
        Err(mismatch) => {
            diagnostics.push(Diagnostic::new(
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
            ));
            return;
        }
    };
    if let Some((operator, count)) = analysis.invalid_shift {
        diagnostics.push(invalid_schema_predicate_shift_count_diagnostic(
            span, operator, count,
        ));
    }
    match analysis.actual {
        Some(actual) if actual != Type::bool() => diagnostics.push(Diagnostic::new(
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
        _ => {}
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

fn analyze_schema_predicate_expression(
    expression: Expr,
    decoded_fields: &BTreeMap<String, Type>,
) -> Result<SchemaPredicateAnalysis, SchemaPredicateOperandMismatch> {
    let mut tasks = vec![SchemaPredicateAnalysisTask::Visit(Box::new(expression))];
    let mut results = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            SchemaPredicateAnalysisTask::Visit(expression) => {
                #[cfg(test)]
                SCHEMA_PREDICATE_VISITED_NODES
                    .set(SCHEMA_PREDICATE_VISITED_NODES.get().saturating_add(1));
                match expression.kind {
                    ExprKind::NamePath { segments, .. } => {
                        let reference = (segments.len() == 1)
                            .then(|| segments.into_iter().next())
                            .flatten();
                        results.push(SchemaPredicateExpressionInfo {
                            ty: reference.and_then(|reference| {
                                schema_field_reference_type(decoded_fields, &reference).cloned()
                            }),
                            literal_int: None,
                            invalid_shift: None,
                        });
                    }
                    ExprKind::FieldAccess { base, field, .. } => {
                        let reference = schema_predicate_reference_path_from_field(*base, field);
                        results.push(SchemaPredicateExpressionInfo {
                            ty: reference.and_then(|reference| {
                                schema_field_reference_type(decoded_fields, &reference).cloned()
                            }),
                            literal_int: None,
                            invalid_shift: None,
                        });
                    }
                    ExprKind::IntLiteral(text) => {
                        results.push(SchemaPredicateExpressionInfo {
                            ty: Some(Type::int()),
                            literal_int: parse_schema_predicate_int_literal(&text),
                            invalid_shift: None,
                        });
                    }
                    ExprKind::BoolLiteral(_) => results.push(SchemaPredicateExpressionInfo {
                        ty: Some(Type::bool()),
                        literal_int: None,
                        invalid_shift: None,
                    }),
                    ExprKind::StringLiteral(_) => results.push(SchemaPredicateExpressionInfo {
                        ty: Some(Type::string()),
                        literal_int: None,
                        invalid_shift: None,
                    }),
                    ExprKind::FloatLiteral(_) => results.push(SchemaPredicateExpressionInfo {
                        ty: Some(Type::float()),
                        literal_int: None,
                        invalid_shift: None,
                    }),
                    ExprKind::Prefix { op, expr } => {
                        tasks.push(SchemaPredicateAnalysisTask::FinishPrefix(op));
                        tasks.push(SchemaPredicateAnalysisTask::Visit(expr));
                    }
                    ExprKind::Binary { op, left, right } => {
                        tasks.push(SchemaPredicateAnalysisTask::FinishBinary(op));
                        tasks.push(SchemaPredicateAnalysisTask::Visit(right));
                        tasks.push(SchemaPredicateAnalysisTask::Visit(left));
                    }
                    _ => results.push(SchemaPredicateExpressionInfo {
                        ty: None,
                        literal_int: None,
                        invalid_shift: None,
                    }),
                }
            }
            SchemaPredicateAnalysisTask::FinishPrefix(operator) => {
                let operand = results.pop().expect("schema predicate prefix operand");
                let expected = match operator {
                    PrefixOp::Not => Type::bool(),
                    PrefixOp::Negate | PrefixOp::BitwiseNot => Type::int(),
                };
                require_schema_predicate_operand(
                    prefix_operator_text(operator),
                    &expected,
                    operand.ty,
                )?;
                let literal_int = match operator {
                    PrefixOp::Negate => operand.literal_int.map(i64::wrapping_neg),
                    PrefixOp::BitwiseNot => operand.literal_int.map(|value| !value),
                    PrefixOp::Not => None,
                };
                results.push(SchemaPredicateExpressionInfo {
                    ty: Some(expected),
                    literal_int,
                    invalid_shift: operand.invalid_shift,
                });
            }
            SchemaPredicateAnalysisTask::FinishBinary(operator) => {
                let right = results.pop().expect("schema predicate right operand");
                let left = results.pop().expect("schema predicate left operand");
                results.push(schema_predicate_binary_expression_info(
                    operator, left, right,
                )?);
            }
        }
    }
    let result = results.pop().expect("schema predicate result");
    debug_assert!(results.is_empty());
    Ok(SchemaPredicateAnalysis {
        actual: result.ty,
        invalid_shift: result.invalid_shift,
    })
}

fn schema_predicate_binary_expression_info(
    operator: BinaryOp,
    left: SchemaPredicateExpressionInfo,
    right: SchemaPredicateExpressionInfo,
) -> Result<SchemaPredicateExpressionInfo, SchemaPredicateOperandMismatch> {
    let invalid_shift = if matches!(
        operator,
        BinaryOp::ShiftLeft | BinaryOp::ShiftRight | BinaryOp::ShiftRightLogical
    ) && let Some(count) = right.literal_int
        && !(0..=63).contains(&count)
    {
        Some((binary_operator_text(operator), count))
    } else {
        left.invalid_shift.or(right.invalid_shift)
    };
    let ty = match operator {
        BinaryOp::Or | BinaryOp::And => {
            schema_predicate_typed_binary_expression(operator, Type::bool(), left.ty, right.ty)?
        }
        BinaryOp::Equal | BinaryOp::NotEqual => {
            schema_predicate_equality_expression_type(operator, left.ty, right.ty)?
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
            schema_predicate_typed_binary_expression(operator, Type::int(), left.ty, right.ty)?
        }
        BinaryOp::PipeGreater => None,
    };
    let literal_int =
        schema_predicate_literal_binary_value(operator, left.literal_int, right.literal_int);
    Ok(SchemaPredicateExpressionInfo {
        ty,
        literal_int,
        invalid_shift,
    })
}

fn schema_predicate_equality_expression_type(
    operator: BinaryOp,
    left: Option<Type>,
    right: Option<Type>,
) -> Result<Option<Type>, SchemaPredicateOperandMismatch> {
    let Some(left) = left else {
        return Ok(None);
    };
    let Some(right) = right else {
        return Ok(None);
    };
    if left == right && (left == Type::int() || left == Type::bool()) {
        return Ok(Some(Type::bool()));
    }
    Err(SchemaPredicateOperandMismatch {
        operator: binary_operator_text(operator),
        expected: "matching Int or Bool",
        actual: Box::new(right),
    })
}

fn schema_predicate_typed_binary_expression(
    operator: BinaryOp,
    expected: Type,
    left: Option<Type>,
    right: Option<Type>,
) -> Result<Option<Type>, SchemaPredicateOperandMismatch> {
    let operator_text = binary_operator_text(operator);
    require_schema_predicate_operand(operator_text, &expected, left)?;
    require_schema_predicate_operand(operator_text, &expected, right)?;
    let result = if is_schema_predicate_ordering_operator(operator) {
        Type::bool()
    } else {
        expected
    };
    Ok(Some(result))
}

fn is_schema_predicate_ordering_operator(operator: BinaryOp) -> bool {
    matches!(
        operator,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}

fn schema_predicate_reference_path_from_field(mut base: Expr, field: String) -> Option<String> {
    let mut fields = vec![field];
    loop {
        match base.kind {
            ExprKind::NamePath { segments, .. } if segments.len() == 1 => {
                let mut path = segments.into_iter().next()?;
                for field in fields.into_iter().rev() {
                    path.push('.');
                    path.push_str(&field);
                }
                return Some(path);
            }
            ExprKind::FieldAccess {
                base: next, field, ..
            } => {
                fields.push(field);
                base = *next;
            }
            _ => return None,
        }
    }
}

fn parse_schema_predicate_int_literal(text: &str) -> Option<i64> {
    let text = text.replace('_', "");
    if let Some(binary) = text.strip_prefix("0b") {
        i64::from_str_radix(binary, 2).ok()
    } else if let Some(hex) = text.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

fn schema_predicate_literal_binary_value(
    operator: BinaryOp,
    left: Option<i64>,
    right: Option<i64>,
) -> Option<i64> {
    let left = left?;
    let right = right?;
    match operator {
        BinaryOp::BitwiseOr => Some(left | right),
        BinaryOp::BitwiseXor => Some(left ^ right),
        BinaryOp::BitwiseAnd => Some(left & right),
        BinaryOp::ShiftLeft if (0..=63).contains(&right) => Some(left.wrapping_shl(right as u32)),
        BinaryOp::ShiftRight if (0..=63).contains(&right) => Some(left >> right),
        BinaryOp::ShiftRightLogical if (0..=63).contains(&right) => {
            Some(((left as u64) >> right) as i64)
        }
        BinaryOp::Add => Some(left.wrapping_add(right)),
        BinaryOp::Subtract => Some(left.wrapping_sub(right)),
        BinaryOp::Multiply => Some(left.wrapping_mul(right)),
        BinaryOp::Divide if right != 0 => Some(left.wrapping_div(right)),
        BinaryOp::Remainder if right != 0 => Some(left.wrapping_rem(right)),
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

fn invalid_schema_predicate_shift_count_diagnostic(
    span: &SourceSpan,
    operator: &'static str,
    count: i64,
) -> Diagnostic {
    Diagnostic::new(
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
    )
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
