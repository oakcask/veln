use veln_ast::{ContractKind, PredicateRemainder};
use veln_literals::parse_integer_literal;

use std::ops::Range;

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
    pub(crate) arg_ranges: Vec<Range<usize>>,
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
    if remainders.is_empty() {
        return contract_predicate_is_statically_true(predicate);
    }
    let Some(folded) = predicate_with_folded_literal_remainders(predicate, remainders) else {
        return false;
    };
    static_boolean_value_for_remainder_contract(&folded) == StaticBooleanValue::True
}

pub(crate) fn predicate_with_folded_literal_remainders(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<String> {
    if remainders.is_empty() {
        return Some(predicate.to_string());
    }
    let flow_shape = predicate_with_remainder_markers(predicate, remainders)?;
    let folded = predicate_with_folded_remainder_chains(predicate, remainders)?;
    predicate_with_runtime_integer_remainder_arithmetic(&flow_shape, &folded)
}

fn predicate_with_remainder_markers(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<String> {
    let chains = veln_ast::index_predicate_remainder_chains(predicate, remainders)?;
    let mut marked = String::with_capacity(predicate.len());
    let mut cursor = 0;
    for chain in chains {
        if chain.predicate_range.start < cursor {
            continue;
        }
        marked.push_str(&predicate[cursor..chain.predicate_range.start]);
        marked.push_str("0%1");
        cursor = chain.predicate_range.end;
    }
    marked.push_str(&predicate[cursor..]);
    Some(marked)
}

fn predicate_with_runtime_integer_remainder_arithmetic(
    marked: &str,
    folded: &str,
) -> Option<String> {
    // Preserve each source arithmetic node's type after its literal remainder
    // children have been folded. Int-only ancestors use runtime fixed-width
    // behavior, while a Float ancestor remains visible to exact-number
    // reasoning instead of inheriting the remainder node's Int type.
    let marked = strip_balanced_outer_parens(marked.trim());
    let folded = strip_balanced_outer_parens(folded.trim());
    if !marked.contains('%') {
        return Some(folded.to_string());
    }

    for keyword in ["or", "and"] {
        let marked_clauses = split_top_level_keyword(marked, keyword);
        if marked_clauses.len() < 2 {
            continue;
        }
        let folded_clauses = split_top_level_keyword(folded, keyword);
        if marked_clauses.len() != folded_clauses.len() {
            return None;
        }
        let mut transformed = String::with_capacity(folded.len());
        for (index, (marked_clause, folded_clause)) in
            marked_clauses.into_iter().zip(folded_clauses).enumerate()
        {
            if index > 0 {
                transformed.push(' ');
                transformed.push_str(keyword);
                transformed.push(' ');
            }
            transformed.push('(');
            transformed.push_str(&predicate_with_runtime_integer_remainder_arithmetic(
                marked_clause,
                folded_clause,
            )?);
            transformed.push(')');
        }
        return Some(transformed);
    }

    for operators in [&["==", "!="][..], &["<=", ">=", "<", ">"][..]] {
        let Some((marked_left, operator, marked_right)) =
            split_top_level_operator_group(marked, operators)
        else {
            continue;
        };
        let (folded_left, folded_operator, folded_right) =
            split_top_level_operator_group(folded, operators)?;
        if operator != folded_operator {
            return None;
        }
        return Some(format!(
            "({}) {operator} ({})",
            predicate_with_runtime_integer_remainder_arithmetic(marked_left, folded_left)?,
            predicate_with_runtime_integer_remainder_arithmetic(marked_right, folded_right)?,
        ));
    }

    if let (Some(marked_inner), Some(folded_inner)) =
        (marked.strip_prefix("not "), folded.strip_prefix("not "))
    {
        return Some(format!(
            "not ({})",
            predicate_with_runtime_integer_remainder_arithmetic(marked_inner, folded_inner)?,
        ));
    }

    folded_remainder_numeric_expression(marked, folded).map(|expression| expression.text)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FoldedNumericType {
    Int,
    Float,
}

struct FoldedNumericExpression {
    text: String,
    ty: FoldedNumericType,
    contains_remainder: bool,
}

fn folded_remainder_numeric_expression(
    marked: &str,
    folded: &str,
) -> Option<FoldedNumericExpression> {
    let marked = strip_balanced_outer_parens(marked.trim());
    let folded = strip_balanced_outer_parens(folded.trim());

    for operators in [
        &["|"][..],
        &["^"][..],
        &["&"][..],
        &[">>>", ">>", "<<"][..],
        &["+", "-"][..],
        &["*", "/", "%"][..],
    ] {
        let Some((marked_left, operator, marked_right)) =
            split_top_level_operator_group(marked, operators)
        else {
            continue;
        };
        let Some((folded_left, folded_operator, folded_right)) =
            split_top_level_operator_group(folded, operators)
        else {
            break;
        };
        if operator != folded_operator {
            return None;
        }
        let left = folded_remainder_numeric_expression(marked_left, folded_left)?;
        let right = folded_remainder_numeric_expression(marked_right, folded_right)?;
        let contains_remainder = left.contains_remainder || right.contains_remainder;
        let ty = if matches!(operator, "|" | "^" | "&" | ">>>" | ">>" | "<<" | "%") {
            FoldedNumericType::Int
        } else if left.ty == FoldedNumericType::Float || right.ty == FoldedNumericType::Float {
            FoldedNumericType::Float
        } else {
            FoldedNumericType::Int
        };
        let text = format!("({} {operator} {})", left.text, right.text);
        let text = if ty == FoldedNumericType::Int && contains_remainder {
            static_integer_expression(&text)?.to_string()
        } else {
            text
        };
        return Some(FoldedNumericExpression {
            text,
            ty,
            contains_remainder,
        });
    }

    for prefix in ["-", "~"] {
        let (Some(marked_inner), Some(folded_inner)) =
            (marked.strip_prefix(prefix), folded.strip_prefix(prefix))
        else {
            continue;
        };
        let inner = folded_remainder_numeric_expression(marked_inner, folded_inner)?;
        let ty = if prefix == "~" {
            FoldedNumericType::Int
        } else {
            inner.ty
        };
        let text = format!("{prefix}({})", inner.text);
        let text = if ty == FoldedNumericType::Int && inner.contains_remainder {
            static_integer_expression(&text)?.to_string()
        } else {
            text
        };
        return Some(FoldedNumericExpression {
            text,
            ty,
            contains_remainder: inner.contains_remainder,
        });
    }

    if marked.contains('%') {
        return Some(FoldedNumericExpression {
            text: static_integer_expression(folded)?.to_string(),
            ty: FoldedNumericType::Int,
            contains_remainder: true,
        });
    }

    let ty = if is_float_literal(marked) {
        FoldedNumericType::Float
    } else {
        static_integer_expression(marked)?;
        FoldedNumericType::Int
    };
    Some(FoldedNumericExpression {
        text: folded.to_string(),
        ty,
        contains_remainder: false,
    })
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
            &flat_remainder_chain_value(predicate, chain_remainders)?
                .as_i64()?
                .to_string(),
        );
        cursor = chain.predicate_range.end;
    }
    folded.push_str(&predicate[cursor..]);
    Some(folded)
}

fn flat_remainder_chain_value(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<StaticNumber> {
    let first = remainders.first()?;
    let mut value = StaticNumber::integer(static_integer_expression(first.left_text(predicate))?);
    for remainder in remainders {
        let right =
            StaticNumber::integer(static_integer_expression(remainder.right_text(predicate))?);
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
        .any(|range| flat_remainder_chain_value(predicate, &remainders[range]).is_none())
}

mod boolean_cases;
mod boolean_core;
mod case_and_order;
mod contract_calls;
mod exact_numbers;
mod field_access;
mod literal_order;
mod predicate_remainder_typing;
mod predicate_shapes;
mod predicate_typing;
mod static_literals;

use boolean_cases::*;
use boolean_core::*;
use case_and_order::*;
pub(crate) use exact_numbers::{ExactNumber, ExactRational, parse_quoted_string_literal};
use field_access::*;
use literal_order::*;
use predicate_remainder_typing::*;
use predicate_shapes::*;
use predicate_typing::*;
use static_literals::*;

#[cfg(test)]
pub(crate) use boolean_core::{reset_static_boolean_work, take_static_boolean_work};
pub(crate) use contract_calls::{contract_calls, is_contract_keyword, referenced_names};
pub(crate) use predicate_remainder_typing::{
    PredicateRemainderTypingContext, missing_contract_field, predicate_is_boolean_with_calls,
    predicate_type_with_remainder_context,
};
#[cfg(test)]
pub(crate) use predicate_remainder_typing::{
    RemainderTypingIndexWork, reset_predicate_typing_work, reset_remainder_typing_index_work,
    take_predicate_typing_work, take_remainder_typing_index_work,
};
pub(crate) use predicate_typing::{
    predicate_has_invalid_remainder_operands, predicate_literal_is_zero,
    predicate_remainder_operand_mismatch, predicate_rendered_type_with_calls,
};
pub(crate) use static_literals::predicate_has_nonfoldable_remainder;

#[cfg(test)]
mod static_reasoning_tests;
