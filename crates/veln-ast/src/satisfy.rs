use veln_syntax::SatisfyClause as SyntaxSatisfyClause;

use crate::SatisfyClause;

pub(crate) fn lower_satisfy_clause(clause: &SyntaxSatisfyClause) -> SatisfyClause {
    SatisfyClause {
        candidate: clause.candidate.clone(),
        predicate: clause.predicate.clone(),
        span: clause.span.clone(),
    }
}
