use veln_ast::{ContractKind, PredicateRemainder};
use veln_literals::parse_integer_literal;

use crate::semantic_model::{Binding, Type};

pub(crate) enum ContractValidation {
    Valid,
    InvalidRemainderOperands,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
    MissingField { base_type: String, field: String },
}

pub(crate) struct ContractCall {
    pub(crate) callee: String,
    pub(crate) args: Vec<String>,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
        ContractKind::Invariant => "invariant",
    }
}

pub(crate) fn predicate_is_statically_true(predicate: &str) -> bool {
    !predicate_has_nonfoldable_remainder(predicate)
        && static_boolean_value(predicate) == StaticBooleanValue::True
}

pub(crate) fn predicate_is_statically_true_with_literal_bounds(predicate: &str) -> bool {
    !predicate_has_nonfoldable_remainder(predicate)
        && static_boolean_value_with_literal_bounds(predicate) == StaticBooleanValue::True
}

pub(crate) fn contract_predicate_is_statically_true(predicate: &str) -> bool {
    !predicate_has_nonfoldable_remainder(predicate)
        && static_boolean_value_for_contract(predicate) == StaticBooleanValue::True
}

pub(crate) fn contract_predicate_is_statically_true_with_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> bool {
    if remainder_flows_into_division(predicate) {
        return false;
    }
    let Some(folded) = predicate_with_folded_remainder_chains(predicate, remainders) else {
        return false;
    };
    static_boolean_value_for_contract(&folded) == StaticBooleanValue::True
}

fn remainder_flows_into_division(predicate: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    let mut has_remainder = false;
    let mut has_division = false;
    for character in predicate.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '%' => has_remainder = true,
            '/' => has_division = true,
            _ => {}
        }
    }
    has_remainder && has_division
}

fn predicate_with_folded_remainder_chains(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<String> {
    let chains = veln_ast::index_predicate_remainder_chains(predicate, remainders)?;
    let mut folded = String::with_capacity(predicate.len());
    let mut cursor = 0;
    for chain in chains {
        if chain.predicate_range.start < cursor {
            continue;
        }
        let chain_remainders = &remainders[chain.remainder_range];
        folded.push_str(&predicate[cursor..chain.predicate_range.start]);
        folded.push_str(
            &flat_remainder_chain_value(chain_remainders)?
                .as_i64()?
                .to_string(),
        );
        cursor = chain.predicate_range.end;
    }
    folded.push_str(&predicate[cursor..]);
    Some(folded)
}

fn flat_remainder_chain_value(remainders: &[PredicateRemainder]) -> Option<StaticNumber> {
    let first = remainders.first()?;
    let mut value = static_numeric_expression(&first.left_text)?;
    for remainder in remainders {
        let right = static_numeric_expression(&remainder.right_text)?;
        value = static_multiplicative_operation(value, "%", right)?;
    }
    Some(value)
}

pub(crate) fn predicate_is_statically_false(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::False
}

pub(crate) fn predicate_has_nonfoldable_remainder_with_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> bool {
    if remainders.is_empty() {
        return predicate_has_nonfoldable_remainder(predicate);
    }
    veln_ast::predicate_remainder_chain_ranges(remainders)
        .into_iter()
        .any(|range| flat_remainder_chain_value(&remainders[range]).is_none())
}

mod boolean_cases;
mod boolean_core;
mod case_and_order;
mod contract_calls;
mod exact_numbers;
mod field_access;
mod literal_order;
mod predicate_shapes;
mod predicate_typing;
mod static_literals;

use boolean_cases::*;
use boolean_core::*;
use case_and_order::*;
pub(crate) use exact_numbers::{ExactNumber, ExactRational, parse_quoted_string_literal};
use field_access::*;
use literal_order::*;
use predicate_shapes::*;
use predicate_typing::*;
use static_literals::*;

pub(crate) use contract_calls::{contract_calls, is_contract_keyword, referenced_names};
pub(crate) use predicate_typing::{
    missing_contract_field, predicate_has_invalid_remainder_operands,
    predicate_is_boolean_with_calls, predicate_is_boolean_with_remainders,
    predicate_literal_is_zero, predicate_remainder_operand_mismatch,
    predicate_rendered_type_with_calls, predicate_type_with_remainders,
};
#[cfg(test)]
pub(crate) use predicate_typing::{reset_predicate_typing_work, take_predicate_typing_work};
pub(crate) use static_literals::predicate_has_nonfoldable_remainder;

#[cfg(test)]
mod static_reasoning_tests;
