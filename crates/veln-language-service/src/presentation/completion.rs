use veln_syntax::{FunctionDecl, FunctionKind, SyntaxItem, Token, TokenKind, lex, parse};

use super::{
    CompletionCandidate, CompletionCandidateKind, EffectiveProjectSnapshot, SourcePosition,
    presentation_parse_structure_is_bounded, source_offset,
};

pub fn completion_at(
    snapshot: &EffectiveProjectSnapshot,
    position: &SourcePosition,
) -> Vec<CompletionCandidate> {
    let Some(source) = snapshot.workspace_source(&position.source) else {
        return Vec::new();
    };
    let Some(offset) = source_offset(source, position.line, position.column) else {
        return Vec::new();
    };
    let tokens = lex(source).tokens;
    if !presentation_parse_structure_is_bounded(&tokens) {
        return Vec::new();
    }
    let parsed = parse(source);
    let Some(function) = enclosing_function(&parsed.tree.items, offset) else {
        return Vec::new();
    };
    if function.kind != FunctionKind::Function {
        return Vec::new();
    }

    let header_end = function_header_end(source.text(), function.span.start.offset);
    if offset > header_end {
        return body_completion(function, position.line);
    }
    modifier_completion(function, &tokens, offset, header_end)
}

fn enclosing_function(items: &[SyntaxItem], offset: usize) -> Option<&FunctionDecl> {
    items.iter().find_map(|item| match item {
        SyntaxItem::Function(function)
            if function.span.start.offset <= offset && offset <= function.span.end.offset =>
        {
            Some(function.as_ref())
        }
        _ => None,
    })
}

fn function_header_end(text: &str, start: usize) -> usize {
    text[start..]
        .find('\n')
        .map_or(text.len(), |relative| start + relative)
}

fn body_completion(function: &FunctionDecl, line: usize) -> Vec<CompletionCandidate> {
    let body_start_line = function
        .contracts
        .last()
        .map_or(function.span.start.line + 1, |contract| {
            contract.span.end.line
        });
    (function.callsite.is_some() && body_start_line <= line && line < function.span.end.line)
        .then_some(CompletionCandidate {
            label: "callsite",
            detail: "built-in SourceLocation local",
            kind: CompletionCandidateKind::BuiltinLocal,
        })
        .into_iter()
        .collect()
}

fn modifier_completion(
    function: &FunctionDecl,
    tokens: &[Token],
    offset: usize,
    header_end: usize,
) -> Vec<CompletionCandidate> {
    let available = function.callsite.is_none()
        && terminal_modifier_slot_is_clear(tokens, offset, header_end)
        && function_parameters_are_closed(tokens, function.span.start.offset, offset);
    available
        .then_some(CompletionCandidate {
            label: "callsite",
            detail: "call-site declaration modifier",
            kind: CompletionCandidateKind::DeclarationModifier,
        })
        .into_iter()
        .collect()
}

fn terminal_modifier_slot_is_clear(tokens: &[Token], offset: usize, header_end: usize) -> bool {
    let trailing_comment_start = tokens
        .iter()
        .find(|token| {
            token.kind == TokenKind::Comment
                && token.range.start < header_end
                && offset <= token.range.end
        })
        .map(|token| token.range.start);
    if trailing_comment_start.is_some_and(|comment_start| comment_start < offset) {
        return false;
    }
    let suffix_end = trailing_comment_start.unwrap_or(header_end);
    tokens
        .iter()
        .filter(|token| token.range.end > offset && token.range.start < suffix_end)
        .all(|token| token.kind == TokenKind::Whitespace)
}

fn function_parameters_are_closed(tokens: &[Token], start: usize, offset: usize) -> bool {
    let mut saw_parameters = false;
    let mut depth = 0usize;
    for token in tokens {
        if token.range.start < start || token.range.start >= offset {
            continue;
        }
        match token.kind {
            TokenKind::LParen => {
                saw_parameters = true;
                depth += 1;
            }
            TokenKind::RParen if saw_parameters => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    saw_parameters && depth == 0
}
