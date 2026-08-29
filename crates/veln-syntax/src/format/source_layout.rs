use super::*;

pub(super) fn lossless_text(tree: &SyntaxTree) -> String {
    tree.lossless_tokens()
        .filter(|token| token.kind != TokenKind::Eof)
        .map(|token| token.text.as_str())
        .collect()
}

pub(super) fn push_source_line(
    out: &mut String,
    comments: &LineComments,
    source_line: usize,
    indent: usize,
    content: String,
) {
    comments.emit_before(source_line, out, indent);
    push_indent(out, indent);
    out.push_str(&content);
    comments.emit_after(source_line, out);
    out.push('\n');
}

pub(super) fn push_indent(out: &mut String, level: usize) {
    out.push_str(&"\t".repeat(level));
}

#[derive(Default)]
pub(super) struct LineComments {
    before: std::cell::RefCell<std::collections::BTreeMap<usize, Vec<String>>>,
    after: std::cell::RefCell<std::collections::BTreeMap<usize, Vec<String>>>,
    pub(super) requires_lossless_preservation: bool,
}

impl LineComments {
    pub(super) fn from_tree(tree: &SyntaxTree) -> Self {
        let mut line = 1usize;
        let mut seen_code_on_line = false;
        let mut pending = Vec::new();
        let mut before = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let mut after = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let mut requires_lossless_preservation = false;

        for token in tree.lossless_tokens() {
            match token.kind {
                TokenKind::Newline => {
                    line += 1;
                    seen_code_on_line = false;
                }
                TokenKind::Whitespace => {}
                TokenKind::Comment => {
                    if seen_code_on_line {
                        after
                            .entry(line)
                            .or_default()
                            .push(token.text.trim_start().to_string());
                    } else {
                        pending.push(token.text.trim_start().to_string());
                    }
                }
                TokenKind::Eof => {}
                _ => {
                    if !seen_code_on_line && !pending.is_empty() {
                        before.entry(line).or_default().append(&mut pending);
                    }
                    seen_code_on_line = true;
                }
            }
        }

        if !pending.is_empty() {
            requires_lossless_preservation = true;
        }

        Self {
            before: std::cell::RefCell::new(before),
            after: std::cell::RefCell::new(after),
            requires_lossless_preservation,
        }
    }

    pub(super) fn emit_before(&self, source_line: usize, out: &mut String, indent: usize) {
        let Some(comments) = self.before.borrow_mut().remove(&source_line) else {
            return;
        };
        for comment in comments {
            push_indent(out, indent);
            out.push_str(&comment);
            out.push('\n');
        }
    }

    pub(super) fn emit_before_first_after(
        &self,
        after_line: usize,
        through_line: usize,
        out: &mut String,
        indent: usize,
    ) {
        let Some(line) = self
            .before
            .borrow()
            .keys()
            .copied()
            .find(|line| *line > after_line && *line <= through_line)
        else {
            return;
        };
        self.emit_before(line, out, indent);
    }

    pub(super) fn emit_after(&self, source_line: usize, out: &mut String) {
        let Some(comments) = self.after.borrow_mut().remove(&source_line) else {
            return;
        };
        for comment in comments {
            out.push_str("  ");
            out.push_str(&comment);
        }
    }

    pub(super) fn has_comment_in_span(&self, span: &veln_source::SourceSpan) -> bool {
        let start = span.start.line;
        let end = if span.end.column == 1 {
            span.end.line.saturating_sub(1)
        } else {
            span.end.line
        };
        self.before
            .borrow()
            .keys()
            .chain(self.after.borrow().keys())
            .any(|line| *line >= start && *line <= end)
    }

    pub(super) fn all_emitted(&self) -> bool {
        self.before.borrow().is_empty() && self.after.borrow().is_empty()
    }
}

pub(super) fn function_body_end_line(function: &FunctionDecl) -> usize {
    function
        .body
        .last()
        .map(|line| match line {
            BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } => span.start.line,
        })
        .or_else(|| {
            function
                .contracts
                .last()
                .map(|contract| contract.span.start.line)
        })
        .unwrap_or(function.span.start.line)
}

pub(super) fn function_end_line(function: &FunctionDecl) -> usize {
    if function.end_present && function.span.end.column == 1 {
        function.span.end.line.saturating_sub(1)
    } else {
        function.span.end.line
    }
}

pub(super) fn type_body_end_line(type_decl: &TypeDecl) -> usize {
    type_decl
        .variants
        .last()
        .map(|variant| variant.span.start.line)
        .unwrap_or(type_decl.span.start.line)
}

pub(super) fn type_end_line(type_decl: &TypeDecl) -> usize {
    if type_decl.end_present && type_decl.span.end.column == 1 {
        type_decl.span.end.line.saturating_sub(1)
    } else {
        type_decl.span.end.line
    }
}
