use super::*;

struct ExpressionLineCollector {
    start: TextRange,
    end: TextRange,
    tokens: Vec<Token>,
    delimiter_depth: usize,
    block_stack: Vec<TokenKind>,
    previous_kind: Option<TokenKind>,
    at_line_start: bool,
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
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            let contextual_callsite_type = self.at(TokenKind::Callsite)
                && (self.peek_at(TokenKind::DoubleColon)
                    || parts.is_empty()
                    || tokens
                        .last()
                        .is_some_and(|token: &Token| token.kind == TokenKind::Arrow));
            if depth == 0 && stop.iter().any(|kind| self.at(*kind)) && !contextual_callsite_type {
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
        let refinements = self.variant_refinements_from_tokens(context, &tokens);
        (
            normalize_type_text(parts),
            self.type_paths_from_tokens(&tokens),
            refinements,
        )
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
    fn type_paths_from_tokens(&self, tokens: &[Token]) -> Vec<TypePathSegments> {
        let mut paths = Vec::new();
        let mut cursor = 0usize;
        while cursor < tokens.len() {
            if tokens[cursor].kind == TokenKind::Effects {
                cursor = skip_effect_clause(tokens, cursor);
                continue;
            }
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

pub(super) fn malformed_refinement_syntax(tokens: &[Token]) -> Vec<(usize, &'static str)> {
    let mut errors = Vec::new();
    let has_refinement_union = tokens.iter().any(|token| token.kind == TokenKind::Pipe);
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::DoubleColon {
            let left_can_be_base = index > 0
                && (is_type_path_segment(&tokens[index - 1])
                    || closing_angle_count(tokens[index - 1].kind) > 0);
            let right_is_variant = tokens
                .get(index + 1)
                .is_some_and(|token| is_type_path_segment(token) && starts_uppercase(&token.text));
            if !left_can_be_base && right_is_variant {
                errors.push((index, "variant refinement is missing its ADT base type"));
            } else if left_can_be_base && tokens.get(index + 1).is_none() {
                errors.push((
                    index,
                    "variant refinement is missing its final variant name",
                ));
            }
        }
        if has_refinement_union
            && token.kind == TokenKind::Less
            && index >= 3
            && is_type_path_segment(&tokens[index - 1])
            && starts_uppercase(&tokens[index - 1].text)
            && tokens[index - 2].kind == TokenKind::DoubleColon
            && is_type_path_segment(&tokens[index - 3])
            && starts_uppercase(&tokens[index - 3].text)
        {
            errors.push((
                index,
                "variant refinement type arguments must precede the final `::Variant` segment",
            ));
        }
        if token.kind == TokenKind::Less
            && tokens
                .get(index + 1)
                .is_some_and(|next| closing_angle_count(next.kind) > 0)
            && matching_angle_token(tokens, index).is_some_and(|close| {
                tokens.get(close + 1).map(|next| next.kind) == Some(TokenKind::DoubleColon)
            })
        {
            errors.push((
                index,
                "variant refinement generic base has an empty type argument",
            ));
        }
        if closing_angle_count(token.kind) > 0
            && tokens
                .get(index + 1)
                .is_some_and(|next| is_type_path_segment(next) && starts_uppercase(&next.text))
        {
            errors.push((
                index + 1,
                "variant refinement generic base must be followed by `::Variant`",
            ));
        }
    }
    errors
}

pub(super) fn build_variant_refinements(
    source: &SourceFile,
    tokens: &[Token],
) -> (Vec<VariantRefinementType>, Vec<bool>) {
    let candidates = refinement_alternatives(source, tokens);
    let mut grouped = vec![false; candidates.len()];
    let mut consumed_pipes = vec![false; tokens.len()];
    let mut refinements = Vec::new();

    for start in 0..candidates.len() {
        if grouped[start] {
            continue;
        }
        let mut alternative_indexes = vec![start];
        let mut pipe_indexes = Vec::new();
        let mut current = start;
        loop {
            let pipe_index = candidates[current].end_index + 1;
            if tokens.get(pipe_index).map(|token| token.kind) != Some(TokenKind::Pipe) {
                break;
            }
            let Some(next) = candidates.iter().position(|candidate| {
                candidate.start_index == pipe_index + 1
                    && !alternative_indexes.contains(&candidate.index)
            }) else {
                break;
            };
            pipe_indexes.push(pipe_index);
            alternative_indexes.push(next);
            current = next;
        }
        if pipe_indexes.is_empty() {
            continue;
        }
        for index in &alternative_indexes {
            grouped[*index] = true;
        }
        for index in &pipe_indexes {
            consumed_pipes[*index] = true;
        }
        let alternatives = alternative_indexes
            .iter()
            .map(|index| candidates[*index].value.clone())
            .collect::<Vec<_>>();
        let first_span = &alternatives
            .first()
            .expect("refinement union has alternatives")
            .span;
        let span = SourceSpan {
            file: first_span.file.clone(),
            start: first_span.start,
            end: alternatives
                .last()
                .expect("refinement union has alternatives")
                .span
                .end,
        };
        refinements.push(VariantRefinementType {
            alternatives,
            pipe_spans: pipe_indexes
                .iter()
                .map(|index| source.span(tokens[*index].range))
                .collect(),
            span,
        });
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if !grouped[index] {
            refinements.push(VariantRefinementType {
                alternatives: vec![candidate.value.clone()],
                pipe_spans: Vec::new(),
                span: candidate.value.span.clone(),
            });
        }
    }
    refinements.sort_by_key(|refinement| refinement.span.start.offset);
    (refinements, consumed_pipes)
}

#[derive(Clone)]
struct RefinementCandidate {
    index: usize,
    start_index: usize,
    end_index: usize,
    value: VariantRefinementAlternative,
}

fn refinement_alternatives(source: &SourceFile, tokens: &[Token]) -> Vec<RefinementCandidate> {
    let mut candidates = Vec::new();
    for start in 0..tokens.len() {
        if !is_type_path_segment(&tokens[start])
            || (start > 0 && tokens[start - 1].kind == TokenKind::DoubleColon)
        {
            continue;
        }
        if let Some(candidate) = refinement_alternative_at(source, tokens, start, candidates.len())
        {
            candidates.push(candidate);
        }
    }
    candidates
}

fn refinement_alternative_at(
    source: &SourceFile,
    tokens: &[Token],
    start: usize,
    index: usize,
) -> Option<RefinementCandidate> {
    let mut cursor = start;
    let mut base_end = start;
    while tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::DoubleColon)
        && tokens.get(cursor + 2).is_some_and(is_type_path_segment)
    {
        cursor += 2;
        base_end = cursor;
    }

    let (base_end, type_arguments, variant_index) =
        if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::Less) {
            let open = cursor + 1;
            let close = matching_angle_token(tokens, open)?;
            if tokens.get(close + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon)
                || !tokens.get(close + 2).is_some_and(is_type_path_segment)
            {
                return None;
            }
            (
                cursor,
                refinement_type_arguments(source, tokens, open + 1, close),
                close + 2,
            )
        } else {
            let segment_count = (base_end - start) / 2 + 1;
            if segment_count < 2 {
                return None;
            }
            let variant_index = base_end;
            base_end = base_end.saturating_sub(2);
            (base_end, Vec::new(), variant_index)
        };

    let base_leaf = tokens.get(base_end)?;
    let variant = tokens.get(variant_index)?;
    if !starts_uppercase(&base_leaf.text) || !starts_uppercase(&variant.text) {
        return None;
    }

    let mut segments = Vec::new();
    let mut segment_spans = Vec::new();
    let mut base_cursor = start;
    loop {
        segments.push(tokens[base_cursor].text.clone());
        segment_spans.push(source.span(tokens[base_cursor].range));
        if base_cursor == base_end {
            break;
        }
        base_cursor += 2;
    }
    let span = source.span(tokens[start].range.cover(variant.range));
    Some(RefinementCandidate {
        index,
        start_index: start,
        end_index: variant_index,
        value: VariantRefinementAlternative {
            base: TypePathSegments {
                segments,
                segment_spans,
            },
            type_arguments,
            variant: variant.text.clone(),
            variant_span: source.span(variant.range),
            span,
        },
    })
}

fn matching_angle_token(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (index, token) in tokens.iter().enumerate().skip(open + 1) {
        if token.kind == TokenKind::Less {
            depth += 1;
            continue;
        }
        let close_count = closing_angle_count(token.kind);
        if close_count >= depth {
            return Some(index);
        }
        depth -= close_count;
    }
    None
}

fn refinement_type_arguments(
    source: &SourceFile,
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Vec<VariantRefinementTypeArgument> {
    let mut arguments = Vec::new();
    let mut argument_start = start;
    let mut delimiter_depth = 0usize;
    for index in start..end {
        match tokens[index].kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::Less => {
                delimiter_depth += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                delimiter_depth = delimiter_depth.saturating_sub(1);
            }
            kind if closing_angle_count(kind) > 0 => {
                delimiter_depth = delimiter_depth.saturating_sub(closing_angle_count(kind));
            }
            TokenKind::Comma if delimiter_depth == 0 => {
                push_refinement_type_argument(
                    source,
                    tokens,
                    argument_start,
                    index,
                    None,
                    &mut arguments,
                );
                argument_start = index + 1;
            }
            _ => {}
        }
    }
    let content_end = tokens
        .get(end)
        .map(|token| token.range.end.saturating_sub(1));
    push_refinement_type_argument(
        source,
        tokens,
        argument_start,
        end,
        content_end,
        &mut arguments,
    );
    arguments
}

fn push_refinement_type_argument(
    source: &SourceFile,
    tokens: &[Token],
    start: usize,
    end: usize,
    explicit_end: Option<usize>,
    arguments: &mut Vec<VariantRefinementTypeArgument>,
) {
    let Some(first) = tokens.get(start) else {
        return;
    };
    let range = if let Some(end) = explicit_end {
        TextRange::new(first.range.start, end)
    } else {
        let Some(last) = tokens.get(end.saturating_sub(1)) else {
            return;
        };
        first.range.cover(last.range)
    };
    let text = &source.text()[range.start..range.end];
    let leading = text.len() - text.trim_start().len();
    let trailing = text.len() - text.trim_end().len();
    let range = TextRange::new(range.start + leading, range.end.saturating_sub(trailing));
    arguments.push(VariantRefinementTypeArgument {
        text: source.text()[range.start..range.end].to_string(),
        span: source.span(range),
    });
}

fn starts_uppercase(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

fn skip_effect_clause(tokens: &[Token], cursor: usize) -> usize {
    let mut cursor = cursor + 1;
    if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::LBracket) {
        return cursor;
    }
    cursor += 1;
    let mut depth = 1usize;
    while let Some(token) = tokens.get(cursor) {
        match token.kind {
            TokenKind::LBracket => depth += 1,
            TokenKind::RBracket => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return cursor + 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

fn is_type_path_segment(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole
    )
}
