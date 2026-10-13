use super::*;

impl<'a> FunctionChecker<'a> {
    pub(in crate::analysis) fn report_contract_literal_zero_remainder(
        &mut self,
        remainder: &PredicateRemainder,
    ) {
        let mut diagnostic = Diagnostic::new(
            "type.remainder_by_zero",
            Severity::Error,
            DiagnosticKind::Type,
            "remainder divisor is zero",
            Some(remainder.right_span.clone()),
            JsonValue::object([
                ("operator", JsonValue::string("%")),
                ("divisor", JsonValue::Number(0)),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("operation")),
            ("message", JsonValue::string("Remainder operation is here.")),
            ("span", span_json(&remainder.operation_span)),
        ]));
        self.diagnostics.push(diagnostic);
    }

    pub(in crate::analysis) fn report_predicate_remainder_operand_mismatches(
        &mut self,
        predicate: &str,
        predicate_tokens: &[veln_ast::PredicateTokenSpan],
        remainders: &[PredicateRemainder],
        bindings: &[Binding],
        node_id: NodeId,
    ) {
        let mut invalid = remainder_operand_ranges(remainders)
            .into_iter()
            .filter_map(|range| {
                self.remainder_operand_mismatch(predicate, predicate_tokens, bindings, range)
            })
            .collect::<Vec<_>>();
        invalid.sort_by_key(|(_, span)| (span.start.offset, span.end.offset));
        invalid.dedup_by(|left, right| {
            left.0 == right.0
                && left.1.start.offset == right.1.start.offset
                && left.1.end.offset == right.1.end.offset
        });
        for (actual_type, span) in invalid {
            self.diagnostics.push(Diagnostic::new(
                "type.mismatch",
                Severity::Error,
                DiagnosticKind::Type,
                format!("expected `Int`, but found `{actual_type}`"),
                Some(span),
                type_details(
                    node_id.display("predicate"),
                    "Int",
                    actual_type,
                    "inferred_expression",
                    "inferred_expression",
                    "operator_operand",
                    [
                        self.function.node_id.display("fn"),
                        node_id.display("predicate"),
                    ],
                ),
            ));
        }
    }

    fn remainder_operand_mismatch(
        &self,
        predicate: &str,
        predicate_tokens: &[veln_ast::PredicateTokenSpan],
        bindings: &[Binding],
        range: std::ops::Range<usize>,
    ) -> Option<(String, SourceSpan)> {
        let text = &predicate[range.clone()];
        let mismatch = predicate_remainder_operand_mismatch(text, bindings, &|callee| {
            self.contract_call_signature(callee)
                .map(|(_, _, return_type, _, _)| return_type)
        })?;
        let mismatch_range = (range.start + mismatch.start)..(range.start + mismatch.end);
        let first = predicate_tokens
            .iter()
            .find(|token| token.text_range.end > mismatch_range.start)?;
        let last = predicate_tokens
            .iter()
            .rev()
            .find(|token| token.text_range.start < mismatch_range.end)?;
        let mut span = first.source_span.clone();
        span.end = last.source_span.end;
        Some((mismatch.actual.render(), span))
    }
}

fn remainder_operand_ranges(remainders: &[PredicateRemainder]) -> Vec<std::ops::Range<usize>> {
    remainders
        .iter()
        .flat_map(|remainder| {
            [
                remainder
                    .left_remainder
                    .is_none()
                    .then_some(remainder.left_range.clone()),
                Some(remainder.right_range.clone()),
            ]
        })
        .flatten()
        .collect()
}
