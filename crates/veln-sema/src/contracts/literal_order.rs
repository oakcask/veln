use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NumericLiteralBoundKind {
    Lower,
    Upper,
}

pub(super) struct NumericLiteralBound {
    pub(super) subject: String,
    pub(super) value: StaticRational,
    pub(super) inclusive: bool,
    pub(super) kind: NumericLiteralBoundKind,
}

pub(super) fn has_exclusive_numeric_literal_bounds_top_level_and(predicate: &str) -> bool {
    has_complementary_numeric_literal_bounds(predicate, "and", literal_bounds_do_not_overlap)
}

pub(super) fn has_covering_numeric_literal_bounds_top_level_or(predicate: &str) -> bool {
    has_complementary_numeric_literal_bounds(predicate, "or", literal_bounds_cover_all_values)
}

fn has_complementary_numeric_literal_bounds(
    predicate: &str,
    keyword: &str,
    relationship: impl Fn(&NumericLiteralBound, &NumericLiteralBound) -> bool,
) -> bool {
    let bounds = flattened_keyword_clauses(predicate, keyword)
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .collect::<Vec<_>>();
    if bounds.len() < 2 {
        return false;
    }

    bounds.iter().enumerate().any(|(index, left)| {
        bounds.iter().skip(index + 1).any(|right| {
            left.subject == right.subject
                && matches!(
                    (left.kind, right.kind),
                    (
                        NumericLiteralBoundKind::Lower,
                        NumericLiteralBoundKind::Upper
                    ) | (
                        NumericLiteralBoundKind::Upper,
                        NumericLiteralBoundKind::Lower
                    )
                )
                && relationship(left, right)
        })
    })
}

pub(super) fn numeric_literal_bound_shape(predicate: &str) -> Option<NumericLiteralBound> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["<=", ">=", "<", ">"] {
        let Some((left, right)) = split_top_level_operator(predicate, operator) else {
            continue;
        };
        let left_value = static_rational_expression(left);
        let right_value = static_rational_expression(right);
        match (left_value, right_value) {
            (Some(value), None) => {
                let kind = match operator {
                    "<" | "<=" => NumericLiteralBoundKind::Lower,
                    ">" | ">=" => NumericLiteralBoundKind::Upper,
                    _ => return None,
                };
                return Some(NumericLiteralBound {
                    subject: compact_predicate_text(right),
                    value,
                    inclusive: matches!(operator, "<=" | ">="),
                    kind,
                });
            }
            (None, Some(value)) => {
                let kind = match operator {
                    "<" | "<=" => NumericLiteralBoundKind::Upper,
                    ">" | ">=" => NumericLiteralBoundKind::Lower,
                    _ => return None,
                };
                return Some(NumericLiteralBound {
                    subject: compact_predicate_text(left),
                    value,
                    inclusive: matches!(operator, "<=" | ">="),
                    kind,
                });
            }
            _ => {}
        }
    }
    None
}

pub(super) fn literal_bounds_do_not_overlap(
    left: &NumericLiteralBound,
    right: &NumericLiteralBound,
) -> bool {
    let Some((lower, upper)) = oriented_literal_bounds(left, right) else {
        return false;
    };
    lower.value > upper.value
        || (lower.value == upper.value && (!lower.inclusive || !upper.inclusive))
}

pub(super) fn literal_bounds_cover_all_values(
    left: &NumericLiteralBound,
    right: &NumericLiteralBound,
) -> bool {
    let Some((lower, upper)) = oriented_literal_bounds(left, right) else {
        return false;
    };
    lower.value < upper.value
        || (lower.value == upper.value && (lower.inclusive || upper.inclusive))
}

fn oriented_literal_bounds<'a>(
    left: &'a NumericLiteralBound,
    right: &'a NumericLiteralBound,
) -> Option<(&'a NumericLiteralBound, &'a NumericLiteralBound)> {
    match (left.kind, right.kind) {
        (NumericLiteralBoundKind::Lower, NumericLiteralBoundKind::Upper) => Some((left, right)),
        (NumericLiteralBoundKind::Upper, NumericLiteralBoundKind::Lower) => Some((right, left)),
        _ => None,
    }
}

pub(super) struct LiteralEqualityShape {
    pub(super) subject: String,
    pub(super) value: StaticLiteral,
}

pub(super) fn has_exclusive_literal_equalities_top_level_and(predicate: &str) -> bool {
    let equalities: Vec<_> = flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(literal_equality_shape)
        .collect();

    equalities.iter().enumerate().any(|(index, left)| {
        equalities[index + 1..]
            .iter()
            .any(|right| left.subject == right.subject && left.value != right.value)
    })
}

pub(super) fn literal_equality_shape(predicate: &str) -> Option<LiteralEqualityShape> {
    let (left, right) = split_top_level_operator(predicate, "==")?;
    literal_equality_shape_from_parts(left, right)
        .or_else(|| literal_equality_shape_from_parts(right, left))
}

pub(super) fn literal_equality_shape_from_parts(
    subject: &str,
    value: &str,
) -> Option<LiteralEqualityShape> {
    if StaticLiteral::parse(subject.trim()).is_some() {
        return None;
    }
    Some(LiteralEqualityShape {
        subject: compact_predicate_text(subject),
        value: StaticLiteral::parse(value.trim())?,
    })
}

pub(super) struct OrderBoundShape {
    pub(super) left: String,
    pub(super) right: String,
    pub(super) strict: bool,
}

pub(super) fn order_bound_shape(predicate: &str) -> Option<OrderBoundShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let mut left = compact_predicate_text(left);
            let mut right = compact_predicate_text(right);
            if left == right {
                return None;
            }
            if matches!(operator, ">" | ">=") {
                std::mem::swap(&mut left, &mut right);
            }
            return Some(OrderBoundShape {
                left,
                right,
                strict: matches!(operator, "<" | ">"),
            });
        }
    }
    None
}

pub(super) fn has_order_bound_transitive_implication_top_level_or(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 2 {
        return false;
    }

    disjuncts.iter().any(|antecedent| {
        let Some(inner) = negated_predicate_inner(antecedent) else {
            return false;
        };
        order_bound_antecedent_implies_any_consequent(inner, antecedent, &disjuncts)
    })
}

pub(super) fn order_bound_antecedent_implies_any_consequent(
    antecedent_inner: &str,
    antecedent: &str,
    disjuncts: &[&str],
) -> bool {
    let branches = flattened_keyword_clauses(antecedent_inner, "or");
    if branches.len() > 1 {
        return disjuncts.iter().any(|consequent| {
            !same_predicate(antecedent, consequent)
                && branches.iter().all(|branch| {
                    order_bound_branch_implies_consequent(branch, consequent, antecedent, disjuncts)
                })
        });
    }

    disjuncts.iter().any(|consequent| {
        !same_predicate(antecedent, consequent)
            && order_bound_branch_implies_consequent(
                antecedent_inner,
                consequent,
                antecedent,
                disjuncts,
            )
    })
}

pub(super) fn order_bound_branch_implies_consequent(
    branch: &str,
    consequent: &str,
    antecedent: &str,
    disjuncts: &[&str],
) -> bool {
    let edges: Vec<_> = flattened_keyword_clauses(branch, "and")
        .into_iter()
        .flat_map(order_bound_transitive_edges)
        .collect();
    if edges.is_empty() {
        return false;
    }
    order_bound_shape(consequent).is_some_and(|wanted| {
        order_bound_edges_imply(&edges, &wanted.left, &wanted.right, wanted.strict)
            || numeric_literal_bounds_imply(branch, &wanted)
    }) || equality_shape(consequent).is_some_and(|(left, right)| {
        order_bound_edges_imply_non_strict(&edges, &left, &right)
            && order_bound_edges_imply_non_strict(&edges, &right, &left)
    }) || disequality_shape(consequent).is_some_and(|(left, right)| {
        order_bound_edges_imply(&edges, &left, &right, true)
            || order_bound_edges_imply(&edges, &right, &left, true)
            || equality_disequality_edges_imply_disequality(branch, &left, &right)
            || numeric_literal_bounds_imply_disequality(branch, &left, &right)
    }) || order_bound_edges_imply_strict_or_equality_disjunction(disjuncts, antecedent, &edges)
}

pub(super) fn order_bound_edges_imply_strict_or_equality_disjunction(
    disjuncts: &[&str],
    antecedent: &str,
    edges: &[OrderBoundShape],
) -> bool {
    disjuncts
        .iter()
        .filter(|disjunct| !same_predicate(antecedent, disjunct))
        .filter_map(|disjunct| order_bound_shape(disjunct).filter(|bound| bound.strict))
        .any(|strict_bound| {
            order_bound_edges_imply(edges, &strict_bound.left, &strict_bound.right, false)
                && disjuncts
                    .iter()
                    .filter(|disjunct| !same_predicate(antecedent, disjunct))
                    .filter_map(|disjunct| equality_shape(disjunct))
                    .any(|(left, right)| {
                        (left == strict_bound.left && right == strict_bound.right)
                            || (left == strict_bound.right && right == strict_bound.left)
                    })
        })
}

pub(super) fn numeric_literal_bounds_imply(predicate: &str, wanted: &OrderBoundShape) -> bool {
    let Some(wanted) = numeric_literal_bound_shape_from_order_bound(wanted) else {
        return false;
    };
    let equality_edges = equality_edges(predicate);
    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .any(|required| {
            numeric_literal_bound_implies(&required, &wanted)
                || (equality_edges_imply(&equality_edges, &required.subject, &wanted.subject)
                    && numeric_literal_bound_strength_implies(&required, &wanted))
        })
}

pub(super) fn numeric_literal_bound_shape_from_order_bound(
    bound: &OrderBoundShape,
) -> Option<NumericLiteralBound> {
    numeric_literal_bound_shape(&format!(
        "{} {} {}",
        bound.left,
        if bound.strict { "<" } else { "<=" },
        bound.right
    ))
}

pub(super) fn numeric_literal_bound_implies(
    required: &NumericLiteralBound,
    wanted: &NumericLiteralBound,
) -> bool {
    required.subject == wanted.subject && numeric_literal_bound_strength_implies(required, wanted)
}

pub(super) fn numeric_literal_bounds_imply_disequality(
    predicate: &str,
    left: &str,
    right: &str,
) -> bool {
    let equality_edges = equality_edges(predicate);
    let Some((wanted_subject, wanted_value)) =
        numeric_literal_disequality_subject_value(left, right)
    else {
        return false;
    };

    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .any(|required| {
            equality_edges_imply(&equality_edges, &required.subject, &wanted_subject)
                && numeric_literal_bound_excludes_value(&required, wanted_value)
        })
}

fn equality_edges(predicate: &str) -> Vec<(String, String)> {
    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(equality_shape)
        .flat_map(|(left, right)| [(left.clone(), right.clone()), (right, left)])
        .collect()
}

pub(super) fn numeric_literal_disequality_subject_value(
    left: &str,
    right: &str,
) -> Option<(String, StaticRational)> {
    static_rational_expression(left)
        .map(|value| (compact_predicate_text(right), value))
        .or_else(|| {
            static_rational_expression(right).map(|value| (compact_predicate_text(left), value))
        })
}

pub(super) fn numeric_literal_bound_excludes_value(
    required: &NumericLiteralBound,
    wanted_value: StaticRational,
) -> bool {
    match required.kind {
        NumericLiteralBoundKind::Lower => {
            wanted_value < required.value || (wanted_value == required.value && !required.inclusive)
        }
        NumericLiteralBoundKind::Upper => {
            wanted_value > required.value || (wanted_value == required.value && !required.inclusive)
        }
    }
}

pub(super) fn numeric_literal_bound_strength_implies(
    required: &NumericLiteralBound,
    wanted: &NumericLiteralBound,
) -> bool {
    required.kind == wanted.kind
        && match required.kind {
            NumericLiteralBoundKind::Lower => {
                required.value > wanted.value
                    || (required.value == wanted.value && (wanted.inclusive || !required.inclusive))
            }
            NumericLiteralBoundKind::Upper => {
                required.value < wanted.value
                    || (required.value == wanted.value && (wanted.inclusive || !required.inclusive))
            }
        }
}

pub(super) fn equality_shape(predicate: &str) -> Option<(String, String)> {
    let (left, right) = split_top_level_operator(predicate, "==")?;
    let left = compact_predicate_text(left);
    let right = compact_predicate_text(right);
    (left != right).then_some((left, right))
}

pub(super) fn disequality_shape(predicate: &str) -> Option<(String, String)> {
    let (left, right) = split_top_level_operator(predicate, "!=")?;
    let left = compact_predicate_text(left);
    let right = compact_predicate_text(right);
    (left != right).then_some((left, right))
}

pub(super) fn equality_disequality_edges_imply_disequality(
    predicate: &str,
    left: &str,
    right: &str,
) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    let equality_edges: Vec<_> = clauses
        .iter()
        .filter_map(|clause| equality_shape(clause))
        .flat_map(|(left, right)| [(left.clone(), right.clone()), (right, left)])
        .collect();
    let disequalities: Vec<_> = clauses
        .iter()
        .filter_map(|clause| disequality_shape(clause))
        .collect();

    disequalities.iter().any(|(disequal_left, disequal_right)| {
        (equality_edges_imply(&equality_edges, left, disequal_left)
            && equality_edges_imply(&equality_edges, right, disequal_right))
            || (equality_edges_imply(&equality_edges, left, disequal_right)
                && equality_edges_imply(&equality_edges, right, disequal_left))
    })
}

pub(super) fn equality_edges_imply(edges: &[(String, String)], left: &str, right: &str) -> bool {
    graph_edges_imply(edges, left, right)
}

trait GraphEdge {
    fn endpoints(&self) -> Option<(&str, &str)>;
}

impl GraphEdge for (String, String) {
    fn endpoints(&self) -> Option<(&str, &str)> {
        Some((&self.0, &self.1))
    }
}

impl GraphEdge for OrderBoundShape {
    fn endpoints(&self) -> Option<(&str, &str)> {
        (!self.strict).then_some((&self.left, &self.right))
    }
}

fn graph_edges_imply(edges: &[impl GraphEdge], left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    let mut stack = vec![left.to_string()];
    let mut visited = Vec::new();

    while let Some(current) = stack.pop() {
        if visited.iter().any(|node| node == &current) {
            continue;
        }
        visited.push(current.clone());

        for (_, edge_right) in edges
            .iter()
            .filter_map(|edge| edge.endpoints())
            .filter(|(edge_left, _)| edge_left == &current)
        {
            if edge_right == right {
                return true;
            }
            stack.push(edge_right.to_string());
        }
    }

    false
}

pub(super) fn order_bound_transitive_edges(predicate: &str) -> Vec<OrderBoundShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some((left, right)) = split_top_level_operator(predicate, "==") {
        let left = compact_predicate_text(left);
        let right = compact_predicate_text(right);
        if left == right {
            return Vec::new();
        }
        return vec![
            OrderBoundShape {
                left: left.clone(),
                right: right.clone(),
                strict: false,
            },
            OrderBoundShape {
                left: right,
                right: left,
                strict: false,
            },
        ];
    }
    order_bound_shape(predicate).into_iter().collect()
}

pub(super) fn order_bound_edges_imply(
    edges: &[OrderBoundShape],
    left: &str,
    right: &str,
    strict: bool,
) -> bool {
    let mut stack = vec![(left.to_string(), false)];
    let mut visited = Vec::new();

    while let Some((current, path_strict)) = stack.pop() {
        if visited
            .iter()
            .any(|(node, strictness)| node == &current && *strictness == path_strict)
        {
            continue;
        }
        visited.push((current.clone(), path_strict));

        for edge in edges.iter().filter(|edge| edge.left == current) {
            let next_strict = path_strict || edge.strict;
            if edge.right == right && (!strict || next_strict) {
                return true;
            }
            stack.push((edge.right.clone(), next_strict));
        }
    }

    false
}

pub(super) fn order_bound_edges_imply_non_strict(
    edges: &[OrderBoundShape],
    left: &str,
    right: &str,
) -> bool {
    graph_edges_imply(edges, left, right)
}
