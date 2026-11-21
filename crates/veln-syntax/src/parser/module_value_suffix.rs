use std::cell::RefCell;
use std::collections::HashMap;

use crate::{Token, TokenKind};

pub(super) struct ModuleValueSuffixClassifier {
    cache: RefCell<HashMap<usize, ModuleValueSuffixEvidence>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ModuleValueSuffixEvidence {
    TopLevelItem,
    EnclosingDeclarationEnd,
    EndOfInput,
    EndOfInputAfterBlock,
}

struct ScanState {
    cursor: usize,
    equivalent_initializers: Vec<usize>,
    block_depth: usize,
    delimiter_depth: usize,
    previous_kind: Option<TokenKind>,
    at_line_start: bool,
    closed_initializer_block: bool,
}

impl ModuleValueSuffixClassifier {
    pub(super) fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub(super) fn classify(
        &self,
        tokens: &[Token],
        initializer: usize,
    ) -> ModuleValueSuffixEvidence {
        if let Some(evidence) = self.cache.borrow().get(&initializer) {
            return *evidence;
        }

        let mut state = ScanState::new(initializer);
        while let Some(token) = tokens.get(state.cursor) {
            super::record_cleanup_boundary_classifier_work();
            let kind = token.kind;
            state.prepare_for(kind);
            state.record_equivalent_initializer(tokens, kind);
            if state.starts_top_level_item(kind) {
                return self.finish(state, ModuleValueSuffixEvidence::TopLevelItem);
            }
            if let Some(evidence) = state.record_structure(kind) {
                return self.finish(state, evidence);
            }
            state.advance(kind);
        }
        let evidence = state.end_of_input_evidence();
        self.finish(state, evidence)
    }

    fn finish(
        &self,
        state: ScanState,
        evidence: ModuleValueSuffixEvidence,
    ) -> ModuleValueSuffixEvidence {
        let mut cache = self.cache.borrow_mut();
        for initializer in state.equivalent_initializers {
            cache.insert(initializer, evidence);
        }
        evidence
    }
}

impl ScanState {
    fn new(initializer: usize) -> Self {
        Self {
            cursor: initializer,
            equivalent_initializers: vec![initializer],
            block_depth: 0,
            delimiter_depth: 0,
            previous_kind: Some(TokenKind::Equal),
            at_line_start: false,
            closed_initializer_block: false,
        }
    }

    fn prepare_for(&mut self, kind: TokenKind) {
        if self.closed_initializer_block && !matches!(kind, TokenKind::Newline | TokenKind::Eof) {
            self.closed_initializer_block = false;
        }
    }

    fn record_equivalent_initializer(&mut self, tokens: &[Token], kind: TokenKind) {
        if self.at_top_level()
            && kind == TokenKind::Let
            && let Some(initializer) = module_value_initializer_cursor(tokens, self.cursor)
        {
            self.equivalent_initializers.push(initializer);
        }
    }

    fn starts_top_level_item(&self, kind: TokenKind) -> bool {
        self.at_top_level()
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
    }

    fn record_structure(&mut self, kind: TokenKind) -> Option<ModuleValueSuffixEvidence> {
        match kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                self.delimiter_depth += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
            }
            TokenKind::Match | TokenKind::Begin => self.block_depth += 1,
            TokenKind::Defer if self.at_line_start && self.delimiter_depth == 0 => {
                self.block_depth += 1;
            }
            TokenKind::If if self.previous_kind != Some(TokenKind::Else) => self.block_depth += 1,
            TokenKind::End if self.block_depth == 0 => {
                return Some(ModuleValueSuffixEvidence::EnclosingDeclarationEnd);
            }
            TokenKind::End => {
                self.block_depth -= 1;
                self.closed_initializer_block = self.block_depth == 0;
            }
            TokenKind::Eof => return Some(self.end_of_input_evidence()),
            _ => {}
        }
        None
    }

    fn at_top_level(&self) -> bool {
        self.at_line_start && self.delimiter_depth == 0 && self.block_depth == 0
    }

    fn end_of_input_evidence(&self) -> ModuleValueSuffixEvidence {
        // A block-closing `end` immediately before EOF can belong to the
        // enclosing declaration after block recovery. Keep that established
        // typed-local shape distinct from a plain module value at EOF.
        if self.closed_initializer_block {
            ModuleValueSuffixEvidence::EndOfInputAfterBlock
        } else {
            ModuleValueSuffixEvidence::EndOfInput
        }
    }

    fn advance(&mut self, kind: TokenKind) {
        self.at_line_start = kind == TokenKind::Newline;
        self.previous_kind = Some(kind);
        self.cursor += 1;
    }
}

pub(super) fn module_value_initializer_cursor(
    tokens: &[Token],
    mut cursor: usize,
) -> Option<usize> {
    if tokens.get(cursor)?.kind == TokenKind::Pub {
        cursor += 1;
    }
    if tokens.get(cursor)?.kind != TokenKind::Let {
        return None;
    }
    cursor += 1;

    if !is_module_value_name(tokens.get(cursor)?) {
        return None;
    }
    cursor += 1;
    if tokens.get(cursor)?.kind != TokenKind::Colon {
        return None;
    }
    cursor += 1;

    let type_start = cursor;
    while !matches!(
        tokens.get(cursor).map(|token| token.kind),
        Some(TokenKind::Equal | TokenKind::Newline | TokenKind::Eof) | None
    ) {
        cursor += 1;
    }
    if cursor == type_start || tokens.get(cursor)?.kind != TokenKind::Equal {
        return None;
    }
    cursor += 1;
    if matches!(
        tokens.get(cursor).map(|token| token.kind),
        Some(TokenKind::Newline | TokenKind::Eof) | None
    ) {
        return None;
    }
    Some(cursor)
}

pub(super) fn is_module_value_name(token: &Token) -> bool {
    token.kind == TokenKind::Ident
        && token
            .text
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        && token.text != crate::TRUE_LITERAL
        && token.text != crate::FALSE_LITERAL
}
