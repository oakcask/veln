fn function_scopes(tokens: &[Token]) -> Vec<FunctionScope> {
    let structure = scope_structure(tokens);
    let mut scopes = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| matches!(token.kind, TokenKind::Fn | TokenKind::Test))
        .filter_map(|(index, _)| function_scope(tokens, index, &structure.defer_block_openers))
        .collect::<Vec<_>>();
    // Keep clauses as a source-ordered suffix so lookup can binary-search them
    // without changing the precedence of ordinary scopes.
    scopes.extend(handler_operation_clause_scopes(tokens, &structure));
    scopes
}

fn function_scope(
    tokens: &[Token],
    index: usize,
    defer_block_openers: &[bool],
) -> Option<FunctionScope> {
    let body_start = tokens[index..]
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)?;
    let end = function_scope_end_with_defer_openers(tokens, index + 1, defer_block_openers)
        .unwrap_or(body_start);
    let params = parameter_names(tokens, index, body_start);
    let result_binding = result_binding_name(tokens, index, body_start);
    let local_bindings =
        local_bindings_with_defer_openers(tokens, body_start, end, defer_block_openers);
    let local_bindings_by_name = local_binding_index_by_name(&local_bindings);
    Some(FunctionScope {
        body_start,
        end,
        is_handler_clause: false,
        params,
        result_binding,
        local_bindings,
        local_bindings_by_name,
    })
}

fn handler_operation_clause_scopes(
    tokens: &[Token],
    structure: &ScopeStructure,
) -> Vec<FunctionScope> {
    let file_end = tokens.last().map_or(0, |token| token.range.end);
    structure
        .handler_operation_clause_headers
        .iter()
        .enumerate()
        .filter_map(|(index, is_header)| is_header.then_some(index))
        .map(|arrow_index| {
            let arrow = &tokens[arrow_index];
            let body_start = arrow.range.end;
            let end = handler_operation_clause_body_end_with_defer_openers_and_visit(
                tokens,
                arrow_index,
                file_end,
                &structure.defer_block_openers,
                &structure.handler_operation_clause_headers,
                record_handler_clause_scope_token_visit,
            );
            let local_bindings = local_bindings_with_defer_openers(
                tokens,
                body_start,
                end,
                &structure.defer_block_openers,
            );
            let local_bindings_by_name = local_binding_index_by_name(&local_bindings);
            FunctionScope {
                body_start,
                end,
                is_handler_clause: true,
                params: Vec::new(),
                result_binding: None,
                local_bindings,
                local_bindings_by_name,
            }
        })
        .collect()
}

struct ScopeStructure {
    defer_block_openers: Vec<bool>,
    handler_operation_clause_headers: Vec<bool>,
}

fn scope_structure(tokens: &[Token]) -> ScopeStructure {
    scope_structure_with(tokens, record_handler_clause_scope_token_visit)
}

fn scope_structure_with(tokens: &[Token], mut record_token_visit: impl FnMut()) -> ScopeStructure {
    let mut scanner = ScopeStructureScanner::new(tokens.len());
    for (index, token) in tokens.iter().enumerate() {
        record_token_visit();
        scanner.visit(index, token);
    }
    scanner.finish()
}

#[derive(Clone, Copy)]
struct BlockContext {
    kind: TokenKind,
    delimiter_depths: (usize, usize, usize),
}

struct ScopeStructureScanner {
    structure: ScopeStructure,
    defer_blocks: Vec<BlockContext>,
    handler_blocks: Vec<TokenKind>,
    delimiter_depths: (usize, usize, usize),
    line_has_non_whitespace: bool,
    first_line_token_is_identifier: Option<bool>,
    saw_lparen: bool,
    saw_rparen: bool,
    saw_fat_arrow: bool,
    previous_defer_non_layout: Option<TokenKind>,
    previous_handler_non_layout: Option<TokenKind>,
}

impl ScopeStructureScanner {
    fn new(token_count: usize) -> Self {
        Self {
            structure: ScopeStructure {
                defer_block_openers: vec![false; token_count],
                handler_operation_clause_headers: vec![false; token_count],
            },
            defer_blocks: Vec::new(),
            handler_blocks: Vec::new(),
            delimiter_depths: (0, 0, 0),
            line_has_non_whitespace: false,
            first_line_token_is_identifier: None,
            saw_lparen: false,
            saw_rparen: false,
            saw_fat_arrow: false,
            previous_defer_non_layout: None,
            previous_handler_non_layout: None,
        }
    }

    fn visit(&mut self, index: usize, token: &Token) {
        let defer_opener = self.is_defer_block_opener(token);
        self.structure.defer_block_openers[index] = defer_opener;
        self.structure.handler_operation_clause_headers[index] =
            self.is_handler_operation_clause_header(token);
        self.update_delimiter_depths(token.kind);
        self.update_defer_blocks(token.kind, defer_opener);
        self.update_handler_blocks(token.kind, defer_opener);
        self.update_line_state(token);
    }

    fn finish(self) -> ScopeStructure {
        self.structure
    }

    fn is_defer_block_opener(&self, token: &Token) -> bool {
        token.kind == TokenKind::Defer
            && !self.line_has_non_whitespace
            && self.defer_blocks.last().is_some_and(|context| {
                matches!(
                    context.kind,
                    TokenKind::Fn | TokenKind::Test | TokenKind::Begin | TokenKind::Defer
                ) && context.delimiter_depths == self.delimiter_depths
            })
    }

    fn is_handler_operation_clause_header(&self, token: &Token) -> bool {
        token.kind == TokenKind::FatArrow
            && self.handler_blocks.last() == Some(&TokenKind::Handler)
            && !self.saw_fat_arrow
            && self.first_line_token_is_identifier == Some(true)
            && self.saw_lparen
            && self.saw_rparen
    }

    fn update_delimiter_depths(&mut self, kind: TokenKind) {
        match kind {
            TokenKind::LParen => self.delimiter_depths.0 += 1,
            TokenKind::RParen => {
                self.delimiter_depths.0 = self.delimiter_depths.0.saturating_sub(1)
            }
            TokenKind::LBracket => self.delimiter_depths.1 += 1,
            TokenKind::RBracket => {
                self.delimiter_depths.1 = self.delimiter_depths.1.saturating_sub(1)
            }
            TokenKind::LBrace => self.delimiter_depths.2 += 1,
            TokenKind::RBrace => {
                self.delimiter_depths.2 = self.delimiter_depths.2.saturating_sub(1)
            }
            _ => {}
        }
    }

    fn update_defer_blocks(&mut self, kind: TokenKind, defer_opener: bool) {
        let opens_top_level_declaration = matches!(
            kind,
            TokenKind::Fn
                | TokenKind::Test
                | TokenKind::Type
                | TokenKind::Schema
                | TokenKind::Effect
                | TokenKind::Handler
                | TokenKind::Codec
        ) && self.delimiter_depths == (0, 0, 0)
            && (!self.line_has_non_whitespace
                || self.previous_defer_non_layout == Some(TokenKind::Pub));
        let opens_nested_block = matches!(kind, TokenKind::Match | TokenKind::Begin)
            || (kind == TokenKind::If && self.previous_defer_non_layout != Some(TokenKind::Else))
            || defer_opener;
        if opens_top_level_declaration || opens_nested_block {
            self.defer_blocks.push(BlockContext {
                kind,
                delimiter_depths: self.delimiter_depths,
            });
        } else if kind == TokenKind::End {
            self.defer_blocks.pop();
        }
    }

    fn update_handler_blocks(&mut self, kind: TokenKind, defer_opener: bool) {
        let opens_declaration = matches!(
            kind,
            TokenKind::Fn | TokenKind::Test | TokenKind::Handler | TokenKind::Codec
        ) && (!self.line_has_non_whitespace
            || self.previous_handler_non_layout == Some(TokenKind::Pub));
        let opens_nested_block = matches!(kind, TokenKind::Match | TokenKind::Begin)
            || (kind == TokenKind::If && self.previous_handler_non_layout != Some(TokenKind::Else))
            || defer_opener;
        if opens_declaration || opens_nested_block {
            self.handler_blocks.push(kind);
        } else if kind == TokenKind::End {
            self.handler_blocks.pop();
        }
    }

    fn update_line_state(&mut self, token: &Token) {
        match token.kind {
            TokenKind::Newline => {
                self.line_has_non_whitespace = false;
                self.first_line_token_is_identifier = None;
                self.saw_lparen = false;
                self.saw_rparen = false;
                self.saw_fat_arrow = false;
            }
            TokenKind::Whitespace => {}
            TokenKind::LParen => {
                self.mark_non_whitespace();
                self.saw_lparen = true;
            }
            TokenKind::RParen => {
                self.mark_non_whitespace();
                self.saw_rparen = true;
            }
            TokenKind::FatArrow => {
                self.mark_non_whitespace();
                self.saw_fat_arrow = true;
            }
            _ => {
                self.line_has_non_whitespace = true;
                self.first_line_token_is_identifier
                    .get_or_insert(token.kind == TokenKind::Ident && is_identifier(&token.text));
            }
        }
        if !is_layout_token_kind(token.kind) {
            self.previous_defer_non_layout = Some(token.kind);
        }
        if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline) {
            self.previous_handler_non_layout = Some(token.kind);
        }
    }

    fn mark_non_whitespace(&mut self) {
        self.line_has_non_whitespace = true;
        self.first_line_token_is_identifier.get_or_insert(false);
    }
}
