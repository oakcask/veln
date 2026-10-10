use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn new(source: &'a SourceFile, context: &'static str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            context,
            tokens,
            cursor: 0,
            cleanup_depth: 0,
            control_blocks: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(super) fn with_cleanup_depth(mut self, cleanup_depth: usize) -> Self {
        self.cleanup_depth = cleanup_depth;
        self
    }

    pub(super) fn parse(mut self) -> (Expr, Vec<ParseDiagnostic>) {
        let expr = self.parse_expr(0);
        self.report_trailing_tokens(
            "parse.expected_newline",
            "expected a newline before this token",
        );
        (expr, self.diagnostics)
    }

    pub(super) fn parse_pattern_only(mut self) -> (Pattern, Vec<ParseDiagnostic>) {
        let pattern = self.parse_pattern();
        self.report_trailing_tokens_with_expected(
            "parse.pattern",
            "expected the pattern to end before this token",
            vec!["pattern end"],
            None,
        );
        (pattern, self.diagnostics)
    }

    pub(super) fn report_trailing_tokens(&mut self, id: &'static str, message: &'static str) {
        self.report_trailing_tokens_with_expected(id, message, vec!["newline"], Some("newline"));
    }

    pub(super) fn report_trailing_tokens_with_expected(
        &mut self,
        id: &'static str,
        message: &'static str,
        expected: Vec<&'static str>,
        anchor: Option<&'static str>,
    ) {
        if self.cursor < self.tokens.len() {
            self.error_current(id, message, expected, RecoveryStrategy::InsertToken, anchor);
        }
    }

    pub(super) fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                let token = self.bump();
                lhs = Expr {
                    span: self.source.span(lhs_range(&lhs).cover(token.range)),
                    kind: ExprKind::Try {
                        expr: Box::new(lhs),
                        question_span: self.source.span(token.range),
                    },
                };
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr(right_bp);
            let span = self.source.span(lhs_range(&lhs).cover(lhs_range(&rhs)));
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            };
        }

        lhs
    }

    pub(super) fn parse_prefix(&mut self) -> Expr {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) || self.at(TokenKind::Tilde) {
            let token = self.bump();
            let op = if token.kind == TokenKind::Not {
                PrefixOp::Not
            } else if token.kind == TokenKind::Minus {
                PrefixOp::Negate
            } else {
                PrefixOp::BitwiseNot
            };
            let expr = self.parse_expr(25);
            return Expr {
                span: self.source.span(token.range.cover(lhs_range(&expr))),
                kind: ExprKind::Prefix {
                    op,
                    expr: Box::new(expr),
                },
            };
        }

        self.parse_postfix()
    }

    pub(super) fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.call_type_arguments_start(&expr) {
                expr = self.parse_call_type_apply(expr);
                continue;
            }
            if self.at(TokenKind::LParen) {
                expr = self.parse_call_postfix(expr);
                continue;
            }
            if self.at(TokenKind::Dot) {
                expr = self.parse_field_postfix(expr);
                continue;
            }
            if self.at(TokenKind::Question) {
                expr = self.parse_try_postfix(expr);
                continue;
            }
            break;
        }
        expr
    }

    pub(super) fn call_type_arguments_start(&self, expr: &Expr) -> bool {
        self.at(TokenKind::Less)
            && matches!(expr.kind, ExprKind::NamePath { .. })
            && self.angle_type_arguments_are_followed_by_call()
    }

    pub(super) fn parse_call_type_apply(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        self.parse_type_apply(expr, start, TokenKind::Greater)
    }

    pub(super) fn parse_type_apply(
        &mut self,
        expr: Expr,
        start: TextRange,
        closing: TokenKind,
    ) -> Expr {
        let (type_args, type_arg_spans, type_arg_paths, type_arg_refinements, surplus_closers, end) =
            self.parse_type_argument_list(closing);
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::TypeApply {
                callee: Box::new(expr),
                type_args,
                type_arg_spans,
                type_arg_paths,
                type_arg_refinements,
                surplus_closers,
            },
        }
    }

    pub(super) fn parse_call_postfix(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        self.bump();
        let mut args = Vec::new();
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            if self.at(TokenKind::RParen) || self.is_at_end() {
                break;
            }
            self.error_current(
                "parse.call_argument",
                "call argument is missing `,` or `)`",
                vec![",", ")"],
                RecoveryStrategy::InsertToken,
                Some(","),
            );
        }
        let end = self.eat(TokenKind::RParen).map_or_else(
            || lhs_range(args.last().unwrap_or(&expr)),
            |token| token.range,
        );
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::Call {
                callee: Box::new(expr),
                args,
            },
        }
    }

    pub(super) fn parse_field_postfix(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        let dot = self.bump();
        let (field, field_range) = if self.at_contextual_identifier() {
            let field = self.bump();
            (field.text, field.range)
        } else {
            self.error_current(
                "parse.field_access",
                "field access is missing a field name",
                vec!["field name"],
                RecoveryStrategy::InsertToken,
                None,
            );
            (String::new(), dot.range)
        };
        Expr {
            span: self.source.span(start.cover(field_range)),
            kind: ExprKind::FieldAccess {
                base: Box::new(expr),
                field,
                field_span: self.source.span(field_range),
            },
        }
    }

    pub(super) fn parse_try_postfix(&mut self, expr: Expr) -> Expr {
        let token = self.bump();
        Expr {
            span: self.source.span(lhs_range(&expr).cover(token.range)),
            kind: ExprKind::Try {
                expr: Box::new(expr),
                question_span: self.source.span(token.range),
            },
        }
    }

    pub(super) fn angle_type_arguments_are_followed_by_call(&self) -> bool {
        if !self.at(TokenKind::Less) {
            return false;
        }
        let mut cursor = self.cursor + 1;
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::Less => angle_depth += 1,
                kind if closing_angle_count(kind) > angle_depth
                    && paren_depth == 0
                    && brace_depth == 0
                    && bracket_depth == 0 =>
                {
                    return self
                        .tokens
                        .get(cursor + 1)
                        .is_some_and(|next| next.kind == TokenKind::LParen);
                }
                kind if closing_angle_count(kind) > 0 => {
                    angle_depth = angle_depth.saturating_sub(closing_angle_count(kind));
                }
                TokenKind::Newline | TokenKind::Eof => return false,
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    pub(super) fn parse_type_argument_list(
        &mut self,
        close: TokenKind,
    ) -> (
        Vec<String>,
        Vec<SourceSpan>,
        Vec<Vec<TypePathSegments>>,
        Vec<Vec<VariantRefinementType>>,
        usize,
        TextRange,
    ) {
        let start = self.bump();
        let mut state = TypeArgumentListState::default();
        let mut end = start.range;

        while !self.is_at_end() {
            let token = self.bump();
            end = token.range;
            if state.consume(&token, close) {
                return self.finish_type_argument_list(state, end, true);
            }
        }

        self.error_current(
            "parse.type_argument_list",
            "type argument list is missing its closing delimiter",
            vec![if close == TokenKind::Greater {
                ">"
            } else {
                "]"
            }],
            RecoveryStrategy::CloseBlock,
            Some(if close == TokenKind::Greater {
                ">"
            } else {
                "]"
            }),
        );
        self.finish_type_argument_list(state, end, false)
    }

    fn finish_type_argument_list(
        &mut self,
        state: TypeArgumentListState,
        end: TextRange,
        report_refinement_errors: bool,
    ) -> (
        Vec<String>,
        Vec<SourceSpan>,
        Vec<Vec<TypePathSegments>>,
        Vec<Vec<VariantRefinementType>>,
        usize,
        TextRange,
    ) {
        let (arguments, argument_ranges, argument_tokens, surplus_closers) = state.finish();
        let argument_spans = argument_ranges
            .into_iter()
            .map(|range| self.source.span(range))
            .collect();
        let refinements = argument_tokens
            .iter()
            .map(|tokens| self.build_type_argument_refinements(tokens, report_refinement_errors))
            .collect();
        let paths = argument_tokens
            .iter()
            .map(|tokens| super::type_paths::type_paths_from_tokens(self.source, tokens))
            .collect();
        (
            arguments,
            argument_spans,
            paths,
            refinements,
            surplus_closers,
            end,
        )
    }

    fn build_type_argument_refinements(
        &mut self,
        tokens: &[Token],
        report_errors: bool,
    ) -> Vec<VariantRefinementType> {
        let (refinements, consumed_pipes) =
            super::body_and_types::build_variant_refinements(self.source, tokens);
        if report_errors {
            self.report_type_argument_refinement_errors(tokens, &consumed_pipes);
        }
        refinements
    }

    fn report_type_argument_refinement_errors(
        &mut self,
        tokens: &[Token],
        consumed_pipes: &[bool],
    ) {
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Pipe && !consumed_pipes[index] {
                self.error_at_token(
                    token,
                    DiagnosticRequest {
                        id: "parse.variant_refinement_type",
                        message: "`|` must join complete ADT variant refinement alternatives"
                            .to_string(),
                        parser_context: self.context,
                        expected: vec!["NamedAdtType::Variant"],
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("type argument"),
                        repair_candidates: Vec::new(),
                    },
                );
            }
        }
        for (index, message) in super::body_and_types::malformed_refinement_syntax(tokens) {
            self.error_at_token(
                &tokens[index],
                DiagnosticRequest {
                    id: "parse.variant_refinement_type",
                    message: message.to_string(),
                    parser_context: self.context,
                    expected: vec!["NamedAdtType::Variant"],
                    strategy: RecoveryStrategy::InsertToken,
                    anchor: Some("type argument"),
                    repair_candidates: Vec::new(),
                },
            );
        }
    }
}
