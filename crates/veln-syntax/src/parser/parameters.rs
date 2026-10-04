use super::*;

#[derive(Default)]
struct ParsedParameterType {
    ty: Option<String>,
    span: Option<SourceSpan>,
    paths: Vec<TypePathSegments>,
    refinements: Vec<VariantRefinementType>,
    is_variadic: bool,
}

impl<'a> Parser<'a> {
    pub(super) fn parse_params(&mut self) -> Vec<Param> {
        self.parse_params_in_context("function_parameters", false)
    }

    pub(super) fn parse_params_in_context(
        &mut self,
        context: &'static str,
        require_types: bool,
    ) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            params.push(self.parse_param_in_context(context, require_types));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_param_in_context(&mut self, context: &'static str, require_type: bool) -> Param {
        let start = self.current().range;
        let (name, name_span) = self.expect_covered_name(context, "parameter name");
        let parsed_type = self.parse_parameter_type(context);
        if require_type && parsed_type.ty.is_none() {
            self.report_missing_parameter_type(context, start, name.as_deref().unwrap_or_default());
        }
        let end = self.previous().map_or(start, |token| token.range);
        Param {
            name: name.unwrap_or_default(),
            name_span: name_span.unwrap_or_else(|| self.source.span(start)),
            ty: parsed_type.ty,
            ty_span: parsed_type.span,
            ty_paths: parsed_type.paths,
            ty_refinements: parsed_type.refinements,
            is_variadic: parsed_type.is_variadic,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_parameter_type(&mut self, context: &'static str) -> ParsedParameterType {
        let Some(colon) = self.eat(TokenKind::Colon) else {
            return ParsedParameterType::default();
        };
        let is_variadic = self.eat_variadic_marker();
        let start = self.current().range;
        let (ty, paths, refinements) = self.collect_type_paths_until(
            context,
            &[TokenKind::Comma, TokenKind::RParen, TokenKind::Eof],
        );
        let end = self.previous().map_or(colon.range, |token| token.range);
        ParsedParameterType {
            ty: Some(ty),
            span: Some(self.source.span(start.cover(end))),
            paths,
            refinements,
            is_variadic,
        }
    }

    fn report_missing_parameter_type(
        &mut self,
        context: &'static str,
        range: TextRange,
        name: &str,
    ) {
        self.diagnostics.push(ParseDiagnostic {
            id: "parse.effect_operation_parameter_type",
            message: "effect operation parameter is missing a type annotation".to_string(),
            span: Some(self.source.span(range)),
            parser_context: context,
            unexpected: UnexpectedToken {
                kind: "identifier".to_string(),
                text: name.to_string(),
            },
            expected: vec![":"],
            recovery: Recovery {
                strategy: RecoveryStrategy::InsertToken,
                anchor: Some("parameter type".to_string()),
                dropped_token_count: 0,
            },
            repair_candidates: Vec::new(),
        });
    }

    pub(super) fn eat_variadic_marker(&mut self) -> bool {
        if self.at(TokenKind::Dot)
            && self.peek_at(TokenKind::Dot)
            && self.peek_kind(2) == Some(TokenKind::Dot)
        {
            self.bump();
            self.bump();
            self.bump();
            true
        } else {
            false
        }
    }
}
