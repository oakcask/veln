use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn report_literal_zero_remainder(&mut self, left: &Expr, right: &Expr) {
        if !is_literal_zero(right) {
            return;
        }
        let mut operation_span = left.span.clone();
        operation_span.end = right.span.end;
        let mut diagnostic = Diagnostic::new(
            "type.remainder_by_zero",
            Severity::Error,
            DiagnosticKind::Type,
            "remainder divisor is zero",
            Some(right.span.clone()),
            JsonValue::object([
                ("operator", JsonValue::string("%")),
                ("divisor", JsonValue::Number(0)),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("operation")),
            ("message", JsonValue::string("Remainder operation is here.")),
            ("span", span_json(&operation_span)),
        ]));
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn report_invalid_literal_shift_count(&mut self, op: BinaryOp, right: &Expr) {
        let Some(count) = invalid_literal_shift_count(op, right) else {
            return;
        };
        let operator = shift_operator_text(op).expect("shift operator should have text");
        self.diagnostics.push(Diagnostic::new(
            "type.invalid_shift_count",
            Severity::Error,
            DiagnosticKind::Type,
            format!("shift count {count} is outside the permitted range 0 through 63"),
            Some(right.span.clone()),
            JsonValue::object([
                ("operator", JsonValue::string(operator)),
                ("actual_count", JsonValue::Number(count)),
                ("minimum_count", JsonValue::Number(0)),
                ("maximum_count", JsonValue::Number(63)),
            ]),
        ));
    }
}

fn is_literal_zero(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::IntLiteral(value) => {
            veln_literals::parse_integer_literal(value).is_ok_and(|literal| literal.value == 0)
        }
        ExprKind::Prefix {
            op: veln_ast::PrefixOp::Negate,
            expr,
        } => is_literal_zero(expr),
        _ => false,
    }
}
