use super::expressions::format_expr_at_indent;
use super::{LineComments, bool_match_rewrite, literal_match_rewrite};
use crate::{BodyLine, Expr, ExprChildren, ExprKind, SyntaxItem, SyntaxTree};

pub(super) fn tree_has_commented_match_rewrite(tree: &SyntaxTree, comments: &LineComments) -> bool {
    tree.items.iter().any(|item| match item {
        SyntaxItem::Function(function) => function.body.iter().any(|line| match line {
            BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => {
                expr_has_commented_match_rewrite(expr, comments)
            }
            BodyLine::Defer { body, .. } => body_lines_have_commented_match_rewrite(body, comments),
        }),
        SyntaxItem::Schema(_) | SyntaxItem::Effect(_) | SyntaxItem::Handler(_) => false,
        SyntaxItem::Type(_) | SyntaxItem::PublicAlias(_) => false,
    })
}

fn expr_has_commented_match_rewrite(expr: &Expr, comments: &LineComments) -> bool {
    expr_is_commented_match_rewrite(expr, comments)
        || expr_children_have_commented_match_rewrite(expr, comments)
}

fn expr_is_commented_match_rewrite(expr: &Expr, comments: &LineComments) -> bool {
    let ExprKind::Match { scrutinee, arms } = &expr.kind else {
        return false;
    };
    (literal_match_rewrite(scrutinee, arms, &|left, right| {
        format_expr_at_indent(left, 0) == format_expr_at_indent(right, 0)
    })
    .is_some()
        || bool_match_rewrite(arms).is_some())
        && comments.has_comment_in_span(&expr.span)
}

fn expr_children_have_commented_match_rewrite(expr: &Expr, comments: &LineComments) -> bool {
    match expr.children() {
        ExprChildren::None => false,
        ExprChildren::One { child, .. } => expr_has_commented_match_rewrite(child, comments),
        ExprChildren::Pair { first, second, .. } => {
            expr_has_commented_match_rewrite(first, comments)
                || expr_has_commented_match_rewrite(second, comments)
        }
        ExprChildren::Slice(children) => expr_slice_has_commented_match_rewrite(children, comments),
        ExprChildren::HeadAndSlice(head, children) => {
            expr_has_commented_match_rewrite(head, comments)
                || expr_slice_has_commented_match_rewrite(children, comments)
        }
        ExprChildren::Record(fields) => record_has_commented_match_rewrite(fields, comments),
        ExprChildren::Dict(entries) => dict_has_commented_match_rewrite(entries, comments),
        ExprChildren::Match(scrutinee, arms) => {
            match_has_commented_match_rewrite(scrutinee, arms, comments)
        }
        ExprChildren::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => if_has_commented_match_rewrite(
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            comments,
        ),
        ExprChildren::BeginBody(body) => body_lines_have_commented_match_rewrite(body, comments),
    }
}

fn body_lines_have_commented_match_rewrite(body: &[BodyLine], comments: &LineComments) -> bool {
    body.iter().any(|line| match line {
        BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => {
            expr_has_commented_match_rewrite(expr, comments)
        }
        BodyLine::Defer { body, .. } => body_lines_have_commented_match_rewrite(body, comments),
    })
}

fn expr_slice_has_commented_match_rewrite(exprs: &[Expr], comments: &LineComments) -> bool {
    exprs
        .iter()
        .any(|expr| expr_has_commented_match_rewrite(expr, comments))
}

fn record_has_commented_match_rewrite(
    fields: &[crate::RecordField],
    comments: &LineComments,
) -> bool {
    fields
        .iter()
        .any(|field| expr_has_commented_match_rewrite(&field.expr, comments))
}

fn dict_has_commented_match_rewrite(entries: &[crate::DictEntry], comments: &LineComments) -> bool {
    entries.iter().any(|entry| {
        expr_has_commented_match_rewrite(&entry.key, comments)
            || expr_has_commented_match_rewrite(&entry.value, comments)
    })
}

fn match_has_commented_match_rewrite(
    scrutinee: &Expr,
    arms: &[crate::MatchArm],
    comments: &LineComments,
) -> bool {
    expr_has_commented_match_rewrite(scrutinee, comments)
        || arms
            .iter()
            .any(|arm| expr_has_commented_match_rewrite(&arm.expr, comments))
}

fn if_has_commented_match_rewrite(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[crate::IfBranch],
    else_branch: &Expr,
    comments: &LineComments,
) -> bool {
    expr_has_commented_match_rewrite(condition, comments)
        || expr_has_commented_match_rewrite(then_branch, comments)
        || else_if_branches.iter().any(|branch| {
            expr_has_commented_match_rewrite(&branch.condition, comments)
                || expr_has_commented_match_rewrite(&branch.expr, comments)
        })
        || expr_has_commented_match_rewrite(else_branch, comments)
}
