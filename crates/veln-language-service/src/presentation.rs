use std::collections::{HashMap, HashSet};

#[cfg(test)]
use std::cell::Cell;

use veln_source::SourceFile;
use veln_syntax::{
    FunctionDecl, FunctionKind, SyntaxItem, Token, TokenKind, canonical_type_text,
    declaration_function_signature, lex, parse,
};

use crate::navigation::{SignatureShadowIndex, function_signature_definition_at};
use crate::{EffectiveProjectSnapshot, NavigationSource, SourcePosition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionCandidateKind {
    DeclarationModifier,
    BuiltinLocal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionCandidate {
    pub label: &'static str,
    pub detail: &'static str,
    pub kind: CompletionCandidateKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
}

#[derive(Clone, Debug)]
enum LocalSignatureDeclaration {
    Function(String),
    Alias(String),
}

#[cfg(test)]
thread_local! {
    static SIGNATURE_INDEX_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_HEADER_PARSES: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_NAME_LOOKUPS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn reset_signature_help_work() {
    SIGNATURE_INDEX_TOKEN_VISITS.set(0);
    SIGNATURE_CANDIDATE_VISITS.set(0);
    SIGNATURE_HEADER_PARSES.set(0);
    SIGNATURE_NAME_LOOKUPS.set(0);
}

#[cfg(test)]
fn signature_help_work() -> (usize, usize, usize) {
    (
        SIGNATURE_INDEX_TOKEN_VISITS.get(),
        SIGNATURE_CANDIDATE_VISITS.get(),
        SIGNATURE_HEADER_PARSES.get(),
    )
}

#[cfg(test)]
fn record_signature_index_token_visits(count: usize) {
    SIGNATURE_INDEX_TOKEN_VISITS.set(SIGNATURE_INDEX_TOKEN_VISITS.get() + count);
}

#[cfg(not(test))]
fn record_signature_index_token_visits(_: usize) {}

#[cfg(test)]
fn record_signature_candidate_visit() {
    SIGNATURE_CANDIDATE_VISITS.set(SIGNATURE_CANDIDATE_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_signature_candidate_visit() {}

#[cfg(test)]
fn record_signature_header_parse() {
    SIGNATURE_HEADER_PARSES.set(SIGNATURE_HEADER_PARSES.get() + 1);
}

#[cfg(not(test))]
fn record_signature_header_parse() {}

#[cfg(test)]
fn record_signature_name_lookup() {
    SIGNATURE_NAME_LOOKUPS.set(SIGNATURE_NAME_LOOKUPS.get() + 1);
}

#[cfg(not(test))]
fn record_signature_name_lookup() {}

#[cfg(test)]
fn signature_name_lookups() -> usize {
    SIGNATURE_NAME_LOOKUPS.get()
}

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
    let parsed = parse(source);
    let Some(function) = parsed.tree.items.iter().find_map(|item| match item {
        SyntaxItem::Function(function)
            if function.span.start.offset <= offset && offset <= function.span.end.offset =>
        {
            Some(function.as_ref())
        }
        _ => None,
    }) else {
        return Vec::new();
    };
    if function.kind != FunctionKind::Function {
        return Vec::new();
    }
    let tokens = lex(source).tokens;
    let header_end = source.text()[function.span.start.offset..]
        .find('\n')
        .map(|relative| function.span.start.offset + relative)
        .unwrap_or(source.len());
    if offset > header_end {
        let body_start_line = function
            .contracts
            .last()
            .map_or(function.span.start.line + 1, |contract| {
                contract.span.end.line
            });
        return (function.callsite.is_some()
            && body_start_line <= position.line
            && position.line < function.span.end.line)
            .then_some(CompletionCandidate {
                label: "callsite",
                detail: "built-in SourceLocation local",
                kind: CompletionCandidateKind::BuiltinLocal,
            })
            .into_iter()
            .collect();
    }
    if function.callsite.is_none()
        && terminal_modifier_slot_is_clear(&tokens, offset, header_end)
        && function_parameters_are_closed(&tokens, function.span.start.offset, offset)
    {
        return vec![CompletionCandidate {
            label: "callsite",
            detail: "call-site declaration modifier",
            kind: CompletionCandidateKind::DeclarationModifier,
        }];
    }
    Vec::new()
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
    for token in tokens
        .iter()
        .filter(|token| token.range.end > offset && token.range.start < suffix_end)
    {
        match token.kind {
            TokenKind::Whitespace => {}
            _ => return false,
        }
    }
    true
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

pub fn signature_help_at(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<SignatureHelp> {
    let source = snapshot.workspace_source(&position.source)?;
    let offset = source_offset(source, position.line, position.column)?;
    let tokens = lex(source);
    let significant = tokens
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Newline | TokenKind::Comment | TokenKind::Eof
            )
        })
        .collect::<Vec<_>>();
    record_signature_index_token_visits(tokens.tokens.len());
    let shadow_index = SignatureShadowIndex::new(&tokens.tokens, offset);
    let local_signatures = local_signature_declarations(source, &significant);
    let function_names = signature_function_names(snapshot);
    let mut open = Vec::new();
    for (significant_index, (_, token)) in significant.iter().enumerate() {
        if token.range.start >= offset {
            break;
        }
        match token.kind {
            TokenKind::LParen => open.push(significant_index),
            TokenKind::RParen => {
                open.pop();
            }
            _ => {}
        }
    }
    for open_index in open.into_iter().rev() {
        record_signature_candidate_visit();
        let Some(callee_index) = open_index.checked_sub(1) else {
            continue;
        };
        let Some((callee_token_index, callee)) = significant.get(callee_index).copied() else {
            continue;
        };
        if !callee.kind.is_bare_expression_identifier() {
            continue;
        }
        if declaration_name_token(&significant, callee_index) {
            continue;
        }
        if shadow_index.shadows(&tokens.tokens, callee_token_index, &callee.text) {
            continue;
        }
        let local_signature_allowed = shadow_index
            .allows_local_signature(&tokens.tokens, callee_token_index)
            && callee_index
                .checked_sub(1)
                .is_none_or(|previous| significant[previous].1.kind != TokenKind::DoubleColon);
        if local_signature_allowed
            && let Some(function) = local_signature_function(&local_signatures, &callee.text)
        {
            let active_parameter = active_parameter(&significant, open_index, offset);
            return Some(signature_help(&function, active_parameter));
        }
        record_signature_name_lookup();
        if !function_names.contains(&callee.text) {
            continue;
        }
        let Some((selection, definition)) = function_signature_definition_at(
            snapshot,
            &SourcePosition {
                source: position.source.clone(),
                line: source.line_col(callee.range.start).line,
                column: source.line_col(callee.range.start).column,
            },
        ) else {
            continue;
        };
        if matches!(&definition.source, NavigationSource::Workspace)
            && definition.span.file == position.source
            && selection.start.offset == definition.span.start.offset
        {
            continue;
        }
        let Some(declaration_source) = (match &definition.source {
            NavigationSource::Workspace => {
                snapshot.workspace_source(&definition.span.file).cloned()
            }
            NavigationSource::Package { uri } => snapshot
                .resolve_virtual_source(uri)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(|text| SourceFile::new(definition.span.file.clone(), text)),
        }) else {
            continue;
        };
        let Some(function) =
            parse_function_signature_at(&declaration_source, definition.span.start.offset)
        else {
            continue;
        };
        let active_parameter = active_parameter(&significant, open_index, offset);
        return Some(signature_help(&function, active_parameter));
    }
    None
}

fn signature_function_names(snapshot: &EffectiveProjectSnapshot) -> HashSet<String> {
    snapshot
        .source_texts_for_signature_index()
        .flat_map(|text| {
            let tokens = lex(&SourceFile::new("signature-index.veln", text)).tokens;
            let significant = tokens
                .iter()
                .filter(|token| {
                    !matches!(
                        token.kind,
                        TokenKind::Whitespace
                            | TokenKind::Newline
                            | TokenKind::Comment
                            | TokenKind::Eof
                    )
                })
                .collect::<Vec<_>>();
            significant
                .windows(2)
                .filter(|pair| {
                    matches!(pair[0].kind, TokenKind::Fn | TokenKind::Test)
                        && pair[1].kind.is_contextual_identifier()
                })
                .map(|pair| pair[1].text.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn local_signature_declarations(
    source: &SourceFile,
    significant: &[(usize, &Token)],
) -> HashMap<String, Vec<LocalSignatureDeclaration>> {
    let mut declarations = HashMap::<String, Vec<LocalSignatureDeclaration>>::new();
    for (index, (_, token)) in significant.iter().enumerate() {
        record_signature_index_token_visits(1);
        if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
            continue;
        }
        let Some((_, name)) = significant.get(index + 1).copied() else {
            continue;
        };
        if !name.kind.is_contextual_identifier() {
            continue;
        }
        let declaration = if significant
            .get(index + 2)
            .is_some_and(|(_, token)| token.kind == TokenKind::Equal)
        {
            let Some((_, target)) = significant.get(index + 3).copied() else {
                continue;
            };
            if !target.kind.is_contextual_identifier()
                || significant
                    .get(index + 4)
                    .is_some_and(|(_, token)| token.kind == TokenKind::DoubleColon)
            {
                continue;
            }
            LocalSignatureDeclaration::Alias(target.text.clone())
        } else {
            let Some(header_end) = source.text()[token.range.start..]
                .find('\n')
                .map(|relative| token.range.start + relative)
                .or(Some(source.len()))
            else {
                continue;
            };
            let Some(header) = source.text().get(token.range.start..header_end) else {
                continue;
            };
            LocalSignatureDeclaration::Function(header.to_string())
        };
        declarations
            .entry(name.text.clone())
            .or_default()
            .push(declaration);
    }
    declarations
}

fn local_signature_function(
    declarations: &HashMap<String, Vec<LocalSignatureDeclaration>>,
    name: &str,
) -> Option<FunctionDecl> {
    let mut name = name;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(name.to_string()) {
            return None;
        }
        let [declaration] = declarations.get(name)?.as_slice() else {
            return None;
        };
        match declaration {
            LocalSignatureDeclaration::Alias(target) => name = target,
            LocalSignatureDeclaration::Function(header) => {
                record_signature_header_parse();
                let declaration =
                    SourceFile::new("signature-help.veln", format!("{header}\n  0\nend\n"));
                let parsed = parse(&declaration);
                return parsed.tree.items.into_iter().find_map(|item| {
                    let SyntaxItem::Function(function) = item else {
                        return None;
                    };
                    Some(*function)
                });
            }
        }
    }
}

fn parse_function_signature_at(source: &SourceFile, name_offset: usize) -> Option<FunctionDecl> {
    record_signature_header_parse();
    let tokens = lex(source).tokens;
    let name_index = tokens.iter().position(|token| {
        token.range.start == name_offset && token.kind.is_contextual_identifier()
    })?;
    let start = tokens[..name_index]
        .iter()
        .rposition(|token| matches!(token.kind, TokenKind::Fn | TokenKind::Test))?;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    let mut saw_parameters = false;
    let end = tokens[start..]
        .iter()
        .find_map(|token| {
            match token.kind {
                TokenKind::LParen => {
                    saw_parameters = true;
                    parens += 1;
                }
                TokenKind::RParen => parens = parens.saturating_sub(1),
                TokenKind::LBracket => brackets += 1,
                TokenKind::RBracket => brackets = brackets.saturating_sub(1),
                TokenKind::LBrace => braces += 1,
                TokenKind::RBrace => braces = braces.saturating_sub(1),
                _ => {}
            }
            (token.kind == TokenKind::Newline
                && saw_parameters
                && parens == 0
                && brackets == 0
                && braces == 0)
                .then_some(token.range.start)
        })
        .unwrap_or(source.len());
    let header = source.text().get(tokens[start].range.start..end)?;
    let declaration = SourceFile::new("signature-help.veln", format!("{header}\n  0\nend\n"));
    parse(&declaration).tree.items.into_iter().find_map(|item| {
        let SyntaxItem::Function(function) = item else {
            return None;
        };
        Some(*function)
    })
}

fn declaration_name_token(significant: &[(usize, &Token)], callee_index: usize) -> bool {
    callee_index
        .checked_sub(1)
        .is_some_and(|index| matches!(significant[index].1.kind, TokenKind::Fn | TokenKind::Test))
}

fn signature_help(function: &FunctionDecl, active_parameter: usize) -> SignatureHelp {
    SignatureHelp {
        label: declaration_function_signature(function, true),
        parameters: function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) if param.is_variadic => {
                    format!("{}: ...{}", param.name, canonical_type_text(ty))
                }
                Some(ty) => format!("{}: {}", param.name, canonical_type_text(ty)),
                None => param.name.clone(),
            })
            .collect(),
        active_parameter: active_parameter.min(function.params.len().saturating_sub(1)),
    }
}

fn active_parameter(
    significant: &[(usize, &veln_syntax::Token)],
    open_index: usize,
    offset: usize,
) -> usize {
    let mut depth = 0usize;
    let mut active = 0usize;
    for (_, token) in significant.iter().skip(open_index + 1) {
        if token.range.start >= offset {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1)
            }
            TokenKind::Comma if depth == 0 => active += 1,
            _ => {}
        }
    }
    active
}

fn source_offset(source: &SourceFile, line: usize, column: usize) -> Option<usize> {
    if line == 0 || column == 0 {
        return None;
    }
    let line_start = source
        .text()
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let selected = source.text().get(line_start..)?.split('\n').next()?;
    let byte_column = selected
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or_else(
            || (column == selected.chars().count() + 1).then_some(selected.len()),
            |(index, _)| Some(index),
        )?;
    Some(line_start + byte_column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_source::SourcePath;

    fn snapshot(text: &str) -> EffectiveProjectSnapshot {
        EffectiveProjectSnapshot::new(vec![SourceFile::new("main.veln", text)])
    }

    #[test]
    fn completion_distinguishes_modifier_builtin_and_ordinary_contexts() {
        let snapshot = snapshot(concat!(
            "fn located() -> SourceLocation callsite\n",
            "require callsite.start_line > 0\n",
            "  callsite\n",
            "end\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
        ));
        let modifier = completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 21,
            },
        );
        assert_eq!(
            modifier[0].kind,
            CompletionCandidateKind::DeclarationModifier
        );
        let builtin = completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 3,
                column: 3,
            },
        );
        assert_eq!(builtin[0].kind, CompletionCandidateKind::BuiltinLocal);
        assert!(
            completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 2,
                    column: 9
                }
            )
            .is_empty()
        );
        assert!(
            completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 6,
                    column: 3
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn completion_covers_empty_bodies_and_only_the_terminal_modifier_slot() {
        let snapshot = snapshot(concat!(
            "fn empty() -> SourceLocation callsite\n",
            "\n",
            "end\n",
            "fn higher(callback: fn() -> Int) -> Int\n",
            "  1\n",
            "end\n",
            "test example() -> Int\n",
            "  1\n",
            "end\n",
        ));
        let empty_body = completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 1,
            },
        );
        assert_eq!(empty_body[0].kind, CompletionCandidateKind::BuiltinLocal);
        assert!(
            completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 4,
                    column: 25,
                }
            )
            .is_empty()
        );
        let terminal = completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 40,
            },
        );
        assert_eq!(
            terminal[0].kind,
            CompletionCandidateKind::DeclarationModifier
        );
        assert!(
            completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 8,
                    column: 3,
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn modifier_completion_stops_at_a_trailing_comment() {
        let snapshot = snapshot(concat!(
            "fn noted() -> Int # keep this note\n",
            "  1\n",
            "end\n",
        ));

        for column in [18, 19] {
            let candidates = completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 1,
                    column,
                },
            );
            assert_eq!(
                candidates.first().map(|candidate| candidate.kind),
                Some(CompletionCandidateKind::DeclarationModifier),
                "column {column}"
            );
        }
        for column in [20, 35] {
            assert!(
                completion_at(
                    &snapshot,
                    &SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line: 1,
                        column,
                    },
                )
                .is_empty(),
                "column {column}"
            );
        }
    }

    #[test]
    fn signature_help_keeps_callsite_outside_the_parameter_list() {
        let snapshot = snapshot(concat!(
            "fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  located(\"hello\")\n",
            "end\n",
        ));
        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 18,
            },
        )
        .expect("signature help");
        assert_eq!(
            help.label,
            "fn located(message: String) -> SourceLocation callsite"
        );
        assert_eq!(help.parameters, ["message: String"]);
        assert_eq!(help.active_parameter, 0);
    }

    #[test]
    fn signature_help_resolves_contextual_function_names_across_sources() {
        let declarations = ["callsite", "handler", "handles"]
            .into_iter()
            .map(|name| format!("pub fn {name}(value: Int) -> Int callsite\n  value\nend\n"))
            .collect::<String>();
        let caller = concat!(
            "use declarations\n",
            "fn caller() -> Int\n",
            "  declarations::callsite(1)\n",
            "  declarations::handler(1)\n",
            "  declarations::handles(1)\n",
            "end\n",
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            SourceFile::new("declarations.veln", declarations),
            SourceFile::new("main.veln", caller),
        ]);

        for (line, name) in [(3, "callsite"), (4, "handler"), (5, "handles")] {
            let source_line = caller.lines().nth(line - 1).expect("call line");
            let column = source_line.find(')').expect("closing parenthesis") + 1;
            let help = signature_help_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            )
            .expect("contextual-name signature help");
            assert_eq!(help.label, format!("fn {name}(value: Int) -> Int callsite"));
        }
    }

    #[test]
    fn signature_help_is_absent_in_a_function_declaration_header() {
        let snapshot = snapshot(concat!(
            "fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
        ));

        assert!(
            signature_help_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 1,
                    column: 20,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn signature_help_is_absent_in_a_test_declaration_header() {
        let snapshot = snapshot(concat!(
            "test example(value: Int) -> Int\n",
            "  value\n",
            "end\n",
        ));

        assert!(
            signature_help_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 1,
                    column: 22,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn signature_help_skips_grouping_inside_a_call_argument() {
        let snapshot = snapshot(concat!(
            "fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  located((1 + 2))\n",
            "end\n",
        ));

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 15,
            },
        )
        .expect("outer call signature help");
        assert_eq!(
            help.label,
            "fn located(value: Int) -> SourceLocation callsite"
        );
    }

    #[test]
    fn signature_help_prefers_the_innermost_nested_call() {
        let snapshot = snapshot(concat!(
            "fn outer(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn inner(message: String) -> Int callsite\n",
            "  1\n",
            "end\n",
            "fn caller() -> Int\n",
            "  outer(inner(\"hello\"))\n",
            "end\n",
        ));

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 8,
                column: 21,
            },
        )
        .expect("inner call signature help");
        assert_eq!(help.label, "fn inner(message: String) -> Int callsite");
        assert_eq!(help.parameters, ["message: String"]);
    }

    #[test]
    fn signature_help_skips_a_non_function_identifier_before_grouping() {
        let snapshot = snapshot(concat!(
            "fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  let value: Int = 1\n",
            "  located(value(1))\n",
            "end\n",
        ));

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column: 18,
            },
        )
        .expect("outer call signature help");
        assert_eq!(
            help.label,
            "fn located(value: Int) -> SourceLocation callsite"
        );
    }

    #[test]
    fn signature_help_does_not_bypass_parameter_shadowing() {
        let snapshot = snapshot(concat!(
            "fn located(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn caller(located: fn(Int) -> Int) -> Int\n",
            "  located(1)\n",
            "end\n",
        ));

        assert!(
            signature_help_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 5,
                    column: 12,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn signature_help_resolves_workspace_function_aliases() {
        let snapshot = snapshot(concat!(
            "pub fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "pub fn renamed = located\n",
            "fn caller() -> SourceLocation\n",
            "  renamed(\"hello\")\n",
            "end\n",
        ));
        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column: 18,
            },
        )
        .expect("alias signature help");
        assert_eq!(
            help.label,
            "fn located(message: String) -> SourceLocation callsite"
        );
        assert_eq!(help.parameters, ["message: String"]);
    }

    #[test]
    fn signature_help_resolves_alias_chains_beyond_the_former_depth_limit() {
        let mut source = String::from(
            "pub fn located(message: String) -> SourceLocation callsite\n  callsite\nend\n",
        );
        source.push_str("pub fn alias_0 = located\n");
        for index in 1..70 {
            source.push_str(&format!("pub fn alias_{index} = alias_{}\n", index - 1));
        }
        source.push_str("fn caller() -> SourceLocation\n  alias_69(\"hello\")\nend\n");
        let snapshot = snapshot(&source);

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 75,
                column: 19,
            },
        )
        .expect("deep alias signature help");

        assert_eq!(
            help.label,
            "fn located(message: String) -> SourceLocation callsite"
        );
    }

    #[test]
    fn signature_help_rejects_a_public_alias_cycle() {
        let snapshot = snapshot(concat!(
            "pub fn first = second\n",
            "pub fn second = first\n",
            "fn caller() -> Int\n",
            "  first()\n",
            "end\n",
        ));

        assert!(
            signature_help_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 4,
                    column: 9,
                },
            )
            .is_none()
        );
    }

    fn rejected_nested_candidate_reference_collections(depth: usize) -> usize {
        let mut source = String::from(concat!(
            "fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  let value: Int = 1\n",
            "  located("
        ));
        source.push_str(&"value(".repeat(depth));
        source.push('1');
        let column = source.lines().last().expect("call line").chars().count() + 1;
        source.push_str(&")".repeat(depth));
        source.push_str(")\nend\n");
        let snapshot = snapshot(&source);
        crate::navigation::reset_function_scope_collections();

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column,
            },
        )
        .expect("outer signature help");
        assert_eq!(
            help.label,
            "fn located(value: Int) -> SourceLocation callsite"
        );
        crate::navigation::function_scope_collections()
    }

    #[test]
    fn rejected_nested_signature_candidates_do_not_collect_references() {
        assert_eq!(rejected_nested_candidate_reference_collections(100), 0);
        assert_eq!(rejected_nested_candidate_reference_collections(200), 0);
    }

    fn unmatched_parenthesis_work(depth: usize) -> (usize, usize, usize) {
        let mut source = String::from(concat!(
            "fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  let value: Int = 1\n",
            "  located("
        ));
        source.push_str(&"value(".repeat(depth));
        let column = source.lines().last().expect("call line").chars().count() + 1;
        let snapshot = snapshot(&source);
        reset_signature_help_work();

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column,
            },
        )
        .expect("outer signature help");

        assert_eq!(
            help.label,
            "fn located(value: Int) -> SourceLocation callsite"
        );
        signature_help_work()
    }

    #[test]
    fn signature_help_handles_deep_unmatched_parentheses_with_linear_work() {
        let smaller = unmatched_parenthesis_work(5_000);
        let larger = unmatched_parenthesis_work(10_000);

        assert!(
            larger.0 <= smaller.0 * 2,
            "signature index token visits grew too quickly: {smaller:?} -> {larger:?}"
        );
        assert_eq!(larger.1, smaller.1 * 2 - 1);
        assert_eq!(smaller.2, 1);
        assert_eq!(larger.2, 1);
    }

    fn undefined_callee_work(depth: usize) -> ((usize, usize, usize), usize, std::time::Duration) {
        let mut source = String::from(concat!(
            "fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  located("
        ));
        for index in 0..depth {
            source.push_str(&format!("missing_{index}("));
        }
        let column = source.lines().last().expect("call line").chars().count() + 1;
        let snapshot = snapshot(&source);
        reset_signature_help_work();
        let started = std::time::Instant::now();

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column,
            },
        )
        .expect("outer local signature help");

        assert_eq!(
            help.label,
            "fn located(value: Int) -> SourceLocation callsite"
        );
        assert!(!snapshot.navigation_index_is_prepared());
        (
            signature_help_work(),
            signature_name_lookups(),
            started.elapsed(),
        )
    }

    #[test]
    fn undefined_callees_use_a_parse_free_linear_name_index() {
        let smaller = undefined_callee_work(5_000);
        let larger = undefined_callee_work(10_000);
        eprintln!(
            "undefined callee signature help: 5000={:?}, 10000={:?}",
            smaller.2, larger.2
        );

        assert!(
            larger.0.0 <= smaller.0.0 * 2,
            "signature index token visits grew too quickly: {smaller:?} -> {larger:?}"
        );
        assert_eq!(larger.0.1, smaller.0.1 * 2 - 1);
        assert_eq!(smaller.0.2, 1);
        assert_eq!(larger.0.2, 1);
        assert_eq!(smaller.1, 5_000);
        assert_eq!(larger.1, 10_000);
    }

    fn alias_target_lookups(alias_count: usize) -> usize {
        let mut source = String::from(
            "pub fn located(message: String) -> SourceLocation callsite\n  callsite\nend\n",
        );
        source.push_str("pub fn alias_0 = located\n");
        for index in 1..alias_count {
            source.push_str(&format!("pub fn alias_{index} = alias_{}\n", index - 1));
        }
        source.push_str(&format!(
            "fn caller() -> SourceLocation\n  alias_{}(\"hello\")\nend\n",
            alias_count - 1
        ));
        let snapshot = snapshot(&source);
        crate::navigation::reset_function_alias_target_lookups();
        function_signature_definition_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: alias_count + 5,
                column: 3,
            },
        )
        .expect("resolved alias target");
        crate::navigation::function_alias_target_lookups()
    }

    #[test]
    fn function_alias_resolution_uses_one_index_lookup_per_hop() {
        assert_eq!(alias_target_lookups(128), 128);
        assert_eq!(alias_target_lookups(256), 256);
    }

    fn qualified_alias_chain_work(alias_count: usize, filler_count: usize) -> (usize, usize) {
        let mut sources = vec![SourceFile::new(
            "target.veln",
            concat!(
                "pub fn located(message: String) -> SourceLocation callsite\n",
                "  callsite\n",
                "end\n",
            ),
        )];
        for index in 0..alias_count {
            let target_module = if index == 0 {
                "target".to_string()
            } else {
                format!("alias_{}", index - 1)
            };
            let target_name = if index == 0 {
                "located".to_string()
            } else {
                format!("alias_{}", index - 1)
            };
            sources.push(SourceFile::new(
                format!("alias_{index}.veln"),
                format!(
                    "use {target_module}\npub fn alias_{index} = {target_module}::{target_name}\n"
                ),
            ));
        }
        for index in 0..filler_count {
            sources.push(SourceFile::new(
                format!("filler_{index}.veln"),
                format!("pub fn filler_{index}() -> Int\n  {index}\nend\n"),
            ));
        }
        let final_alias = format!("alias_{}", alias_count - 1);
        let main = format!(
            "use {final_alias}\nfn caller() -> SourceLocation\n  {final_alias}::{final_alias}(\"hello\")\nend\n"
        );
        let column = main.lines().nth(2).expect("call line").chars().count();
        sources.push(SourceFile::new("main.veln", main));
        let snapshot = EffectiveProjectSnapshot::new(sources);
        crate::navigation::reset_function_alias_target_lookups();
        crate::navigation::reset_function_alias_declaring_file_lookups();

        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 3,
                column,
            },
        )
        .expect("qualified alias signature help");

        assert_eq!(
            help.label,
            "fn located(message: String) -> SourceLocation callsite"
        );
        (
            crate::navigation::function_alias_target_lookups(),
            crate::navigation::function_alias_declaring_file_lookups(),
        )
    }

    #[test]
    fn qualified_alias_resolution_indexes_each_declaring_file() {
        assert_eq!(qualified_alias_chain_work(64, 64), (64, 64));
        assert_eq!(qualified_alias_chain_work(128, 64), (128, 128));
        assert_eq!(qualified_alias_chain_work(64, 128), (64, 64));
        assert_eq!(qualified_alias_chain_work(128, 128), (128, 128));
    }
}
