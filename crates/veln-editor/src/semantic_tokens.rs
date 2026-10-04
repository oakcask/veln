use std::collections::{BTreeMap, BTreeSet};

use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{
    SyntaxItem, Token, TokenKind, lex, parse, presentation_parse_structure_is_bounded,
};

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
    pub fn is_custom(self) -> bool {
        !matches!(
            self,
            Self::Namespace
                | Self::Type
                | Self::Parameter
                | Self::Variable
                | Self::Property
                | Self::EnumMember
                | Self::Function
                | Self::Keyword
                | Self::Comment
                | Self::String
                | Self::Number
                | Self::Operator
        )
    }
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
    pub fn is_custom(self) -> bool {
        !matches!(
            self,
            Self::Declaration | Self::Readonly | Self::DefaultLibrary
        )
    }
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

pub fn custom_semantic_token_modifiers() -> Vec<&'static str> {
    TOKEN_MODIFIERS
        .iter()
        .copied()
        .filter(|modifier| modifier.is_custom())
        .map(SemanticTokenModifier::as_lsp_str)
        .collect()
}

pub fn custom_semantic_token_types() -> Vec<&'static str> {
    TOKEN_TYPES
        .iter()
        .copied()
        .filter(|kind| kind.is_custom())
        .map(SemanticTokenType::as_lsp_str)
        .collect()
}

pub fn collect_semantic_tokens(source: &SourceFile) -> Vec<SemanticToken> {
    let lexed = lex(source);
    let tokens = lexed.tokens;
    let function_names = collect_function_names(&tokens);
    let mut classifier = Classifier::new(source, &tokens, function_names);
    let mut semantic_tokens = classifier.collect();
    if !presentation_parse_structure_is_bounded(&tokens) {
        return semantic_tokens;
    }

    let callsite_context = collect_callsite_context(source, &tokens);
    callsite_context.apply(source, &mut semantic_tokens);
    semantic_tokens
}

struct CallsiteContext {
    modifier_offsets: BTreeSet<usize>,
    qualified_offsets: BTreeSet<usize>,
    scopes: Vec<(usize, usize)>,
    modifier_line_starts: BTreeMap<usize, usize>,
}

impl CallsiteContext {
    fn apply(&self, source: &SourceFile, semantic_tokens: &mut [SemanticToken]) {
        let mut scope_cursor = CallsiteScopeCursor::new(&self.scopes);
        for token in semantic_tokens {
            self.apply_to_token(source, token, &mut scope_cursor);
        }
    }

    fn apply_to_token(
        &self,
        source: &SourceFile,
        token: &mut SemanticToken,
        scope_cursor: &mut CallsiteScopeCursor<'_>,
    ) {
        if &source.text()[token.span.start.offset..token.span.end.offset] != "callsite" {
            return;
        }
        if self.is_modifier(token) {
            token.kind.token_type = SemanticTokenType::Keyword;
            token.modifiers = SemanticTokenModifiers::empty();
            return;
        }

        let is_in_scope = scope_cursor.contains(token.span.start.offset, token.span.end.offset);
        let is_body_reference =
            matches!(
                token.kind.token_type,
                SemanticTokenType::Variable | SemanticTokenType::Function
            ) && token.modifiers.bits() & SemanticTokenModifier::Declaration.bit() == 0
                && !self.qualified_offsets.contains(&token.span.start.offset);
        if is_body_reference && is_in_scope {
            token.kind.token_type = SemanticTokenType::Variable;
            token.modifiers = SemanticTokenModifiers::empty().with(SemanticTokenModifier::Readonly);
        }
    }

    fn is_modifier(&self, token: &SemanticToken) -> bool {
        self.modifier_offsets.contains(&token.span.start.offset)
            || self
                .modifier_line_starts
                .get(&token.span.start.line)
                .is_some_and(|start| *start <= token.span.start.offset)
    }
}

fn collect_callsite_context(source: &SourceFile, tokens: &[Token]) -> CallsiteContext {
    let qualified_offsets = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.text == "callsite")
        .filter_map(|(index, token)| {
            previous_significant_index(tokens, index)
                .is_some_and(|previous| tokens[previous].kind == TokenKind::DoubleColon)
                .then_some(token.range.start)
        })
        .collect();
    let mut modifier_offsets = BTreeSet::new();
    let mut scopes = Vec::new();
    let mut modifier_line_starts = BTreeMap::new();
    let parsed = parse(source);
    for item in &parsed.tree.items {
        let SyntaxItem::Function(function) = item else {
            continue;
        };
        let Some(modifier) = function.callsite.as_ref() else {
            continue;
        };
        modifier_offsets.insert(modifier.start.offset);
        scopes.push((
            function
                .contracts
                .last()
                .map_or(modifier.end.offset, |contract| contract.span.end.offset),
            function.span.end.offset,
        ));
        modifier_line_starts
            .entry(modifier.start.line)
            .and_modify(|start: &mut usize| *start = (*start).min(modifier.start.offset))
            .or_insert(modifier.start.offset);
    }
    scopes.sort_unstable();
    CallsiteContext {
        modifier_offsets,
        qualified_offsets,
        scopes,
        modifier_line_starts,
    }
}

struct CallsiteScopeCursor<'a> {
    scopes: &'a [(usize, usize)],
    index: usize,
    #[cfg(test)]
    visits: usize,
}

impl<'a> CallsiteScopeCursor<'a> {
    fn new(scopes: &'a [(usize, usize)]) -> Self {
        Self {
            scopes,
            index: 0,
            #[cfg(test)]
            visits: 0,
        }
    }

    fn contains(&mut self, token_start: usize, token_end: usize) -> bool {
        loop {
            #[cfg(test)]
            {
                self.visits += 1;
            }
            let Some((_, scope_end)) = self.scopes.get(self.index) else {
                return false;
            };
            if *scope_end >= token_end {
                break;
            }
            self.index += 1;
        }
        #[cfg(test)]
        {
            self.visits += 1;
        }
        let (scope_start, scope_end) = self.scopes[self.index];
        scope_start <= token_start && token_end <= scope_end
    }

    #[cfg(test)]
    fn visits(&self) -> usize {
        self.visits
    }
}

pub fn encode_lsp_semantic_tokens(
    source: &SourceFile,
    tokens: &[SemanticToken],
) -> Vec<LspSemanticToken> {
    let mut sorted = tokens.to_vec();
    sorted.sort_by_key(|token| (token.span.start.offset, token.span.end.offset));

    let mut encoded = Vec::new();
    let mut previous_line = 0usize;
    let mut previous_start = 0usize;
    let mut previous_end = 0usize;
    let mut source_cursor = 0usize;
    let mut utf16_column = 0usize;

    for token in sorted {
        if token.span.start.offset < previous_end {
            continue;
        }
        let line = token.span.start.line.saturating_sub(1);
        if line + 1 != token.span.end.line {
            continue;
        }
        advance_utf16_column(
            source.text(),
            &mut source_cursor,
            &mut utf16_column,
            token.span.start.offset,
        );
        let start = utf16_column;
        advance_utf16_column(
            source.text(),
            &mut source_cursor,
            &mut utf16_column,
            token.span.end.offset,
        );
        let end = utf16_column;
        if end <= start {
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

fn advance_utf16_column(text: &str, cursor: &mut usize, column: &mut usize, target: usize) {
    for ch in text[*cursor..target].chars() {
        if ch == '\n' {
            *column = 0;
        } else {
            *column += ch.len_utf16();
        }
    }
    *cursor = target;
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
                .filter(|token| token.kind.is_contextual_identifier())
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
    tokens[satisfy_index].kind == TokenKind::Ident
        && tokens[satisfy_index].text == veln_syntax::SATISFY_MARKER
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
