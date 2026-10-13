use veln_syntax::SatisfyClause as SyntaxSatisfyClause;

use crate::{PredicateRemainder, PredicateTokenSpan, SatisfyClause};

pub(crate) fn lower_satisfy_clause(clause: &SyntaxSatisfyClause) -> SatisfyClause {
    SatisfyClause {
        candidate: clause.candidate.clone(),
        candidate_span: clause.candidate_span.clone(),
        predicate: clause.predicate.clone(),
        predicate_tokens: clause
            .predicate_tokens
            .iter()
            .map(|token| PredicateTokenSpan {
                text_range: token.text_range.clone(),
                source_span: token.source_span.clone(),
            })
            .collect(),
        remainder_spans: clause
            .remainder_spans
            .iter()
            .map(|remainder| PredicateRemainder {
                left_range: remainder.left_range.clone(),
                right_range: remainder.right_range.clone(),
                operation_range: remainder.operation_range.clone(),
                left_remainder: remainder.left_remainder,
                operator_span: remainder.operator_span.clone(),
                left_span: remainder.left_span.clone(),
                right_span: remainder.right_span.clone(),
                operation_span: remainder.operation_span.clone(),
            })
            .collect(),
        span: clause.span.clone(),
    }
}
