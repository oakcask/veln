use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EffectNameBoundary {
    BeforeEffects,
    ThroughRightBracket,
}

impl<'a> Classifier<'a> {
    pub(super) fn new(
        source: &'a SourceFile,
        tokens: &'a [Token],
        function_names: BTreeSet<String>,
    ) -> Self {
        Self {
            source,
            tokens,
            function_names,
            params: BTreeSet::new(),
            locals: BTreeSet::new(),
            cursor: 0,
        }
    }

    pub(super) fn collect(&mut self) -> Vec<SemanticToken> {
        let mut semantic_tokens = Vec::new();
        while self.cursor < self.tokens.len() {
            match self.tokens[self.cursor].kind {
                TokenKind::Mod | TokenKind::Use => {
                    self.collect_namespace_directive(&mut semantic_tokens);
                }
                TokenKind::Type
                | TokenKind::Schema
                | TokenKind::Handler
                | TokenKind::Codec
                | TokenKind::Fn
                | TokenKind::Test
                | TokenKind::Pub => self.collect_declaration(&mut semantic_tokens),
                TokenKind::Format | TokenKind::Let | TokenKind::Effects => {
                    self.collect_clause(&mut semantic_tokens);
                }
                _ => self.collect_plain_token(&mut semantic_tokens),
            }
        }
        semantic_tokens
    }

    fn collect_namespace_directive(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        let kind = self.tokens[self.cursor].kind;
        self.collect_keyword(semantic_tokens);
        match kind {
            TokenKind::Mod => self.collect_module_name(semantic_tokens),
            TokenKind::Use => self.collect_use_name(semantic_tokens),
            _ => unreachable!("namespace directive dispatch only accepts mod or use"),
        }
    }

    fn collect_declaration(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        match self.tokens[self.cursor].kind {
            TokenKind::Type => self.collect_type_header(semantic_tokens),
            TokenKind::Schema => self.collect_schema_header(semantic_tokens),
            TokenKind::Handler => self.collect_handler_header(semantic_tokens),
            TokenKind::Codec => self.collect_codec_header(semantic_tokens),
            TokenKind::Fn | TokenKind::Test => self.collect_function_header(semantic_tokens),
            TokenKind::Pub => self.collect_public_declaration(semantic_tokens),
            _ => unreachable!("declaration dispatch only accepts declaration keywords"),
        }
    }

    fn collect_public_declaration(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        match self.next_significant_kind() {
            Some(TokenKind::Type) => self.collect_type_header(semantic_tokens),
            Some(TokenKind::Schema) => self.collect_schema_header(semantic_tokens),
            Some(TokenKind::Handler) => self.collect_handler_header(semantic_tokens),
            Some(TokenKind::Codec) => self.collect_codec_header(semantic_tokens),
            _ => self.collect_function_header(semantic_tokens),
        }
    }

    fn collect_clause(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        match self.tokens[self.cursor].kind {
            TokenKind::Format => self.collect_format_clause(semantic_tokens),
            TokenKind::Let => {
                self.collect_keyword(semantic_tokens);
                self.collect_let_pattern(semantic_tokens);
            }
            TokenKind::Effects => {
                self.collect_keyword(semantic_tokens);
                self.collect_effect_list(semantic_tokens);
            }
            _ => unreachable!("clause dispatch only accepts clause keywords"),
        }
    }

    fn collect_type_header(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        while self.at(TokenKind::Pub) || self.at(TokenKind::Type) {
            self.collect_keyword(semantic_tokens);
            self.skip_trivia();
        }
        if self.at(TokenKind::Ident) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.modified(
                token,
                SemanticTokenType::Type,
                &[SemanticTokenModifier::Declaration],
            ));
            self.cursor += 1;
        }
    }

    fn collect_codec_header(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        while self.at(TokenKind::Pub) || self.at(TokenKind::Codec) {
            self.collect_keyword(semantic_tokens);
            self.skip_trivia();
        }
    }

    fn collect_format_clause(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_keyword(semantic_tokens);
        self.skip_trivia();
        if self.at(TokenKind::Ident) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.simple(token, SemanticTokenType::EnumMember));
            self.cursor += 1;
        }
    }

    fn collect_keyword(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        let token = &self.tokens[self.cursor];
        semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
        self.cursor += 1;
    }

    fn collect_plain_token(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        if let Some(classified) = self.classify_current_token() {
            semantic_tokens.push(classified);
        }
        self.cursor += 1;
    }

    fn collect_handler_header(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.params.clear();
        self.locals.clear();
        self.collect_named_declaration_header(
            semantic_tokens,
            TokenKind::Handler,
            SemanticTokenType::Function,
            true,
        );
        if self.eat(TokenKind::LParen, semantic_tokens) {
            self.collect_parameters(semantic_tokens);
        }
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Handles) {
                let token = &self.tokens[self.cursor];
                semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                self.cursor += 1;
                self.collect_effect_path(semantic_tokens);
            } else if self.at(TokenKind::Effects) {
                let token = &self.tokens[self.cursor];
                semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                self.cursor += 1;
                self.collect_effect_list(semantic_tokens);
            } else {
                self.collect_plain_token(semantic_tokens);
            }
        }
        while self.at(TokenKind::Newline) {
            self.cursor += 1;
        }
        let handler_params = self.params.clone();
        while !self.at(TokenKind::End) && !self.at(TokenKind::Eof) {
            self.params = handler_params.clone();
            self.collect_handler_operation_clause(semantic_tokens);
            while self.at(TokenKind::Newline) {
                self.cursor += 1;
            }
        }
        self.params = handler_params;
        if self.at(TokenKind::End) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
            self.cursor += 1;
        }
    }

    fn collect_handler_operation_clause(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.skip_trivia();
        if self.at(TokenKind::Ident) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.simple(token, SemanticTokenType::Property));
            self.cursor += 1;
        }
        if self.eat(TokenKind::LParen, semantic_tokens) {
            self.collect_handler_operation_parameters(semantic_tokens);
        }
        let body_end = self.handler_operation_clause_body_end();
        while self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind != TokenKind::Eof && token.range.start < body_end)
        {
            self.collect_plain_token(semantic_tokens);
        }
    }

    fn handler_operation_clause_body_end(&self) -> usize {
        let Some(arrow_index) = self
            .tokens
            .iter()
            .enumerate()
            .skip(self.cursor)
            .take_while(|(_, token)| !matches!(token.kind, TokenKind::Newline | TokenKind::Eof))
            .find_map(|(index, token)| (token.kind == TokenKind::FatArrow).then_some(index))
        else {
            return self.source.text().len();
        };
        let mut nested_blocks = 0usize;
        for (relative_index, token) in self.tokens[arrow_index + 1..].iter().enumerate() {
            let index = arrow_index + 1 + relative_index;
            match token.kind {
                TokenKind::Eof => return self.source.text().len(),
                TokenKind::If if !is_else_if(self.tokens, index) => nested_blocks += 1,
                TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
                TokenKind::End if nested_blocks == 0 => return token.range.start,
                TokenKind::End => nested_blocks = nested_blocks.saturating_sub(1),
                TokenKind::FatArrow
                    if nested_blocks == 0 && !is_satisfy_arrow(self.tokens, index) =>
                {
                    return handler_clause_pattern_start_from_arrow(self.tokens, token.range.start);
                }
                _ => {}
            }
        }
        self.source.text().len()
    }

    fn collect_handler_operation_parameters(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_parameter_list(semantic_tokens, false);
    }

    fn collect_parameter_list(
        &mut self,
        semantic_tokens: &mut Vec<SemanticToken>,
        require_type_separator: bool,
    ) {
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident
                && (!require_type_separator
                    || self.next_significant_kind() == Some(TokenKind::Colon))
            {
                self.params.insert(token.text.clone());
                semantic_tokens.push(self.modified(
                    token,
                    SemanticTokenType::Parameter,
                    &[
                        SemanticTokenModifier::Declaration,
                        SemanticTokenModifier::Readonly,
                    ],
                ));
                self.cursor += 1;
            } else {
                self.collect_plain_token(semantic_tokens);
            }
        }
        self.eat(TokenKind::RParen, semantic_tokens);
    }

    fn collect_effect_path(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_effect_names(semantic_tokens, EffectNameBoundary::BeforeEffects);
    }

    fn collect_schema_header(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.params.clear();
        self.locals.clear();
        self.collect_named_declaration_header(
            semantic_tokens,
            TokenKind::Schema,
            SemanticTokenType::Type,
            false,
        );
    }

    fn collect_named_declaration_header(
        &mut self,
        semantic_tokens: &mut Vec<SemanticToken>,
        declaration_keyword: TokenKind,
        declaration_type: SemanticTokenType,
        skip_name_trivia: bool,
    ) {
        while self.at(TokenKind::Pub) || self.at(declaration_keyword) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
            self.cursor += 1;
            self.skip_trivia();
        }
        if self.at(TokenKind::Ident) {
            let token = &self.tokens[self.cursor];
            semantic_tokens.push(self.modified(
                token,
                declaration_type,
                &[SemanticTokenModifier::Declaration],
            ));
            self.cursor += 1;
            if skip_name_trivia {
                self.skip_trivia();
            }
        }
    }

    fn collect_module_name(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_namespace_line(semantic_tokens, |_, token| token.kind == TokenKind::Ident);
    }

    fn collect_use_name(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        let mut alias = None;
        for (index, token) in self.tokens.iter().enumerate().skip(self.cursor) {
            if matches!(token.kind, TokenKind::Newline | TokenKind::Eof) {
                break;
            }
            if token.kind == TokenKind::Ident {
                alias = Some(index);
            }
        }

        self.collect_namespace_line(semantic_tokens, |index, _| Some(index) == alias);
    }

    fn collect_namespace_line(
        &mut self,
        semantic_tokens: &mut Vec<SemanticToken>,
        is_declaration: impl Fn(usize, &Token) -> bool,
    ) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if is_declaration(self.cursor, token) {
                semantic_tokens.push(self.modified(
                    token,
                    SemanticTokenType::Namespace,
                    &[SemanticTokenModifier::Declaration],
                ));
            } else if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            }
            self.cursor += 1;
        }
    }

    fn collect_function_header(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.params.clear();
        self.locals.clear();
        let mut kind = TokenKind::Fn;
        while self.at(TokenKind::Pub) || self.at(TokenKind::Fn) || self.at(TokenKind::Test) {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Test {
                kind = TokenKind::Test;
            }
            semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
            self.cursor += 1;
            self.skip_trivia();
        }
        if self.at(TokenKind::Ident) {
            let token = &self.tokens[self.cursor];
            let modifiers = if kind == TokenKind::Test {
                vec![
                    SemanticTokenModifier::Declaration,
                    SemanticTokenModifier::Test,
                ]
            } else {
                vec![SemanticTokenModifier::Declaration]
            };
            semantic_tokens.push(self.modified(token, SemanticTokenType::Function, &modifiers));
            self.cursor += 1;
            self.skip_trivia();
        }
        if self.eat(TokenKind::LParen, semantic_tokens) {
            self.collect_parameters(semantic_tokens);
        }
        self.collect_return_and_effects(semantic_tokens);
    }

    fn collect_parameters(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_parameter_list(semantic_tokens, true);
    }

    fn collect_return_and_effects(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Arrow) {
                let token = &self.tokens[self.cursor];
                semantic_tokens.push(self.simple(token, SemanticTokenType::Operator));
                self.cursor += 1;
                self.skip_trivia();
                if self.at(TokenKind::Ident)
                    && self.next_significant_kind() == Some(TokenKind::Colon)
                {
                    let binding = &self.tokens[self.cursor];
                    self.locals.insert(binding.text.clone());
                    semantic_tokens.push(self.modified(
                        binding,
                        SemanticTokenType::Variable,
                        &[
                            SemanticTokenModifier::Declaration,
                            SemanticTokenModifier::Readonly,
                            SemanticTokenModifier::Result,
                        ],
                    ));
                    self.cursor += 1;
                }
            } else if self.at(TokenKind::Effects) {
                let token = &self.tokens[self.cursor];
                semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                self.cursor += 1;
                self.collect_effect_list(semantic_tokens);
            } else {
                self.collect_plain_token(semantic_tokens);
            }
        }
    }

    fn collect_effect_list(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        self.collect_effect_names(semantic_tokens, EffectNameBoundary::ThroughRightBracket);
    }

    fn collect_effect_names(
        &mut self,
        semantic_tokens: &mut Vec<SemanticToken>,
        boundary: EffectNameBoundary,
    ) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            if boundary == EffectNameBoundary::BeforeEffects && self.at(TokenKind::Effects) {
                break;
            }
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident {
                semantic_tokens.push(self.simple(token, SemanticTokenType::EnumMember));
            } else if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            }
            self.cursor += 1;
            if boundary == EffectNameBoundary::ThroughRightBracket
                && token.kind == TokenKind::RBracket
            {
                break;
            }
        }
    }

    fn collect_let_pattern(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        let mut depth = 0usize;
        while !self.at(TokenKind::Equal) && !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof)
        {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident {
                if depth > 0 && self.next_significant_kind() == Some(TokenKind::Colon) {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Property));
                } else if is_type_name(&token.text) {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Type));
                } else {
                    self.locals.insert(token.text.clone());
                    semantic_tokens.push(self.modified(
                        token,
                        SemanticTokenType::Variable,
                        &[
                            SemanticTokenModifier::Declaration,
                            SemanticTokenModifier::Readonly,
                        ],
                    ));
                }
            } else if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            }
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            self.cursor += 1;
        }
    }
}
