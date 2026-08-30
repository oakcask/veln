use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_match(&mut self) -> Expr {
        let start = self.bump().range;
        let scrutinee = self.parse_expr(0);
        self.eat_newlines();
        let mut arms = Vec::new();
        while !self.at(TokenKind::End) && !self.is_at_end() {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            let arm_start = self.current().range;
            let pattern = self.parse_pattern();
            self.expect_expr_token(
                TokenKind::FatArrow,
                "parse.match_arm",
                "match arm is missing `=>`",
                vec!["=>"],
            );
            let expr = self.parse_expr(0);
            let arm_end = lhs_range(&expr);
            arms.push(MatchArm {
                pattern,
                expr,
                span: self.source.span(arm_start.cover(arm_end)),
            });
            self.eat_newlines();
        }
        let end = self.eat(TokenKind::End).map_or_else(
            || {
                arms.last().map_or(lhs_range(&scrutinee), |arm| {
                    TextRange::new(arm.span.start.offset, arm.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_if(&mut self) -> Expr {
        let start = self.bump().range;
        let condition = self.parse_if_condition("if condition is missing an expression");
        self.eat_newlines();
        let then_branch = self.parse_if_branch_expr();
        self.eat_newlines();

        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        while self.at(TokenKind::Else) {
            let else_token = self.bump();
            if self.at(TokenKind::If) {
                self.bump();
                let condition =
                    self.parse_if_condition("else if condition is missing an expression");
                self.eat_newlines();
                let expr = self.parse_if_branch_expr();
                let span = self.source.span(else_token.range.cover(lhs_range(&expr)));
                else_if_branches.push(IfBranch {
                    condition,
                    expr,
                    span,
                });
                self.eat_newlines();
                continue;
            }

            self.eat_newlines();
            let branch = self.parse_if_branch_expr();
            else_branch = Some(branch);
            self.eat_newlines();
            break;
        }

        let else_branch = else_branch.unwrap_or_else(|| {
            self.error_current(
                "parse.if_missing_else",
                "if expression is missing a final `else` branch",
                vec!["else"],
                RecoveryStrategy::InsertToken,
                Some("else"),
            );
            self.missing_expr_at_current()
        });
        let end = if let Some(token) = self.eat(TokenKind::End) {
            token.range
        } else {
            self.error_current(
                "parse.if_missing_end",
                "if expression is missing `end`",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
            lhs_range(&else_branch)
        };

        Expr {
            kind: ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_if_branches,
                else_branch: Box::new(else_branch),
            },
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_if_condition(&mut self, message: &'static str) -> Expr {
        if self.at(TokenKind::Newline)
            || self.at(TokenKind::Else)
            || self.at(TokenKind::End)
            || self.is_at_end()
        {
            self.error_current(
                "parse.if_condition",
                message,
                vec!["condition"],
                RecoveryStrategy::InsertToken,
                Some("condition"),
            );
            return self.missing_expr_at_current();
        }
        self.parse_expr(0)
    }

    pub(super) fn parse_if_branch_expr(&mut self) -> Expr {
        if self.at(TokenKind::Else) || self.at(TokenKind::End) || self.is_at_end() {
            self.error_current(
                "parse.if_branch",
                "if branch is missing an expression",
                vec!["expression"],
                RecoveryStrategy::InsertToken,
                Some("expression"),
            );
            return self.missing_expr_at_current();
        }
        self.parse_expr(0)
    }

    pub(super) fn parse_pattern(&mut self) -> Pattern {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return Pattern {
                kind: PatternKind::Wildcard,
                span: self.source.span(TextRange::at(self.source.len())),
            };
        };
        match token.kind {
            TokenKind::Underscore => {
                self.bump();
                Pattern {
                    kind: PatternKind::Wildcard,
                    span: self.source.span(token.range),
                }
            }
            TokenKind::String => {
                self.bump();
                Pattern {
                    kind: PatternKind::StringLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Int => {
                self.bump();
                Pattern {
                    kind: PatternKind::IntLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::MalformedInt => {
                self.bump();
                Pattern {
                    kind: PatternKind::Wildcard,
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Float => {
                self.bump();
                Pattern {
                    kind: PatternKind::FloatLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::LParen => {
                let start = self.bump().range;
                if let Some(end) = self.eat(TokenKind::RParen) {
                    Pattern {
                        kind: PatternKind::Unit,
                        span: self.source.span(start.cover(end.range)),
                    }
                } else {
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
            }
            TokenKind::LBrace => self.parse_record_pattern(),
            TokenKind::Ident | TokenKind::Hole => self.parse_name_pattern(),
            _ => {
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
        }
    }

    pub(super) fn parse_record_pattern(&mut self) -> Pattern {
        let start = self.bump().range;
        let (fields, end) = self.parse_braced_items(
            start,
            |this| {
                let field_start = this.current().range;
                let name = if this.at(TokenKind::Ident) {
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
        let start = self.current().range;
        let mut end = start;
        let first_segment = self.bump();
        let mut segment_spans = vec![self.source.span(first_segment.range)];
        let mut segments = vec![first_segment.text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                let segment = self.bump();
                end = segment.range;
                segment_spans.push(self.source.span(segment.range));
                segments.push(segment.text);
            } else {
                break;
            }
        }
        if segments == ["true"] {
            return Pattern {
                kind: PatternKind::BoolLiteral(true),
                span: self.source.span(start.cover(end)),
            };
        }
        if segments == ["false"] {
            return Pattern {
                kind: PatternKind::BoolLiteral(false),
                span: self.source.span(start.cover(end)),
            };
        }
        let is_constructor = segments.len() > 1
            || segments
                .last()
                .and_then(|name| name.chars().next())
                .is_some_and(char::is_uppercase);
        if !is_constructor {
            return Pattern {
                kind: PatternKind::Binding(segments.remove(0)),
                span: self.source.span(start.cover(end)),
            };
        }
        let mut args = Vec::new();
        if self.eat(TokenKind::LParen).is_some() {
            while !self.at(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_pattern());
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            if let Some(close) = self.eat(TokenKind::RParen) {
                end = close.range;
            }
        }
        Pattern {
            kind: PatternKind::Constructor {
                name: segments,
                name_spans: segment_spans,
                args,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_satisfy_clause(&mut self) -> Option<SatisfyClause> {
        if !self.at_ident_text("satisfy") {
            return None;
        }
        let start = self.bump().range;
        let (candidate, candidate_span) =
            if matches!(self.current().kind, TokenKind::Ident | TokenKind::Hole) {
                let token = self.bump();
                let span = self.source.span(token.range);
                (Some(token.text), Some(span))
            } else {
                self.error_current(
                    "parse.satisfy_candidate",
                    "satisfy clause is missing a candidate binding",
                    vec!["candidate binding"],
                    RecoveryStrategy::InsertToken,
                    Some("=>"),
                );
                (None, None)
            };
        let mut end = if let Some(token) = self.eat(TokenKind::FatArrow) {
            token.range
        } else {
            self.error_current(
                "parse.satisfy_arrow",
                "satisfy clause is missing `=>`",
                vec!["=>"],
                RecoveryStrategy::InsertToken,
                None,
            );
            candidate_span.as_ref().map_or(start, |span| {
                TextRange::new(span.start.offset, span.end.offset)
            })
        };
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.cursor) {
            if depth == 0
                && matches!(
                    token.kind,
                    TokenKind::Comma
                        | TokenKind::RParen
                        | TokenKind::RBracket
                        | TokenKind::RBrace
                        | TokenKind::Eof
                )
            {
                break;
            }
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        self.diagnostics.extend(
            ContractPredicateParser::new(
                self.source,
                "satisfy_predicate",
                "parse.satisfy_predicate",
                &predicate_tokens,
            )
            .parse(),
        );
        Some(SatisfyClause {
            candidate,
            candidate_span,
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        })
    }

    pub(super) fn parse_name_path(&mut self) -> Expr {
        let start = self.current().range;
        let mut end = start;
        let mut segments = vec![self.bump().text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at_contextual_identifier() || self.at(TokenKind::Decode) {
                let segment = self.bump();
                end = segment.range;
                segments.push(segment.text);
            } else {
                break;
            }
        }
        if let Some(value) = bare_expression_bool_literal(&segments) {
            return Expr {
                kind: ExprKind::BoolLiteral(value),
                span: self.source.span(start.cover(end)),
            };
        }
        Expr {
            kind: ExprKind::NamePath(segments),
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_name_path_segments(
        &mut self,
        context: &'static str,
        expected_name: &'static str,
    ) -> Vec<String> {
        let mut segments = Vec::new();
        if self.at_contextual_identifier() {
            segments.push(self.bump().text);
        } else {
            self.error_current(
                "parse.name_path",
                format!("{context} is missing {expected_name}"),
                vec![expected_name],
                RecoveryStrategy::InsertToken,
                None,
            );
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at_contextual_identifier() {
                segments.push(self.bump().text);
            } else {
                self.error_current(
                    "parse.name_path",
                    format!("{context} has an incomplete path"),
                    vec!["path segment"],
                    RecoveryStrategy::InsertToken,
                    None,
                );
                break;
            }
        }
        segments
    }
}
