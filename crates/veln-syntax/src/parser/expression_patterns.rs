use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_pattern(&mut self) -> Pattern {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return Pattern {
                kind: PatternKind::Wildcard,
                span: self.source.span(TextRange::at(self.source.len())),
            };
        };
        match token.kind {
            TokenKind::Underscore => self.parse_atomic_pattern(token, PatternKind::Wildcard),
            TokenKind::String => {
                let text = token.text.clone();
                self.parse_atomic_pattern(token, PatternKind::StringLiteral(text))
            }
            TokenKind::Int => {
                let text = token.text.clone();
                self.parse_atomic_pattern(token, PatternKind::IntLiteral(text))
            }
            TokenKind::MalformedInt => self.parse_atomic_pattern(token, PatternKind::Wildcard),
            TokenKind::Float => {
                let text = token.text.clone();
                self.parse_atomic_pattern(token, PatternKind::FloatLiteral(text))
            }
            TokenKind::LParen => self.parse_parenthesized_pattern(),
            TokenKind::LBrace => self.parse_record_pattern(),
            TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole => self.parse_name_pattern(),
            _ => self.recover_invalid_pattern(token),
        }
    }

    fn parse_atomic_pattern(&mut self, token: Token, kind: PatternKind) -> Pattern {
        self.bump();
        Pattern {
            kind,
            span: self.source.span(token.range),
        }
    }

    fn parse_parenthesized_pattern(&mut self) -> Pattern {
        let start = self.bump().range;
        if let Some(end) = self.eat(TokenKind::RParen) {
            return Pattern {
                kind: PatternKind::Unit,
                span: self.source.span(start.cover(end.range)),
            };
        }
        self.error_current(
            "parse.pattern",
            "unsupported parenthesized pattern",
            vec!["pattern"],
            RecoveryStrategy::SkipToken,
            None,
        );
        Pattern {
            kind: PatternKind::Wildcard,
            span: self.source.span(start),
        }
    }

    fn recover_invalid_pattern(&mut self, token: Token) -> Pattern {
        self.error_current(
            "parse.pattern",
            "expected a match pattern",
            vec!["pattern"],
            RecoveryStrategy::SkipToken,
            None,
        );
        self.bump();
        Pattern {
            kind: PatternKind::Wildcard,
            span: self.source.span(token.range),
        }
    }

    pub(super) fn parse_record_pattern(&mut self) -> Pattern {
        let start = self.bump().range;
        let (fields, end) = self.parse_braced_items(
            start,
            |this| {
                let field_start = this.current().range;
                let name = if this.at_contextual_identifier() {
                    this.bump().text
                } else {
                    this.error_current(
                        "parse.pattern",
                        "record pattern field is missing a name",
                        vec!["field name"],
                        RecoveryStrategy::SkipToken,
                        None,
                    );
                    this.bump();
                    String::new()
                };
                this.expect_expr_token(
                    TokenKind::Colon,
                    "parse.pattern",
                    "record pattern field is missing `:`",
                    vec![":"],
                );
                let pattern = this.parse_pattern();
                let span = this.source.span(field_start.cover(pattern_range(&pattern)));
                PatternField {
                    name,
                    pattern,
                    span,
                }
            },
            |field| TextRange::new(field.span.start.offset, field.span.end.offset),
        );
        Pattern {
            kind: PatternKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_name_pattern(&mut self) -> Pattern {
        let (start, mut end, mut segments, segment_spans) = self.parse_pattern_name_path();
        if let Some(pattern) = self.finish_non_constructor_pattern(start, end, &mut segments) {
            return pattern;
        }
        let args = self.parse_constructor_pattern_arguments(&mut end);
        Pattern {
            kind: PatternKind::Constructor {
                name: segments,
                name_spans: segment_spans,
                args,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_pattern_name_path(&mut self) -> (TextRange, TextRange, Vec<String>, Vec<SourceSpan>) {
        let start = self.current().range;
        let mut end = start;
        let first_segment = self.bump();
        let mut segment_spans = vec![self.source.span(first_segment.range)];
        let mut segments = vec![first_segment.text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if !self.at_contextual_identifier() {
                break;
            }
            let segment = self.bump();
            end = segment.range;
            segment_spans.push(self.source.span(segment.range));
            segments.push(segment.text);
        }
        (start, end, segments, segment_spans)
    }

    fn finish_non_constructor_pattern(
        &self,
        start: TextRange,
        end: TextRange,
        segments: &mut Vec<String>,
    ) -> Option<Pattern> {
        let kind = if segments.as_slice() == ["true"] {
            PatternKind::BoolLiteral(true)
        } else if segments.as_slice() == ["false"] {
            PatternKind::BoolLiteral(false)
        } else if !is_constructor_pattern_name(segments) {
            PatternKind::Binding(segments.remove(0))
        } else {
            return None;
        };
        Some(Pattern {
            kind,
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_constructor_pattern_arguments(&mut self, end: &mut TextRange) -> Vec<Pattern> {
        let mut args = Vec::new();
        if self.eat(TokenKind::LParen).is_none() {
            return args;
        }
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_pattern());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        if let Some(close) = self.eat(TokenKind::RParen) {
            *end = close.range;
        }
        args
    }
}

fn is_constructor_pattern_name(segments: &[String]) -> bool {
    segments.len() > 1
        || segments
            .last()
            .and_then(|name| name.chars().next())
            .is_some_and(char::is_uppercase)
}
