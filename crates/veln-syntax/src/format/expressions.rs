use super::cleanup::{
    format_cleanup_body_line, format_cleanup_body_line_with_comments, format_cleanup_region,
    format_cleanup_region_with_comments,
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
    continued_comments.extend(comments.take_after_owner_ending_at(expr.span.end.offset));
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
        ExprChildren::Handle(body, entries) => {
            match body {
                HandleBody::Expression(expr) => visit(expr, true),
                HandleBody::Block { body, .. } => {
                    for line in body {
                        if let BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } = line {
                            visit(expr, true);
                        }
                    }
                }
            }
            if matches!(
                expr.kind,
                ExprKind::Handle {
                    terminated: false,
                    ..
                }
            ) {
                for entry in entries {
                    for arg in &entry.args {
                        visit(arg, true);
                    }
                }
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
    format_expr_prec_ctx(expr, parent_prec, side, indent, comments, true)
}

fn format_expr_prec_ctx(
    expr: &Expr,
    parent_prec: u8,
    side: ExprSide,
    indent: usize,
    comments: Option<&LineComments>,
    allow_handle_chain_migration: bool,
) -> String {
    let prec = expr_prec(expr);
    let mut rendered =
        format_expr_inner(expr, prec, indent, comments, allow_handle_chain_migration);

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
    allow_handle_chain_migration: bool,
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
        | ExprKind::SchemaValidate { .. }
        | ExprKind::SchemaEncode { .. }) => {
            format_keyword_expr(expr, kind, indent, comments, allow_handle_chain_migration)
        }
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

fn format_keyword_expr(
    expr: &Expr,
    kind: &ExprKind,
    indent: usize,
    comments: Option<&LineComments>,
    allow_handle_chain_migration: bool,
) -> String {
    match kind {
        ExprKind::Perform {
            effect,
            operation,
            args,
            ..
        } => {
            let args = format_parenthesized_expr_args(args, indent, comments);
            format!("perform {}::{operation}{args}", effect.join("::"))
        }
        ExprKind::Handle {
            body,
            entries,
            terminated,
            ..
        } => format_handle_expr(
            expr,
            body,
            entries,
            *terminated,
            indent,
            comments,
            allow_handle_chain_migration,
        ),
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
        ExprKind::SchemaValidate {
            schema,
            value,
            base,
            ..
        } => {
            let schema = schema.join("::");
            let from = if schema.is_empty() { "from" } else { " from" };
            format!(
                "validate {schema}{from} {} at {}",
                format_expr_at_indent_ctx(value, indent, comments),
                format_expr_at_indent_ctx(base, indent, comments)
            )
        }
        _ => unreachable!("keyword formatter received a non-keyword expression"),
    }
}

fn format_handle_expr(
    expr: &Expr,
    body: &HandleBody,
    entries: &[HandlerEntry],
    terminated: bool,
    indent: usize,
    comments: Option<&LineComments>,
    allow_handle_chain_migration: bool,
) -> String {
    if allow_handle_chain_migration
        && let Some(formatted) = format_direct_handle_chain(expr, indent, comments)
    {
        return formatted;
    }

    if !terminated {
        return format_unterminated_handle(body, entries, indent, comments);
    }

    let header_trailing_comments = match (body, entries.first(), comments) {
        (HandleBody::Expression(body), Some(first_entry), Some(comments)) => {
            comments.take_after_in_range(body.span.end.offset, first_entry.span.start.offset)
        }
        _ => Vec::new(),
    };
    let mut text = String::from("handle");
    format_terminated_handle_body(&mut text, body, indent, comments, &header_trailing_comments);
    format_handler_entries(&mut text, entries.iter(), indent, comments);
    if let Some(comments) = comments {
        comments.emit_before(expr_end_line(expr), &mut text, indent + 1);
    }
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

#[derive(Clone, Copy)]
enum HandleChainKind {
    Short,
    Mixed,
}

struct HandleChainLevel<'a> {
    entries: &'a [HandlerEntry],
    terminated: bool,
}

struct DirectHandleChain<'a> {
    body: &'a Expr,
    entries: Vec<&'a HandlerEntry>,
    levels: Vec<HandleChainLevel<'a>>,
    kind: HandleChainKind,
}

fn format_direct_handle_chain(
    expr: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> Option<String> {
    let chain = direct_handle_chain(expr)?;
    #[cfg(test)]
    if matches!(chain.kind, HandleChainKind::Mixed) {
        super::record_mixed_handle_chain_candidate();
    }

    if comments.is_some_and(|comments| comments.has_comment_in_span_or_separator(&expr.span)) {
        return None;
    }

    let body_indent = match chain.kind {
        HandleChainKind::Short => indent + 1,
        HandleChainKind::Mixed => indent,
    };
    let formatted_body = format_expr_at_indent_ctx(chain.body, body_indent, None);
    if !formatted_body.contains('\n') {
        return Some(format_flattened_short_handle_chain(
            &formatted_body,
            &chain.entries,
            indent,
        ));
    }

    match chain.kind {
        HandleChainKind::Short => Some(format_retained_short_handle_chain(
            formatted_body,
            &chain.entries,
            indent,
        )),
        HandleChainKind::Mixed => Some(format_retained_mixed_handle_chain(
            formatted_body,
            &chain.levels,
            indent,
        )),
    }
}

fn format_retained_mixed_handle_chain(
    formatted_body: String,
    levels: &[HandleChainLevel<'_>],
    indent: usize,
) -> String {
    let mut text = formatted_body;
    for level in levels.iter().rev() {
        if level.terminated {
            let mut retained = format!("handle {text} with\n");
            format_handler_entries(&mut retained, level.entries.iter(), indent, None);
            push_indent(&mut retained, indent);
            retained.push_str("end");
            text = retained;
        } else {
            let entry = level
                .entries
                .first()
                .expect("unterminated handle must have one handler entry");
            let args = format_parenthesized_expr_args(&entry.args, indent, None);
            text = format!("handle {text} with {}{args}", entry.handler.join("::"));
        }
    }
    text
}

fn direct_handle_chain(expr: &Expr) -> Option<DirectHandleChain<'_>> {
    let mut current = expr;
    let mut entries = Vec::new();
    let mut levels = Vec::new();
    let mut saw_short = false;
    let mut saw_terminated = false;

    while let ExprKind::Handle {
        body: HandleBody::Expression(body),
        entries: current_entries,
        terminated,
    } = &current.kind
    {
        if !terminated && current_entries.len() != 1 {
            return None;
        }
        saw_short |= !terminated;
        saw_terminated |= *terminated;
        entries.extend(current_entries);
        levels.push(HandleChainLevel {
            entries: current_entries,
            terminated: *terminated,
        });
        current = body;
    }

    if matches!(
        current.kind,
        ExprKind::Handle { .. } | ExprKind::Begin { .. }
    ) {
        return None;
    }

    let kind = if saw_short && saw_terminated {
        HandleChainKind::Mixed
    } else if saw_short && levels.len() >= 2 {
        HandleChainKind::Short
    } else {
        return None;
    };
    Some(DirectHandleChain {
        body: current,
        entries,
        levels,
        kind,
    })
}

fn format_flattened_short_handle_chain(
    formatted_body: &str,
    entries: &[&HandlerEntry],
    indent: usize,
) -> String {
    let mut text = format!("handle {formatted_body} with\n");
    format_handler_entries(&mut text, entries.iter().copied(), indent, None);
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_retained_short_handle_chain(
    formatted_body: String,
    entries: &[&HandlerEntry],
    indent: usize,
) -> String {
    let mut text = String::from("handle (\n");
    push_indent(&mut text, indent + 1);
    for _ in &entries[1..] {
        text.push_str("handle ");
    }
    text.push_str(&formatted_body);
    for entry in entries[1..].iter().rev() {
        text.push_str(" with ");
        text.push_str(&format_handler_entry(entry, indent, None));
    }
    text.push('\n');
    push_indent(&mut text, indent);
    text.push_str(") with ");
    text.push_str(&format_handler_entry(entries[0], indent, None));
    text
}

fn format_unterminated_handle(
    body: &HandleBody,
    entries: &[HandlerEntry],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let HandleBody::Expression(body) = body else {
        unreachable!("unterminated handle must have an expression body");
    };
    let entry = entries
        .first()
        .expect("unterminated handle must have one handler entry");
    let args = format_parenthesized_expr_args(&entry.args, indent, comments);
    let body = format_continued_handle_body(body, indent, comments);
    format!("handle {} with {}{args}", body, entry.handler.join("::"))
}

fn format_continued_handle_body(
    body: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let formatted = format_expr_prec_ctx(body, 0, ExprSide::Root, indent, comments, false);
    let trailing_comments = match (&body.kind, comments) {
        (
            ExprKind::Handle {
                terminated: true, ..
            },
            Some(comments),
        ) => comments.take_after_last_end_in_span(&body.span),
        _ => Vec::new(),
    };
    if trailing_comments.is_empty() {
        return formatted;
    }

    let mut grouped = format!("({formatted}");
    for comment in trailing_comments {
        grouped.push_str("  ");
        grouped.push_str(&comment);
    }
    grouped.push('\n');
    push_indent(&mut grouped, indent);
    grouped.push(')');
    grouped
}

fn format_terminated_handle_body(
    text: &mut String,
    body: &HandleBody,
    indent: usize,
    comments: Option<&LineComments>,
    header_trailing_comments: &[String],
) {
    match body {
        HandleBody::Expression(body) => {
            text.push(' ');
            text.push_str(&format_continued_handle_body(body, indent, comments));
            text.push_str(" with");
            for comment in header_trailing_comments {
                text.push_str("  ");
                text.push_str(comment);
            }
            text.push('\n');
        }
        HandleBody::Block { body, block_span } => {
            text.push('\n');
            for line in body {
                match comments {
                    Some(comments) => {
                        let (source_line, through_line, content) =
                            format_cleanup_body_line_with_comments(
                                line,
                                indent + 1,
                                comments,
                                &|expr, child_indent| {
                                    format_expr_at_indent_with_comments(
                                        expr,
                                        child_indent,
                                        comments,
                                    )
                                },
                            );
                        push_source_lines_through(
                            text,
                            comments,
                            source_line,
                            through_line,
                            indent + 1,
                            content,
                        );
                    }
                    None => {
                        push_indent(text, indent + 1);
                        text.push_str(&format_cleanup_body_line(
                            line,
                            indent + 1,
                            &format_expr_at_indent,
                        ));
                        text.push('\n');
                    }
                }
            }
            if let Some(comments) = comments {
                comments.emit_before(block_span.end.line, text, indent + 1);
            }
            push_indent(text, indent);
            text.push_str("with\n");
        }
    }
}

fn format_handler_entries<'a>(
    text: &mut String,
    entries: impl IntoIterator<Item = &'a HandlerEntry>,
    indent: usize,
    comments: Option<&LineComments>,
) {
    let mut entries = entries.into_iter().peekable();
    while let Some(entry) = entries.next() {
        let trailing_comments = take_handler_entry_trailing_comments(entry, comments);
        let mut line = format_handler_entry(entry, indent, comments);
        if entries.peek().is_some() {
            line.push(',');
        }
        for comment in trailing_comments {
            line.push_str("  ");
            line.push_str(&comment);
        }
        if let Some(comments) = comments {
            push_source_lines_through(
                text,
                comments,
                entry.span.start.line,
                span_end_line(&entry.span),
                indent + 1,
                line,
            );
        } else {
            push_indent(text, indent + 1);
            text.push_str(&line);
            text.push('\n');
        }
    }
}

fn take_handler_entry_trailing_comments(
    entry: &HandlerEntry,
    comments: Option<&LineComments>,
) -> Vec<String> {
    let mut trailing_comments = Vec::new();
    if let Some(comments) = comments {
        for arg in &entry.args {
            take_continued_begin_comments(arg, true, comments, &mut trailing_comments);
        }
        trailing_comments.extend(comments.take_after_in_span_or_separator(&entry.span));
    }
    trailing_comments
}

fn format_handler_entry(
    entry: &HandlerEntry,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let mut line = entry.handler.join("::");
    line.push_str(&format_parenthesized_expr_args(
        &entry.args,
        indent + 1,
        comments,
    ));
    line
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
    let args = format_parenthesized_expr_args(args, indent, comments);
    format!(
        "{}{args}",
        format_expr_prec(callee, prec, ExprSide::Left, indent, comments)
    )
}

fn format_parenthesized_expr_args(
    args: &[Expr],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    format_delimited_subjects(
        args.iter()
            .map(|arg| (format_expr_at_indent_ctx(arg, indent, comments), &arg.span)),
        indent,
        comments,
        "(",
        ")",
        true,
    )
}

fn format_delimited_subjects<'a>(
    subjects: impl ExactSizeIterator<Item = (String, &'a veln_source::SourceSpan)>,
    indent: usize,
    comments: Option<&LineComments>,
    open: &str,
    close: &str,
    break_before_close_for_final_comment: bool,
) -> String {
    let subjects = subjects.collect::<Vec<_>>();
    let subject_count = subjects.len();
    let mut rendered = String::from(open);
    let mut needs_inline_space = false;
    for (index, (subject, span)) in subjects.iter().enumerate() {
        if needs_inline_space {
            rendered.push(' ');
        }
        rendered.push_str(subject);
        let is_last = index + 1 == subject_count;
        if !is_last {
            rendered.push(',');
        }

        let trailing = comments.map_or_else(Vec::new, |comments| {
            if let Some((_, next_span)) = subjects.get(index + 1) {
                comments.take_after_in_range(span.start.offset, next_span.start.offset)
            } else {
                let mut trailing = comments.take_after_in_range(span.start.offset, span.end.offset);
                trailing.extend(comments.take_after_separator(span));
                trailing
            }
        });
        if is_last && (!break_before_close_for_final_comment || trailing.is_empty()) {
            rendered.push_str(close);
        }
        if !trailing.is_empty() {
            for comment in trailing {
                rendered.push_str("  ");
                rendered.push_str(&comment);
            }
            if !is_last {
                rendered.push('\n');
                push_indent(&mut rendered, indent + 1);
            } else if break_before_close_for_final_comment {
                rendered.push('\n');
                push_indent(&mut rendered, indent);
                rendered.push_str(close.trim_start());
            }
            needs_inline_space = false;
        } else {
            needs_inline_space = index + 1 < subject_count;
        }
    }
    if subject_count == 0 {
        rendered.push_str(close);
    }
    rendered
}

fn format_record_expr(
    fields: &[crate::RecordField],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }
    format_delimited_subjects(
        fields.iter().map(|field| {
            (
                format!(
                    "{}: {}",
                    field.name,
                    format_expr_at_indent_ctx(&field.expr, indent, comments)
                ),
                &field.span,
            )
        }),
        indent,
        comments,
        "{ ",
        " }",
        true,
    )
}

fn format_dict_expr(
    entries: &[crate::DictEntry],
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    format_delimited_subjects(
        entries.iter().map(|entry| {
            (
                format!(
                    "{}: {}",
                    format_expr_at_indent_ctx(&entry.key, indent, comments),
                    format_expr_at_indent_ctx(&entry.value, indent, comments)
                ),
                &entry.span,
            )
        }),
        indent,
        comments,
        "{ ",
        " }",
        true,
    )
}

fn format_list_expr(items: &[Expr], indent: usize, comments: Option<&LineComments>) -> String {
    format_delimited_subjects(
        items.iter().map(|item| {
            (
                format_expr_at_indent_ctx(item, indent, comments),
                &item.span,
            )
        }),
        indent,
        comments,
        "[",
        "]",
        true,
    )
}

fn format_if_expr(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[crate::IfBranch],
    else_branch: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let mut text = String::from("if ");
    text.push_str(&format_child_with_trailing_comments(
        condition, indent, comments,
    ));
    text.push('\n');
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
        text.push_str(&format_child_with_trailing_comments(
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

fn format_child_with_trailing_comments(
    child: &Expr,
    indent: usize,
    comments: Option<&LineComments>,
) -> String {
    let mut text = format_expr_at_indent_ctx(child, indent, comments);
    if let Some(comments) = comments {
        for comment in comments.take_after_owner_ending_at(child.span.end.offset) {
            text.push_str("  ");
            text.push_str(&comment);
        }
    }
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
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 21,
        },
        ExprKind::Prefix { .. } => 25,
        ExprKind::Call { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaValidate { .. }
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
        BinaryOp::Remainder => "%",
    }
}
