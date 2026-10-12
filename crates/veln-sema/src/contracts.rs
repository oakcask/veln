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
    let has_boolean_composition = split_top_level_keyword_operator(predicate, "or").is_some()
        || split_top_level_keyword_operator(predicate, "and").is_some();
    if !has_boolean_composition
        && let Some(is_true) = flat_remainder_comparison_is_statically_true(predicate, remainders)
    {
        return is_true;
    }
    !flat_remainder_chain_is_nonfoldable(remainders)
        .unwrap_or_else(|| predicate_has_nonfoldable_remainder(predicate))
        && static_boolean_value_for_contract(predicate) == StaticBooleanValue::True
}

fn flat_remainder_comparison_is_statically_true(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<bool> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        let Some((left, right)) = split_top_level_operator(predicate, operator) else {
            continue;
        };
        let (chain, other, chain_is_left) = if flat_remainder_chain_matches(left, remainders) {
            (flat_remainder_chain_value(remainders), right, true)
        } else if flat_remainder_chain_matches(right, remainders) {
            (flat_remainder_chain_value(remainders), left, false)
        } else {
            continue;
        };
        let Some(chain) = chain else {
            return Some(false);
        };
        let Some(other) = static_numeric_expression(other) else {
            return Some(false);
        };
        let (left, right) = if chain_is_left {
            (chain, other)
        } else {
            (other, chain)
        };
        return Some(static_number_comparison(left, operator, right).unwrap_or(false));
    }
    None
}

pub(crate) fn flat_remainder_chain_matches(text: &str, remainders: &[PredicateRemainder]) -> bool {
    let Some((first, rest)) = remainders.split_first() else {
        return false;
    };
    if first.left_is_prior_remainder
        || rest
            .iter()
            .any(|remainder| !remainder.left_is_prior_remainder)
    {
        return false;
    }
    let mut expected = first.left_text.clone();
    for remainder in remainders {
        expected.push('%');
        expected.push_str(&remainder.right_text);
    }
    let compact = |value: &str| {
        strip_balanced_outer_parens(value.trim())
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
    };
    compact(text) == compact(&expected)
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

fn flat_remainder_chain_is_nonfoldable(remainders: &[PredicateRemainder]) -> Option<bool> {
    let (first, rest) = remainders.split_first()?;
    if first.left_is_prior_remainder
        || rest
            .iter()
            .any(|remainder| !remainder.left_is_prior_remainder)
    {
        return None;
    }
    Some(flat_remainder_chain_value(remainders).is_none())
}

pub(crate) fn predicate_is_statically_false(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::False
}

pub(crate) fn predicate_has_nonfoldable_remainder_with_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> bool {
    flat_remainder_chain_is_nonfoldable(remainders)
        .unwrap_or_else(|| predicate_has_nonfoldable_remainder(predicate))
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
    predicate_literal_is_zero, predicate_remainder_operand_type,
    predicate_rendered_type_with_calls, predicate_type_with_remainders,
};
#[cfg(test)]
pub(crate) use predicate_typing::{reset_predicate_typing_work, take_predicate_typing_work};
pub(crate) use static_literals::predicate_has_nonfoldable_remainder;

#[cfg(test)]
mod static_reasoning_tests;
