use super::patterns::format_pattern;
use super::source_layout::{LineComments, push_indent, push_source_line};
use super::type_text::canonical_type_text;
use crate::{BodyLine, Expr};

pub(super) fn format_cleanup_region(
    keyword: &str,
    body: &[BodyLine],
    indent: usize,
    render_expr: &impl Fn(&Expr, usize) -> String,
) -> String {
    let mut text = format!("{keyword}\n");
    for line in body {
        push_indent(&mut text, indent + 1);
        text.push_str(&format_cleanup_body_line(line, indent + 1, render_expr));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

pub(super) fn format_cleanup_region_with_comments(
    keyword: &str,
    body: &[BodyLine],
    span: &veln_source::SourceSpan,
    indent: usize,
    comments: &LineComments,
    render_expr: &impl Fn(&Expr, usize) -> String,
) -> String {
    let mut text = keyword.to_string();
    comments.emit_after(span.start.line, &mut text);
    text.push('\n');
    for line in body {
        let (source_line, content) =
            format_cleanup_body_line_with_comments(line, indent + 1, comments, render_expr);
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

fn format_defer_statement(
    body: &[BodyLine],
    indent: usize,
    render_expr: &impl Fn(&Expr, usize) -> String,
) -> String {
    format_cleanup_region("defer", body, indent, render_expr)
}

pub(super) fn format_defer_statement_with_comments(
    body: &[BodyLine],
    span: &veln_source::SourceSpan,
    indent: usize,
    comments: &LineComments,
    render_expr: &impl Fn(&Expr, usize) -> String,
) -> String {
    format_cleanup_region_with_comments("defer", body, span, indent, comments, render_expr)
}

fn format_cleanup_body_line(
    line: &BodyLine,
    indent: usize,
    render_expr: &impl Fn(&Expr, usize) -> String,
) -> String {
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
            text.push_str(&render_expr(expr, indent));
            text
        }
        BodyLine::Expr { expr, .. } => render_expr(expr, indent),
        BodyLine::Defer { body, .. } => format_defer_statement(body, indent, render_expr),
    }
}

fn format_cleanup_body_line_with_comments(
    line: &BodyLine,
    indent: usize,
    comments: &LineComments,
    render_expr: &impl Fn(&Expr, usize) -> String,
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
            text.push_str(&render_expr(expr, indent));
            (span.start.line, text)
        }
        BodyLine::Expr { expr, span } => (span.start.line, render_expr(expr, indent)),
        BodyLine::Defer { body, span, .. } => (
            span.start.line,
            format_defer_statement_with_comments(body, span, indent, comments, render_expr),
        ),
    }
}
