use super::*;

pub(super) use super::variant_refinement_diagnostics::malformed_refinement_syntax;
pub(super) use super::variant_refinements::build_variant_refinements;

struct ExpressionLineCollector {
    start: TextRange,
    end: TextRange,
    tokens: Vec<Token>,
    delimiter_depth: usize,
    block_stack: Vec<TokenKind>,
    previous_kind: Option<TokenKind>,
    at_line_start: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TypeNesting {
    paren: usize,
    bracket: usize,
    brace: usize,
    angle: usize,
}

struct FunctionTypeScope {
    nesting: TypeNesting,
    unstarted_returns: usize,
    started_returns: usize,
}

impl ExpressionLineCollector {
    fn new(start: TextRange) -> Self {
        Self {
            start,
            end: start,
            tokens: Vec::new(),
            delimiter_depth: 0,
            block_stack: Vec::new(),
            previous_kind: None,
            at_line_start: false,
        }
    }

    fn stops_before(&self, parser: &Parser<'_>) -> bool {
        if self.delimiter_depth == 0 && self.block_stack.is_empty() && parser.at(TokenKind::Newline)
        {
            return true;
        }
        parser.at(TokenKind::End)
            && self.block_stack.iter().copied().any(is_cleanup_block)
            && parser.end_closes_enclosing_declaration()
    }

    fn recover_before_branch(&mut self, parser: &Parser<'_>) {
        if parser.at(TokenKind::Else) {
            recover_cleanup_blocks_before_branch(&mut self.block_stack, TokenKind::If);
        } else if self.delimiter_depth == 0
            && self.at_line_start
            && line_starts_match_arm(&parser.tokens, parser.cursor)
        {
            recover_cleanup_blocks_before_branch(&mut self.block_stack, TokenKind::Match);
        }
    }

    fn push(&mut self, token: Token) {
        self.end = token.range;
        let token_kind = token.kind;
        match token_kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                self.delimiter_depth += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
            }
            TokenKind::Match | TokenKind::Begin => self.block_stack.push(token_kind),
            TokenKind::Defer
                if self.delimiter_depth == 0
                    && !self.block_stack.is_empty()
                    && self.at_line_start =>
            {
                self.block_stack.push(token_kind);
            }
            TokenKind::If if self.previous_kind != Some(TokenKind::Else) => {
                self.block_stack.push(token_kind);
            }
            TokenKind::End if !self.block_stack.is_empty() => {
                self.block_stack.pop();
            }
            _ => {}
        }
        if token_kind != TokenKind::Invalid
            && (token_kind != TokenKind::Newline || !self.block_stack.is_empty())
        {
            self.tokens.push(token);
        }
        self.at_line_start = token_kind == TokenKind::Newline;
        self.previous_kind = Some(token_kind);
    }

    fn range(&self) -> TextRange {
        self.start.cover(self.end)
    }
}

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

    pub(super) fn collect_type_paths_until(
        &mut self,
        context: &'static str,
        stop: &[TokenKind],
    ) -> (String, Vec<TypePathSegments>, Vec<VariantRefinementType>) {
        let mut parts = Vec::new();
        let mut tokens = Vec::new();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut function_type_scopes = Vec::new();
        let mut reported_unmatched_angle = false;
        while !self.at(TokenKind::Eof) {
            let contextual_callsite_type = self.at(TokenKind::Callsite)
                && (self.peek_at(TokenKind::DoubleColon)
                    || parts.is_empty()
                    || tokens
                        .last()
                        .is_some_and(|token: &Token| token.kind == TokenKind::Arrow));
            let at_stop = stop.iter().any(|kind| self.at(*kind));
            if at_stop && !contextual_callsite_type {
                let nesting = TypeNesting {
                    paren: paren_depth,
                    bracket: bracket_depth,
                    brace: brace_depth,
                    angle: angle_depth,
                };
                let outside_nested_type =
                    paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && angle_depth == 0;
                if outside_nested_type {
                    break;
                }
                if angle_depth > 0
                    && paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0
                    && self.at(TokenKind::Comma)
                    && stop.contains(&TokenKind::Comma)
                    && self.comma_precedes_named_type_sibling()
                {
                    self.report_unmatched_generic_opener(context);
                    reported_unmatched_angle = true;
                    break;
                }
                if angle_depth > 0
                    && unmatched_angle_reaches_annotation_boundary(
                        self.current().kind,
                        paren_depth,
                        bracket_depth,
                        brace_depth,
                    )
                    && !effect_clause_belongs_to_nested_function_type(
                        self.current().kind,
                        nesting,
                        &function_type_scopes,
                    )
                {
                    self.report_unmatched_generic_opener(context);
                    reported_unmatched_angle = true;
                    break;
                }
            }
            let token = self.current().clone();
            let nesting = TypeNesting {
                paren: paren_depth,
                bracket: bracket_depth,
                brace: brace_depth,
                angle: angle_depth,
            };
            update_function_type_scopes(token.kind, nesting, &mut function_type_scopes);
            match token.kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Less => angle_depth += 1,
                kind if closing_angle_count(kind) > 0 => {
                    angle_depth = angle_depth.saturating_sub(closing_angle_count(kind));
                }
                _ => {}
            }
            prune_function_type_scopes(
                &mut function_type_scopes,
                TypeNesting {
                    paren: paren_depth,
                    bracket: bracket_depth,
                    brace: brace_depth,
                    angle: angle_depth,
                },
            );
            let token = self.bump();
            parts.push(token.text.clone());
            tokens.push(token);
        }
        if angle_depth > 0 && !reported_unmatched_angle {
            self.report_unmatched_generic_opener(context);
        }
        let refinements = self.variant_refinements_from_tokens(context, &tokens);
        (
            normalize_type_text(parts),
            type_paths_from_tokens(self.source, &tokens),
            refinements,
        )
    }

    fn report_unmatched_generic_opener(&mut self, context: &'static str) {
        self.error_current(
            "parse.variant_refinement_type",
            "generic type arguments are missing a closing `>`",
            context,
            vec![">"],
            RecoveryStrategy::InsertToken,
            Some(">"),
        );
    }

    fn comma_precedes_named_type_sibling(&self) -> bool {
        self.peek_kind(1)
            .is_some_and(|kind| is_contextual_identifier(kind) || kind == TokenKind::Hole)
            && self.peek_kind(2) == Some(TokenKind::Colon)
    }

    pub(super) fn collect_return_type_until(
        &mut self,
        context: &'static str,
        stop: &[TokenKind],
    ) -> (String, Vec<TypePathSegments>, Vec<VariantRefinementType>) {
        let (mut ty, paths, refinements) = self.collect_type_paths_until(context, stop);
        if return_type_can_take_effects(&ty)
            && self.at(TokenKind::Effects)
            && (self.after_effect_clause_is(TokenKind::Effects)
                || self.after_effect_clause_is(TokenKind::Callsite)
                || self.after_effect_clause_is(TokenKind::Newline)
                || self.after_effect_clause_is(TokenKind::Eof))
        {
            let effects = self.collect_effect_clause_text();
            if !effects.is_empty() {
                ty.push(' ');
                ty.push_str(&effects);
            }
        }
        (ty, paths, refinements)
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
        self.parse_expr_until_newline(context)
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

    pub(super) fn parse_expr_until_newline(&mut self, context: &'static str) -> (Expr, TextRange) {
        let mut collector = ExpressionLineCollector::new(self.current().range);
        while !self.at(TokenKind::Eof) {
            if collector.stops_before(self) {
                break;
            }
            collector.recover_before_branch(self);
            let token = self.bump();
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(invalid_expression_token_diagnostic(
                    self.source,
                    &token,
                    context,
                    "newline",
                ));
            }
            collector.push(token);
        }
        if self.at(TokenKind::Newline) {
            collector.end = self.bump().range;
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &collector.tokens)
            .with_cleanup_depth(self.cleanup_depth)
            .parse();
        self.diagnostics.extend(diagnostics);
        (expr, collector.range())
    }
    fn variant_refinements_from_tokens(
        &mut self,
        context: &'static str,
        tokens: &[Token],
    ) -> Vec<VariantRefinementType> {
        let (refinements, consumed_pipes) = build_variant_refinements(self.source, tokens);
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Pipe && !consumed_pipes[index] {
                self.error_at_token(
                    token,
                    DiagnosticRequest {
                        id: "parse.variant_refinement_type",
                        message: "`|` must join complete ADT variant refinement alternatives"
                            .to_string(),
                        parser_context: context,
                        expected: vec!["NamedAdtType::Variant"],
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("type annotation"),
                        repair_candidates: Vec::new(),
                    },
                );
            }
        }
        for (index, message) in malformed_refinement_syntax(tokens) {
            self.error_at_token(
                &tokens[index],
                DiagnosticRequest {
                    id: "parse.variant_refinement_type",
                    message: message.to_string(),
                    parser_context: context,
                    expected: vec!["NamedAdtType::Variant"],
                    strategy: RecoveryStrategy::InsertToken,
                    anchor: Some("type annotation"),
                    repair_candidates: Vec::new(),
                },
            );
        }

        refinements
    }
}

fn unmatched_angle_reaches_annotation_boundary(
    boundary: TokenKind,
    paren_depth: usize,
    bracket_depth: usize,
    brace_depth: usize,
) -> bool {
    match boundary {
        TokenKind::Comma => false,
        TokenKind::RParen => paren_depth == 0,
        TokenKind::RBracket => bracket_depth == 0,
        TokenKind::RBrace => brace_depth == 0,
        _ => paren_depth == 0 && bracket_depth == 0 && brace_depth == 0,
    }
}

fn effect_clause_belongs_to_nested_function_type(
    boundary: TokenKind,
    nesting: TypeNesting,
    function_type_scopes: &[FunctionTypeScope],
) -> bool {
    boundary == TokenKind::Effects
        && nesting.angle > 0
        && function_type_scopes
            .last()
            .is_some_and(|scope| scope.nesting == nesting && scope.started_returns > 0)
}

fn update_function_type_scopes(
    token: TokenKind,
    nesting: TypeNesting,
    scopes: &mut Vec<FunctionTypeScope>,
) {
    record_function_type_scope_work(1);
    match token {
        TokenKind::Comma => {
            if scopes.last().is_some_and(|scope| scope.nesting == nesting) {
                scopes.pop();
            }
        }
        TokenKind::Fn => {
            if let Some(scope) = scopes.last_mut().filter(|scope| scope.nesting == nesting) {
                scope.unstarted_returns += 1;
            } else {
                scopes.push(FunctionTypeScope {
                    nesting,
                    unstarted_returns: 1,
                    started_returns: 0,
                });
            }
        }
        TokenKind::Arrow => {
            if let Some(scope) = scopes
                .last_mut()
                .filter(|scope| scope.nesting == nesting && scope.unstarted_returns > 0)
            {
                scope.unstarted_returns -= 1;
                scope.started_returns += 1;
            }
        }
        TokenKind::Effects => {
            if let Some(scope) = scopes
                .last_mut()
                .filter(|scope| scope.nesting == nesting && scope.started_returns > 0)
            {
                scope.started_returns -= 1;
                if scope.started_returns == 0 && scope.unstarted_returns == 0 {
                    scopes.pop();
                }
            }
        }
        _ => {}
    }
}

fn prune_function_type_scopes(scopes: &mut Vec<FunctionTypeScope>, nesting: TypeNesting) {
    while scopes.last().is_some_and(|scope| {
        scope.nesting.paren > nesting.paren
            || scope.nesting.bracket > nesting.bracket
            || scope.nesting.brace > nesting.brace
            || scope.nesting.angle > nesting.angle
    }) {
        record_function_type_scope_work(1);
        scopes.pop();
    }
}
