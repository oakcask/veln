use super::*;

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        let diagnostics = integer_literal_diagnostics(source, &tokens);
        let parse_tokens = tokens
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .cloned()
            .collect();
        Self {
            source,
            tokens: parse_tokens,
            lossless_tokens: tokens,
            cursor: 0,
            cleanup_depth: 0,
            top_level_boundary_cache: Cell::new(None),
            module_value_suffix_cache: RefCell::new(HashMap::new()),
            diagnostics,
        }
    }

    pub(super) fn parse(mut self) -> ParseOutput {
        self.eat_newlines();
        let module = if self.at(TokenKind::Mod) {
            Some(
                self.parse_named_header(TokenKind::Mod, "module_declaration")
                    .0,
            )
        } else {
            None
        };

        let uses = self.parse_use_declarations();

        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::Eof) {
                break;
            }
            if let Some(item) = self.parse_top_level_item() {
                items.push(item);
            }
        }

        let adr_lite_records =
            collect_adr_lite_records(self.source, &self.lossless_tokens, module.as_ref(), &items);

        let root = build_lossless_root(
            self.lossless_tokens,
            self.source.len(),
            module.as_ref(),
            &uses,
            &items,
        );

        ParseOutput {
            tree: SyntaxTree {
                root,
                module,
                adr_lite_records,
                uses,
                items,
            },
            diagnostics: self.diagnostics,
        }
    }

    pub(super) fn parse_top_level_item(&mut self) -> Option<SyntaxItem> {
        if self.at_public_alias_header() {
            return Some(SyntaxItem::PublicAlias(self.parse_public_alias()));
        }
        if self.at(TokenKind::Pub) {
            return self.parse_public_top_level_item();
        }

        self.parse_private_top_level_item()
    }

    pub(super) fn parse_private_top_level_item(&mut self) -> Option<SyntaxItem> {
        match self.current().kind {
            TokenKind::Let => Some(SyntaxItem::ModuleValue(Box::new(
                self.parse_module_value_decl(),
            ))),
            TokenKind::Fn => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Function),
            ))),
            TokenKind::Type => Some(SyntaxItem::Type(self.parse_type_decl())),
            TokenKind::Schema => Some(SyntaxItem::Schema(self.parse_schema_decl())),
            TokenKind::Effect => Some(SyntaxItem::Effect(self.parse_effect_decl())),
            TokenKind::Handler => Some(SyntaxItem::Handler(self.parse_handler_decl())),
            TokenKind::Codec => {
                self.parse_removed_codec_decl();
                None
            }
            TokenKind::Test => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Test),
            ))),
            _ => self.reject_top_level_token(),
        }
    }

    pub(super) fn parse_public_top_level_item(&mut self) -> Option<SyntaxItem> {
        match self.peek_kind(1) {
            Some(TokenKind::Let) => Some(SyntaxItem::ModuleValue(Box::new(
                self.parse_module_value_decl(),
            ))),
            Some(TokenKind::Type) => Some(SyntaxItem::Type(self.parse_type_decl())),
            Some(TokenKind::Schema) => Some(SyntaxItem::Schema(self.parse_schema_decl())),
            Some(TokenKind::Effect) => Some(SyntaxItem::Effect(self.parse_effect_decl())),
            Some(TokenKind::Handler) => Some(SyntaxItem::Handler(self.parse_handler_decl())),
            Some(TokenKind::Codec) => {
                self.parse_removed_codec_decl();
                None
            }
            _ => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Function),
            ))),
        }
    }

    pub(super) fn reject_top_level_token(&mut self) -> Option<SyntaxItem> {
        self.error_current(
            "parse.expected_item",
            "expected a function, test, type, effect, handler, schema, or module value declaration",
            "module",
            vec![
                "pub", "let", "fn", "test", "type", "effect", "handler", "schema",
            ],
            RecoveryStrategy::SynchronizeToAnchor,
            Some("fn"),
        );
        self.synchronize_to_item();
        None
    }

    pub(super) fn parse_module_value_decl(&mut self) -> ModuleValueDecl {
        let start = self.current().range;
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        self.expect(TokenKind::Let, "module_value_declaration", vec!["let"]);

        let (name, name_span) = self.parse_module_value_name();
        if self.at(TokenKind::Less) {
            self.error_current(
                "parse.module_value_declaration",
                "module value declarations cannot declare type parameters",
                "module_value_declaration",
                vec![":"],
                RecoveryStrategy::SynchronizeToAnchor,
                Some(":"),
            );
            while !self.at(TokenKind::Colon)
                && !self.at(TokenKind::Equal)
                && !self.at(TokenKind::Newline)
                && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }

        let (annotation, annotation_paths, annotation_refinements) =
            if self.eat(TokenKind::Colon).is_some() {
                let annotation = self.collect_type_paths_until(
                    "module_value_declaration",
                    &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                );
                if annotation.0.is_empty() {
                    self.error_current(
                        "parse.module_value_declaration",
                        "module value declaration requires a type annotation",
                        "module_value_declaration",
                        vec!["type"],
                        RecoveryStrategy::InsertToken,
                        Some("="),
                    );
                }
                annotation
            } else {
                self.error_current(
                    "parse.module_value_declaration",
                    "module value declaration requires a type annotation",
                    "module_value_declaration",
                    vec![":"],
                    RecoveryStrategy::InsertToken,
                    Some(":"),
                );
                (String::new(), Vec::new(), Vec::new())
            };

        self.expect(TokenKind::Equal, "module_value_declaration", vec!["="]);
        if self.at(TokenKind::Newline) || self.at(TokenKind::Eof) {
            self.error_current(
                "parse.module_value_declaration",
                "module value declaration requires an initializer",
                "module_value_declaration",
                vec!["expression"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
        }
        let (initializer, end) = self.parse_expr_for_body_line("module_value_declaration");
        ModuleValueDecl {
            visibility,
            name,
            name_span,
            annotation,
            annotation_paths,
            annotation_refinements,
            initializer,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_module_value_name(&mut self) -> (Option<String>, Option<SourceSpan>) {
        if self.at(TokenKind::Ident) {
            let token = self.bump();
            let span = self.source.span(token.range);
            if !token
                .text
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase)
            {
                self.error_at_token(
                    &token,
                    DiagnosticRequest {
                        id: "parse.module_value_declaration",
                        message: "module value name must start with an ASCII lowercase letter"
                            .to_string(),
                        parser_context: "module_value_declaration",
                        expected: vec!["lowercase value identifier"],
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some(":"),
                        repair_candidates: Vec::new(),
                    },
                );
            }
            return (Some(token.text), Some(span));
        }

        self.error_current(
            "parse.module_value_declaration",
            "module value declaration requires one lowercase value identifier",
            "module_value_declaration",
            vec!["lowercase value identifier"],
            RecoveryStrategy::SynchronizeToAnchor,
            Some(":"),
        );
        while !self.at(TokenKind::Colon)
            && !self.at(TokenKind::Equal)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::Eof)
        {
            self.bump();
        }
        (None, None)
    }

    pub(super) fn at_public_alias_header(&self) -> bool {
        self.at(TokenKind::Pub)
            && matches!(
                (self.peek_kind(1), self.peek_kind(2), self.peek_kind(3)),
                (
                    Some(TokenKind::Fn | TokenKind::Type | TokenKind::Schema),
                    Some(TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole),
                    Some(TokenKind::Equal)
                )
            )
    }

    pub(super) fn parse_public_alias(&mut self) -> PublicAliasDecl {
        let start = self
            .expect(TokenKind::Pub, "public_alias", vec!["pub"])
            .range;
        let kind = if self.eat(TokenKind::Fn).is_some() {
            PublicAliasKind::Function
        } else if self.eat(TokenKind::Type).is_some() {
            PublicAliasKind::Type
        } else {
            self.expect(
                TokenKind::Schema,
                "public_alias",
                vec!["fn", "type", "schema"],
            );
            PublicAliasKind::Schema
        };
        let (name, name_span) = self.expect_covered_name("public_alias", "public member name");
        self.expect(TokenKind::Equal, "public_alias", vec!["="]);
        let (target, target_spans) = self.parse_member_alias_target();
        let end = self.expect_newline("public_alias").range;
        PublicAliasDecl {
            kind,
            name,
            name_span,
            target,
            target_spans,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_member_alias_target(&mut self) -> (Vec<String>, Vec<SourceSpan>) {
        let mut segments = Vec::new();
        let mut spans = Vec::new();
        if let (Some(segment), Some(span)) = self.expect_covered_name("public_alias", "member path")
        {
            segments.push(segment);
            spans.push(span);
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if let (Some(segment), Some(span)) =
                self.expect_covered_name("public_alias", "member path segment")
            {
                segments.push(segment);
                spans.push(span);
            }
        }
        (segments, spans)
    }

    pub(super) fn parse_type_decl(&mut self) -> TypeDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Type, "type_declaration", vec!["type"])
            .range;
        let (name, name_span) = self.expect_covered_name("type_declaration", "type name");
        let params = if self.eat(TokenKind::Less).is_some() {
            let params = self.parse_type_params_until(TokenKind::Greater);
            self.expect(TokenKind::Greater, "type_declaration", vec![">"]);
            params
        } else {
            Vec::new()
        };
        let header_cursor = self.cursor;
        let header_end = self.expect_newline("type_declaration").range;
        if self.cursor == header_cursor {
            self.skip_to_next_line();
        }

        let (variants, end_present) =
            self.parse_declaration_body(|parser| parser.parse_type_variant());

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close type declaration",
                "type_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(header_end, |token| token.range);
        TypeDecl {
            visibility,
            name,
            name_span,
            params,
            variants,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    pub(super) fn parse_type_params_until(&mut self, close: TokenKind) -> Vec<String> {
        let mut params = Vec::new();
        while !self.at(close) && !self.at(TokenKind::Eof) {
            if let Some(param) = self.expect_ident("type_declaration", "type parameter") {
                params.push(param);
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    pub(super) fn parse_declaration_body<T>(
        &mut self,
        mut parse_item: impl FnMut(&mut Self) -> T,
    ) -> (Vec<T>, bool) {
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                if self.cleanup_depth > 0 && self.end_closes_enclosing_declaration() {
                    return (items, false);
                }
                self.bump();
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                return (items, true);
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            let item_cursor = self.cursor;
            items.push(parse_item(self));
            if self.cursor == item_cursor {
                self.skip_to_next_line();
            }
        }
        (items, false)
    }

    pub(super) fn end_is_followed_by_top_level_item(&self) -> bool {
        if let Some((cursor, result)) = self.top_level_boundary_cache.get()
            && cursor == self.cursor
        {
            return result;
        }

        let mut cursor = self.cursor + 1;
        while self.tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Newline) {
            cursor += 1;
        }

        let result = match self.tokens.get(cursor).map(|token| token.kind) {
            Some(
                TokenKind::Fn
                | TokenKind::Test
                | TokenKind::Type
                | TokenKind::Schema
                | TokenKind::Effect
                | TokenKind::Handler
                | TokenKind::Codec,
            ) => true,
            Some(TokenKind::Let) => {
                self.module_value_initializer_cursor(cursor)
                    .is_some_and(|initializer| {
                        matches!(
                            self.classify_module_value_suffix(initializer),
                            ModuleValueSuffixEvidence::TopLevelItem
                                | ModuleValueSuffixEvidence::EndOfInput
                        )
                    })
            }
            Some(TokenKind::Pub) => {
                matches!(
                    self.tokens.get(cursor + 1).map(|token| token.kind),
                    Some(
                        TokenKind::Fn
                            | TokenKind::Type
                            | TokenKind::Schema
                            | TokenKind::Effect
                            | TokenKind::Handler
                            | TokenKind::Codec
                    )
                ) || matches!(
                    self.tokens.get(cursor + 1).map(|token| token.kind),
                    Some(TokenKind::Let)
                )
            }
            _ => false,
        };
        self.top_level_boundary_cache
            .set(Some((self.cursor, result)));
        result
    }

    fn module_value_initializer_cursor(&self, mut cursor: usize) -> Option<usize> {
        if self.tokens.get(cursor)?.kind == TokenKind::Pub {
            cursor += 1;
        }
        if self.tokens.get(cursor)?.kind != TokenKind::Let {
            return None;
        }
        cursor += 1;

        let name = self.tokens.get(cursor)?;
        if name.kind != TokenKind::Ident
            || !name
                .text
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase)
        {
            return None;
        }
        cursor += 1;
        if self.tokens.get(cursor)?.kind != TokenKind::Colon {
            return None;
        }
        cursor += 1;

        let type_start = cursor;
        while !matches!(
            self.tokens.get(cursor).map(|token| token.kind),
            Some(TokenKind::Equal | TokenKind::Newline | TokenKind::Eof) | None
        ) {
            cursor += 1;
        }
        if cursor == type_start || self.tokens.get(cursor)?.kind != TokenKind::Equal {
            return None;
        }
        cursor += 1;
        if matches!(
            self.tokens.get(cursor).map(|token| token.kind),
            Some(TokenKind::Newline | TokenKind::Eof) | None
        ) {
            return None;
        }
        Some(cursor)
    }

    fn classify_module_value_suffix(&self, mut cursor: usize) -> ModuleValueSuffixEvidence {
        if let Some(result) = self.module_value_suffix_cache.borrow().get(&cursor) {
            return *result;
        }

        let mut equivalent_initializers = vec![cursor];
        let mut block_depth = 0usize;
        let mut delimiter_depth = 0usize;
        let mut previous_kind = Some(TokenKind::Equal);
        let mut at_line_start = false;
        let mut closed_initializer_block = false;

        while let Some(token) = self.tokens.get(cursor) {
            super::record_cleanup_boundary_classifier_work();
            let kind = token.kind;
            if closed_initializer_block && !matches!(kind, TokenKind::Newline | TokenKind::Eof) {
                closed_initializer_block = false;
            }
            if at_line_start
                && delimiter_depth == 0
                && block_depth == 0
                && kind == TokenKind::Let
                && let Some(initializer) = self.module_value_initializer_cursor(cursor)
            {
                equivalent_initializers.push(initializer);
            }
            if at_line_start
                && delimiter_depth == 0
                && block_depth == 0
                && matches!(
                    kind,
                    TokenKind::Fn
                        | TokenKind::Test
                        | TokenKind::Type
                        | TokenKind::Schema
                        | TokenKind::Effect
                        | TokenKind::Handler
                        | TokenKind::Codec
                        | TokenKind::Pub
                )
            {
                let evidence = ModuleValueSuffixEvidence::TopLevelItem;
                self.cache_module_value_suffix(&equivalent_initializers, evidence);
                return evidence;
            }

            match kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    delimiter_depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    delimiter_depth = delimiter_depth.saturating_sub(1);
                }
                TokenKind::Match | TokenKind::Begin => block_depth += 1,
                TokenKind::Defer if at_line_start && delimiter_depth == 0 => block_depth += 1,
                TokenKind::If if previous_kind != Some(TokenKind::Else) => block_depth += 1,
                TokenKind::End => {
                    if block_depth == 0 {
                        let evidence = ModuleValueSuffixEvidence::EnclosingDeclarationEnd;
                        self.cache_module_value_suffix(&equivalent_initializers, evidence);
                        return evidence;
                    }
                    block_depth -= 1;
                    closed_initializer_block = block_depth == 0;
                }
                TokenKind::Eof => {
                    // A block-closing `end` immediately before EOF can belong to the
                    // enclosing declaration after block recovery. Keep that established
                    // typed-local shape distinct from a plain module value at EOF.
                    let evidence = if closed_initializer_block {
                        ModuleValueSuffixEvidence::EndOfInputAfterBlock
                    } else {
                        ModuleValueSuffixEvidence::EndOfInput
                    };
                    self.cache_module_value_suffix(&equivalent_initializers, evidence);
                    return evidence;
                }
                _ => {}
            }
            at_line_start = kind == TokenKind::Newline;
            previous_kind = Some(kind);
            cursor += 1;
        }
        let evidence = if closed_initializer_block {
            ModuleValueSuffixEvidence::EndOfInputAfterBlock
        } else {
            ModuleValueSuffixEvidence::EndOfInput
        };
        self.cache_module_value_suffix(&equivalent_initializers, evidence);
        evidence
    }

    fn cache_module_value_suffix(
        &self,
        initializers: &[usize],
        evidence: ModuleValueSuffixEvidence,
    ) {
        let mut cache = self.module_value_suffix_cache.borrow_mut();
        for &initializer in initializers {
            cache.insert(initializer, evidence);
        }
    }

    pub(super) fn end_closes_enclosing_declaration(&self) -> bool {
        if self.end_is_followed_by_top_level_item() {
            return true;
        }

        let mut cursor = self.cursor + 1;
        while self.tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Newline) {
            cursor += 1;
        }
        self.tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Eof)
    }

    pub(super) fn parse_effect_decl(&mut self) -> EffectDecl {
        let diagnostic_start = self.diagnostics.len();
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Effect, "effect_declaration", vec!["effect"])
            .range;
        let name = self.expect_ident("effect_declaration", "effect name");
        self.expect_newline("effect_declaration");

        let (operations, end_present) =
            self.parse_declaration_body(|parser| parser.parse_effect_operation_decl());

        if operations.is_empty() {
            self.error_current(
                "parse.effect_operation_required",
                "effect declaration requires at least one operation",
                "effect_declaration",
                vec!["operation declaration"],
                RecoveryStrategy::InsertToken,
                Some("end"),
            );
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close effect declaration",
                "effect_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(start, |token| token.range);
        let recovered = self.diagnostics.len() > diagnostic_start;
        EffectDecl {
            visibility,
            name,
            operations,
            span: self.source.span(start.cover(end)),
            end_present,
            recovered,
        }
    }

    pub(super) fn parse_effect_operation_decl(&mut self) -> EffectOperationDecl {
        let start = self.current().range;
        let name_span = self.source.span(start);
        let name = self.expect_ident("effect_operation", "operation name");
        self.expect(TokenKind::LParen, "effect_operation", vec!["("]);
        let params = self.parse_params_in_context("effect_operation", true);
        self.expect(TokenKind::RParen, "effect_operation", vec![")"]);
        let (return_type, return_type_paths, return_type_refinements) =
            if self.eat(TokenKind::Arrow).is_some() {
                let (ty, paths, refinements) = self.collect_return_type_until(
                    "effect_operation",
                    &[TokenKind::Newline, TokenKind::Eof],
                );
                (Some(ty), paths, refinements)
            } else {
                self.error_current(
                    "parse.effect_operation_return",
                    "effect operation is missing `->` and a result type",
                    "effect_operation",
                    vec!["->"],
                    RecoveryStrategy::InsertToken,
                    Some("newline"),
                );
                (None, Vec::new(), Vec::new())
            };
        let end = self.expect_newline("effect_operation").range;
        EffectOperationDecl {
            name,
            name_span,
            params,
            return_type,
            return_type_paths,
            return_type_refinements,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_handler_decl(&mut self) -> HandlerDecl {
        let header = self.parse_handler_header();
        let body = self.parse_handler_body(header.start);
        HandlerDecl {
            visibility: header.visibility,
            name: header.name,
            params: header.params,
            effect: header.effect.path,
            effect_span: header.effect.span,
            effect_recovered: header.effect.recovered,
            effects: header.effect.effects,
            effect_spans: header.effect.effect_spans,
            effects_recovered: header.effect.effects_recovered,
            operation_clauses: body.operation_clauses,
            span: self.source.span(header.start.cover(body.end)),
            end_present: body.end_present,
        }
    }

    pub(super) fn parse_handler_header(&mut self) -> HandlerHeader {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Handler, "handler_declaration", vec!["handler"])
            .range;
        let name = self.expect_ident("handler_declaration", "handler name");
        self.expect(TokenKind::LParen, "handler_parameters", vec!["("]);
        let params = self.parse_params_in_context("handler_parameters", true);
        self.expect(TokenKind::RParen, "handler_parameters", vec![")"]);
        if self.at(TokenKind::For) {
            self.bump();
        } else if self.at_ident_text("handles")
            && self.peek_kind(1).is_some_and(is_contextual_identifier)
        {
            self.error_current(
                "parse.expected_token",
                "expected for",
                "handler_declaration",
                vec!["for"],
                RecoveryStrategy::SkipToken,
                None,
            );
            self.bump();
        } else {
            self.expect(TokenKind::For, "handler_declaration", vec!["for"]);
        }
        let mut effect = self.parse_handler_effect();
        let diagnostic_count = self.diagnostics.len();
        self.expect_newline("handler_declaration");
        if self.diagnostics.len() > diagnostic_count {
            if effect.effects.is_some() {
                effect.effects_recovered = true;
            } else {
                effect.recovered = true;
            }
        }
        HandlerHeader {
            visibility,
            start,
            name,
            params,
            effect,
        }
    }

    pub(super) fn parse_handler_effect(&mut self) -> HandlerEffect {
        let effect_start = self.current().range;
        let diagnostic_count = self.diagnostics.len();
        let path = self.parse_name_path_segments("handler_declaration", "handled effect");
        let recovered = self.diagnostics.len() > diagnostic_count;
        let effect_end = self.previous().map_or(effect_start, |token| token.range);
        let span = self.source.span(effect_start.cover(effect_end));
        let (effects, effect_spans, effects_recovered) = if self.eat(TokenKind::Effects).is_some() {
            let diagnostic_count = self.diagnostics.len();
            let labels = self.parse_effect_list();
            let (effects, spans): (Vec<_>, Vec<_>) = labels.into_iter().unzip();
            (
                Some(effects),
                Some(spans),
                self.diagnostics.len() > diagnostic_count,
            )
        } else {
            (None, None, false)
        };
        HandlerEffect {
            path,
            span,
            recovered,
            effects,
            effect_spans,
            effects_recovered,
        }
    }

    pub(super) fn parse_handler_body(&mut self, start: TextRange) -> HandlerBody {
        let (operation_clauses, end_present) =
            self.parse_declaration_body(|parser| parser.parse_handler_operation_clause_decl());
        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close handler declaration",
                "handler_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }
        let end = self.previous().map_or(start, |token| token.range);
        HandlerBody {
            operation_clauses,
            end,
            end_present,
        }
    }

    pub(super) fn parse_handler_operation_clause_decl(&mut self) -> HandlerOperationClauseDecl {
        let start = self.current().range;
        let operation_span = self.source.span(start);
        let operation = self.expect_ident("handler_operation_clause", "operation name");
        if self.at(TokenKind::Equal) {
            let equal = self.current().clone();
            self.error_at_token(
                &equal,
                DiagnosticRequest {
                    id: "parse.handler_operation_old_syntax",
                    message: "handler operation clause must bind operation parameters with `(` and evaluate an expression with `=>`".to_string(),
                    parser_context: "handler_operation_clause",
                    expected: vec!["("],
                    strategy: RecoveryStrategy::SynchronizeToAnchor,
                    anchor: Some("newline"),
                    repair_candidates: Vec::new(),
                },
            );
            self.skip_to_next_line();
            let body = Expr {
                kind: ExprKind::Missing,
                span: self.source.span(equal.range),
            };
            return HandlerOperationClauseDecl {
                operation,
                operation_span,
                params: Vec::new(),
                body,
                span: self.source.span(start.cover(equal.range)),
            };
        }
        self.expect(TokenKind::LParen, "handler_operation_clause", vec!["("]);
        let params = self.parse_handler_operation_params();
        self.expect(TokenKind::RParen, "handler_operation_clause", vec![")"]);
        self.expect(TokenKind::FatArrow, "handler_operation_clause", vec!["=>"]);
        let (body, body_range) = self.parse_expr_for_body_line("handler_operation_clause");
        HandlerOperationClauseDecl {
            operation,
            operation_span,
            params,
            body,
            span: self.source.span(start.cover(body_range)),
        }
    }

    pub(super) fn parse_handler_operation_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.at(TokenKind::RParen) || self.at(TokenKind::Eof) {
            return params;
        }
        loop {
            let start = self.current().range;
            let (name, name_span) =
                self.expect_covered_name("handler_operation_clause", "operation parameter");
            params.push(Param {
                name: name.unwrap_or_default(),
                name_span: name_span.unwrap_or_else(|| self.source.span(start)),
                ty: None,
                ty_span: None,
                ty_paths: Vec::new(),
                ty_refinements: Vec::new(),
                is_variadic: false,
                span: self.source.span(start),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
            if self.at(TokenKind::RParen) {
                self.error_current(
                    "parse.handler_operation_parameter",
                    "handler operation parameter list cannot end with a comma",
                    "handler_operation_clause",
                    vec!["operation parameter"],
                    RecoveryStrategy::InsertToken,
                    Some("parameter"),
                );
                break;
            }
        }
        params
    }
}
