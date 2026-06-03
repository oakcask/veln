use std::collections::BTreeSet;

use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{Token, TokenKind, lex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub span: SourceSpan,
    pub kind: SemanticTokenKind,
    pub modifiers: SemanticTokenModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticTokenKind {
    pub token_type: SemanticTokenType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticTokenType {
    Namespace,
    Type,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Function,
    Keyword,
    Comment,
    String,
    Number,
    Operator,
}

impl SemanticTokenType {
    pub fn as_lsp_str(self) -> &'static str {
        match self {
            Self::Namespace => "namespace",
            Self::Type => "type",
            Self::Parameter => "parameter",
            Self::Variable => "variable",
            Self::Property => "property",
            Self::EnumMember => "enumMember",
            Self::Function => "function",
            Self::Keyword => "keyword",
            Self::Comment => "comment",
            Self::String => "string",
            Self::Number => "number",
            Self::Operator => "operator",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SemanticTokenModifiers {
    bits: u32,
}

impl SemanticTokenModifiers {
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn with(mut self, modifier: SemanticTokenModifier) -> Self {
        self.bits |= modifier.bit();
        self
    }

    pub fn bits(self) -> u32 {
        self.bits
    }

    pub fn contains(self, modifier: SemanticTokenModifier) -> bool {
        self.bits & modifier.bit() != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticTokenModifier {
    Declaration,
    Readonly,
    DefaultLibrary,
    Test,
    Result,
    Hole,
}

impl SemanticTokenModifier {
    pub fn as_lsp_str(self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
            Self::Readonly => "readonly",
            Self::DefaultLibrary => "defaultLibrary",
            Self::Test => "test",
            Self::Result => "result",
            Self::Hole => "hole",
        }
    }

    fn bit(self) -> u32 {
        1 << modifier_index(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LspSemanticToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers: u32,
}

pub fn semantic_token_legend() -> (Vec<&'static str>, Vec<&'static str>) {
    (
        TOKEN_TYPES
            .iter()
            .map(|token_type| token_type.as_lsp_str())
            .collect(),
        TOKEN_MODIFIERS
            .iter()
            .map(|modifier| modifier.as_lsp_str())
            .collect(),
    )
}

pub fn collect_semantic_tokens(source: &SourceFile) -> Vec<SemanticToken> {
    let lexed = lex(source);
    let tokens = lexed.tokens;
    let function_names = collect_function_names(&tokens);
    let mut classifier = Classifier::new(source, &tokens, function_names);
    classifier.collect()
}

pub fn encode_lsp_semantic_tokens(tokens: &[SemanticToken]) -> Vec<LspSemanticToken> {
    let mut sorted = tokens.to_vec();
    sorted.sort_by_key(|token| (token.span.start.offset, token.span.end.offset));

    let mut encoded = Vec::new();
    let mut previous_line = 0usize;
    let mut previous_start = 0usize;
    let mut previous_end = 0usize;

    for token in sorted {
        if token.span.start.offset < previous_end {
            continue;
        }
        let line = token.span.start.line.saturating_sub(1);
        let start = token.span.start.column.saturating_sub(1);
        let end = token.span.end.column.saturating_sub(1);
        if line + 1 != token.span.end.line || end <= start {
            continue;
        }

        let delta_line = line.saturating_sub(previous_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(previous_start)
        } else {
            start
        };

        encoded.push(LspSemanticToken {
            delta_line: delta_line as u32,
            delta_start: delta_start as u32,
            length: (end - start) as u32,
            token_type: token_type_index(token.kind.token_type) as u32,
            token_modifiers: token.modifiers.bits(),
        });
        previous_line = line;
        previous_start = start;
        previous_end = token.span.end.offset;
    }

    encoded
}

struct Classifier<'a> {
    source: &'a SourceFile,
    tokens: &'a [Token],
    function_names: BTreeSet<String>,
    params: BTreeSet<String>,
    locals: BTreeSet<String>,
    cursor: usize,
}

impl<'a> Classifier<'a> {
    fn new(source: &'a SourceFile, tokens: &'a [Token], function_names: BTreeSet<String>) -> Self {
        Self {
            source,
            tokens,
            function_names,
            params: BTreeSet::new(),
            locals: BTreeSet::new(),
            cursor: 0,
        }
    }

    fn collect(&mut self) -> Vec<SemanticToken> {
        let mut semantic_tokens = Vec::new();
        while self.cursor < self.tokens.len() {
            let token = &self.tokens[self.cursor];
            match token.kind {
                TokenKind::Mod => {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                    self.cursor += 1;
                    self.collect_module_name(&mut semantic_tokens);
                }
                TokenKind::Use => {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                    self.cursor += 1;
                    self.collect_use_name(&mut semantic_tokens);
                }
                TokenKind::Type => {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                    self.cursor += 1;
                    self.skip_trivia();
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
                TokenKind::Fn | TokenKind::Test | TokenKind::Pub => {
                    self.collect_function_header(&mut semantic_tokens);
                }
                TokenKind::Let => {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                    self.cursor += 1;
                    self.collect_let_pattern(&mut semantic_tokens);
                }
                TokenKind::Effects => {
                    semantic_tokens.push(self.simple(token, SemanticTokenType::Keyword));
                    self.cursor += 1;
                    self.collect_effect_list(&mut semantic_tokens);
                }
                _ => {
                    if let Some(classified) = self.classify_current_token() {
                        semantic_tokens.push(classified);
                    }
                    self.cursor += 1;
                }
            }
        }
        semantic_tokens
    }

    fn collect_module_name(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident {
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

        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if Some(self.cursor) == alias {
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
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident
                && self.next_significant_kind() == Some(TokenKind::Colon)
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
                if let Some(classified) = self.classify_current_token() {
                    semantic_tokens.push(classified);
                }
                self.cursor += 1;
            }
        }
        self.eat(TokenKind::RParen, semantic_tokens);
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
                if let Some(classified) = self.classify_current_token() {
                    semantic_tokens.push(classified);
                }
                self.cursor += 1;
            }
        }
    }

    fn collect_effect_list(&mut self, semantic_tokens: &mut Vec<SemanticToken>) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = &self.tokens[self.cursor];
            if token.kind == TokenKind::Ident {
                semantic_tokens.push(self.simple(token, SemanticTokenType::EnumMember));
            } else if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            }
            self.cursor += 1;
            if token.kind == TokenKind::RBracket {
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

    fn classify_current_token(&self) -> Option<SemanticToken> {
        let token = &self.tokens[self.cursor];
        match token.kind {
            TokenKind::Whitespace | TokenKind::Newline | TokenKind::Eof | TokenKind::Invalid => {
                None
            }
            TokenKind::Comment => Some(self.simple(token, SemanticTokenType::Comment)),
            TokenKind::String => Some(self.simple(token, SemanticTokenType::String)),
            TokenKind::Int | TokenKind::Float => {
                Some(self.simple(token, SemanticTokenType::Number))
            }
            TokenKind::Hole | TokenKind::Underscore => Some(self.modified(
                token,
                SemanticTokenType::Variable,
                &[SemanticTokenModifier::Hole],
            )),
            TokenKind::Pub
            | TokenKind::Fn
            | TokenKind::Type
            | TokenKind::Test
            | TokenKind::Effects
            | TokenKind::Let
            | TokenKind::End
            | TokenKind::Require
            | TokenKind::Ensure
            | TokenKind::Invariant
            | TokenKind::Mod
            | TokenKind::Use
            | TokenKind::From
            | TokenKind::Match
            | TokenKind::Or
            | TokenKind::And
            | TokenKind::Not => Some(self.simple(token, SemanticTokenType::Keyword)),
            TokenKind::Ident => Some(self.classify_ident(token)),
            TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::DoubleColon
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::PipeGreater
            | TokenKind::Question
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash => Some(self.simple(token, SemanticTokenType::Operator)),
        }
    }

    fn classify_ident(&self, token: &Token) -> SemanticToken {
        if matches!(token.text.as_str(), "true" | "false") {
            return self.simple(token, SemanticTokenType::Keyword);
        }
        if token.text == "satisfy"
            && matches!(
                self.previous_significant_kind(),
                Some(TokenKind::Hole | TokenKind::Underscore)
            )
        {
            return self.simple(token, SemanticTokenType::Keyword);
        }
        if self.previous_significant_kind() == Some(TokenKind::Dot) {
            return self.simple(token, SemanticTokenType::Property);
        }
        if self.next_significant_kind() == Some(TokenKind::Colon) {
            return self.simple(token, SemanticTokenType::Property);
        }
        if is_type_name(&token.text) {
            return self.simple(token, SemanticTokenType::Type);
        }
        if self.next_significant_kind() == Some(TokenKind::LParen)
            || self.function_names.contains(&token.text)
            || is_prelude_function(&token.text)
        {
            let modifiers = if is_prelude_function(&token.text) {
                SemanticTokenModifiers::empty().with(SemanticTokenModifier::DefaultLibrary)
            } else {
                SemanticTokenModifiers::empty()
            };
            return self.token(token, SemanticTokenType::Function, modifiers);
        }
        if self.params.contains(&token.text) {
            return self.modified(
                token,
                SemanticTokenType::Parameter,
                &[SemanticTokenModifier::Readonly],
            );
        }
        if self.locals.contains(&token.text) {
            return self.modified(
                token,
                SemanticTokenType::Variable,
                &[SemanticTokenModifier::Readonly],
            );
        }
        self.simple(token, SemanticTokenType::Variable)
    }

    fn simple(&self, token: &Token, token_type: SemanticTokenType) -> SemanticToken {
        self.token(token, token_type, SemanticTokenModifiers::empty())
    }

    fn modified(
        &self,
        token: &Token,
        token_type: SemanticTokenType,
        modifiers: &[SemanticTokenModifier],
    ) -> SemanticToken {
        let modifiers = modifiers
            .iter()
            .fold(SemanticTokenModifiers::empty(), |set, modifier| {
                set.with(*modifier)
            });
        self.token(token, token_type, modifiers)
    }

    fn token(
        &self,
        token: &Token,
        token_type: SemanticTokenType,
        modifiers: SemanticTokenModifiers,
    ) -> SemanticToken {
        SemanticToken {
            span: self.source.span(token.range),
            kind: SemanticTokenKind { token_type },
            modifiers,
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn eat(&mut self, kind: TokenKind, semantic_tokens: &mut Vec<SemanticToken>) -> bool {
        if self.at(kind) {
            let token = &self.tokens[self.cursor];
            if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            } else if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline) {
                semantic_tokens.push(self.simple(token, SemanticTokenType::Operator));
            }
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn skip_trivia(&mut self) {
        while self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        {
            self.cursor += 1;
        }
    }

    fn previous_significant_kind(&self) -> Option<TokenKind> {
        self.tokens[..self.cursor]
            .iter()
            .rev()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
            .map(|token| token.kind)
    }

    fn next_significant_kind(&self) -> Option<TokenKind> {
        self.tokens
            .iter()
            .skip(self.cursor + 1)
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
            .map(|token| token.kind)
    }
}

fn collect_function_names(tokens: &[Token]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::Fn | TokenKind::Test)
            && let Some(name) = tokens
                .iter()
                .skip(index + 1)
                .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
                .filter(|token| token.kind == TokenKind::Ident)
        {
            names.insert(name.text.clone());
        }
        index += 1;
    }
    names
}

fn token_type_index(token_type: SemanticTokenType) -> usize {
    TOKEN_TYPES
        .iter()
        .position(|candidate| *candidate == token_type)
        .expect("semantic token type must be in legend")
}

fn modifier_index(modifier: SemanticTokenModifier) -> usize {
    TOKEN_MODIFIERS
        .iter()
        .position(|candidate| *candidate == modifier)
        .expect("semantic token modifier must be in legend")
}

fn is_type_name(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase)
}

fn is_prelude_function(text: &str) -> bool {
    matches!(
        text,
        "float_negate"
            | "float_add"
            | "float_subtract"
            | "float_multiply"
            | "float_divide"
            | "float_less"
            | "float_less_equal"
            | "float_greater"
            | "float_greater_equal"
            | "string_split_once"
            | "string_parse_int"
            | "int_to_string"
            | "vec_len"
            | "vec_is_empty"
            | "vec_push"
            | "vec_concat"
            | "vec_map"
            | "vec_filter"
            | "vec_fold"
            | "vec_try_map"
            | "vec_try_map_with"
            | "list_nil"
            | "list_cons"
            | "list_is_empty"
            | "list_fold"
            | "list_reverse"
            | "list_map"
            | "list_filter"
            | "list_try_map"
            | "dict_get"
            | "dict_contains"
            | "dict_insert"
            | "dict_remove"
            | "option_map"
            | "option_and_then"
            | "option_unwrap_or"
            | "result_map"
            | "result_map_err"
            | "result_and_then"
    )
}

const TOKEN_TYPES: [SemanticTokenType; 12] = [
    SemanticTokenType::Namespace,
    SemanticTokenType::Type,
    SemanticTokenType::Parameter,
    SemanticTokenType::Variable,
    SemanticTokenType::Property,
    SemanticTokenType::EnumMember,
    SemanticTokenType::Function,
    SemanticTokenType::Keyword,
    SemanticTokenType::Comment,
    SemanticTokenType::String,
    SemanticTokenType::Number,
    SemanticTokenType::Operator,
];

const TOKEN_MODIFIERS: [SemanticTokenModifier; 6] = [
    SemanticTokenModifier::Declaration,
    SemanticTokenModifier::Readonly,
    SemanticTokenModifier::DefaultLibrary,
    SemanticTokenModifier::Test,
    SemanticTokenModifier::Result,
    SemanticTokenModifier::Hole,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_text(source: &SourceFile) -> Vec<(String, SemanticTokenType, u32)> {
        collect_semantic_tokens(source)
            .into_iter()
            .map(|token| {
                (
                    source.text()[token.span.start.offset..token.span.end.offset].to_string(),
                    token.kind.token_type,
                    token.modifiers.bits(),
                )
            })
            .collect()
    }

    #[test]
    fn collector_classifies_declarations_references_holes_and_prelude_calls() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "mod app.core\n",
                "use stdio\n",
                "test parses(value: Int) -> result: Result<Int, String> effects [stdio]\n",
                "  let next: Int = int_to_string(value)\n",
                "  _todo satisfy candidate => candidate > 0\n",
                "end\n",
            ),
        );

        let tokens = collect_text(&source);

        assert!(
            tokens.contains(&(
                "core".to_string(),
                SemanticTokenType::Namespace,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "parses".to_string(),
                SemanticTokenType::Function,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .with(SemanticTokenModifier::Test)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "value".to_string(),
                SemanticTokenType::Parameter,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .with(SemanticTokenModifier::Readonly)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "result".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .with(SemanticTokenModifier::Readonly)
                    .with(SemanticTokenModifier::Result)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "int_to_string".to_string(),
                SemanticTokenType::Function,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::DefaultLibrary)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "_todo".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Hole)
                    .bits()
            ))
        );
        assert!(tokens.contains(&(
            "satisfy".to_string(),
            SemanticTokenType::Keyword,
            SemanticTokenModifiers::empty().bits()
        )));
    }

    #[test]
    fn collector_classifies_unnamed_holes_and_boolean_literals() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "# boolean path\n",
                "fn main(flag: Bool) -> Bool\n",
                "  _ satisfy candidate => true # always true\n",
                "end\n"
            ),
        );

        let tokens = collect_text(&source);

        assert!(
            tokens.contains(&(
                "_".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Hole)
                    .bits()
            ))
        );
        assert!(tokens.contains(&(
            "true".to_string(),
            SemanticTokenType::Keyword,
            SemanticTokenModifiers::empty().bits()
        )));
        assert!(tokens.contains(&(
            "# boolean path".to_string(),
            SemanticTokenType::Comment,
            SemanticTokenModifiers::empty().bits()
        )));
        assert!(tokens.contains(&(
            "# always true".to_string(),
            SemanticTokenType::Comment,
            SemanticTokenModifiers::empty().bits()
        )));
    }

    #[test]
    fn collector_keeps_let_bindings_distinct_from_record_fields() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn main(value: {count: Int}) -> Int\n",
                "  let message: String = \"ready\"\n",
                "  let {count: amount}: {count: Int} = value\n",
                "  amount\n",
                "end\n"
            ),
        );

        let tokens = collect_text(&source);

        assert!(
            tokens.contains(&(
                "message".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .with(SemanticTokenModifier::Readonly)
                    .bits()
            ))
        );
        assert!(tokens.contains(&(
            "count".to_string(),
            SemanticTokenType::Property,
            SemanticTokenModifiers::empty().bits()
        )));
        assert!(
            tokens.contains(&(
                "amount".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .with(SemanticTokenModifier::Readonly)
                    .bits()
            ))
        );
        assert!(
            tokens.contains(&(
                "amount".to_string(),
                SemanticTokenType::Variable,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Readonly)
                    .bits()
            ))
        );
    }

    #[test]
    fn lsp_encoding_uses_relative_positions_and_legend_indices() {
        let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");
        let semantic_tokens = collect_semantic_tokens(&source);
        let function_tokens = semantic_tokens
            .into_iter()
            .filter(|token| token.kind.token_type == SemanticTokenType::Function)
            .collect::<Vec<_>>();

        let encoded = encode_lsp_semantic_tokens(&function_tokens);

        assert_eq!(
            encoded,
            vec![
                LspSemanticToken {
                    delta_line: 0,
                    delta_start: 3,
                    length: 4,
                    token_type: token_type_index(SemanticTokenType::Function) as u32,
                    token_modifiers: SemanticTokenModifiers::empty()
                        .with(SemanticTokenModifier::Declaration)
                        .bits(),
                },
                LspSemanticToken {
                    delta_line: 1,
                    delta_start: 2,
                    length: 4,
                    token_type: token_type_index(SemanticTokenType::Function) as u32,
                    token_modifiers: 0,
                },
            ]
        );
    }

    #[test]
    fn lsp_encoding_sorts_and_drops_overlapping_ranges() {
        let source = SourceFile::new("main.veln", "fn main()\nend\n");
        let outer = SemanticToken {
            span: source.span(veln_source::TextRange::new(0, 7)),
            kind: SemanticTokenKind {
                token_type: SemanticTokenType::Function,
            },
            modifiers: SemanticTokenModifiers::empty(),
        };
        let inner = SemanticToken {
            span: source.span(veln_source::TextRange::new(3, 7)),
            kind: SemanticTokenKind {
                token_type: SemanticTokenType::Function,
            },
            modifiers: SemanticTokenModifiers::empty(),
        };

        let encoded = encode_lsp_semantic_tokens(&[inner, outer]);

        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0].delta_start, 0);
        assert_eq!(encoded[0].length, 7);
    }
}
