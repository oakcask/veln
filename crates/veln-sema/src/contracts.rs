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
    !flat_remainder_chain_is_nonfoldable(remainders)
        .unwrap_or_else(|| predicate_has_nonfoldable_remainder(predicate))
        && static_boolean_value_for_contract(predicate) == StaticBooleanValue::True
}

fn flat_remainder_chain_is_nonfoldable(remainders: &[PredicateRemainder]) -> Option<bool> {
    let (first, rest) = remainders.split_first()?;
    if first.left_is_prior_remainder
        || rest
            .iter()
            .any(|remainder| !remainder.left_is_prior_remainder)
    {
        return None;
    }
    let Some(mut value) = static_numeric_expression(&first.left_text) else {
        return Some(true);
    };
    for remainder in remainders {
        let Some(right) = static_numeric_expression(&remainder.right_text) else {
            return Some(true);
        };
        let Some(next) = static_multiplicative_operation(value, "%", right) else {
            return Some(true);
        };
        value = next;
    }
    Some(false)
}

pub(crate) fn predicate_is_statically_false(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::False
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
    predicate_is_boolean_with_calls, predicate_literal_is_zero, predicate_rendered_type_with_calls,
    predicate_type_with_calls,
};
pub(crate) use static_literals::predicate_has_nonfoldable_remainder;

#[cfg(test)]
mod static_reasoning_tests;
