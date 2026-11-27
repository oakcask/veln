use super::*;
use crate::Token;

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
    push_source_lines_through(out, comments, source_line, usize::MAX, indent, content);
}

pub(super) fn push_source_lines_through(
    out: &mut String,
    comments: &LineComments,
    source_line: usize,
    through_line: usize,
    indent: usize,
    content: String,
) {
    comments.emit_before(source_line, out, indent);
    push_indent(out, indent);
    out.push_str(&format_source_lines_through(
        comments,
        source_line,
        through_line,
        content,
    ));
    out.push('\n');
}

pub(super) fn format_source_lines_through(
    comments: &LineComments,
    source_line: usize,
    through_line: usize,
    content: String,
) -> String {
    let mut out = String::new();
    for (index, line) in content.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(line);
        let rendered_source_line = source_line.saturating_add(index);
        if rendered_source_line <= through_line {
            comments.emit_after(rendered_source_line, &mut out);
        }
    }
    out
}

pub(super) fn span_end_line(span: &veln_source::SourceSpan) -> usize {
    if span.end.column == 1 {
        span.end.line.saturating_sub(1)
    } else {
        span.end.line
    }
}

pub(super) fn push_indent(out: &mut String, level: usize) {
    out.push_str(&"\t".repeat(level));
}

#[derive(Default)]
pub(super) struct LineComments {
    before: std::cell::RefCell<std::collections::BTreeMap<usize, Vec<String>>>,
    after: std::cell::RefCell<TrailingComments>,
    owner_offsets: std::collections::BTreeSet<usize>,
    owner_start_by_end: std::collections::BTreeMap<usize, usize>,
    end_token_offsets: std::collections::BTreeSet<usize>,
    separator_owner_by_preceding_end: std::collections::BTreeMap<usize, usize>,
    pub(super) requires_lossless_preservation: bool,
}

struct OwnedTrailingComment {
    source_line: usize,
    text: String,
}

#[derive(Clone, Copy)]
struct LineCodeToken {
    kind: TokenKind,
    start: usize,
    end: usize,
    preceding_end: Option<usize>,
}

struct LineCommentCollector {
    line: usize,
    pending: Vec<String>,
    before: std::collections::BTreeMap<usize, Vec<String>>,
    after: TrailingComments,
    owner_offsets: std::collections::BTreeSet<usize>,
    owner_start_by_end: std::collections::BTreeMap<usize, usize>,
    end_token_offsets: std::collections::BTreeSet<usize>,
    separator_owner_by_preceding_end: std::collections::BTreeMap<usize, usize>,
    last_code: Option<LineCodeToken>,
    previous_code_end: Option<usize>,
}

impl Default for LineCommentCollector {
    fn default() -> Self {
        Self {
            line: 1,
            pending: Vec::new(),
            before: std::collections::BTreeMap::new(),
            after: TrailingComments::default(),
            owner_offsets: std::collections::BTreeSet::new(),
            owner_start_by_end: std::collections::BTreeMap::new(),
            end_token_offsets: std::collections::BTreeSet::new(),
            separator_owner_by_preceding_end: std::collections::BTreeMap::new(),
            last_code: None,
            previous_code_end: None,
        }
    }
}

impl LineCommentCollector {
    fn push(&mut self, token: &Token) {
        match token.kind {
            TokenKind::Newline => {
                self.line += 1;
                self.last_code = None;
            }
            TokenKind::Whitespace | TokenKind::Eof => {}
            TokenKind::Comment => self.push_comment(token),
            _ => self.push_code(token),
        }
    }

    fn push_comment(&mut self, token: &Token) {
        let text = token.text.trim_start().to_string();
        let Some(owner) = self.last_code else {
            self.pending.push(text);
            return;
        };

        self.owner_offsets.insert(owner.start);
        self.owner_start_by_end.insert(owner.end, owner.start);
        if owner.kind == TokenKind::Comma
            && let Some(preceding_end) = owner.preceding_end
        {
            self.separator_owner_by_preceding_end
                .insert(preceding_end, owner.start);
        }
        self.after.insert(self.line, owner.start, text);
    }

    fn push_code(&mut self, token: &Token) {
        if token.kind == TokenKind::End {
            self.end_token_offsets.insert(token.range.start);
        }
        if self.last_code.is_none() && !self.pending.is_empty() {
            self.owner_offsets.insert(token.range.start);
            self.before
                .entry(self.line)
                .or_default()
                .append(&mut self.pending);
        }
        self.last_code = Some(LineCodeToken {
            kind: token.kind,
            start: token.range.start,
            end: token.range.end,
            preceding_end: self.previous_code_end,
        });
        self.previous_code_end = Some(token.range.end);
    }

    fn finish(self) -> LineComments {
        LineComments {
            before: std::cell::RefCell::new(self.before),
            after: std::cell::RefCell::new(self.after),
            owner_offsets: self.owner_offsets,
            owner_start_by_end: self.owner_start_by_end,
            end_token_offsets: self.end_token_offsets,
            separator_owner_by_preceding_end: self.separator_owner_by_preceding_end,
            requires_lossless_preservation: !self.pending.is_empty(),
        }
    }
}

#[derive(Default)]
struct TrailingComments {
    by_owner: std::collections::BTreeMap<usize, Vec<OwnedTrailingComment>>,
    owners_by_line: std::collections::BTreeMap<usize, std::collections::BTreeSet<usize>>,
}

impl TrailingComments {
    fn insert(&mut self, source_line: usize, owner_offset: usize, text: String) {
        self.by_owner
            .entry(owner_offset)
            .or_default()
            .push(OwnedTrailingComment { source_line, text });
        self.owners_by_line
            .entry(source_line)
            .or_default()
            .insert(owner_offset);
    }

    fn take_line(&mut self, source_line: usize) -> Vec<String> {
        let Some(owner_offsets) = self.owners_by_line.remove(&source_line) else {
            return Vec::new();
        };
        self.take_owner_offsets(owner_offsets)
    }

    fn take_range(&mut self, start_offset: usize, end_offset: usize) -> Vec<String> {
        let owner_offsets = self
            .by_owner
            .range(start_offset..end_offset)
            .map(|(owner_offset, _comments)| {
                #[cfg(test)]
                super::record_trailing_comment_entries_inspected(_comments.len());
                *owner_offset
            })
            .collect::<Vec<_>>();
        self.take_owner_offsets(owner_offsets)
    }

    fn take_owner(&mut self, owner_offset: usize) -> Vec<String> {
        self.take_owner_offsets(std::iter::once(owner_offset))
    }

    fn take_owner_offsets(
        &mut self,
        owner_offsets: impl IntoIterator<Item = usize>,
    ) -> Vec<String> {
        let mut taken = Vec::new();
        for owner_offset in owner_offsets {
            let Some(comments) = self.by_owner.remove(&owner_offset) else {
                continue;
            };
            for comment in comments {
                let remove_line =
                    if let Some(line_owners) = self.owners_by_line.get_mut(&comment.source_line) {
                        line_owners.remove(&owner_offset);
                        line_owners.is_empty()
                    } else {
                        false
                    };
                if remove_line {
                    self.owners_by_line.remove(&comment.source_line);
                }
                taken.push(comment.text);
            }
        }
        taken
    }

    fn is_empty(&self) -> bool {
        self.by_owner.is_empty()
    }
}

impl LineComments {
    pub(super) fn from_tree(tree: &SyntaxTree) -> Self {
        let mut collector = LineCommentCollector::default();
        for token in tree.lossless_tokens() {
            collector.push(token);
        }
        collector.finish()
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
        for comment in self.after.borrow_mut().take_line(source_line) {
            out.push_str("  ");
            out.push_str(&comment);
        }
    }

    pub(super) fn take_after(&self, source_line: usize) -> Vec<String> {
        self.after.borrow_mut().take_line(source_line)
    }

    pub(super) fn take_after_in_span_or_separator(
        &self,
        span: &veln_source::SourceSpan,
    ) -> Vec<String> {
        let mut taken = self.take_after_owner_ending_at(span.end.offset);
        taken.extend(self.take_after_separator(span));
        taken
    }

    pub(super) fn take_after_separator(&self, span: &veln_source::SourceSpan) -> Vec<String> {
        if let Some(owner_offset) = self
            .separator_owner_by_preceding_end
            .get(&span.end.offset)
            .copied()
        {
            return self.after.borrow_mut().take_owner(owner_offset);
        }
        Vec::new()
    }

    pub(super) fn take_after_owner_ending_at(&self, end_offset: usize) -> Vec<String> {
        let Some(owner_offset) = self.owner_start_by_end.get(&end_offset).copied() else {
            return Vec::new();
        };
        self.after.borrow_mut().take_owner(owner_offset)
    }

    pub(super) fn take_after_last_end_in_span(
        &self,
        span: &veln_source::SourceSpan,
    ) -> Vec<String> {
        let Some(owner_offset) = self
            .end_token_offsets
            .range(span.start.offset..span.end.offset)
            .next_back()
            .copied()
        else {
            return Vec::new();
        };
        self.after.borrow_mut().take_owner(owner_offset)
    }

    pub(super) fn take_after_in_range(
        &self,
        start_offset: usize,
        end_offset: usize,
    ) -> Vec<String> {
        if start_offset >= end_offset {
            return Vec::new();
        }

        self.after.borrow_mut().take_range(start_offset, end_offset)
    }

    pub(super) fn has_comment_in_span(&self, span: &veln_source::SourceSpan) -> bool {
        set_has_key_in_range(&self.owner_offsets, span.start.offset, span.end.offset)
    }

    pub(super) fn has_comment_in_span_or_separator(&self, span: &veln_source::SourceSpan) -> bool {
        self.has_comment_in_span(span)
            || self
                .separator_owner_by_preceding_end
                .contains_key(&span.end.offset)
    }

    pub(super) fn all_emitted(&self) -> bool {
        self.before.borrow().is_empty() && self.after.borrow().is_empty()
    }
}

fn set_has_key_in_range(set: &std::collections::BTreeSet<usize>, start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    #[cfg(test)]
    super::record_comment_range_query();
    let found = set.range(start..end).next().is_some();
    #[cfg(test)]
    if found {
        super::record_comment_range_key_inspected();
    }
    found
}

pub(super) fn function_body_end_line(function: &FunctionDecl) -> usize {
    function
        .body
        .last()
        .map(|line| match line {
            BodyLine::Let { span, .. }
            | BodyLine::Expr { span, .. }
            | BodyLine::Defer { span, .. } => span.start.line,
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
