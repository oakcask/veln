use veln_source::SourceFile;
use veln_syntax::{
    FunctionDecl, FunctionKind, SyntaxItem, Token, TokenKind, canonical_type_text,
    declaration_function_signature, lex, parse,
};

use crate::navigation::function_signature_definition;
use crate::{EffectiveProjectSnapshot, NavigationSource, SourcePosition, navigate};

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
        let Some(callee_index) = open_index.checked_sub(1) else {
            continue;
        };
        let Some((_, callee)) = significant.get(callee_index).copied() else {
            continue;
        };
        if !matches!(callee.kind, TokenKind::Ident | TokenKind::Callsite) {
            continue;
        }
        let Some(result) = navigate(
            snapshot,
            SourcePosition {
                source: position.source.clone(),
                line: source.line_col(callee.range.start).line,
                column: source.line_col(callee.range.start).column,
            },
        ) else {
            continue;
        };
        if matches!(
            &result.selected_symbol.declaration.source,
            NavigationSource::Workspace
        ) && result.selected_symbol.declaration.span.file == position.source
            && result.selection.start.offset == result.selected_symbol.declaration.span.start.offset
        {
            continue;
        }
        let Some(definition) = function_signature_definition(snapshot, &result) else {
            continue;
        };
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
        let parsed = parse(&declaration_source);
        let Some(function) = parsed.tree.items.iter().find_map(|item| match item {
            SyntaxItem::Function(function)
                if function
                    .name_span
                    .as_ref()
                    .is_some_and(|span| span.start.offset == definition.span.start.offset) =>
            {
                Some(function.as_ref())
            }
            _ => None,
        }) else {
            continue;
        };
        let active_parameter = active_parameter(&significant, open_index, offset);
        return Some(signature_help(function, active_parameter));
    }
    None
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
}
