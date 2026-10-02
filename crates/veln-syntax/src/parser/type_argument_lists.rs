use super::*;

#[derive(Default)]
struct TypeArgumentNesting {
    parentheses: usize,
    braces: usize,
    brackets: usize,
    angles: usize,
}

struct AngleClosers {
    total: usize,
    nested: usize,
}

enum TypeArgumentTokenAction {
    Finish { nested_angle_closers: usize },
    Separate,
    Append,
}

#[derive(Default)]
pub(super) struct TypeArgumentListState {
    args: Vec<String>,
    arg_ranges: Vec<TextRange>,
    arg_tokens: Vec<Vec<Token>>,
    current: String,
    current_range: Option<TextRange>,
    current_tokens: Vec<Token>,
    nesting: TypeArgumentNesting,
}

impl TypeArgumentNesting {
    fn is_outer_level(&self) -> bool {
        self.parentheses == 0 && self.braces == 0 && self.brackets == 0 && self.angles == 0
    }

    fn consume_delimiter(&mut self, kind: TokenKind) {
        match kind {
            TokenKind::LParen => self.parentheses += 1,
            TokenKind::RParen => self.parentheses = self.parentheses.saturating_sub(1),
            TokenKind::LBrace => self.braces += 1,
            TokenKind::RBrace => self.braces = self.braces.saturating_sub(1),
            TokenKind::LBracket => self.brackets += 1,
            TokenKind::RBracket => self.brackets = self.brackets.saturating_sub(1),
            TokenKind::Less => self.angles += 1,
            _ => {}
        }
    }

    fn consume_angle_closers(&mut self, kind: TokenKind) -> Option<AngleClosers> {
        let total = closing_angle_count(kind);
        if total == 0 {
            return None;
        }
        let nested = total.min(self.angles);
        self.angles -= nested;
        Some(AngleClosers { total, nested })
    }

    fn classify(&mut self, kind: TokenKind, close: TokenKind) -> TypeArgumentTokenAction {
        if kind == close && self.is_outer_level() {
            return TypeArgumentTokenAction::Finish {
                nested_angle_closers: 0,
            };
        }
        if kind == TokenKind::Comma && self.is_outer_level() {
            return TypeArgumentTokenAction::Separate;
        }
        if let Some(closers) = self.consume_angle_closers(kind) {
            return if closers.total > closers.nested {
                TypeArgumentTokenAction::Finish {
                    nested_angle_closers: closers.nested,
                }
            } else {
                TypeArgumentTokenAction::Append
            };
        }
        self.consume_delimiter(kind);
        TypeArgumentTokenAction::Append
    }
}

impl TypeArgumentListState {
    pub(super) fn consume(&mut self, token: &Token, close: TokenKind) -> bool {
        match self.nesting.classify(token.kind, close) {
            TypeArgumentTokenAction::Finish {
                nested_angle_closers,
            } => {
                self.current.push_str(&">".repeat(nested_angle_closers));
                if nested_angle_closers > 0 {
                    self.extend_current_range(TextRange::new(
                        token.range.start,
                        token.range.start + nested_angle_closers,
                    ));
                    self.current_tokens.push(token.clone());
                }
                self.flush_current(false);
                true
            }
            TypeArgumentTokenAction::Separate => {
                self.flush_current(true);
                false
            }
            TypeArgumentTokenAction::Append => {
                self.current.push_str(&token.text);
                self.extend_current_range(token.range);
                self.current_tokens.push(token.clone());
                false
            }
        }
    }

    fn flush_current(&mut self, include_empty: bool) {
        if include_empty || !self.current.is_empty() {
            let current = std::mem::take(&mut self.current);
            self.args.push(normalize_type_text(vec![current]));
            self.arg_ranges
                .push(self.current_range.take().unwrap_or_default());
            self.arg_tokens
                .push(std::mem::take(&mut self.current_tokens));
        }
    }

    fn extend_current_range(&mut self, range: TextRange) {
        self.current_range = Some(
            self.current_range
                .map_or(range, |current| current.cover(range)),
        );
    }

    pub(super) fn finish(mut self) -> (Vec<String>, Vec<TextRange>, Vec<Vec<Token>>) {
        self.flush_current(false);
        (self.args, self.arg_ranges, self.arg_tokens)
    }
}
