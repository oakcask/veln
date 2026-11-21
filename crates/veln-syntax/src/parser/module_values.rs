use super::*;

impl<'a> Parser<'a> {
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

        let (annotation, annotation_span, annotation_paths, annotation_refinements) =
            self.parse_module_value_annotation();

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
            annotation_span,
            annotation_paths,
            annotation_refinements,
            initializer,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_module_value_annotation(
        &mut self,
    ) -> (
        String,
        SourceSpan,
        Vec<TypePathSegments>,
        Vec<VariantRefinementType>,
    ) {
        if self.eat(TokenKind::Colon).is_none() {
            self.error_current(
                "parse.module_value_declaration",
                "module value declaration requires a type annotation",
                "module_value_declaration",
                vec![":"],
                RecoveryStrategy::InsertToken,
                Some(":"),
            );
            return (
                String::new(),
                self.source.span(self.current().range),
                Vec::new(),
                Vec::new(),
            );
        }

        let annotation_start = self.current().range;
        let (annotation, paths, refinements) = self.collect_type_paths_until(
            "module_value_declaration",
            &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
        );
        let annotation_end = self
            .previous()
            .map_or(annotation_start, |token| token.range);
        if annotation.is_empty() {
            self.error_current(
                "parse.module_value_declaration",
                "module value declaration requires a type annotation",
                "module_value_declaration",
                vec!["type"],
                RecoveryStrategy::InsertToken,
                Some("="),
            );
        }
        (
            annotation,
            self.source.span(annotation_start.cover(annotation_end)),
            paths,
            refinements,
        )
    }

    fn parse_module_value_name(&mut self) -> (Option<String>, Option<SourceSpan>) {
        if self.at(TokenKind::Ident) {
            let token = self.bump();
            let span = self.source.span(token.range);
            if is_module_value_name(&token) {
                return (Some(token.text), Some(span));
            }
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
                return (Some(token.text), Some(span));
            }
            self.error_at_token(
                &token,
                DiagnosticRequest {
                    id: "parse.module_value_declaration",
                    message: "module value declaration requires one lowercase value identifier"
                        .to_string(),
                    parser_context: "module_value_declaration",
                    expected: vec!["lowercase value identifier"],
                    strategy: RecoveryStrategy::SkipToken,
                    anchor: Some(":"),
                    repair_candidates: Vec::new(),
                },
            );
            return (None, None);
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
}
