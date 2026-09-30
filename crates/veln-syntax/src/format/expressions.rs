use super::*;

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

    let mut visit = |child: &Expr, parent_emits_after_child| {
        take_continued_begin_comments(
            child,
            has_continuation || parent_emits_after_child,
            comments,
            continued_comments,
        );
    };
    match &expr.kind {
        ExprKind::TypeApply { callee, .. } => visit(callee, true),
        ExprKind::Call { callee, args } => {
            visit(callee, true);
            for arg in args {
                visit(arg, true);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                visit(arg, true);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            visit(body, true);
            for arg in args {
                visit(arg, true);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            visit(input, true);
            visit(base, false);
        }
        ExprKind::SchemaEncode { value, .. } => visit(value, false),
        ExprKind::FieldAccess { base, .. } | ExprKind::Try { expr: base, .. } => visit(base, true),
        ExprKind::Record(fields) => {
            for field in fields {
                visit(&field.expr, true);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                visit(&entry.key, true);
                visit(&entry.value, true);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                visit(item, true);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, true);
            for arm in arms {
                visit(&arm.expr, true);
            }
        }
        ExprKind::If {
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
        ExprKind::Prefix { expr, .. } => visit(expr, false),
        ExprKind::Binary { left, right, .. } => {
            visit(left, true);
            visit(right, false);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::Begin { .. } => {}
    }
}

fn expr_end_line(expr: &Expr) -> usize {
    if expr.span.end.column == 1 {
        expr.span.end.line.saturating_sub(1)
    } else {
        expr.span.end.line
    }
}

fn format_expr_at_indent_ctx(
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
        ExprKind::Missing => "_".to_string(),
        ExprKind::Hole { name, satisfy } => format_hole_expr(name.as_deref(), satisfy.as_ref()),
        ExprKind::NamePath { segments, .. } => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            let type_args = type_args
                .iter()
                .map(|arg| canonical_type_text(arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}<{}>",
                format_expr_at_indent_ctx(callee, indent, comments),
                type_args
            )
        }
        ExprKind::Call { callee, args } => format_call_expr(callee, args, prec, indent, comments),
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
        ExprKind::Record(fields) => format_record_expr(fields, indent, comments),
        ExprKind::Dict(entries) => format_dict_expr(entries, indent, comments),
        ExprKind::List(items) => format_list_expr(items, indent, comments),
        ExprKind::Match { scrutinee, arms } => format_match_expr(scrutinee, arms, indent, comments),
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
            Some(comments) => {
                format_cleanup_region_with_comments("begin", body, &expr.span, indent, comments)
            }
            None => format_cleanup_region("begin", body, indent),
        },
        ExprKind::Prefix { op, expr: inner } => {
            format_prefix_expr(*op, inner, prec, indent, comments)
        }
        ExprKind::Binary { op, left, right } => {
            format_binary_expr(*op, left, right, prec, indent, comments)
        }
    }
}

pub(super) fn format_defer_statement(body: &[BodyLine], indent: usize) -> String {
    format_cleanup_region("defer", body, indent)
}

pub(super) fn format_defer_statement_with_comments(
    body: &[BodyLine],
    span: &veln_source::SourceSpan,
    indent: usize,
    comments: &LineComments,
) -> String {
    format_cleanup_region_with_comments("defer", body, span, indent, comments)
}

fn format_cleanup_region(keyword: &str, body: &[BodyLine], indent: usize) -> String {
    let mut text = format!("{keyword}\n");
    for line in body {
        push_indent(&mut text, indent + 1);
        text.push_str(&format_cleanup_body_line(line, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_cleanup_region_with_comments(
    keyword: &str,
    body: &[BodyLine],
    span: &veln_source::SourceSpan,
    indent: usize,
    comments: &LineComments,
) -> String {
    let mut text = keyword.to_string();
    comments.emit_after(span.start.line, &mut text);
    text.push('\n');
    for line in body {
        let (source_line, content) =
            format_cleanup_body_line_with_comments(line, indent + 1, comments);
        push_source_line(&mut text, comments, source_line, indent + 1, content);
    }
    let end_line = if span.end.column == 1 {
        span.end.line.saturating_sub(1)
    } else {
        span.end.line
    };
    comments.emit_before(end_line, &mut text, indent + 1);
    push_indent(&mut text, indent);
    text.push_str("end");
    comments.emit_after(end_line, &mut text);
    text
}

fn format_cleanup_body_line(line: &BodyLine, indent: usize) -> String {
    match line {
        BodyLine::Let {
            pattern,
            annotation,
            expr,
            ..
        } => {
            let mut text = format!("let {}", format_pattern(pattern));
            if let Some(annotation) = annotation {
                text.push_str(": ");
                text.push_str(&canonical_type_text(annotation));
            }
            text.push_str(" = ");
            text.push_str(&format_expr_at_indent(expr, indent));
            text
        }
        BodyLine::Expr { expr, .. } => format_expr_at_indent(expr, indent),
        BodyLine::Defer { body, .. } => format_defer_statement(body, indent),
    }
}

fn format_cleanup_body_line_with_comments(
    line: &BodyLine,
    indent: usize,
    comments: &LineComments,
) -> (usize, String) {
    match line {
        BodyLine::Let {
            pattern,
            annotation,
            expr,
            span,
            ..
        } => {
            let mut text = format!("let {}", format_pattern(pattern));
            if let Some(annotation) = annotation {
                text.push_str(": ");
                text.push_str(&canonical_type_text(annotation));
            }
            text.push_str(" = ");
            text.push_str(&format_expr_at_indent_with_comments(expr, indent, comments));
            (span.start.line, text)
        }
        BodyLine::Expr { expr, span } => (
            span.start.line,
            format_expr_at_indent_with_comments(expr, indent, comments),
        ),
        BodyLine::Defer { body, span, .. } => (
            span.start.line,
            format_defer_statement_with_comments(body, span, indent, comments),
        ),
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

fn format_match_expr(
    scrutinee: &Expr,
    arms: &[crate::MatchArm],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    if let Some(rewrite) = literal_match_rewrite(scrutinee, arms) {
        return format_literal_match_rewrite(&rewrite, indent);
    }
    if let Some(rewrite) = bool_match_rewrite(arms) {
        return format_bool_match_rewrite(scrutinee, &rewrite, indent);
    }

    let mut text = format!(
        "match {}\n",
        format_expr_at_indent_ctx(scrutinee, indent, comments)
    );
    for arm in arms {
        push_indent(&mut text, indent + 1);
        text.push_str(&format_pattern(&arm.pattern));
        text.push_str(" => ");
        text.push_str(&format_expr_at_indent_ctx(&arm.expr, indent + 1, comments));
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
) -> Option<LiteralMatchRewrite<'a>> {
    let mut literal_arms = Vec::new();
    let (rewritten_scrutinee, fallback) =
        collect_literal_match_chain(scrutinee, arms, None, &mut literal_arms)?;
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
) -> Option<(&'a Expr, &'a Expr)> {
    let (true_arm, false_arm) = bool_match_arms(arms)?;
    let condition_literals = literal_match_conditions(condition)?;
    let active_scrutinee = condition_literals.first()?.0;

    if let Some(expected) = expected_scrutinee
        && !exprs_equivalent(expected, active_scrutinee)
    {
        return None;
    }
    if condition_literals
        .iter()
        .any(|(scrutinee, _)| !exprs_equivalent(active_scrutinee, scrutinee))
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
        ) {
            literal_arms.extend(nested_arms);
            return Some((active_scrutinee, fallback));
        }
    }

    Some((active_scrutinee, &false_arm.expr))
}

fn bool_match_arms(arms: &[crate::MatchArm]) -> Option<(&crate::MatchArm, &crate::MatchArm)> {
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

fn exprs_equivalent(left: &Expr, right: &Expr) -> bool {
    format_expr_at_indent(left, 0) == format_expr_at_indent(right, 0)
}

fn format_literal_match_rewrite(rewrite: &LiteralMatchRewrite<'_>, indent: usize) -> String {
    let mut text = format!(
        "match {}\n",
        format_expr_at_indent(rewrite.scrutinee, indent)
    );
    for (literal, expr) in &rewrite.arms {
        push_indent(&mut text, indent + 1);
        text.push_str(literal);
        text.push_str(" => ");
        text.push_str(&format_expr_at_indent(expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent + 1);
    text.push_str("_ => ");
    text.push_str(&format_expr_at_indent(rewrite.fallback, indent + 1));
    text.push('\n');
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_bool_match_rewrite(
    condition: &Expr,
    rewrite: &BoolMatchRewrite<'_>,
    indent: usize,
) -> String {
    let mut text = format!("if {}\n", format_expr_at_indent(condition, indent));
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent(rewrite.true_expr, indent + 1));
    text.push('\n');
    format_bool_match_else(&mut text, rewrite.false_expr, indent);
    text
}

fn format_bool_match_else(text: &mut String, false_expr: &Expr, indent: usize) {
    if let ExprKind::Match {
        scrutinee,
        arms: nested_arms,
    } = &false_expr.kind
        && literal_match_rewrite(scrutinee, nested_arms).is_none()
        && let Some(rewrite) = bool_match_rewrite(nested_arms)
    {
        push_indent(text, indent);
        text.push_str("else if ");
        text.push_str(&format_expr_at_indent(scrutinee, indent));
        text.push('\n');
        push_indent(text, indent + 1);
        text.push_str(&format_expr_at_indent(rewrite.true_expr, indent + 1));
        text.push('\n');
        format_bool_match_else(text, rewrite.false_expr, indent);
        return;
    }

    push_indent(text, indent);
    text.push_str("else\n");
    push_indent(text, indent + 1);
    text.push_str(&format_expr_at_indent(false_expr, indent + 1));
    text.push('\n');
    push_indent(text, indent);
    text.push_str("end");
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

pub(super) fn format_pattern(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Binding(name) => name.clone(),
        PatternKind::StringLiteral(value)
        | PatternKind::IntLiteral(value)
        | PatternKind::FloatLiteral(value) => value.clone(),
        PatternKind::BoolLiteral(true) => "true".to_string(),
        PatternKind::BoolLiteral(false) => "false".to_string(),
        PatternKind::Unit => "()".to_string(),
        PatternKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, format_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        PatternKind::Constructor { name, args, .. } => {
            if args.is_empty() {
                name.join("::")
            } else {
                let args = args
                    .iter()
                    .map(format_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({args})", name.join("::"))
            }
        }
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
