use super::{LineComments, bool_match_rewrite, literal_match_rewrite};
use crate::{BodyLine, Expr, ExprKind, SyntaxItem, SyntaxTree};

pub(super) fn tree_has_commented_match_rewrite(tree: &SyntaxTree, comments: &LineComments) -> bool {
    tree.items.iter().any(|item| match item {
        SyntaxItem::Function(function) => function.body.iter().any(|line| match line {
            BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => {
                expr_has_commented_match_rewrite(expr, comments)
            }
        }),
        SyntaxItem::Schema(_) | SyntaxItem::Effect(_) | SyntaxItem::Handler(_) => false,
        SyntaxItem::Type(_) | SyntaxItem::Codec(_) | SyntaxItem::PublicAlias(_) => false,
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
    (literal_match_rewrite(scrutinee, arms).is_some() || bool_match_rewrite(arms).is_some())
        && comments.has_comment_in_span(&expr.span)
}

enum ExprChildren<'a> {
    None,
    One(&'a Expr),
    Pair(&'a Expr, &'a Expr),
    Slice(&'a [Expr]),
    HeadAndSlice(&'a Expr, &'a [Expr]),
    Record(&'a [crate::RecordField]),
    Dict(&'a [crate::DictEntry]),
    Match(&'a Expr, &'a [crate::MatchArm]),
    If {
        condition: &'a Expr,
        then_branch: &'a Expr,
        else_if_branches: &'a [crate::IfBranch],
        else_branch: &'a Expr,
    },
}

fn expr_children(expr: &Expr) -> ExprChildren<'_> {
    match &expr.kind {
        ExprKind::TypeApply { callee: child, .. }
        | ExprKind::SchemaEncode { value: child, .. }
        | ExprKind::FieldAccess { base: child, .. }
        | ExprKind::Try(child)
        | ExprKind::Prefix { expr: child, .. } => ExprChildren::One(child),
        ExprKind::SchemaDecode {
            input: left,
            base: right,
            ..
        }
        | ExprKind::Binary { left, right, .. } => ExprChildren::Pair(left, right),
        ExprKind::Perform { args, .. } | ExprKind::List(args) => ExprChildren::Slice(args),
        ExprKind::Call { callee: head, args }
        | ExprKind::Handle {
            body: head, args, ..
        } => ExprChildren::HeadAndSlice(head, args),
        ExprKind::Record(fields) => ExprChildren::Record(fields),
        ExprKind::Dict(entries) => ExprChildren::Dict(entries),
        ExprKind::Match { scrutinee, arms } => ExprChildren::Match(scrutinee, arms),
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => ExprChildren::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        },
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => ExprChildren::None,
    }
}

fn expr_children_have_commented_match_rewrite(expr: &Expr, comments: &LineComments) -> bool {
    match expr_children(expr) {
        ExprChildren::None => false,
        ExprChildren::One(child) => expr_has_commented_match_rewrite(child, comments),
        ExprChildren::Pair(left, right) => {
            expr_has_commented_match_rewrite(left, comments)
                || expr_has_commented_match_rewrite(right, comments)
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
    }
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
