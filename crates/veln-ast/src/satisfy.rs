use veln_syntax::SatisfyClause as SyntaxSatisfyClause;

use crate::{PredicateRemainder, SatisfyClause};

pub(crate) fn lower_satisfy_clause(clause: &SyntaxSatisfyClause) -> SatisfyClause {
    SatisfyClause {
        candidate: clause.candidate.clone(),
        candidate_span: clause.candidate_span.clone(),
        predicate: clause.predicate.clone(),
        remainder_spans: clause
            .remainder_spans
            .iter()
            .map(|remainder| PredicateRemainder {
                left_text: remainder.left_text.clone(),
                right_text: remainder.right_text.clone(),
                operator_span: remainder.operator_span.clone(),
                left_span: remainder.left_span.clone(),
                right_span: remainder.right_span.clone(),
                operation_span: remainder.operation_span.clone(),
            })
            .collect(),
        span: clause.span.clone(),
    }
}
