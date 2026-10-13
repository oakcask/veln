use std::collections::HashSet;

use super::*;

#[cfg(test)]
thread_local! {
    static PREDICATE_TYPING_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static REMAINDER_TYPING_INDEX_WORK: std::cell::Cell<RemainderTypingIndexWork> = const {
        std::cell::Cell::new(RemainderTypingIndexWork::new())
    };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RemainderTypingIndexWork {
    pub(crate) index_builds: usize,
    pub(crate) indexed_chains: usize,
    pub(crate) range_lookups: usize,
}

#[cfg(test)]
impl RemainderTypingIndexWork {
    const fn new() -> Self {
        Self {
            index_builds: 0,
            indexed_chains: 0,
            range_lookups: 0,
        }
    }
}

#[cfg(test)]
pub(super) fn record_predicate_typing_work(predicate: &str) {
    PREDICATE_TYPING_WORK.with(|work| work.set(work.get() + predicate.len()));
}

#[cfg(not(test))]
pub(super) fn record_predicate_typing_work(_predicate: &str) {}

#[cfg(test)]
pub(crate) fn reset_predicate_typing_work() {
    PREDICATE_TYPING_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_predicate_typing_work() -> usize {
    PREDICATE_TYPING_WORK.with(|work| work.replace(0))
}

#[cfg(test)]
pub(crate) fn reset_remainder_typing_index_work() {
    REMAINDER_TYPING_INDEX_WORK.with(|work| work.set(RemainderTypingIndexWork::new()));
}

#[cfg(test)]
pub(crate) fn take_remainder_typing_index_work() -> RemainderTypingIndexWork {
    REMAINDER_TYPING_INDEX_WORK.with(|work| work.replace(RemainderTypingIndexWork::new()))
}

pub(crate) struct PredicateRemainderTypingContext<'a> {
    root_predicate: &'a str,
    chain_ranges: HashSet<(usize, usize)>,
}

impl<'a> PredicateRemainderTypingContext<'a> {
    pub(crate) fn new(root_predicate: &'a str, remainders: &[PredicateRemainder]) -> Self {
        let chain_ranges = veln_ast::predicate_remainder_chain_ranges(remainders)
            .into_iter()
            .filter_map(|range| {
                let operation_range = remainders[range.end - 1].operation_range.clone();
                root_predicate
                    .get(operation_range.clone())
                    .map(|_| (operation_range.start, operation_range.end))
            })
            .collect::<HashSet<_>>();
        #[cfg(test)]
        REMAINDER_TYPING_INDEX_WORK.with(|work| {
            let mut next = work.get();
            next.index_builds += 1;
            next.indexed_chains += chain_ranges.len();
            work.set(next);
        });
        Self {
            root_predicate,
            chain_ranges,
        }
    }

    fn contains(&self, predicate: &str) -> bool {
        #[cfg(test)]
        REMAINDER_TYPING_INDEX_WORK.with(|work| {
            let mut next = work.get();
            next.range_lookups += 1;
            work.set(next);
        });
        let root_start = self.root_predicate.as_ptr() as usize;
        let start = (predicate.as_ptr() as usize).checked_sub(root_start);
        let Some(start) = start.filter(|start| *start <= self.root_predicate.len()) else {
            return false;
        };
        let Some(end) = start.checked_add(predicate.len()) else {
            return false;
        };
        end <= self.root_predicate.len() && self.chain_ranges.contains(&(start, end))
    }
}

pub(crate) fn missing_contract_field(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<(String, String)> {
    for access in field_accesses(predicate) {
        let Some(mut current) = predicate_type_with_calls(&access.base, bindings, call_type) else {
            continue;
        };
        for field in access.fields {
            let Some(next) = current.record_field(&field) else {
                return Some((current.render(), field));
            };
            current = next.clone();
        }
    }
    None
}

pub(crate) fn predicate_is_boolean_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> bool {
    let trimmed = predicate.trim();
    predicate_type_with_calls(trimmed, bindings, call_type).is_some_and(
        |ty| matches!(ty, Type::Named { name, args, .. } if name == "Bool" && args.is_empty()),
    )
}

pub(crate) fn predicate_type_with_remainder_context(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    predicate_type_with_flat_remainder_chains(predicate, context, bindings, call_type)
}

fn predicate_type_with_flat_remainder_chains(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    record_predicate_typing_work(predicate);
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if context.contains(predicate) {
        return Some(Type::int());
    }
    if let Some(ty) = predicate_literal_type(predicate) {
        return Some(ty);
    }
    if let Some(ty) =
        predicate_unary_type_with_flat_remainders(predicate, context, bindings, call_type)
    {
        return ty;
    }
    if let Some(ty) =
        predicate_boolean_type_with_flat_remainders(predicate, context, bindings, call_type)
    {
        return ty;
    }
    if let Some(ty) =
        predicate_comparison_type_with_flat_remainders(predicate, context, bindings, call_type)
    {
        return ty;
    }
    if let Some(ty) =
        predicate_bitwise_type_with_flat_remainders(predicate, context, bindings, call_type)
    {
        return ty;
    }
    if let Some(ty) =
        predicate_arithmetic_type_with_flat_remainders(predicate, context, bindings, call_type)
    {
        return ty;
    }
    predicate_field_access_type(predicate, bindings, call_type)
        .or_else(|| predicate_contract_call_type(predicate, call_type))
        .or_else(|| predicate_binding_type(predicate, bindings))
}

fn predicate_unary_type_with_flat_remainders(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Option<Type>> {
    if let Some(rest) = predicate.strip_prefix('-') {
        let ty = predicate_type_with_flat_remainder_chains(rest, context, bindings, call_type);
        return ty.filter(|ty| {
            matches!(ty, Type::Named { name, args, .. } if args.is_empty() && (name == "Int" || name == "Float"))
        }).map(Some);
    }
    if let Some(rest) = predicate.strip_prefix('~') {
        let ty = predicate_type_with_flat_remainder_chains(rest, context, bindings, call_type);
        return (ty == Some(Type::int())).then(|| Some(Type::int()));
    }
    let boolean_operand = predicate.strip_prefix("not ").or_else(|| {
        predicate
            .strip_prefix("not(")
            .and_then(|inner| inner.strip_suffix(')'))
    });
    boolean_operand.and_then(|operand| {
        (predicate_type_with_flat_remainder_chains(operand, context, bindings, call_type)
            == Some(Type::bool()))
        .then(|| Some(Type::bool()))
    })
}

fn predicate_boolean_type_with_flat_remainders(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Option<Type>> {
    for operator in ["or", "and"] {
        let clauses = split_top_level_keyword(predicate, operator);
        if clauses.len() > 1 {
            return Some(
                clauses
                    .into_iter()
                    .all(|clause| {
                        predicate_type_with_flat_remainder_chains(
                            clause, context, bindings, call_type,
                        ) == Some(Type::bool())
                    })
                    .then(Type::bool),
            );
        }
    }
    None
}

fn predicate_comparison_type_with_flat_remainders(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Option<Type>> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left =
                predicate_type_with_flat_remainder_chains(left, context, bindings, call_type);
            let right =
                predicate_type_with_flat_remainder_chains(right, context, bindings, call_type);
            return Some(match (left, right) {
                (Some(left), Some(right)) => {
                    comparable_predicate_operands(&left, &right).then(Type::bool)
                }
                _ => None,
            });
        }
    }
    None
}

fn predicate_bitwise_type_with_flat_remainders(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Option<Type>> {
    if contains_binary_bitwise_operator(predicate) {
        for operators in [&["|"][..], &["^"][..], &["&"][..], &[">>>", ">>", "<<"][..]] {
            for operator in operators {
                if let Some((left, right)) = split_top_level_operator(predicate, operator) {
                    let left = predicate_type_with_flat_remainder_chains(
                        left, context, bindings, call_type,
                    );
                    let right = predicate_type_with_flat_remainder_chains(
                        right, context, bindings, call_type,
                    );
                    return Some(
                        (left == Some(Type::int()) && right == Some(Type::int())).then(Type::int),
                    );
                }
            }
        }
    }
    None
}

fn predicate_arithmetic_type_with_flat_remainders(
    predicate: &str,
    context: &PredicateRemainderTypingContext<'_>,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Option<Type>> {
    for operators in [&["+", "-"][..], &["*", "/", "%"][..]] {
        if let Some((left, operator, right)) = split_top_level_operator_group(predicate, operators)
        {
            let left =
                predicate_type_with_flat_remainder_chains(left, context, bindings, call_type);
            let right =
                predicate_type_with_flat_remainder_chains(right, context, bindings, call_type);
            if operator == "%" {
                return Some(
                    (left == Some(Type::int()) && right == Some(Type::int())).then(Type::int),
                );
            }
            return Some(match (left, right) {
                (Some(left), Some(right)) => numeric_result_type(&left, &right),
                _ => None,
            });
        }
    }
    None
}
