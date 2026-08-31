use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_written_module_path(
        &mut self,
        context: &'static str,
        allow_hole_segment: bool,
    ) -> (String, Vec<SourceSpan>) {
        let (name, span) =
            self.expect_module_path_segment(context, "module name", allow_hole_segment);
        let mut text = name.unwrap_or_else(|| "<missing>".to_string());
        let mut spans = span.into_iter().collect::<Vec<_>>();
        while self.at(TokenKind::Dot) || self.at(TokenKind::DoubleColon) {
            let delimiter = self.bump();
            if let (Some(segment), span) =
                self.expect_module_path_segment(context, "module name segment", allow_hole_segment)
            {
                text.push_str(&delimiter.text);
                text.push_str(&segment);
                if let Some(span) = span {
                    spans.push(span);
                }
            }
        }
        (text, spans)
    }

    pub(super) fn expect_module_path_segment(
        &mut self,
        context: &'static str,
        expected: &'static str,
        allow_hole_segment: bool,
    ) -> (Option<String>, Option<SourceSpan>) {
        self.expect_name(context, expected, allow_hole_segment)
    }

    pub(super) fn collect_type_until(
        &mut self,
        context: &'static str,
        stop: &[TokenKind],
    ) -> String {
        self.collect_type_paths_until(context, stop).0
    }

    pub(super) fn collect_type_paths_until(
        &mut self,
        _context: &'static str,
        stop: &[TokenKind],
    ) -> (String, Vec<TypePathSegments>) {
        let mut parts = Vec::new();
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && stop.iter().any(|kind| self.at(*kind)) {
                break;
            }
            let token = self.current().clone();
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::Less => {
                    depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                kind if closing_angle_count(kind) > 0 => {
                    depth = depth.saturating_sub(closing_angle_count(kind));
                }
                _ => {}
            }
            let token = self.bump();
            parts.push(token.text.clone());
            tokens.push(token);
        }
        (
            normalize_type_text(parts),
            self.type_paths_from_tokens(&tokens),
        )
    }

    pub(super) fn collect_return_type_until(
        &mut self,
        context: &'static str,
        stop: &[TokenKind],
    ) -> (String, Vec<TypePathSegments>) {
        let (mut ty, paths) = self.collect_type_paths_until(context, stop);
        if return_type_can_take_effects(&ty)
            && self.at(TokenKind::Effects)
            && (self.after_effect_clause_is(TokenKind::Effects)
                || self.after_effect_clause_is(TokenKind::Newline)
                || self.after_effect_clause_is(TokenKind::Eof))
        {
            let effects = self.collect_effect_clause_text();
            if !effects.is_empty() {
                ty.push(' ');
                ty.push_str(&effects);
            }
        }
        (ty, paths)
    }

    pub(super) fn after_effect_clause_is(&self, expected: TokenKind) -> bool {
        if !self.at(TokenKind::Effects) {
            return false;
        }
        let mut cursor = self.cursor + 1;
        if !self
            .tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::LBracket)
        {
            return false;
        }
        cursor += 1;
        let mut depth = 1usize;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .tokens
                            .get(cursor + 1)
                            .is_some_and(|next| next.kind == expected);
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    pub(super) fn collect_effect_clause_text(&mut self) -> String {
        let mut parts = Vec::new();
        if !self.at(TokenKind::Effects) {
            return String::new();
        }
        parts.push(self.bump().text);
        if !self.at(TokenKind::LBracket) {
            return normalize_collected_text(parts);
        }
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            match self.current().kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth = depth.saturating_sub(1);
                    parts.push(self.bump().text);
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                _ => {}
            }
            parts.push(self.bump().text);
        }
        normalize_collected_text(parts)
    }

    pub(super) fn collect_until_newline(&mut self) -> (String, Vec<Token>, TextRange) {
        let (parts, tokens, start, mut end) = self.collect_line_parts_and_tokens();
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }
        (
            parts
                .join(" ")
                .replace(" :: ", "::")
                .replace(" (", "(")
                .replace("( ", "(")
                .replace(" )", ")")
                .replace(" . ", ".")
                .replace("[ ", "[")
                .replace(" ]", "]")
                .replace(" ,", ","),
            tokens,
            start.cover(end),
        )
    }

    pub(super) fn collect_line_parts_and_tokens(
        &mut self,
    ) -> (Vec<String>, Vec<Token>, TextRange, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut parts = Vec::new();
        let mut tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            tokens.push(token);
        }
        (parts, tokens, start, end)
    }

    pub(super) fn parse_expr_for_body_line(&mut self, context: &'static str) -> (Expr, TextRange) {
        if self.at(TokenKind::Match) || self.at(TokenKind::If) {
            self.parse_block_expr_for_body_line(context)
        } else {
            self.parse_expr_until_newline(context)
        }
    }

    pub(super) fn parse_let_pattern(&mut self) -> Pattern {
        let start = self.current().range;
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) && !self.at(TokenKind::Newline) {
            if depth == 0 && (self.at(TokenKind::Colon) || self.at(TokenKind::Equal)) {
                break;
            }
            let token = self.bump();
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            tokens.push(token);
        }

        if tokens.is_empty() {
            self.error_current(
                "parse.expected_pattern",
                "expected let pattern",
                "let_statement",
                vec!["pattern"],
                RecoveryStrategy::InsertToken,
                Some("="),
            );
            return Pattern {
                kind: PatternKind::Wildcard,
                span: self.source.span(start),
            };
        }

        let (pattern, diagnostics) =
            ExprParser::new(self.source, "let_statement", &tokens).parse_pattern_only();
        self.diagnostics.extend(diagnostics);
        pattern
    }

    pub(super) fn parse_block_expr_for_body_line(
        &mut self,
        context: &'static str,
    ) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        let mut block_depth = 0usize;
        let mut previous_kind = None;
        while !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(invalid_expression_token_diagnostic(
                    self.source,
                    &token,
                    context,
                    "end",
                ));
                continue;
            }
            if token.kind == TokenKind::Match
                || (token.kind == TokenKind::If && previous_kind != Some(TokenKind::Else))
            {
                block_depth += 1;
            }
            if token.kind == TokenKind::End {
                block_depth = block_depth.saturating_sub(1);
                tokens.push(token);
                if block_depth == 0 {
                    if self.at(TokenKind::Newline) {
                        end = self.bump().range;
                    }
                    break;
                }
                continue;
            }
            previous_kind = Some(token.kind);
            tokens.push(token);
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }

    pub(super) fn parse_expr_until_newline(&mut self, context: &'static str) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        let mut block_depth = 0usize;
        let mut previous_kind = None;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && block_depth == 0 && self.at(TokenKind::Newline) {
                break;
            }
            let token = self.bump();
            end = token.range;
            let token_kind = token.kind;
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                TokenKind::Match => block_depth += 1,
                TokenKind::If if previous_kind != Some(TokenKind::Else) => block_depth += 1,
                TokenKind::End if block_depth > 0 => {
                    block_depth = block_depth.saturating_sub(1);
                }
                _ => {}
            }
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(invalid_expression_token_diagnostic(
                    self.source,
                    &token,
                    context,
                    "newline",
                ));
            } else if token.kind != TokenKind::Newline {
                tokens.push(token);
            } else {
                end = token.range;
            }
            previous_kind = Some(token_kind);
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }
    fn type_paths_from_tokens(&self, tokens: &[Token]) -> Vec<TypePathSegments> {
        let mut paths = Vec::new();
        let mut cursor = 0usize;
        while cursor < tokens.len() {
            if !is_type_path_segment(&tokens[cursor])
                || tokens.get(cursor + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon)
            {
                cursor += 1;
                continue;
            }

            let mut segments = vec![tokens[cursor].text.clone()];
            let mut segment_spans = vec![self.source.span(tokens[cursor].range)];
            cursor += 2;
            while let Some(token) = tokens.get(cursor) {
                if !is_type_path_segment(token) {
                    break;
                }
                segments.push(token.text.clone());
                segment_spans.push(self.source.span(token.range));
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
}

fn is_type_path_segment(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Ident | TokenKind::Hole)
}
