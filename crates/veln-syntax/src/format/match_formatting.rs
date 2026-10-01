use super::patterns::format_pattern;
use super::source_layout::push_indent;
use crate::{BinaryOp, Expr, ExprKind, MatchArm, PatternKind};

pub(super) fn format_match_expr(
    scrutinee: &Expr,
    arms: &[crate::MatchArm],
    indent: usize,
    render_contextual: &impl Fn(&Expr, usize) -> String,
    render_plain: &impl Fn(&Expr, usize) -> String,
) -> String {
    let equivalent = |left: &Expr, right: &Expr| render_plain(left, 0) == render_plain(right, 0);
    if let Some(rewrite) = literal_match_rewrite(scrutinee, arms, &equivalent) {
        return format_literal_match_rewrite(&rewrite, indent, render_plain);
    }
    if let Some(rewrite) = bool_match_rewrite(arms) {
        return format_bool_match_rewrite(scrutinee, &rewrite, indent, render_plain, &equivalent);
    }

    let mut text = format!("match {}\n", render_contextual(scrutinee, indent));
    for arm in arms {
        push_indent(&mut text, indent + 1);
        text.push_str(&format_pattern(&arm.pattern));
        text.push_str(" => ");
        text.push_str(&render_contextual(&arm.expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

pub(super) struct LiteralMatchRewrite<'a> {
    scrutinee: &'a Expr,
    arms: Vec<(String, &'a Expr)>,
    fallback: &'a Expr,
}

pub(super) struct BoolMatchRewrite<'a> {
    true_expr: &'a Expr,
    false_expr: &'a Expr,
}

pub(super) fn bool_match_rewrite(arms: &[crate::MatchArm]) -> Option<BoolMatchRewrite<'_>> {
    let (true_arm, false_arm) = bool_match_arms(arms)?;
    Some(BoolMatchRewrite {
        true_expr: &true_arm.expr,
        false_expr: &false_arm.expr,
    })
}

pub(super) fn literal_match_rewrite<'a>(
    scrutinee: &'a Expr,
    arms: &'a [crate::MatchArm],
    equivalent: &impl Fn(&Expr, &Expr) -> bool,
) -> Option<LiteralMatchRewrite<'a>> {
    let mut literal_arms = Vec::new();
    let (rewritten_scrutinee, fallback) =
        collect_literal_match_chain(scrutinee, arms, None, &mut literal_arms, equivalent)?;
    let mut seen = std::collections::BTreeSet::new();
    if literal_arms
        .iter()
        .any(|(literal, _)| !seen.insert(literal.clone()))
    {
        return None;
    }

    Some(LiteralMatchRewrite {
        scrutinee: rewritten_scrutinee,
        arms: literal_arms,
        fallback,
    })
}

fn collect_literal_match_chain<'a>(
    condition: &'a Expr,
    arms: &'a [crate::MatchArm],
    expected_scrutinee: Option<&'a Expr>,
    literal_arms: &mut Vec<(String, &'a Expr)>,
    equivalent: &impl Fn(&Expr, &Expr) -> bool,
) -> Option<(&'a Expr, &'a Expr)> {
    let (true_arm, false_arm) = bool_match_arms(arms)?;
    let condition_literals = literal_match_conditions(condition)?;
    let active_scrutinee = condition_literals.first()?.0;

    if let Some(expected) = expected_scrutinee
        && !equivalent(expected, active_scrutinee)
    {
        return None;
    }
    if condition_literals
        .iter()
        .any(|(scrutinee, _)| !equivalent(active_scrutinee, scrutinee))
    {
        return None;
    }

    for (_, literal) in condition_literals {
        literal_arms.push((literal, &true_arm.expr));
    }

    if let ExprKind::Match {
        scrutinee: next_condition,
        arms: next_arms,
    } = &false_arm.expr.kind
    {
        let mut nested_arms = Vec::new();
        if let Some((_, fallback)) = collect_literal_match_chain(
            next_condition,
            next_arms,
            Some(active_scrutinee),
            &mut nested_arms,
            equivalent,
        ) {
            literal_arms.extend(nested_arms);
            return Some((active_scrutinee, fallback));
        }
    }

    Some((active_scrutinee, &false_arm.expr))
}

fn bool_match_arms(arms: &[MatchArm]) -> Option<(&MatchArm, &MatchArm)> {
    if arms.len() != 2 {
        return None;
    }

    let mut true_arm = None;
    let mut false_arm = None;
    for arm in arms {
        match arm.pattern.kind {
            PatternKind::BoolLiteral(true) if true_arm.is_none() => true_arm = Some(arm),
            PatternKind::BoolLiteral(false) if false_arm.is_none() => false_arm = Some(arm),
            _ => return None,
        }
    }

    Some((true_arm?, false_arm?))
}

fn literal_match_conditions(condition: &Expr) -> Option<Vec<(&Expr, String)>> {
    match &condition.kind {
        ExprKind::Binary {
            op: BinaryOp::Or,
            left,
            right,
        } => {
            let mut conditions = literal_match_conditions(left)?;
            conditions.extend(literal_match_conditions(right)?);
            Some(conditions)
        }
        ExprKind::Binary {
            op: BinaryOp::Equal,
            left,
            right,
        } => literal_equality_condition(left, right).map(|condition| vec![condition]),
        _ => None,
    }
}

fn literal_equality_condition<'a>(left: &'a Expr, right: &'a Expr) -> Option<(&'a Expr, String)> {
    if let Some(literal) = literal_pattern_text(right) {
        return Some((left, literal));
    }
    literal_pattern_text(left).map(|literal| (right, literal))
}

fn literal_pattern_text(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => Some(value.clone()),
        ExprKind::Unit => Some("()".to_string()),
        _ => None,
    }
}

fn format_literal_match_rewrite(
    rewrite: &LiteralMatchRewrite<'_>,
    indent: usize,
    render: &impl Fn(&Expr, usize) -> String,
) -> String {
    let mut text = format!("match {}\n", render(rewrite.scrutinee, indent));
    for (literal, expr) in &rewrite.arms {
        push_indent(&mut text, indent + 1);
        text.push_str(literal);
        text.push_str(" => ");
        text.push_str(&render(expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent + 1);
    text.push_str("_ => ");
    text.push_str(&render(rewrite.fallback, indent + 1));
    text.push('\n');
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_bool_match_rewrite(
    condition: &Expr,
    rewrite: &BoolMatchRewrite<'_>,
    indent: usize,
    render: &impl Fn(&Expr, usize) -> String,
    equivalent: &impl Fn(&Expr, &Expr) -> bool,
) -> String {
    let mut text = format!("if {}\n", render(condition, indent));
    push_indent(&mut text, indent + 1);
    text.push_str(&render(rewrite.true_expr, indent + 1));
    text.push('\n');
    format_bool_match_else(&mut text, rewrite.false_expr, indent, render, equivalent);
    text
}

fn format_bool_match_else(
    text: &mut String,
    false_expr: &Expr,
    indent: usize,
    render: &impl Fn(&Expr, usize) -> String,
    equivalent: &impl Fn(&Expr, &Expr) -> bool,
) {
    if let ExprKind::Match {
        scrutinee,
        arms: nested_arms,
    } = &false_expr.kind
        && literal_match_rewrite(scrutinee, nested_arms, equivalent).is_none()
        && let Some(rewrite) = bool_match_rewrite(nested_arms)
    {
        push_indent(text, indent);
        text.push_str("else if ");
        text.push_str(&render(scrutinee, indent));
        text.push('\n');
        push_indent(text, indent + 1);
        text.push_str(&render(rewrite.true_expr, indent + 1));
        text.push('\n');
        format_bool_match_else(text, rewrite.false_expr, indent, render, equivalent);
        return;
    }

    push_indent(text, indent);
    text.push_str("else\n");
    push_indent(text, indent + 1);
    text.push_str(&render(false_expr, indent + 1));
    text.push('\n');
    push_indent(text, indent);
    text.push_str("end");
}
