use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_begin(&mut self) -> Expr {
        let start = self.bump().range;
        let header_end = self.expect_begin_newline("begin expression must continue on a new line");
        if self.cleanup_depth >= MAX_CLEANUP_NESTING {
            return self.recover_overdeep_begin(start, header_end);
        }
        self.cleanup_depth += 1;
        let mut body = Vec::new();
        self.eat_newlines();
        while !self.at(TokenKind::End) && !self.at_cleanup_region_boundary() && !self.is_at_end() {
            body.push(self.parse_cleanup_body_line());
            self.eat_newlines();
        }
        let close = self.eat(TokenKind::End);
        if close.is_none() {
            self.error_current(
                "parse.begin_missing_end",
                "begin expression is missing `end`",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }
        let end = close.as_ref().map_or_else(
            || body.last().map_or(header_end, body_line_range),
            |token| token.range,
        );
        let block_end = close.as_ref().map_or(end.end, |token| token.range.start);
        self.cleanup_depth -= 1;
        Expr {
            kind: ExprKind::Begin {
                body,
                block_span: self.source.span(TextRange::new(
                    header_end.end,
                    block_end.max(header_end.end),
                )),
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_cleanup_body_line(&mut self) -> BodyLine {
        let start = self.current().range;
        if self.at(TokenKind::Defer) {
            return self.parse_nested_defer();
        }
        if self.at(TokenKind::Let) {
            return self.parse_cleanup_let_line(start);
        }

        let expr = self.parse_expr(0);
        let end = self.finish_cleanup_body_line(&expr);
        BodyLine::Expr {
            expr,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_cleanup_let_line(&mut self, start: TextRange) -> BodyLine {
        self.bump();
        let pattern = self.parse_pattern();
        let (annotation, annotation_paths, annotation_refinements) =
            self.parse_cleanup_let_annotation();
        self.expect_expr_token(
            TokenKind::Equal,
            "parse.let_statement",
            "let statement is missing `=`",
            vec!["="],
        );
        let expr = self.parse_expr(0);
        let end = self.finish_cleanup_body_line(&expr);
        BodyLine::Let {
            pattern,
            annotation,
            annotation_paths: annotation_paths.into_boxed_slice(),
            annotation_refinements: annotation_refinements.into_boxed_slice(),
            expr,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_cleanup_let_annotation(
        &mut self,
    ) -> (
        Option<String>,
        Vec<TypePathSegments>,
        Vec<VariantRefinementType>,
    ) {
        if self.eat(TokenKind::Colon).is_none() {
            return (None, Vec::new(), Vec::new());
        }
        let tokens = self.collect_cleanup_type_annotation_tokens();
        let refinements = self.validate_cleanup_variant_refinements(&tokens);
        let parts = tokens.iter().map(|token| token.text.clone()).collect();
        (
            Some(normalize_type_text(parts)),
            cleanup_type_paths(self.source, &tokens),
            refinements,
        )
    }

    fn collect_cleanup_type_annotation_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        while !self.is_at_end() {
            if depth == 0 && (self.at(TokenKind::Equal) || self.at(TokenKind::Newline)) {
                break;
            }
            let token = self.bump();
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::Less => {
                    depth += 1
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1)
                }
                kind if closing_angle_count(kind) > 0 => {
                    depth = depth.saturating_sub(closing_angle_count(kind))
                }
                _ => {}
            }
            tokens.push(token);
        }
        tokens
    }

    fn validate_cleanup_variant_refinements(
        &mut self,
        tokens: &[Token],
    ) -> Vec<VariantRefinementType> {
        let (refinements, consumed_pipes) =
            super::body_and_types::build_variant_refinements(self.source, tokens);
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Pipe && !consumed_pipes[index] {
                self.report_cleanup_variant_refinement_error(
                    token,
                    "`|` must join complete ADT variant refinement alternatives",
                    RecoveryStrategy::SkipToken,
                );
            }
        }
        for (index, message) in super::body_and_types::malformed_refinement_syntax(tokens) {
            self.report_cleanup_variant_refinement_error(
                &tokens[index],
                message,
                RecoveryStrategy::InsertToken,
            );
        }
        refinements
    }

    fn report_cleanup_variant_refinement_error(
        &mut self,
        token: &Token,
        message: &str,
        strategy: RecoveryStrategy,
    ) {
        self.error_at_token(
            token,
            DiagnosticRequest {
                id: "parse.variant_refinement_type",
                message: message.to_string(),
                parser_context: "let_statement",
                expected: vec!["NamedAdtType::Variant"],
                strategy,
                anchor: Some("type annotation"),
                repair_candidates: Vec::new(),
            },
        );
    }

    fn parse_nested_defer(&mut self) -> BodyLine {
        let start = self.bump().range;
        let header_end = self.expect_begin_newline("defer statement must continue on a new line");
        if self.cleanup_depth >= MAX_CLEANUP_NESTING {
            return self.recover_overdeep_defer(start, header_end);
        }
        self.cleanup_depth += 1;
        let body = self.parse_nested_cleanup_body();
        let (end, block_end) = self.close_nested_defer(&body, header_end);
        self.cleanup_depth -= 1;
        self.eat(TokenKind::Newline);
        BodyLine::Defer {
            body,
            keyword_span: self.source.span(start),
            block_span: self.source.span(TextRange::new(
                header_end.end,
                block_end.max(header_end.end),
            )),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_nested_cleanup_body(&mut self) -> Vec<BodyLine> {
        let mut body = Vec::new();
        self.eat_newlines();
        while !self.at(TokenKind::End) && !self.at_cleanup_region_boundary() && !self.is_at_end() {
            body.push(self.parse_cleanup_body_line());
            self.eat_newlines();
        }
        body
    }

    fn close_nested_defer(
        &mut self,
        body: &[BodyLine],
        header_end: TextRange,
    ) -> (TextRange, usize) {
        let close = self.eat(TokenKind::End);
        if close.is_none() {
            self.error_current(
                "parse.defer_missing_end",
                "defer statement is missing `end`",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }
        let end = close.as_ref().map_or_else(
            || body.last().map_or(header_end, body_line_range),
            |token| token.range,
        );
        let block_end = close.as_ref().map_or(end.end, |token| token.range.start);
        (end, block_end)
    }

    fn recover_overdeep_begin(&mut self, start: TextRange, header_end: TextRange) -> Expr {
        self.report_cleanup_nesting_limit();
        let end = self.skip_cleanup_region(header_end);
        Expr {
            kind: ExprKind::Begin {
                body: Vec::new(),
                block_span: self.source.span(TextRange::new(header_end.end, end.start)),
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn recover_overdeep_defer(&mut self, start: TextRange, header_end: TextRange) -> BodyLine {
        self.report_cleanup_nesting_limit();
        let end = self.skip_cleanup_region(header_end);
        if self.at(TokenKind::Newline) {
            self.bump();
        }
        BodyLine::Defer {
            body: Vec::new(),
            keyword_span: self.source.span(start),
            block_span: self.source.span(TextRange::new(header_end.end, end.start)),
            span: self.source.span(start.cover(end)),
        }
    }

    fn report_cleanup_nesting_limit(&mut self) {
        self.error_current(
            "parse.cleanup_nesting_limit",
            "cleanup regions are nested too deeply",
            vec!["less deeply nested cleanup regions"],
            RecoveryStrategy::CloseBlock,
            Some("end"),
        );
    }

    fn skip_cleanup_region(&mut self, fallback: TextRange) -> TextRange {
        let mut depth = 1usize;
        let mut end = fallback;
        let mut previous_kind = None;
        let mut at_line_start = true;
        let mut delimiter_depth = 0usize;
        while !self.is_at_end() {
            let token = self.bump();
            let kind = token.kind;
            match kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    delimiter_depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    delimiter_depth = delimiter_depth.saturating_sub(1);
                }
                TokenKind::Match | TokenKind::Begin => depth += 1,
                TokenKind::Defer if at_line_start && delimiter_depth == 0 => depth += 1,
                TokenKind::If if previous_kind != Some(TokenKind::Else) => depth += 1,
                TokenKind::End => {
                    depth -= 1;
                    if depth == 0 {
                        return token.range;
                    }
                }
                _ => {}
            }
            at_line_start = kind == TokenKind::Newline;
            previous_kind = Some(kind);
            end = token.range;
        }
        end
    }

    fn expect_begin_newline(&mut self, message: &'static str) -> TextRange {
        if let Some(token) = self.eat(TokenKind::Newline) {
            token.range
        } else {
            self.error_current(
                "parse.expected_newline",
                message,
                vec!["newline"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
            self.previous()
                .map_or(self.current().range, |token| token.range)
        }
    }

    fn finish_cleanup_body_line(&mut self, expr: &Expr) -> TextRange {
        if !self.at(TokenKind::Newline) && !self.at(TokenKind::End) && !self.is_at_end() {
            self.report_trailing_tokens(
                "parse.expected_newline",
                "expected a newline before this token",
            );
            while !self.at(TokenKind::Newline) && !self.at(TokenKind::End) && !self.is_at_end() {
                self.bump();
            }
        }
        self.eat(TokenKind::Newline)
            .map_or_else(|| lhs_range(expr), |token| token.range)
    }

    fn at_cleanup_region_boundary(&self) -> bool {
        (self.at(TokenKind::Else) && self.control_blocks.contains(&TokenKind::If))
            || (self.control_blocks.contains(&TokenKind::Match)
                && line_starts_match_arm(self.tokens, self.cursor))
    }

    pub(super) fn parse_match(&mut self) -> Expr {
        let start = self.bump().range;
        let scrutinee = self.parse_expr(0);
        self.eat_newlines();
        self.control_blocks.push(TokenKind::Match);
        let arms = self.parse_match_arms();
        let end = self.close_match_expression(&scrutinee, &arms);
        self.control_blocks.pop();
        Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_match_arms(&mut self) -> Vec<MatchArm> {
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
        arms
    }

    fn close_match_expression(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> TextRange {
        self.eat(TokenKind::End).map_or_else(
            || {
                arms.last().map_or(lhs_range(scrutinee), |arm| {
                    TextRange::new(arm.span.start.offset, arm.span.end.offset)
                })
            },
            |token| token.range,
        )
    }

    pub(super) fn parse_if(&mut self) -> Expr {
        let start = self.bump().range;
        let condition = self.parse_if_condition("if condition is missing an expression");
        self.eat_newlines();
        self.control_blocks.push(TokenKind::If);
        let then_branch = self.parse_if_branch_expr();
        self.eat_newlines();
        let (else_if_branches, else_branch) = self.parse_else_branches();
        let else_branch = self.require_final_else_branch(else_branch);
        let end = self.close_if_expression(&else_branch);
        self.control_blocks.pop();

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

    fn parse_else_branches(&mut self) -> (Vec<IfBranch>, Option<Expr>) {
        let mut else_if_branches = Vec::new();
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
            let else_branch = self.parse_if_branch_expr();
            self.eat_newlines();
            return (else_if_branches, Some(else_branch));
        }
        (else_if_branches, None)
    }

    fn require_final_else_branch(&mut self, else_branch: Option<Expr>) -> Expr {
        else_branch.unwrap_or_else(|| {
            self.error_current(
                "parse.if_missing_else",
                "if expression is missing a final `else` branch",
                vec!["else"],
                RecoveryStrategy::InsertToken,
                Some("else"),
            );
            self.missing_expr_at_current()
        })
    }

    fn close_if_expression(&mut self, else_branch: &Expr) -> TextRange {
        if let Some(token) = self.eat(TokenKind::End) {
            token.range
        } else {
            self.error_current(
                "parse.if_missing_end",
                "if expression is missing `end`",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
            lhs_range(else_branch)
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

    pub(super) fn parse_satisfy_clause(&mut self) -> Option<SatisfyClause> {
        if !self.at_ident_text("satisfy") {
            return None;
        }
        let mut clause_recovered = false;
        let start = self.bump().range;
        let (candidate, candidate_span) = if matches!(
            self.current().kind,
            TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole
        ) {
            let token = self.bump();
            let span = self.source.span(token.range);
            (Some(token.text), Some(span))
        } else {
            clause_recovered = true;
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
            clause_recovered = true;
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
        let (parts, predicate_tokens, predicate_end) = self.collect_satisfy_predicate();
        end = predicate_end.unwrap_or(end);
        let predicate_output = ContractPredicateParser::new(
            self.source,
            "satisfy_predicate",
            "parse.satisfy_predicate",
            &predicate_tokens,
        )
        .parse();
        self.diagnostics.extend(predicate_output.diagnostics);
        Some(SatisfyClause {
            candidate,
            candidate_span,
            predicate: normalize_collected_text(parts),
            perform_effect_spans: if clause_recovered {
                Vec::new()
            } else {
                predicate_output.perform_effect_spans
            },
            span: self.source.span(start.cover(end)),
        })
    }

    fn collect_satisfy_predicate(&mut self) -> (Vec<String>, Vec<Token>, Option<TextRange>) {
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        let mut depth = 0usize;
        let mut end = None;
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
            end = Some(token.range);
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        (parts, predicate_tokens, end)
    }
}

fn body_line_range(line: &BodyLine) -> TextRange {
    match line {
        BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } | BodyLine::Defer { span, .. } => {
            TextRange::new(span.start.offset, span.end.offset)
        }
    }
}

fn cleanup_type_paths(source: &SourceFile, tokens: &[Token]) -> Vec<TypePathSegments> {
    let mut paths = Vec::new();
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        if !matches!(
            tokens[cursor].kind,
            TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole
        ) || tokens.get(cursor + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon)
        {
            cursor += 1;
            continue;
        }
        let mut segments = vec![tokens[cursor].text.clone()];
        let mut segment_spans = vec![source.span(tokens[cursor].range)];
        cursor += 2;
        while let Some(token) = tokens.get(cursor) {
            if !matches!(
                token.kind,
                TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole
            ) {
                break;
            }
            segments.push(token.text.clone());
            segment_spans.push(source.span(token.range));
            cursor += 1;
            if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::DoubleColon) {
                break;
            }
            cursor += 1;
        }
        if segments.len() > 1 {
            paths.push(TypePathSegments {
                segments,
                segment_spans,
            });
        }
    }
    paths
}
