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

mod classifier_classification;
mod classifier_collection;

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

fn handler_clause_pattern_start_from_arrow(tokens: &[Token], arrow_start: usize) -> usize {
    let Some(arrow_index) = tokens
        .iter()
        .position(|token| token.range.start == arrow_start)
    else {
        return arrow_start;
    };
    tokens[..arrow_index]
        .iter()
        .rev()
        .find(|token| token.kind == TokenKind::Newline)
        .map_or(arrow_start, |token| token.range.end)
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
            | "dict_map"
            | "dict_map_with"
            | "dict_filter"
            | "dict_filter_with"
            | "dict_fold"
            | "dict_fold_with"
            | "dict_try_map"
            | "dict_try_map_with"
            | "option_map"
            | "option_and_then"
            | "option_unwrap_or"
            | "result_map"
            | "result_map_err"
            | "result_and_then"
    )
}

fn is_else_if(tokens: &[Token], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .is_some_and(|token| token.kind == TokenKind::Else)
}

fn is_satisfy_arrow(tokens: &[Token], index: usize) -> bool {
    let Some(candidate_index) = previous_significant_index(tokens, index) else {
        return false;
    };
    let candidate = &tokens[candidate_index];
    if candidate.kind != TokenKind::Ident {
        return false;
    }
    let Some(satisfy_index) = previous_significant_index(tokens, candidate_index) else {
        return false;
    };
    tokens[satisfy_index].kind == TokenKind::Ident && tokens[satisfy_index].text == "satisfy"
}

fn previous_significant_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .map(|(index, _)| index)
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
#[path = "semantic_tokens/tests.rs"]
mod tests;
