use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[cfg(test)]
use std::cell::Cell;

use veln_source::SourceFile;
use veln_syntax::{
    FunctionDecl, SyntaxItem, Token, TokenKind, canonical_type_text,
    declaration_function_signature, lex, parse, presentation_parse_structure_is_bounded,
};

use crate::navigation::{SignatureShadowIndex, function_signature_definition_at};
use crate::{EffectiveProjectSnapshot, NavigationSource, SourcePosition};

mod completion;

pub use completion::completion_at;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionCandidateKind {
    DeclarationModifier,
    BuiltinLocal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionCandidate {
    pub label: &'static str,
    pub detail: &'static str,
    pub kind: CompletionCandidateKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
}

#[derive(Clone, Debug)]
enum LocalSignatureDeclaration {
    Function(Range<usize>),
    Alias(String),
}

const MAX_SIGNATURE_NAVIGATION_LOOKUPS: usize = 64;

#[cfg(test)]
thread_local! {
    static SIGNATURE_INDEX_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_HEADER_PARSES: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_NAME_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_NAVIGATION_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SIGNATURE_INDEX_OWNED_TEXT_BYTES: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn reset_signature_help_work() {
    SIGNATURE_INDEX_TOKEN_VISITS.set(0);
    SIGNATURE_CANDIDATE_VISITS.set(0);
    SIGNATURE_HEADER_PARSES.set(0);
    SIGNATURE_NAME_LOOKUPS.set(0);
    SIGNATURE_NAVIGATION_LOOKUPS.set(0);
    SIGNATURE_INDEX_OWNED_TEXT_BYTES.set(0);
}

#[cfg(test)]
fn signature_help_work() -> (usize, usize, usize) {
    (
        SIGNATURE_INDEX_TOKEN_VISITS.get(),
        SIGNATURE_CANDIDATE_VISITS.get(),
        SIGNATURE_HEADER_PARSES.get(),
    )
}

#[cfg(test)]
fn record_signature_index_token_visits(count: usize) {
    SIGNATURE_INDEX_TOKEN_VISITS.set(SIGNATURE_INDEX_TOKEN_VISITS.get() + count);
}

#[cfg(not(test))]
fn record_signature_index_token_visits(_: usize) {}

#[cfg(test)]
fn record_signature_candidate_visit() {
    SIGNATURE_CANDIDATE_VISITS.set(SIGNATURE_CANDIDATE_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_signature_candidate_visit() {}

#[cfg(test)]
fn record_signature_header_parse() {
    SIGNATURE_HEADER_PARSES.set(SIGNATURE_HEADER_PARSES.get() + 1);
}

#[cfg(not(test))]
fn record_signature_header_parse() {}

#[cfg(test)]
fn record_signature_name_lookup() {
    SIGNATURE_NAME_LOOKUPS.set(SIGNATURE_NAME_LOOKUPS.get() + 1);
}

#[cfg(not(test))]
fn record_signature_name_lookup() {}

#[cfg(test)]
fn signature_name_lookups() -> usize {
    SIGNATURE_NAME_LOOKUPS.get()
}

#[cfg(test)]
fn record_signature_navigation_lookup() {
    SIGNATURE_NAVIGATION_LOOKUPS.set(SIGNATURE_NAVIGATION_LOOKUPS.get() + 1);
}

#[cfg(not(test))]
fn record_signature_navigation_lookup() {}

#[cfg(test)]
fn signature_navigation_lookups() -> usize {
    SIGNATURE_NAVIGATION_LOOKUPS.get()
}

#[cfg(test)]
fn record_signature_index_owned_text_bytes(
    declarations: &HashMap<String, Vec<LocalSignatureDeclaration>>,
) {
    let bytes = declarations
        .iter()
        .map(|(name, declarations)| {
            name.len()
                + declarations
                    .iter()
                    .map(|declaration| match declaration {
                        LocalSignatureDeclaration::Function(_) => 0,
                        LocalSignatureDeclaration::Alias(target) => target.len(),
                    })
                    .sum::<usize>()
        })
        .sum();
    SIGNATURE_INDEX_OWNED_TEXT_BYTES.set(bytes);
}

#[cfg(not(test))]
fn record_signature_index_owned_text_bytes(_: &HashMap<String, Vec<LocalSignatureDeclaration>>) {}

#[cfg(test)]
fn signature_index_owned_text_bytes() -> usize {
    SIGNATURE_INDEX_OWNED_TEXT_BYTES.get()
}

pub fn signature_help_at(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<SignatureHelp> {
    let source = snapshot.workspace_source(&position.source)?;
    let offset = source_offset(source, position.line, position.column)?;
    let tokens = lex(source);
    let navigation_parse_allowed = presentation_parse_structure_is_bounded(&tokens.tokens);
    let significant = tokens
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Newline | TokenKind::Comment | TokenKind::Eof
            )
        })
        .collect::<Vec<_>>();
    record_signature_index_token_visits(tokens.tokens.len());
    let shadow_index = SignatureShadowIndex::new(&tokens.tokens, offset);
    let recovery_start = shadow_index.recovery_start();
    let local_signatures = local_signature_declarations(source, &significant);
    let function_names = signature_function_names(snapshot);
    let mut search = SignatureHelpSearch {
        snapshot,
        position: &position,
        source,
        offset,
        tokens: &tokens.tokens,
        significant: &significant,
        shadow_index,
        local_signatures,
        function_names,
        navigation_parse_allowed,
        recovered_function_is_callsite_aware: None,
        navigation_lookups: 0,
    };
    for open_index in open_call_indices(&significant, recovery_start, offset)
        .into_iter()
        .rev()
    {
        record_signature_candidate_visit();
        if let Some(help) = search.resolve(open_index) {
            return Some(help);
        }
    }
    None
}

fn open_call_indices(
    significant: &[(usize, &Token)],
    recovery_start: usize,
    offset: usize,
) -> Vec<usize> {
    let mut open = Vec::new();
    for (index, (_, token)) in significant.iter().enumerate() {
        if token.range.start >= offset {
            break;
        }
        if token.range.start < recovery_start {
            continue;
        }
        match token.kind {
            TokenKind::LParen => open.push(index),
            TokenKind::RParen => {
                open.pop();
            }
            _ => {}
        }
    }
    open
}

struct SignatureCallCandidate {
    token_index: usize,
    name: String,
    offset: usize,
    is_bare: bool,
    is_in_function_body: bool,
}

struct SignatureHelpSearch<'a, 'tokens> {
    snapshot: &'a EffectiveProjectSnapshot,
    position: &'a SourcePosition,
    source: &'a SourceFile,
    offset: usize,
    tokens: &'tokens [Token],
    significant: &'tokens [(usize, &'tokens Token)],
    shadow_index: SignatureShadowIndex,
    local_signatures: HashMap<String, Vec<LocalSignatureDeclaration>>,
    function_names: HashSet<String>,
    navigation_parse_allowed: bool,
    recovered_function_is_callsite_aware: Option<bool>,
    navigation_lookups: usize,
}

impl SignatureHelpSearch<'_, '_> {
    fn resolve(&mut self, open_index: usize) -> Option<SignatureHelp> {
        let candidate = self.candidate(open_index)?;
        if self.is_builtin_callsite(&candidate)
            || self
                .shadow_index
                .shadows(self.tokens, candidate.token_index, &candidate.name)
        {
            return None;
        }
        if let Some(help) = self.local_help(&candidate, open_index) {
            return Some(help);
        }
        self.navigation_help(&candidate, open_index)
    }

    fn candidate(&self, open_index: usize) -> Option<SignatureCallCandidate> {
        let significant_index = open_index.checked_sub(1)?;
        let (token_index, callee) = self.significant.get(significant_index).copied()?;
        let is_qualified = significant_index
            .checked_sub(1)
            .is_some_and(|previous| self.significant[previous].1.kind == TokenKind::DoubleColon);
        if !(callee.kind.is_bare_expression_identifier()
            || callee.kind == TokenKind::Handle && is_qualified)
            || declaration_name_token(self.significant, significant_index)
        {
            return None;
        }
        Some(SignatureCallCandidate {
            token_index,
            name: callee.text.clone(),
            offset: callee.range.start,
            is_bare: !is_qualified,
            is_in_function_body: self
                .shadow_index
                .allows_local_signature(self.tokens, token_index),
        })
    }

    fn is_builtin_callsite(&mut self, candidate: &SignatureCallCandidate) -> bool {
        candidate.name == "callsite"
            && candidate.is_bare
            && candidate.is_in_function_body
            && *self
                .recovered_function_is_callsite_aware
                .get_or_insert_with(|| {
                    recovered_function_has_callsite_modifier(
                        self.source,
                        self.tokens,
                        self.shadow_index.recovery_start(),
                    )
                })
    }

    fn local_help(
        &self,
        candidate: &SignatureCallCandidate,
        open_index: usize,
    ) -> Option<SignatureHelp> {
        if !candidate.is_in_function_body || !candidate.is_bare {
            return None;
        }
        let function =
            local_signature_function(self.source, &self.local_signatures, &candidate.name)?;
        Some(signature_help(
            &function,
            active_parameter(self.significant, open_index, self.offset),
        ))
    }

    fn navigation_help(
        &mut self,
        candidate: &SignatureCallCandidate,
        open_index: usize,
    ) -> Option<SignatureHelp> {
        record_signature_name_lookup();
        if !self.function_names.contains(&candidate.name)
            || !self.navigation_parse_allowed
            || self.navigation_lookups >= MAX_SIGNATURE_NAVIGATION_LOOKUPS
        {
            return None;
        }
        self.navigation_lookups += 1;
        record_signature_navigation_lookup();
        let function = self.navigation_function(candidate)?;
        Some(signature_help(
            &function,
            active_parameter(self.significant, open_index, self.offset),
        ))
    }

    fn navigation_function(&self, candidate: &SignatureCallCandidate) -> Option<FunctionDecl> {
        let line_col = self.source.line_col(candidate.offset);
        let (selection, definition) = function_signature_definition_at(
            self.snapshot,
            &SourcePosition {
                source: self.position.source.clone(),
                line: line_col.line,
                column: line_col.column,
            },
        )?;
        if matches!(&definition.source, NavigationSource::Workspace)
            && definition.span.file == self.position.source
            && selection.start.offset == definition.span.start.offset
        {
            return None;
        }
        let declaration_source = match &definition.source {
            NavigationSource::Workspace => self
                .snapshot
                .workspace_source(&definition.span.file)
                .cloned(),
            NavigationSource::Package { uri } => self
                .snapshot
                .resolve_virtual_source(uri)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(|text| SourceFile::new(definition.span.file.clone(), text)),
        }?;
        parse_function_signature_at(&declaration_source, definition.span.start.offset)
    }
}

fn recovered_function_has_callsite_modifier(
    source: &SourceFile,
    tokens: &[Token],
    recovery_start: usize,
) -> bool {
    let Some(start_index) = tokens
        .iter()
        .position(|token| token.range.start == recovery_start && token.kind == TokenKind::Fn)
    else {
        return false;
    };
    let Some(name) = tokens[start_index + 1..].iter().find(|token| {
        !matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::Newline | TokenKind::Comment
        )
    }) else {
        return false;
    };
    name.kind.is_contextual_identifier()
        && parse_function_signature_at(source, name.range.start)
            .is_some_and(|function| function.callsite.is_some())
}

fn signature_function_names(snapshot: &EffectiveProjectSnapshot) -> HashSet<String> {
    snapshot
        .source_texts_for_signature_index()
        .flat_map(|text| {
            let tokens = lex(&SourceFile::new("signature-index.veln", text)).tokens;
            let significant = tokens
                .iter()
                .filter(|token| {
                    !matches!(
                        token.kind,
                        TokenKind::Whitespace
                            | TokenKind::Newline
                            | TokenKind::Comment
                            | TokenKind::Eof
                    )
                })
                .collect::<Vec<_>>();
            significant
                .windows(2)
                .filter(|pair| {
                    matches!(pair[0].kind, TokenKind::Fn | TokenKind::Test)
                        && pair[1].kind.is_contextual_identifier()
                })
                .map(|pair| pair[1].text.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn local_signature_declarations(
    source: &SourceFile,
    significant: &[(usize, &Token)],
) -> HashMap<String, Vec<LocalSignatureDeclaration>> {
    let mut declarations = HashMap::<String, Vec<LocalSignatureDeclaration>>::new();
    let candidates = local_signature_candidates(significant);
    let mut line_end = 0usize;
    for candidate_index in 0..candidates.len() {
        let Some((name, declaration)) = local_signature_declaration(
            source,
            significant,
            &candidates,
            candidate_index,
            &mut line_end,
        ) else {
            continue;
        };
        declarations.entry(name).or_default().push(declaration);
    }
    record_signature_index_owned_text_bytes(&declarations);
    declarations
}

fn local_signature_candidates(significant: &[(usize, &Token)]) -> Vec<usize> {
    significant
        .iter()
        .enumerate()
        .filter_map(|(index, (_, token))| {
            if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
                return None;
            }
            significant
                .get(index + 1)
                .is_some_and(|(_, name)| name.kind.is_contextual_identifier())
                .then_some(index)
        })
        .collect()
}

fn local_signature_declaration(
    source: &SourceFile,
    significant: &[(usize, &Token)],
    candidates: &[usize],
    candidate_index: usize,
    line_end: &mut usize,
) -> Option<(String, LocalSignatureDeclaration)> {
    record_signature_index_token_visits(1);
    let index = candidates[candidate_index];
    let (_, token) = significant[index];
    let (_, name) = significant.get(index + 1).copied()?;
    let declaration = if significant
        .get(index + 2)
        .is_some_and(|(_, token)| token.kind == TokenKind::Equal)
    {
        local_signature_alias(significant, index)?
    } else {
        local_signature_range(
            source,
            significant,
            candidates,
            candidate_index,
            line_end,
            token,
        )
    };
    Some((name.text.clone(), declaration))
}

fn local_signature_alias(
    significant: &[(usize, &Token)],
    index: usize,
) -> Option<LocalSignatureDeclaration> {
    let (_, target) = significant.get(index + 3).copied()?;
    if !target.kind.is_contextual_identifier()
        || significant
            .get(index + 4)
            .is_some_and(|(_, token)| token.kind == TokenKind::DoubleColon)
    {
        return None;
    }
    Some(LocalSignatureDeclaration::Alias(target.text.clone()))
}

fn local_signature_range(
    source: &SourceFile,
    significant: &[(usize, &Token)],
    candidates: &[usize],
    candidate_index: usize,
    line_end: &mut usize,
    token: &Token,
) -> LocalSignatureDeclaration {
    if token.range.start >= *line_end {
        *line_end = source.text()[token.range.start..]
            .find('\n')
            .map_or(source.len(), |relative| token.range.start + relative);
    }
    let next_declaration_start = candidates
        .get(candidate_index + 1)
        .map_or(*line_end, |next| significant[*next].1.range.start);
    LocalSignatureDeclaration::Function(token.range.start..(*line_end).min(next_declaration_start))
}

fn local_signature_function(
    source: &SourceFile,
    declarations: &HashMap<String, Vec<LocalSignatureDeclaration>>,
    name: &str,
) -> Option<FunctionDecl> {
    let mut name = name;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(name.to_string()) {
            return None;
        }
        let [declaration] = declarations.get(name)?.as_slice() else {
            return None;
        };
        match declaration {
            LocalSignatureDeclaration::Alias(target) => name = target,
            LocalSignatureDeclaration::Function(range) => {
                record_signature_header_parse();
                let header = source.text().get(range.clone())?;
                return parse_function_header(header);
            }
        }
    }
}

fn parse_function_signature_at(source: &SourceFile, name_offset: usize) -> Option<FunctionDecl> {
    record_signature_header_parse();
    let tokens = lex(source).tokens;
    let name_index = tokens.iter().position(|token| {
        token.range.start == name_offset && token.kind.is_contextual_identifier()
    })?;
    let start = tokens[..name_index]
        .iter()
        .rposition(|token| matches!(token.kind, TokenKind::Fn | TokenKind::Test))?;
    let end = function_signature_end(&tokens[start..], source.len());
    let header = source.text().get(tokens[start].range.start..end)?;
    parse_function_header(header)
}

#[derive(Default)]
struct SignatureHeaderDepth {
    parens: usize,
    brackets: usize,
    braces: usize,
    saw_parameters: bool,
}

impl SignatureHeaderDepth {
    fn observe(&mut self, kind: TokenKind) {
        match kind {
            TokenKind::LParen => {
                self.saw_parameters = true;
                self.parens += 1;
            }
            TokenKind::RParen => self.parens = self.parens.saturating_sub(1),
            TokenKind::LBracket => self.brackets += 1,
            TokenKind::RBracket => self.brackets = self.brackets.saturating_sub(1),
            TokenKind::LBrace => self.braces += 1,
            TokenKind::RBrace => self.braces = self.braces.saturating_sub(1),
            _ => {}
        }
    }

    fn ends_at(&self, token: &Token) -> Option<usize> {
        (token.kind == TokenKind::Newline
            && self.saw_parameters
            && self.parens == 0
            && self.brackets == 0
            && self.braces == 0)
            .then_some(token.range.start)
    }
}

fn function_signature_end(tokens: &[Token], source_len: usize) -> usize {
    let mut depth = SignatureHeaderDepth::default();
    tokens
        .iter()
        .find_map(|token| {
            depth.observe(token.kind);
            depth.ends_at(token)
        })
        .unwrap_or(source_len)
}

fn parse_function_header(header: &str) -> Option<FunctionDecl> {
    let declaration = SourceFile::new("signature-help.veln", format!("{header}\n  0\nend\n"));
    if !presentation_parse_structure_is_bounded(&lex(&declaration).tokens) {
        return None;
    }
    parse(&declaration).tree.items.into_iter().find_map(|item| {
        let SyntaxItem::Function(function) = item else {
            return None;
        };
        Some(*function)
    })
}

fn declaration_name_token(significant: &[(usize, &Token)], callee_index: usize) -> bool {
    callee_index
        .checked_sub(1)
        .is_some_and(|index| matches!(significant[index].1.kind, TokenKind::Fn | TokenKind::Test))
}

fn signature_help(function: &FunctionDecl, active_parameter: usize) -> SignatureHelp {
    SignatureHelp {
        label: declaration_function_signature(function, true),
        parameters: function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) if param.is_variadic => {
                    format!("{}: ...{}", param.name, canonical_type_text(ty))
                }
                Some(ty) => format!("{}: {}", param.name, canonical_type_text(ty)),
                None => param.name.clone(),
            })
            .collect(),
        active_parameter: active_parameter.min(function.params.len().saturating_sub(1)),
    }
}

fn active_parameter(
    significant: &[(usize, &veln_syntax::Token)],
    open_index: usize,
    offset: usize,
) -> usize {
    let mut depth = 0usize;
    let mut active = 0usize;
    for (_, token) in significant.iter().skip(open_index + 1) {
        if token.range.start >= offset {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1)
            }
            TokenKind::Comma if depth == 0 => active += 1,
            _ => {}
        }
    }
    active
}

fn source_offset(source: &SourceFile, line: usize, column: usize) -> Option<usize> {
    if line == 0 || column == 0 {
        return None;
    }
    let line_start = source
        .text()
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let selected = source.text().get(line_start..)?.split('\n').next()?;
    let byte_column = selected
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or_else(
            || (column == selected.chars().count() + 1).then_some(selected.len()),
            |(index, _)| Some(index),
        )?;
    Some(line_start + byte_column)
}

#[cfg(test)]
#[path = "presentation/tests.rs"]
mod tests;
