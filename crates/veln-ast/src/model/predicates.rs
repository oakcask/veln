use std::ops::Range;

use veln_source::SourceSpan;

use super::NodeId;

#[derive(Clone, Debug)]
pub struct Contract {
    pub node_id: NodeId,
    pub kind: ContractKind,
    pub text: String,
    pub predicate_tokens: Vec<PredicateTokenSpan>,
    pub remainder_spans: Vec<PredicateRemainder>,
    pub callsite_reference_spans: Vec<SourceSpan>,
    pub call_callee_spans: Vec<(String, SourceSpan)>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateRemainder {
    pub left_range: Range<usize>,
    pub right_range: Range<usize>,
    pub operation_range: Range<usize>,
    pub left_remainder: Option<usize>,
    pub operator_span: SourceSpan,
    pub left_span: SourceSpan,
    pub right_span: SourceSpan,
    pub operation_span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateTokenSpan {
    pub text_range: Range<usize>,
    pub source_span: SourceSpan,
}

impl PredicateRemainder {
    pub fn left_text<'a>(&self, predicate: &'a str) -> &'a str {
        &predicate[self.left_range.clone()]
    }

    pub fn right_text<'a>(&self, predicate: &'a str) -> &'a str {
        &predicate[self.right_range.clone()]
    }
}

pub fn predicate_remainder_chain_ranges(remainders: &[PredicateRemainder]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, remainder) in remainders.iter().enumerate() {
        if (index == 0 || remainder.left_remainder != Some(index - 1))
            && let Some(start) = start.replace(index)
        {
            ranges.push(start..index);
        }
    }
    if let Some(start) = start {
        ranges.push(start..remainders.len());
    }
    ranges
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateRemainderChain {
    pub remainder_range: Range<usize>,
    pub predicate_range: Range<usize>,
}

pub fn index_predicate_remainder_chains(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<Vec<PredicateRemainderChain>> {
    let ranges = predicate_remainder_chain_ranges(remainders);
    let mut chains = Vec::with_capacity(ranges.len());
    for remainder_range in ranges {
        let predicate_range = remainders[remainder_range.end - 1].operation_range.clone();
        predicate.get(predicate_range.clone())?;
        chains.push(PredicateRemainderChain {
            remainder_range,
            predicate_range,
        });
    }
    Some(chains)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Require,
    Ensure,
    Invariant,
}
