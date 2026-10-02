use super::*;

#[derive(Clone, Copy)]
pub(super) struct AngleMatch {
    pub(super) token_index: usize,
    pub(super) close_offset: usize,
}

#[derive(Clone, Default)]
pub(super) struct GenericSyntax {
    pub(super) matched: Option<AngleMatch>,
    pub(super) arguments: Vec<TypeArgumentRange>,
}

#[derive(Clone, Copy)]
pub(super) struct TypeArgumentRange {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) explicit_end: Option<usize>,
}

struct GenericContext {
    open: usize,
    argument_start: usize,
    non_angle_depth: usize,
    arguments: Vec<TypeArgumentRange>,
}

pub(super) fn scan_generic_syntax(tokens: &[Token]) -> (Vec<GenericSyntax>, Vec<bool>) {
    let mut syntax = vec![GenericSyntax::default(); tokens.len()];
    let mut surplus_closers = vec![false; tokens.len()];
    let mut opens: Vec<GenericContext> = Vec::new();
    let mut non_angle_depth = 0usize;
    for (token_index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                non_angle_depth += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                non_angle_depth = non_angle_depth.saturating_sub(1);
            }
            TokenKind::Less => {
                opens.push(GenericContext {
                    open: token_index,
                    argument_start: token_index + 1,
                    non_angle_depth,
                    arguments: Vec::new(),
                });
                continue;
            }
            TokenKind::Comma => {
                if let Some(context) = opens.last_mut()
                    && context.non_angle_depth == non_angle_depth
                {
                    context.arguments.push(TypeArgumentRange {
                        start: context.argument_start,
                        end: token_index,
                        explicit_end: None,
                    });
                    context.argument_start = token_index + 1;
                }
            }
            _ => {}
        }
        close_generics_at_token(
            token,
            token_index,
            &mut opens,
            &mut syntax,
            &mut surplus_closers,
        );
    }
    (syntax, surplus_closers)
}

fn close_generics_at_token(
    token: &Token,
    token_index: usize,
    opens: &mut Vec<GenericContext>,
    syntax: &mut [GenericSyntax],
    surplus_closers: &mut [bool],
) {
    let close_count = closing_angle_count(token.kind);
    if close_count == 0 {
        return;
    }
    if close_count > opens.len() {
        surplus_closers[token_index] = true;
        opens.clear();
        return;
    }
    for close_offset in 0..close_count {
        let mut context = opens
            .pop()
            .expect("closing count was bounded by open count");
        context.arguments.push(TypeArgumentRange {
            start: context.argument_start,
            end: token_index,
            explicit_end: Some(token.range.start + close_offset),
        });
        syntax[context.open] = GenericSyntax {
            matched: Some(AngleMatch {
                token_index,
                close_offset,
            }),
            arguments: context.arguments,
        };
    }
}

pub(super) fn type_argument_text_range(
    tokens: &[Token],
    argument: &TypeArgumentRange,
) -> Option<TextRange> {
    let first = tokens.get(argument.start)?;
    if let Some(end) = argument.explicit_end {
        Some(TextRange::new(first.range.start, end))
    } else {
        let last = tokens.get(argument.end.checked_sub(1)?)?;
        Some(first.range.cover(last.range))
    }
}
