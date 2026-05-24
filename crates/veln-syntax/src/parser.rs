use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::tree::build_lossless_root;
use crate::{
    BinaryOp, BodyLine, ContractClause, ContractKind, DictEntry, Expr, ExprKind, FunctionDecl,
    FunctionKind, ModuleDecl, Param, PrefixOp, RecordField, SatisfyClause, SyntaxItem, SyntaxTree,
    Token, TokenKind, UseDecl, Visibility, lex,
};

#[derive(Clone, Debug)]
pub struct ParseOutput {
    pub tree: SyntaxTree,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Clone, Debug)]
pub struct ParseDiagnostic {
    pub id: &'static str,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub parser_context: &'static str,
    pub unexpected: UnexpectedToken,
    pub expected: Vec<&'static str>,
    pub recovery: Recovery,
}

#[derive(Clone, Debug)]
pub struct UnexpectedToken {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Recovery {
    pub strategy: RecoveryStrategy,
    pub anchor: Option<String>,
    pub dropped_token_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryStrategy {
    None,
    SkipToken,
    InsertToken,
    CloseBlock,
    SynchronizeToAnchor,
}

impl RecoveryStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SkipToken => "skip_token",
            Self::InsertToken => "insert_token",
            Self::CloseBlock => "close_block",
            Self::SynchronizeToAnchor => "synchronize_to_anchor",
        }
    }
}
pub fn parse(source: &SourceFile) -> ParseOutput {
    let lexed = lex(source);
    Parser::new(source, lexed.tokens).parse()
}
struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    lossless_tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
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
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> ParseOutput {
        let module = if self.at(TokenKind::Mod) {
            Some(
                self.parse_named_header(TokenKind::Mod, "module_declaration")
                    .0,
            )
        } else {
            None
        };

        let mut uses = Vec::new();
        while self.at(TokenKind::Use) {
            uses.push(self.parse_named_header(TokenKind::Use, "use_declaration").1);
        }

        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::Eof) {
                break;
            }
            if self.at(TokenKind::Pub) || self.at(TokenKind::Fn) {
                items.push(SyntaxItem::Function(
                    self.parse_function_like(FunctionKind::Function),
                ));
            } else if self.at(TokenKind::Test) {
                items.push(SyntaxItem::Function(
                    self.parse_function_like(FunctionKind::Test),
                ));
            } else {
                self.error_current(
                    "parse.expected_item",
                    "expected a function or test declaration",
                    "module",
                    vec!["pub", "fn", "test"],
                    RecoveryStrategy::SynchronizeToAnchor,
                    Some("fn"),
                );
                self.synchronize_to_item();
            }
        }

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
                uses,
                items,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_named_header(
        &mut self,
        keyword: TokenKind,
        context: &'static str,
    ) -> (ModuleDecl, UseDecl) {
        let start = self.expect(keyword, context, vec!["keyword"]).range;
        let name = self.parse_module_name(context);
        let end = self.expect_newline(context).range;
        let span = self.source.span(start.cover(end));
        (
            ModuleDecl {
                name: name.clone(),
                span: span.clone(),
            },
            UseDecl { name, span },
        )
    }

    fn parse_function_like(&mut self, kind: FunctionKind) -> FunctionDecl {
        let start = self.current().range;
        let visibility = match kind {
            FunctionKind::Function => {
                if self.eat(TokenKind::Pub).is_some() {
                    Visibility::Public
                } else {
                    Visibility::Private
                }
            }
            FunctionKind::Test => Visibility::Private,
        };
        let context = match kind {
            FunctionKind::Function => "function_declaration",
            FunctionKind::Test => "test_declaration",
        };
        let parameter_context = match kind {
            FunctionKind::Function => "function_parameters",
            FunctionKind::Test => "test_parameters",
        };
        let return_context = match kind {
            FunctionKind::Function => "function_return",
            FunctionKind::Test => "test_return",
        };
        self.expect(
            match kind {
                FunctionKind::Function => TokenKind::Fn,
                FunctionKind::Test => TokenKind::Test,
            },
            context,
            vec![match kind {
                FunctionKind::Function => "fn",
                FunctionKind::Test => "test",
            }],
        );
        let name = self.expect_ident(context, "declaration name");
        self.expect(TokenKind::LParen, parameter_context, vec!["("]);
        let params = self.parse_params();
        self.expect(TokenKind::RParen, parameter_context, vec![")"]);

        let (return_binding, return_type) = if self.eat(TokenKind::Arrow).is_some() {
            let return_binding = if self.at(TokenKind::Ident) && self.peek_at(TokenKind::Colon) {
                let name = self.bump();
                let colon = self.expect(TokenKind::Colon, return_context, vec![":"]);
                Some(crate::ResultBinding {
                    name: name.text,
                    span: self.source.span(name.range.cover(colon.range)),
                })
            } else {
                None
            };
            let return_type = self.collect_type_until(
                return_context,
                &[TokenKind::Effects, TokenKind::Newline, TokenKind::Eof],
            );
            (return_binding, Some(return_type))
        } else {
            (None, None)
        };

        let effects = if self.eat(TokenKind::Effects).is_some() {
            Some(self.parse_effect_list())
        } else {
            None
        };
        self.expect_newline(context);

        let mut contracts = Vec::new();
        while self.at(TokenKind::Require) || self.at(TokenKind::Ensure) {
            contracts.push(self.parse_contract());
        }

        let mut body = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            body.push(self.parse_body_line());
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                match kind {
                    FunctionKind::Function => {
                        "expected `end` to close function declaration".to_string()
                    }
                    FunctionKind::Test => "expected `end` to close test declaration".to_string(),
                },
                "function_body",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(start, |token| token.range);
        FunctionDecl {
            kind,
            visibility,
            name,
            params,
            return_binding,
            return_type,
            effects,
            contracts,
            body,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let name = self.expect_ident("function_parameters", "parameter name");
            let ty = self.eat(TokenKind::Colon).map(|_| {
                self.collect_type_until(
                    "function_parameters",
                    &[TokenKind::Comma, TokenKind::RParen, TokenKind::Eof],
                )
            });
            let end = self.previous().map_or(start, |token| token.range);
            params.push(Param {
                name: name.unwrap_or_default(),
                ty,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_effect_list(&mut self) -> Vec<String> {
        self.expect(TokenKind::LBracket, "effect_declaration", vec!["["]);
        let mut effects = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.at(TokenKind::Eof) {
            if let Some(effect) = self.expect_ident("effect_declaration", "effect name") {
                effects.push(effect);
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "effect_declaration", vec!["]"]);
        effects
    }

    fn parse_contract(&mut self) -> ContractClause {
        let start_token = self.bump();
        let kind = match start_token.kind {
            TokenKind::Require => ContractKind::Require,
            _ => ContractKind::Ensure,
        };
        let (text, predicate_tokens, end) = self.collect_until_newline();
        self.diagnostics.extend(
            ContractPredicateParser::new(
                self.source,
                "contract_predicate",
                "parse.contract_predicate",
                &predicate_tokens,
            )
            .parse(),
        );
        ContractClause {
            kind,
            text,
            span: self.source.span(start_token.range.cover(end)),
        }
    }

    fn parse_body_line(&mut self) -> BodyLine {
        let start = self.current().range;
        if self.at(TokenKind::Let) {
            self.bump();
            let name = self.expect_ident("let_statement", "binding name");
            let annotation = if self.eat(TokenKind::Colon).is_some() {
                Some(self.collect_type_until(
                    "let_statement",
                    &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                ))
            } else {
                None
            };
            self.expect(TokenKind::Equal, "let_statement", vec!["="]);
            let (expr, end) = self.parse_expr_until_newline("let_statement");
            BodyLine::Let {
                name,
                annotation,
                expr,
                span: self.source.span(start.cover(end)),
            }
        } else {
            let (expr, end) = self.parse_expr_until_newline("expression_line");
            BodyLine::Expr {
                expr,
                span: self.source.span(start.cover(end)),
            }
        }
    }

    fn parse_module_name(&mut self, context: &'static str) -> String {
        let mut name = self
            .expect_ident(context, "module name")
            .unwrap_or_else(|| "<missing>".to_string());
        while self.eat(TokenKind::Dot).is_some() {
            if let Some(segment) = self.expect_ident(context, "module name segment") {
                name.push('.');
                name.push_str(&segment);
            }
        }
        name
    }

    fn collect_type_until(&mut self, _context: &'static str, stop: &[TokenKind]) -> String {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && stop.iter().any(|kind| self.at(kind.clone())) {
                break;
            }
            match self.current().kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            parts.push(self.bump().text);
        }
        parts
            .join(" ")
            .replace(" :: ", "::")
            .replace(" (", "(")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace("[ ", "[")
            .replace(" ]", "]")
            .replace(" ,", ",")
    }

    fn collect_until_newline(&mut self) -> (String, Vec<Token>, TextRange) {
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

    fn parse_expr_until_newline(&mut self, context: &'static str) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.invalid_token",
                    message: "invalid token in expression".to_string(),
                    span: Some(self.source.span(token.range)),
                    parser_context: context,
                    unexpected: UnexpectedToken {
                        kind: token.kind.label().to_string(),
                        text: token.text.clone(),
                    },
                    expected: vec!["expression"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline".to_string()),
                        dropped_token_count: 1,
                    },
                });
            } else {
                tokens.push(token);
            }
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }

    fn expect_ident(&mut self, context: &'static str, expected: &'static str) -> Option<String> {
        if self.at(TokenKind::Ident) {
            Some(self.bump().text)
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            None
        }
    }

    fn expect_newline(&mut self, context: &'static str) -> Token {
        if self.at(TokenKind::Newline) {
            self.bump()
        } else if self.at(TokenKind::Eof) {
            self.current().clone()
        } else {
            self.error_current(
                "parse.expected_newline",
                "expected a newline",
                context,
                vec!["newline"],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        context: &'static str,
        expected: Vec<&'static str>,
    ) -> Token {
        if self.at(kind) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_token",
                format!("expected {}", expected.join(" or ")),
                context,
                expected,
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        parser_context: &'static str,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let current = self.current();
        self.diagnostics.push(ParseDiagnostic {
            id,
            message: message.into(),
            span: Some(self.source.span(current.range)),
            parser_context,
            unexpected: UnexpectedToken {
                kind: current.kind.label().to_string(),
                text: current.text.clone(),
            },
            expected,
            recovery: Recovery {
                strategy,
                anchor: anchor.map(str::to_string),
                dropped_token_count: 0,
            },
        });
    }

    fn synchronize_to_item(&mut self) {
        let start = self.cursor;
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Pub)
            && !self.at(TokenKind::Fn)
            && !self.at(TokenKind::Test)
            && !self.at(TokenKind::End)
        {
            self.bump();
        }
        let at_eof = self.at(TokenKind::Eof);
        let anchor = match self.current().kind {
            TokenKind::Pub => Some("pub".to_string()),
            TokenKind::Fn => Some("fn".to_string()),
            TokenKind::Test => Some("test".to_string()),
            TokenKind::End => Some("end".to_string()),
            TokenKind::Eof => None,
            _ => None,
        };
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
            if anchor.is_some() || at_eof {
                last.recovery.anchor = anchor;
            }
        }
    }

    fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn peek_at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| token.kind == kind)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> Option<&Token> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        token
    }
}

struct ExprParser<'a> {
    source: &'a SourceFile,
    context: &'static str,
    tokens: &'a [Token],
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> ExprParser<'a> {
    fn new(source: &'a SourceFile, context: &'static str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            context,
            tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> (Expr, Vec<ParseDiagnostic>) {
        let expr = self.parse_expr(0);
        (expr, self.diagnostics)
    }

    fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                let token = self.bump();
                lhs = Expr {
                    span: self.source.span(lhs_range(&lhs).cover(token.range)),
                    kind: ExprKind::Try(Box::new(lhs)),
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

    fn parse_prefix(&mut self) -> Expr {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) {
            let token = self.bump();
            let op = if token.kind == TokenKind::Not {
                PrefixOp::Not
            } else {
                PrefixOp::Negate
            };
            let expr = self.parse_expr(13);
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

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.at(TokenKind::LParen) {
                let start = lhs_range(&expr);
                self.bump();
                let mut args = Vec::new();
                while !self.at(TokenKind::RParen) && !self.is_at_end() {
                    args.push(self.parse_expr(0));
                    if self.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                let end = self.eat(TokenKind::RParen).map_or_else(
                    || lhs_range(args.last().unwrap_or(&expr)),
                    |token| token.range,
                );
                expr = Expr {
                    span: self.source.span(start.cover(end)),
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                };
                continue;
            }
            if self.at(TokenKind::Dot) {
                let start = lhs_range(&expr);
                let dot = self.bump();
                let (field, field_range) = if self.at(TokenKind::Ident) {
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
                expr = Expr {
                    span: self.source.span(start.cover(field_range)),
                    kind: ExprKind::FieldAccess {
                        base: Box::new(expr),
                        field,
                        field_span: self.source.span(field_range),
                    },
                };
                continue;
            }
            if self.at(TokenKind::Question) {
                let token = self.bump();
                expr = Expr {
                    span: self.source.span(lhs_range(&expr).cover(token.range)),
                    kind: ExprKind::Try(Box::new(expr)),
                };
                continue;
            }
            break;
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return self.missing_expr();
        };

        match token.kind {
            TokenKind::Underscore | TokenKind::Hole => {
                self.bump();
                let name = token.text.strip_prefix('_').and_then(|suffix| {
                    if suffix.is_empty() {
                        None
                    } else {
                        Some(suffix.to_string())
                    }
                });
                let satisfy = self.parse_satisfy_clause();
                Expr {
                    kind: ExprKind::Hole { name, satisfy },
                    span: self.source.span(token.range),
                }
            }
            TokenKind::String => {
                self.bump();
                Expr {
                    kind: ExprKind::StringLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Int => {
                self.bump();
                Expr {
                    kind: ExprKind::IntLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Float => {
                self.bump();
                Expr {
                    kind: ExprKind::FloatLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Ident => self.parse_name_path(),
            TokenKind::LParen => {
                let start = self.bump();
                if let Some(end) = self.eat(TokenKind::RParen) {
                    Expr {
                        kind: ExprKind::Unit,
                        span: self.source.span(start.range.cover(end.range)),
                    }
                } else {
                    let expr = self.parse_expr(0);
                    let end = self
                        .eat(TokenKind::RParen)
                        .map_or_else(|| lhs_range(&expr), |token| token.range);
                    Expr {
                        span: self.source.span(start.range.cover(end)),
                        ..expr
                    }
                }
            }
            TokenKind::LBrace => {
                if matches!(
                    self.peek_kind(1),
                    Some(TokenKind::Ident | TokenKind::RBrace)
                ) {
                    self.parse_record()
                } else {
                    self.parse_dict()
                }
            }
            TokenKind::LBracket => self.parse_list(),
            _ => {
                self.bump();
                Expr {
                    kind: ExprKind::Missing,
                    span: self.source.span(token.range),
                }
            }
        }
    }

    fn parse_satisfy_clause(&mut self) -> Option<SatisfyClause> {
        if !self.at_ident_text("satisfy") {
            return None;
        }
        let start = self.bump().range;
        let (candidate, candidate_span) = if self.at(TokenKind::Ident) {
            let token = self.bump();
            let span = self.source.span(token.range);
            (Some(token.text), Some(span))
        } else {
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
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        let mut depth = 0usize;
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
            end = token.range;
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        self.diagnostics.extend(
            ContractPredicateParser::new(
                self.source,
                "satisfy_predicate",
                "parse.satisfy_predicate",
                &predicate_tokens,
            )
            .parse(),
        );
        Some(SatisfyClause {
            candidate,
            candidate_span,
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_name_path(&mut self) -> Expr {
        let start = self.current().range;
        let mut end = start;
        let mut segments = vec![self.bump().text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                let segment = self.bump();
                end = segment.range;
                segments.push(segment.text);
            } else {
                break;
            }
        }
        Expr {
            kind: ExprKind::NamePath(segments),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_list(&mut self) -> Expr {
        let start = self.bump().range;
        let mut items = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.is_at_end() {
            items.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBracket).map_or_else(
            || items.last().map_or(start, lhs_range),
            |token| token.range,
        );
        Expr {
            kind: ExprKind::List(items),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_record(&mut self) -> Expr {
        let start = self.bump().range;
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.current().range;
            let name = if self.at(TokenKind::Ident) {
                self.bump().text
            } else {
                self.bump();
                String::new()
            };
            self.eat(TokenKind::Colon);
            let expr = self.parse_expr(0);
            let field_span = self.source.span(field_start.cover(lhs_range(&expr)));
            fields.push(RecordField {
                name,
                expr,
                span: field_span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                fields.last().map_or(start, |field| {
                    TextRange::new(field.span.start.offset, field.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_dict(&mut self) -> Expr {
        let start = self.bump().range;
        let mut entries = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let entry_start = self.current().range;
            let key = self.parse_expr(0);
            self.eat(TokenKind::Colon);
            let value = self.parse_expr(0);
            let entry_span = self.source.span(entry_start.cover(lhs_range(&value)));
            entries.push(DictEntry {
                key,
                value,
                span: entry_span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                entries.last().map_or(start, |entry| {
                    TextRange::new(entry.span.start.offset, entry.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Dict(entries),
            span: self.source.span(start.cover(end)),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::PipeGreater => Some((BinaryOp::PipeGreater, 1, 2)),
            TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
            TokenKind::And => Some((BinaryOp::And, 5, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 7, 8)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 7, 8)),
            TokenKind::Less => Some((BinaryOp::Less, 9, 10)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 9, 10)),
            TokenKind::Greater => Some((BinaryOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 9, 10)),
            TokenKind::Plus => Some((BinaryOp::Add, 11, 12)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 11, 12)),
            TokenKind::Star => Some((BinaryOp::Multiply, 13, 14)),
            TokenKind::Slash => Some((BinaryOp::Divide, 13, 14)),
            _ => None,
        }
    }

    fn missing_expr(&self) -> Expr {
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(TextRange::at(self.source.len())),
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind.clone())
    }

    fn at_ident_text(&self, text: &str) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == TokenKind::Ident && token.text == text)
    }

    fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .unwrap_or_else(|| Token::eof(self.source.len()));
        self.diagnostics.push(ParseDiagnostic {
            id,
            message: message.into(),
            span: Some(self.source.span(token.range)),
            parser_context: self.context,
            unexpected: UnexpectedToken {
                kind: token.kind.label().to_string(),
                text: token.text,
            },
            expected,
            recovery: Recovery {
                strategy,
                anchor: anchor.map(str::to_string),
                dropped_token_count: 0,
            },
        });
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}

struct ContractPredicateParser<'a> {
    source: &'a SourceFile,
    context: &'static str,
    diagnostic_id: &'static str,
    tokens: &'a [Token],
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> ContractPredicateParser<'a> {
    fn new(
        source: &'a SourceFile,
        context: &'static str,
        diagnostic_id: &'static str,
        tokens: &'a [Token],
    ) -> Self {
        Self {
            source,
            context,
            diagnostic_id,
            tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> Vec<ParseDiagnostic> {
        if self.tokens.is_empty() {
            self.error_current(
                "contract predicate is empty",
                vec!["contract predicate"],
                RecoveryStrategy::InsertToken,
                None,
            );
            return self.diagnostics;
        }

        self.parse_predicate(0);
        if !self.is_at_end() {
            if self.at(TokenKind::PipeGreater) {
                self.error_current(
                    "pipeline syntax is not allowed in a contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
            } else {
                self.error_current(
                    "unexpected token in contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
            }
        }
        self.diagnostics
    }

    fn parse_predicate(&mut self, min_bp: u8) {
        self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                self.error_current(
                    "`?` is not allowed in a contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
                continue;
            }

            let Some((_, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            self.parse_predicate(right_bp);
        }
    }

    fn parse_prefix(&mut self) {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) {
            self.bump();
            self.parse_predicate(13);
            return;
        }
        self.parse_postfix();
    }

    fn parse_postfix(&mut self) {
        self.parse_primary();
        loop {
            if self.at(TokenKind::LParen) {
                self.parse_call_args();
                continue;
            }
            if self.at(TokenKind::Dot) {
                self.bump();
                if self.at(TokenKind::Ident) {
                    self.bump();
                } else {
                    self.error_current(
                        "contract predicate field access is missing a field name",
                        vec!["field name"],
                        RecoveryStrategy::InsertToken,
                        None,
                    );
                }
                continue;
            }
            break;
        }
    }

    fn parse_call_args(&mut self) {
        self.bump();
        if self.eat(TokenKind::RParen).is_some() {
            return;
        }
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            self.parse_predicate(0);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        if self.eat(TokenKind::RParen).is_none() {
            self.error_current(
                "contract predicate call is missing `)`",
                vec![")"],
                RecoveryStrategy::InsertToken,
                None,
            );
        }
    }

    fn parse_primary(&mut self) {
        if self.is_at_end() {
            self.error_current(
                "contract predicate is missing an expression",
                vec!["contract predicate atom"],
                RecoveryStrategy::InsertToken,
                None,
            );
            return;
        }

        match self.current().kind {
            TokenKind::String | TokenKind::Int | TokenKind::Float | TokenKind::Ident => {
                self.parse_name_path_or_literal();
            }
            TokenKind::LParen => {
                self.bump();
                if self.eat(TokenKind::RParen).is_some() {
                    return;
                }
                self.parse_predicate(0);
                if self.eat(TokenKind::RParen).is_none() {
                    self.error_current(
                        "contract predicate grouping is missing `)`",
                        vec![")"],
                        RecoveryStrategy::InsertToken,
                        None,
                    );
                }
            }
            TokenKind::Hole | TokenKind::Underscore => {
                self.error_current(
                    "hole syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::LBracket => {
                self.error_current(
                    "list syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::LBrace => {
                self.error_current(
                    "record syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::Match => {
                self.error_current(
                    "`match` is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::PipeGreater => {
                self.error_current(
                    "pipeline syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::Invalid => {
                self.error_current(
                    "invalid token in contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            _ => {
                self.error_current(
                    "expected a contract predicate atom",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
        }
    }

    fn parse_name_path_or_literal(&mut self) {
        self.bump();
        while self.at(TokenKind::DoubleColon) {
            self.bump();
            if self.at(TokenKind::Ident) {
                self.bump();
            } else {
                self.error_current(
                    "qualified contract predicate name is missing a segment",
                    vec!["name segment"],
                    RecoveryStrategy::InsertToken,
                    None,
                );
                break;
            }
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
            TokenKind::And => Some((BinaryOp::And, 5, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 7, 8)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 7, 8)),
            TokenKind::Less => Some((BinaryOp::Less, 9, 10)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 9, 10)),
            TokenKind::Greater => Some((BinaryOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 9, 10)),
            TokenKind::Plus => Some((BinaryOp::Add, 11, 12)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 11, 12)),
            TokenKind::Star => Some((BinaryOp::Multiply, 13, 14)),
            TokenKind::Slash => Some((BinaryOp::Divide, 13, 14)),
            _ => None,
        }
    }

    fn error_current(
        &mut self,
        message: impl Into<String>,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .unwrap_or_else(|| Token::eof(self.source.len()));
        self.diagnostics.push(ParseDiagnostic {
            id: self.diagnostic_id,
            message: message.into(),
            span: Some(self.source.span(token.range)),
            parser_context: self.context,
            unexpected: UnexpectedToken {
                kind: token.kind.label().to_string(),
                text: token.text,
            },
            expected,
            recovery: Recovery {
                strategy,
                anchor: anchor.map(str::to_string),
                dropped_token_count: usize::from(strategy == RecoveryStrategy::SkipToken),
            },
        });
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        self.cursor += 1;
        token
    }
}

fn lhs_range(expr: &Expr) -> TextRange {
    TextRange::new(expr.span.start.offset, expr.span.end.offset)
}

fn normalize_collected_text(parts: Vec<String>) -> String {
    parts
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" . ", ".")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
}
