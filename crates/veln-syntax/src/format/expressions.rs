use super::cleanup::{
    format_cleanup_region, format_cleanup_region_with_comments,
    format_defer_statement_with_comments as format_cleanup_defer_with_comments,
};
use super::match_formatting::format_match_expr;
use super::*;
use crate::ExprChildren;

pub(super) fn format_expr_at_indent(expr: &Expr, indent: usize) -> String {
    format_expr_at_indent_ctx(expr, indent, None)
}

pub(super) fn format_expr_at_indent_with_comments(
    expr: &Expr,
    indent: usize,
    comments: &LineComments,
) -> String {
    let mut continued_comments = Vec::new();
    take_continued_begin_comments(expr, false, comments, &mut continued_comments);
    let mut text = format_expr_at_indent_ctx(expr, indent, Some(comments));
    for comment in continued_comments {
        text.push_str("  ");
        text.push_str(&comment);
    }
    text
}

pub(super) fn format_defer_statement_with_comments(
    body: &[BodyLine],
    span: &veln_source::SourceSpan,
    indent: usize,
    comments: &LineComments,
) -> String {
    format_cleanup_defer_with_comments(body, span, indent, comments, &|expr, child_indent| {
        format_expr_at_indent_with_comments(expr, child_indent, comments)
    })
}

fn take_continued_begin_comments(
    expr: &Expr,
    has_continuation: bool,
    comments: &LineComments,
    continued_comments: &mut Vec<String>,
) {
    if matches!(expr.kind, ExprKind::Begin { .. }) {
        if has_continuation {
            continued_comments.extend(comments.take_after(expr_end_line(expr)));
        }
        return;
    }

    let mut visit = |child: &Expr, parent_emits_after_child: bool| {
        take_continued_begin_comments(
            child,
            has_continuation || parent_emits_after_child,
            comments,
            continued_comments,
        );
    };
    match expr.children() {
        ExprChildren::None | ExprChildren::BeginBody(_) => {}
        ExprChildren::One {
            child,
            followed_by_parent_syntax,
        } => visit(child, followed_by_parent_syntax),
        ExprChildren::Pair {
            first,
            first_followed_by_parent_syntax,
            second,
            second_followed_by_parent_syntax,
        } => {
            visit(first, first_followed_by_parent_syntax);
            visit(second, second_followed_by_parent_syntax);
        }
        ExprChildren::Slice(children) => {
            for child in children {
                visit(child, true);
            }
        }
        ExprChildren::HeadAndSlice(head, children) => {
            visit(head, true);
            for child in children {
                visit(child, true);
            }
        }
        ExprChildren::Record(fields) => {
            for field in fields {
                visit(&field.expr, true);
            }
        }
        ExprChildren::Dict(entries) => {
            for entry in entries {
                visit(&entry.key, true);
                visit(&entry.value, true);
            }
        }
        ExprChildren::Match(scrutinee, arms) => {
            visit(scrutinee, true);
            for arm in arms {
                visit(&arm.expr, true);
            }
        }
        ExprChildren::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            visit(condition, true);
            visit(then_branch, true);
            for branch in else_if_branches {
                visit(&branch.condition, true);
                visit(&branch.expr, true);
            }
            visit(else_branch, true);
        }
    }
}

fn expr_end_line(expr: &Expr) -> usize {
    if expr.span.end.column == 1 {
        expr.span.end.line.saturating_sub(1)
    } else {
        expr.span.end.line
    }
}

pub(super) fn format_expr_at_indent_ctx(
    expr: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    format_expr_prec(expr, 0, ExprSide::Root, indent, comments)
}

#[derive(Clone, Copy)]
enum ExprSide {
    Root,
    Left,
    Right,
}

fn format_expr_prec(
    expr: &Expr,
    parent_prec: u8,
    side: ExprSide,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let prec = expr_prec(expr);
    let mut rendered = format_expr_inner(expr, prec, indent, comments);

    let needs_parens = match side {
        ExprSide::Root | ExprSide::Left => prec < parent_prec,
        ExprSide::Right => prec <= parent_prec && matches!(expr.kind, ExprKind::Binary { .. }),
    };
    if needs_parens {
        rendered.insert(0, '(');
        rendered.push(')');
    }
    rendered
}

fn format_expr_inner(
    expr: &Expr,
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match &expr.kind {
        kind @ (ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit) => format_atomic_expr(kind),
        kind @ (ExprKind::TypeApply { .. }
        | ExprKind::Call { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try { .. }) => format_application_expr(kind, prec, indent, comments),
        kind @ (ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }) => format_keyword_expr(kind, indent, comments),
        kind @ (ExprKind::Record(_) | ExprKind::Dict(_) | ExprKind::List(_)) => {
            format_collection_expr(kind, indent, comments)
        }
        kind @ (ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Begin { .. }) => {
            format_control_expr(expr, kind, indent, comments)
        }
        kind @ (ExprKind::Prefix { .. } | ExprKind::Binary { .. }) => {
            format_operator_expr(kind, prec, indent, comments)
        }
    }
}

fn format_atomic_expr(kind: &ExprKind) -> String {
    match kind {
        ExprKind::Missing => "_".to_string(),
        ExprKind::Hole { name, satisfy } => format_hole_expr(name.as_deref(), satisfy.as_ref()),
        ExprKind::NamePath { segments, .. } => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        _ => unreachable!("atomic expression formatter received a non-atomic expression"),
    }
}

fn format_application_expr(
    kind: &ExprKind,
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match kind {
        ExprKind::TypeApply {
            callee,
            type_args,
            surplus_closers,
            ..
        } => {
            let type_args = type_args
                .iter()
                .map(|arg| canonical_type_text(arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}<{}>{}",
                format_expr_at_indent_ctx(callee, indent, comments),
                type_args,
                ">".repeat(*surplus_closers)
            )
        }
        ExprKind::Call { callee, args } => format_call_expr(callee, args, prec, indent, comments),
        ExprKind::FieldAccess { base, field, .. } => {
            format!(
                "{}.{field}",
                format_expr_prec(base, prec, ExprSide::Left, indent, comments)
            )
        }
        ExprKind::Try { expr: inner, .. } => {
            format!(
                "{}?",
                format_expr_prec(inner, prec, ExprSide::Left, indent, comments)
            )
        }
        _ => unreachable!("application formatter received a non-application expression"),
    }
}

fn format_keyword_expr(kind: &ExprKind, indent: usize, comments: Option<&LineComments>) -> String {
    match kind {
        ExprKind::Perform {
            effect,
            operation,
            args,
            ..
        } => {
            let args = format_expr_args(args, indent, comments);
            format!("perform {}::{}({args})", effect.join("::"), operation)
        }
        ExprKind::Handle {
            body,
            handler,
            args,
            ..
        } => {
            let args = format_expr_args(args, indent, comments);
            format!(
                "handle {} with {}({args})",
                format_expr_at_indent_ctx(body, indent, comments),
                handler.join("::")
            )
        }
        ExprKind::SchemaDecode {
            schema,
            input,
            base,
            ..
        } => format!(
            "decode {} from {} at {}",
            schema.join("::"),
            format_expr_at_indent_ctx(input, indent, comments),
            format_expr_at_indent_ctx(base, indent, comments)
        ),
        ExprKind::SchemaEncode { schema, value, .. } => format!(
            "encode {} from {}",
            schema.join("::"),
            format_expr_at_indent_ctx(value, indent, comments)
        ),
        _ => unreachable!("keyword formatter received a non-keyword expression"),
    }
}

fn format_collection_expr(
    kind: &ExprKind,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match kind {
        ExprKind::Record(fields) => format_record_expr(fields, indent, comments),
        ExprKind::Dict(entries) => format_dict_expr(entries, indent, comments),
        ExprKind::List(items) => format_list_expr(items, indent, comments),
        _ => unreachable!("collection formatter received a non-collection expression"),
    }
}

fn format_control_expr(
    expr: &Expr,
    kind: &ExprKind,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match kind {
        ExprKind::Match { scrutinee, arms } => format_match_expr(
            scrutinee,
            arms,
            indent,
            &|child, child_indent| format_expr_at_indent_ctx(child, child_indent, comments),
            &format_expr_at_indent,
        ),
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => format_if_expr(
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            indent,
            comments,
        ),
        ExprKind::Begin { body, .. } => match comments {
            Some(comments) => format_cleanup_region_with_comments(
                "begin",
                body,
                &expr.span,
                indent,
                comments,
                &|child, child_indent| {
                    format_expr_at_indent_with_comments(child, child_indent, comments)
                },
            ),
            None => format_cleanup_region("begin", body, indent, &format_expr_at_indent),
        },
        _ => unreachable!("control formatter received a non-control expression"),
    }
}

fn format_operator_expr(
    kind: &ExprKind,
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match kind {
        ExprKind::Prefix { op, expr: inner } => {
            format_prefix_expr(*op, inner, prec, indent, comments)
        }
        ExprKind::Binary { op, left, right } => {
            format_binary_expr(*op, left, right, prec, indent, comments)
        }
        _ => unreachable!("operator formatter received a non-operator expression"),
    }
}

fn format_hole_expr(name: Option<&str>, satisfy: Option<&crate::SatisfyClause>) -> String {
    let mut text = String::from("_");
    if let Some(name) = name {
        text.push_str(name);
    }
    if let Some(satisfy) = satisfy {
        text.push_str(" satisfy");
        if let Some(candidate) = &satisfy.candidate {
            text.push(' ');
            text.push_str(candidate);
        }
        if !satisfy.predicate.is_empty() {
            text.push_str(" => ");
            text.push_str(&satisfy.predicate);
        }
    }
    text
}

fn format_call_expr(
    callee: &Expr,
    args: &[Expr],
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let args = format_expr_args(args, indent, comments);
    format!(
        "{}({args})",
        format_expr_prec(callee, prec, ExprSide::Left, indent, comments)
    )
}

fn format_expr_args(args: &[Expr], indent: usize, comments: Option<&LineComments>) -> String {
    args.iter()
        .map(|arg| format_expr_at_indent_ctx(arg, indent, comments))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_record_expr(
    fields: &[crate::RecordField],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }
    let fields = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                field.name,
                format_expr_at_indent_ctx(&field.expr, indent, comments)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {fields} }}")
}

fn format_dict_expr(
    entries: &[crate::DictEntry],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let entries = entries
        .iter()
        .map(|entry| {
            format!(
                "{}: {}",
                format_expr_at_indent_ctx(&entry.key, indent, comments),
                format_expr_at_indent_ctx(&entry.value, indent, comments)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {entries} }}")
}

fn format_list_expr(items: &[Expr], indent: usize, comments: Option<&LineComments>) -> String {
    let items = items
        .iter()
        .map(|item| format_expr_at_indent_ctx(item, indent, comments))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

fn format_if_expr(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[crate::IfBranch],
    else_branch: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let mut text = format!(
        "if {}\n",
        format_expr_at_indent_ctx(condition, indent, comments)
    );
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent_ctx(
        then_branch,
        indent + 1,
        comments,
    ));
    text.push('\n');
    for branch in else_if_branches {
        push_indent(&mut text, indent);
        text.push_str("else if ");
        text.push_str(&format_expr_at_indent_ctx(
            &branch.condition,
            indent,
            comments,
        ));
        text.push('\n');
        push_indent(&mut text, indent + 1);
        text.push_str(&format_expr_at_indent_ctx(
            &branch.expr,
            indent + 1,
            comments,
        ));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("else\n");
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent_ctx(
        else_branch,
        indent + 1,
        comments,
    ));
    text.push('\n');
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_prefix_expr(
    op: PrefixOp,
    inner: &Expr,
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    match op {
        PrefixOp::Not => format!(
            "not {}",
            format_expr_prec(inner, prec, ExprSide::Right, indent, comments)
        ),
        PrefixOp::Negate => format!(
            "-{}",
            format_expr_prec(inner, prec, ExprSide::Right, indent, comments)
        ),
        PrefixOp::BitwiseNot => format!(
            "~{}",
            format_expr_prec(inner, prec, ExprSide::Right, indent, comments)
        ),
    }
}

fn format_binary_expr(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    prec: u8,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let op_text = binary_op_text(op);
    format!(
        "{} {op_text} {}",
        format_expr_prec(left, prec, ExprSide::Left, indent, comments),
        format_expr_prec(right, prec, ExprSide::Right, indent, comments)
    )
}

fn expr_prec(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => match op {
            BinaryOp::PipeGreater => 1,
            BinaryOp::Or => 3,
            BinaryOp::And => 5,
            BinaryOp::BitwiseOr => 7,
            BinaryOp::BitwiseXor => 9,
            BinaryOp::BitwiseAnd => 11,
            BinaryOp::Equal | BinaryOp::NotEqual => 13,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 15,
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight | BinaryOp::ShiftRightLogical => 17,
            BinaryOp::Add | BinaryOp::Subtract => 19,
            BinaryOp::Multiply | BinaryOp::Divide => 21,
        },
        ExprKind::Prefix { .. } => 25,
        ExprKind::Call { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try { .. } => 27,
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Begin { .. } => 29,
        _ => 29,
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "|>",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::BitwiseOr => "|",
        BinaryOp::BitwiseXor => "^",
        BinaryOp::BitwiseAnd => "&",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::ShiftRightLogical => ">>>",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
}
