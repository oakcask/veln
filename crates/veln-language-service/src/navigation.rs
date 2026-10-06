#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::sync::{Arc, OnceLock};

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_ast::{InvalidName, NameClass, QualifiedPathSegment};
use veln_project::classify_companion_source;
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    BodyLine, Expr, ExprKind, FunctionDecl, ParseOutput, PublicAliasKind, SyntaxItem, SyntaxTree,
    Token, TokenKind, TypeVariantDecl, Visibility, lex, parse,
};

include!("navigation/model.rs");
include!("navigation/source_indexing.rs");
include!("navigation/effect_source_indexing.rs");
include!("navigation/handler_source_indexing.rs");
include!("navigation/schema_source_indexing.rs");
include!("navigation/workspace_source_index.rs");
include!("navigation/workspace_schema_composition.rs");
include!("navigation/package_schemas.rs");
include!("navigation/schema_navigation_indexing.rs");
include!("navigation/index_construction.rs");
include!("navigation/index.rs");
include!("navigation/index_visibility.rs");
include!("navigation/selection.rs");
include!("navigation/recovery.rs");
include!("navigation/rename_shared.rs");
include!("navigation/recovery_rename_conflicts.rs");
include!("navigation/function_rename_conflicts.rs");
include!("navigation/rename_conflicts.rs");
include!("navigation/rename_visibility.rs");
include!("navigation/symbol_lookup.rs");
include!("navigation/symbol_references.rs");
include!("navigation/declarations.rs");
include!("navigation/function_alias_declarations.rs");
include!("navigation/recovery_declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/local_binding_scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");

pub(crate) struct SignatureShadowIndex {
    scopes: Vec<FunctionScope>,
    recovery_start: usize,
    #[cfg(test)]
    inspected_tokens: usize,
}

impl SignatureShadowIndex {
    pub(crate) fn new(tokens: &[Token], cursor_offset: usize) -> Self {
        let boundaries = signature_recovery_declaration_indices(tokens);
        let boundary =
            boundaries.partition_point(|index| tokens[*index].range.start <= cursor_offset);
        let Some(start_index) = boundary.checked_sub(1).map(|index| boundaries[index]) else {
            return Self {
                scopes: Vec::new(),
                recovery_start: 0,
                #[cfg(test)]
                inspected_tokens: tokens.len(),
            };
        };
        let end_index = boundaries.get(boundary).copied().unwrap_or(tokens.len());
        let window = &tokens[start_index..end_index];
        let (mut scopes, defer_block_openers) = signature_shadow_scopes(window);
        let window_end = tokens.get(end_index).map_or_else(
            || tokens.last().map_or(0, |token| token.range.end),
            |token| token.range.start,
        );
        widen_recovered_signature_scope(&mut scopes, window, window_end, &defer_block_openers);
        Self {
            scopes,
            recovery_start: tokens[start_index].range.start,
            #[cfg(test)]
            inspected_tokens: tokens.len() + window.len(),
        }
    }

    pub(crate) fn recovery_start(&self) -> usize {
        self.recovery_start
    }

    pub(crate) fn shadows(&self, tokens: &[Token], token_index: usize, name: &str) -> bool {
        local_binding_shadows_call_target_in_scopes(&self.scopes, tokens, token_index, name)
    }

    pub(crate) fn allows_local_signature(&self, tokens: &[Token], token_index: usize) -> bool {
        let offset = tokens[token_index].range.start;
        token_scope(&self.scopes, offset).is_some_and(|scope| !scope.is_handler_clause)
    }

    #[cfg(test)]
    fn work_and_retention(&self) -> (usize, usize, usize) {
        (
            self.inspected_tokens,
            self.scopes.len(),
            self.scopes
                .iter()
                .map(|scope| scope.local_bindings.len())
                .sum(),
        )
    }
}

fn signature_shadow_scopes(tokens: &[Token]) -> (Vec<FunctionScope>, Vec<bool>) {
    let defer_block_openers = defer_block_openers(tokens);
    let mut scopes = tokens
        .first()
        .filter(|token| matches!(token.kind, TokenKind::Fn | TokenKind::Test))
        .and_then(|_| function_scope(tokens, 0, &defer_block_openers))
        .into_iter()
        .collect::<Vec<_>>();
    scopes.extend(handler_operation_clause_scopes(
        tokens,
        &defer_block_openers,
    ));
    (scopes, defer_block_openers)
}

fn widen_recovered_signature_scope(
    scopes: &mut [FunctionScope],
    tokens: &[Token],
    window_end: usize,
    defer_block_openers: &[bool],
) {
    for scope in scopes
        .iter_mut()
        .filter(|scope| !scope.is_handler_clause && scope.end == scope.body_start)
    {
        scope.end = window_end;
        scope.local_bindings = local_bindings_with_defer_openers(
            tokens,
            scope.body_start,
            scope.end,
            defer_block_openers,
        );
        scope.local_bindings_by_name = local_binding_index_by_name(&scope.local_bindings);
    }
}

fn signature_recovery_declaration_indices(tokens: &[Token]) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let mut first_on_line = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Newline => {
                first_on_line = None;
                continue;
            }
            TokenKind::Whitespace if first_on_line.is_none() => continue,
            TokenKind::Comment | TokenKind::Eof => continue,
            _ => {}
        }
        match first_on_line {
            None if token.kind == TokenKind::Pub => first_on_line = Some(TokenKind::Pub),
            None if signature_recovery_declaration_at(tokens, index) => {
                boundaries.push(index);
                first_on_line = Some(token.kind);
            }
            Some(TokenKind::Pub) if signature_recovery_declaration_at(tokens, index) => {
                boundaries.push(index);
                first_on_line = Some(token.kind);
            }
            None | Some(TokenKind::Pub) => first_on_line = Some(token.kind),
            Some(_) => {}
        }
    }
    boundaries
}

fn signature_recovery_declaration_at(tokens: &[Token], index: usize) -> bool {
    let kind = tokens[index].kind;
    if !matches!(
        kind,
        TokenKind::Fn
            | TokenKind::Test
            | TokenKind::Type
            | TokenKind::Schema
            | TokenKind::Effect
            | TokenKind::Handler
            | TokenKind::Codec
    ) {
        return false;
    }
    next_non_layout_token(tokens, index).is_some_and(|token| token.kind.is_contextual_identifier())
}

#[cfg(test)]
mod signature_shadow_tests {
    use super::*;

    fn incomplete_scope_work(function_count: usize) -> (usize, usize, usize) {
        let mut text = String::new();
        for index in 0..function_count {
            text.push_str(&format!(
                "fn unfinished_{index}() -> Int\n  let value_{index} = {index}\n"
            ));
        }
        let source = SourceFile::new("many-incomplete.veln", text);
        let tokens = lex(&source).tokens;
        SignatureShadowIndex::new(&tokens, source.len()).work_and_retention()
    }

    #[test]
    fn incomplete_function_shadow_index_has_linear_work_and_bounded_retention() {
        let smaller = incomplete_scope_work(1_000);
        let larger = incomplete_scope_work(2_000);
        eprintln!("incomplete signature scopes: 1000={smaller:?}, 2000={larger:?}");

        assert!(
            larger.0 <= smaller.0 * 2 + 16,
            "shadow-index work grew too quickly: {smaller:?} -> {larger:?}"
        );
        assert_eq!(smaller.1, 1);
        assert_eq!(larger.1, 1);
        assert_eq!(smaller.2, 1);
        assert_eq!(larger.2, 1);
    }

    fn malformed_header_with_locals_work(
        declaration_count: usize,
        local_count: usize,
    ) -> (usize, usize, usize, usize) {
        let mut text = String::new();
        for index in 0..declaration_count {
            text.push_str(&format!("fn unfinished_{index} "));
        }
        text.push('\n');
        for index in 0..local_count {
            text.push_str(&format!("  let value_{index} = {index}\n"));
        }
        let source = SourceFile::new("malformed-header.veln", text);
        let tokens = lex(&source).tokens;
        reset_local_binding_scope_token_visits();
        let (inspected, scopes, bindings) =
            SignatureShadowIndex::new(&tokens, source.len()).work_and_retention();
        (
            inspected,
            scopes,
            bindings,
            local_binding_scope_token_visits(),
        )
    }

    #[test]
    fn malformed_header_shadow_index_has_linear_work_and_bounded_retention() {
        let smaller = malformed_header_with_locals_work(100, 100);
        let larger = malformed_header_with_locals_work(200, 200);
        eprintln!("malformed signature scopes: 100={smaller:?}, 200={larger:?}");

        assert!(
            larger.0 <= smaller.0 * 2 + 32,
            "shadow-index work grew too quickly: {smaller:?} -> {larger:?}"
        );
        assert!(
            larger.3 <= smaller.3 * 2 + 32,
            "local-binding work grew too quickly: {smaller:?} -> {larger:?}"
        );
        assert_eq!(smaller.1, 1);
        assert_eq!(larger.1, 1);
        assert_eq!(smaller.2, 100);
        assert_eq!(larger.2, 200);
    }
}

#[cfg(test)]
#[path = "navigation/classification_tests.rs"]
mod classification_tests;

#[cfg(test)]
thread_local! {
    static FUNCTION_SCOPE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static LOCAL_REFERENCE_SCOPE_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static LOCAL_REFERENCE_BINDING_CANDIDATE_COMPARISONS: Cell<usize> = const { Cell::new(0) };
    static LOCAL_BINDING_SCOPE_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static HANDLER_CLAUSE_SCOPE_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static HANDLER_CLAUSE_BINDING_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static TYPE_REFERENCE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static TYPE_REFERENCE_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static CONSTRUCTOR_REFERENCE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_INDEXES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_PARSES: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SOURCE_PARSES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_PATH_CLASSIFICATIONS: Cell<usize> = const { Cell::new(0) };
    static PATH_CLASSIFICATION_CONTEXTS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_DECLARATION_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_ELIGIBILITY_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_DECLARATION_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_TARGET_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_PRELUDE_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_BLOCKER_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_COMPOSITION_SCHEMA_VISITS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_COMPOSITION_ALIAS_VISITS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_COMPOSITION_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_ALIAS_RESOLUTION_INDEX_VISITS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_ALIAS_RESOLUTION_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SCHEMA_ALIAS_RESOLUTION_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_OPERATION_PRELUDE_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_OPERATION_BLOCKER_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_OPERATION_LEAF_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_OPERATION_QUALIFIED_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_OPERATION_QUALIFIED_TARGET_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static TYPE_NAMESPACE_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_LIST_CLASSIFICATION_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_LIST_CLASSIFICATION_FRAME_VISITS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_PATH_CLASSIFICATION_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static USE_DIAGNOSTIC_INDEX_VISITS: Cell<usize> = const { Cell::new(0) };
    static USE_DIAGNOSTIC_OVERLAP_QUERIES: Cell<usize> = const { Cell::new(0) };
    static EFFECT_REFERENCE_SOURCE_SCALAR_VISITS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_DECLARATION_INDEX_VISITS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_IDENTITY_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_OPERATION_IDENTITY_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static EFFECT_OPERATION_CANDIDATE_VISITS: Cell<usize> = const { Cell::new(0) };
    static HANDLER_REFERENCE_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
    static HANDLER_DIAGNOSTIC_INDEX_VISITS: Cell<usize> = const { Cell::new(0) };
    static HANDLER_DIAGNOSTIC_OVERLAP_QUERIES: Cell<usize> = const { Cell::new(0) };
    static HANDLER_CLAUSE_BODY_RANGE_INDEX_ENTRIES: Cell<usize> = const { Cell::new(0) };
    static HANDLER_CLAUSE_BODY_MEMBERSHIP_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static FUNCTION_SCOPE_LOOKUP_COMPARISONS: Cell<usize> = const { Cell::new(0) };
    static FUNCTION_ALIAS_TARGET_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static FUNCTION_ALIAS_DECLARING_FILE_LOOKUPS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn record_function_alias_declaring_file_lookup() {
    FUNCTION_ALIAS_DECLARING_FILE_LOOKUPS.set(FUNCTION_ALIAS_DECLARING_FILE_LOOKUPS.get() + 1);
}

#[cfg(not(test))]
fn record_function_alias_declaring_file_lookup() {}

#[cfg(test)]
pub(crate) fn reset_function_alias_declaring_file_lookups() {
    FUNCTION_ALIAS_DECLARING_FILE_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn function_alias_declaring_file_lookups() -> usize {
    FUNCTION_ALIAS_DECLARING_FILE_LOOKUPS.get()
}

#[cfg(test)]
fn record_function_alias_target_lookup() {
    FUNCTION_ALIAS_TARGET_LOOKUPS.set(FUNCTION_ALIAS_TARGET_LOOKUPS.get() + 1);
}

#[cfg(not(test))]
fn record_function_alias_target_lookup() {}

#[cfg(test)]
pub(crate) fn reset_function_alias_target_lookups() {
    FUNCTION_ALIAS_TARGET_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn function_alias_target_lookups() -> usize {
    FUNCTION_ALIAS_TARGET_LOOKUPS.get()
}

#[cfg(test)]
fn record_function_scope_lookup_comparison() {
    FUNCTION_SCOPE_LOOKUP_COMPARISONS.set(FUNCTION_SCOPE_LOOKUP_COMPARISONS.get() + 1);
}

#[cfg(not(test))]
fn record_function_scope_lookup_comparison() {}

#[cfg(test)]
pub(crate) fn reset_function_scope_lookup_comparisons() {
    FUNCTION_SCOPE_LOOKUP_COMPARISONS.set(0);
}

#[cfg(test)]
pub(crate) fn function_scope_lookup_comparisons() -> usize {
    FUNCTION_SCOPE_LOOKUP_COMPARISONS.get()
}

#[cfg(test)]
fn record_handler_clause_body_range_index_entry() {
    HANDLER_CLAUSE_BODY_RANGE_INDEX_ENTRIES.set(HANDLER_CLAUSE_BODY_RANGE_INDEX_ENTRIES.get() + 1);
}

#[cfg(test)]
fn record_handler_clause_body_membership_lookup() {
    HANDLER_CLAUSE_BODY_MEMBERSHIP_LOOKUPS.set(HANDLER_CLAUSE_BODY_MEMBERSHIP_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_handler_clause_body_range_work() {
    HANDLER_CLAUSE_BODY_RANGE_INDEX_ENTRIES.set(0);
    HANDLER_CLAUSE_BODY_MEMBERSHIP_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn handler_clause_body_range_work() -> (usize, usize) {
    (
        HANDLER_CLAUSE_BODY_RANGE_INDEX_ENTRIES.get(),
        HANDLER_CLAUSE_BODY_MEMBERSHIP_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_effect_declaration_index_visit() {
    EFFECT_DECLARATION_INDEX_VISITS.set(EFFECT_DECLARATION_INDEX_VISITS.get() + 1);
}

#[cfg(test)]
fn record_effect_identity_lookup() {
    EFFECT_IDENTITY_LOOKUPS.set(EFFECT_IDENTITY_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_effect_identity_index_work() {
    EFFECT_DECLARATION_INDEX_VISITS.set(0);
    EFFECT_IDENTITY_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn effect_identity_index_work() -> (usize, usize) {
    (
        EFFECT_DECLARATION_INDEX_VISITS.get(),
        EFFECT_IDENTITY_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_effect_operation_identity_lookup() {
    EFFECT_OPERATION_IDENTITY_LOOKUPS.set(EFFECT_OPERATION_IDENTITY_LOOKUPS.get() + 1);
}

#[cfg(test)]
fn record_effect_operation_candidate_visit() {
    EFFECT_OPERATION_CANDIDATE_VISITS.set(EFFECT_OPERATION_CANDIDATE_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_effect_operation_identity_work() {
    EFFECT_OPERATION_IDENTITY_LOOKUPS.set(0);
    EFFECT_OPERATION_CANDIDATE_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn effect_operation_identity_work() -> (usize, usize) {
    (
        EFFECT_OPERATION_IDENTITY_LOOKUPS.get(),
        EFFECT_OPERATION_CANDIDATE_VISITS.get(),
    )
}

#[cfg(test)]
fn record_handler_reference_token_visit() {
    HANDLER_REFERENCE_TOKEN_VISITS.set(HANDLER_REFERENCE_TOKEN_VISITS.get() + 1);
}

#[cfg(test)]
fn record_handler_diagnostic_index_visit() {
    HANDLER_DIAGNOSTIC_INDEX_VISITS.set(HANDLER_DIAGNOSTIC_INDEX_VISITS.get() + 1);
}

#[cfg(test)]
fn record_handler_diagnostic_overlap_query() {
    HANDLER_DIAGNOSTIC_OVERLAP_QUERIES.set(HANDLER_DIAGNOSTIC_OVERLAP_QUERIES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_handler_reference_index_work() {
    HANDLER_REFERENCE_TOKEN_VISITS.set(0);
    HANDLER_DIAGNOSTIC_INDEX_VISITS.set(0);
    HANDLER_DIAGNOSTIC_OVERLAP_QUERIES.set(0);
}

#[cfg(test)]
pub(crate) fn handler_reference_index_work() -> (usize, usize, usize) {
    (
        HANDLER_REFERENCE_TOKEN_VISITS.get(),
        HANDLER_DIAGNOSTIC_INDEX_VISITS.get(),
        HANDLER_DIAGNOSTIC_OVERLAP_QUERIES.get(),
    )
}

#[cfg(test)]
fn record_effect_list_classification_token_visit() {
    EFFECT_LIST_CLASSIFICATION_TOKEN_VISITS.set(EFFECT_LIST_CLASSIFICATION_TOKEN_VISITS.get() + 1);
}

#[cfg(test)]
fn record_effect_list_classification_frame_visit() {
    EFFECT_LIST_CLASSIFICATION_FRAME_VISITS.set(EFFECT_LIST_CLASSIFICATION_FRAME_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_effect_list_classification_token_visits() {
    EFFECT_LIST_CLASSIFICATION_TOKEN_VISITS.set(0);
    EFFECT_LIST_CLASSIFICATION_FRAME_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn effect_list_classification_token_visits() -> usize {
    EFFECT_LIST_CLASSIFICATION_TOKEN_VISITS.get()
}

#[cfg(test)]
pub(crate) fn effect_list_classification_frame_visits() -> usize {
    EFFECT_LIST_CLASSIFICATION_FRAME_VISITS.get()
}

#[cfg(test)]
fn record_effect_path_classification_token_visit() {
    EFFECT_PATH_CLASSIFICATION_TOKEN_VISITS.set(EFFECT_PATH_CLASSIFICATION_TOKEN_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_effect_path_classification_token_visits() {
    EFFECT_PATH_CLASSIFICATION_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn effect_path_classification_token_visits() -> usize {
    EFFECT_PATH_CLASSIFICATION_TOKEN_VISITS.get()
}

#[cfg(test)]
fn record_use_diagnostic_index_visit() {
    USE_DIAGNOSTIC_INDEX_VISITS.set(USE_DIAGNOSTIC_INDEX_VISITS.get() + 1);
}

#[cfg(test)]
fn record_use_diagnostic_overlap_query() {
    USE_DIAGNOSTIC_OVERLAP_QUERIES.set(USE_DIAGNOSTIC_OVERLAP_QUERIES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_use_diagnostic_index_work() {
    USE_DIAGNOSTIC_INDEX_VISITS.set(0);
    USE_DIAGNOSTIC_OVERLAP_QUERIES.set(0);
}

#[cfg(test)]
pub(crate) fn use_diagnostic_index_work() -> (usize, usize) {
    (
        USE_DIAGNOSTIC_INDEX_VISITS.get(),
        USE_DIAGNOSTIC_OVERLAP_QUERIES.get(),
    )
}

#[cfg(test)]
fn record_effect_reference_source_scalar_visit() {
    EFFECT_REFERENCE_SOURCE_SCALAR_VISITS.set(EFFECT_REFERENCE_SOURCE_SCALAR_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_effect_reference_source_scalar_visits() {
    EFFECT_REFERENCE_SOURCE_SCALAR_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn effect_reference_source_scalar_visits() -> usize {
    EFFECT_REFERENCE_SOURCE_SCALAR_VISITS.get()
}

#[cfg(test)]
fn record_type_namespace_candidate_visit() {
    TYPE_NAMESPACE_CANDIDATE_VISITS.set(TYPE_NAMESPACE_CANDIDATE_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_type_namespace_candidate_visits() {
    TYPE_NAMESPACE_CANDIDATE_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn type_namespace_candidate_visits() -> usize {
    TYPE_NAMESPACE_CANDIDATE_VISITS.get()
}

#[cfg(test)]
fn record_schema_alias_declaration_visit() {
    SCHEMA_ALIAS_DECLARATION_VISITS.set(SCHEMA_ALIAS_DECLARATION_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_declaration_visits() {
    SCHEMA_ALIAS_DECLARATION_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_declaration_visits() -> usize {
    SCHEMA_ALIAS_DECLARATION_VISITS.get()
}

#[cfg(test)]
fn record_schema_alias_eligibility_visit() {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.set(SCHEMA_ALIAS_ELIGIBILITY_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_eligibility_visits() {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_eligibility_visits() -> usize {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.get()
}

#[cfg(test)]
fn record_schema_composition_declaration_visit() {
    SCHEMA_COMPOSITION_DECLARATION_VISITS.set(SCHEMA_COMPOSITION_DECLARATION_VISITS.get() + 1);
}

#[cfg(test)]
fn record_schema_composition_field_token_visits(count: usize) {
    SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.set(SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.get() + count);
}

#[cfg(test)]
fn record_schema_composition_prelude_lookup() {
    SCHEMA_COMPOSITION_PRELUDE_LOOKUPS.set(SCHEMA_COMPOSITION_PRELUDE_LOOKUPS.get() + 1);
}

#[cfg(test)]
fn record_schema_composition_blocker_lookup() {
    SCHEMA_COMPOSITION_BLOCKER_LOOKUPS.set(SCHEMA_COMPOSITION_BLOCKER_LOOKUPS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_composition_schema_visit() {
    WORKSPACE_SCHEMA_COMPOSITION_SCHEMA_VISITS
        .set(WORKSPACE_SCHEMA_COMPOSITION_SCHEMA_VISITS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_composition_alias_visit() {
    WORKSPACE_SCHEMA_COMPOSITION_ALIAS_VISITS
        .set(WORKSPACE_SCHEMA_COMPOSITION_ALIAS_VISITS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_composition_token_visit() {
    WORKSPACE_SCHEMA_COMPOSITION_TOKEN_VISITS
        .set(WORKSPACE_SCHEMA_COMPOSITION_TOKEN_VISITS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_alias_resolution_candidate_visit() {
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_CANDIDATE_VISITS
        .set(WORKSPACE_SCHEMA_ALIAS_RESOLUTION_CANDIDATE_VISITS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_alias_resolution_index_visit() {
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_INDEX_VISITS
        .set(WORKSPACE_SCHEMA_ALIAS_RESOLUTION_INDEX_VISITS.get() + 1);
}

#[cfg(test)]
fn record_workspace_schema_alias_resolution_lookup() {
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_LOOKUPS
        .set(WORKSPACE_SCHEMA_ALIAS_RESOLUTION_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_workspace_schema_composition_lookup_work() {
    WORKSPACE_SCHEMA_COMPOSITION_SCHEMA_VISITS.set(0);
    WORKSPACE_SCHEMA_COMPOSITION_ALIAS_VISITS.set(0);
    WORKSPACE_SCHEMA_COMPOSITION_TOKEN_VISITS.set(0);
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_INDEX_VISITS.set(0);
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_LOOKUPS.set(0);
    WORKSPACE_SCHEMA_ALIAS_RESOLUTION_CANDIDATE_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn workspace_schema_composition_lookup_work()
-> (usize, usize, usize, usize, usize, usize) {
    (
        WORKSPACE_SCHEMA_COMPOSITION_SCHEMA_VISITS.get(),
        WORKSPACE_SCHEMA_COMPOSITION_ALIAS_VISITS.get(),
        WORKSPACE_SCHEMA_COMPOSITION_TOKEN_VISITS.get(),
        WORKSPACE_SCHEMA_ALIAS_RESOLUTION_INDEX_VISITS.get(),
        WORKSPACE_SCHEMA_ALIAS_RESOLUTION_LOOKUPS.get(),
        WORKSPACE_SCHEMA_ALIAS_RESOLUTION_CANDIDATE_VISITS.get(),
    )
}

#[cfg(test)]
pub(crate) fn reset_schema_composition_index_work() {
    SCHEMA_COMPOSITION_DECLARATION_VISITS.set(0);
    SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.set(0);
    SCHEMA_COMPOSITION_TARGET_LOOKUPS.set(0);
    SCHEMA_COMPOSITION_PRELUDE_LOOKUPS.set(0);
    SCHEMA_COMPOSITION_BLOCKER_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_composition_index_work() -> (usize, usize) {
    (
        SCHEMA_COMPOSITION_DECLARATION_VISITS.get(),
        SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.get(),
    )
}

#[cfg(test)]
pub(crate) fn schema_composition_target_lookups() -> usize {
    SCHEMA_COMPOSITION_TARGET_LOOKUPS.get()
}

#[cfg(test)]
pub(crate) fn schema_composition_bare_lookup_work() -> (usize, usize) {
    (
        SCHEMA_COMPOSITION_PRELUDE_LOOKUPS.get(),
        SCHEMA_COMPOSITION_BLOCKER_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_schema_operation_prelude_lookup() {
    SCHEMA_OPERATION_PRELUDE_LOOKUPS.set(SCHEMA_OPERATION_PRELUDE_LOOKUPS.get() + 1);
}

#[cfg(test)]
fn record_schema_operation_blocker_lookup() {
    SCHEMA_OPERATION_BLOCKER_LOOKUPS.set(SCHEMA_OPERATION_BLOCKER_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_operation_bare_lookup_work() {
    SCHEMA_OPERATION_PRELUDE_LOOKUPS.set(0);
    SCHEMA_OPERATION_BLOCKER_LOOKUPS.set(0);
    SCHEMA_OPERATION_LEAF_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_operation_bare_lookup_work() -> (usize, usize, usize) {
    (
        SCHEMA_OPERATION_PRELUDE_LOOKUPS.get(),
        SCHEMA_OPERATION_BLOCKER_LOOKUPS.get(),
        SCHEMA_OPERATION_LEAF_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_schema_operation_leaf_lookup() {
    SCHEMA_OPERATION_LEAF_LOOKUPS.set(SCHEMA_OPERATION_LEAF_LOOKUPS.get() + 1);
}

#[cfg(test)]
fn record_schema_operation_qualified_candidate_visit() {
    SCHEMA_OPERATION_QUALIFIED_CANDIDATE_VISITS
        .set(SCHEMA_OPERATION_QUALIFIED_CANDIDATE_VISITS.get() + 1);
}

#[cfg(test)]
fn record_schema_operation_qualified_target_lookup() {
    SCHEMA_OPERATION_QUALIFIED_TARGET_LOOKUPS
        .set(SCHEMA_OPERATION_QUALIFIED_TARGET_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_operation_qualified_lookup_work() {
    SCHEMA_OPERATION_QUALIFIED_CANDIDATE_VISITS.set(0);
    SCHEMA_OPERATION_QUALIFIED_TARGET_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_operation_qualified_lookup_work() -> (usize, usize) {
    (
        SCHEMA_OPERATION_QUALIFIED_CANDIDATE_VISITS.get(),
        SCHEMA_OPERATION_QUALIFIED_TARGET_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_function_scope_collection() {
    FUNCTION_SCOPE_COLLECTIONS.set(FUNCTION_SCOPE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
fn record_local_reference_scope_candidate_visit() {
    LOCAL_REFERENCE_SCOPE_CANDIDATE_VISITS.set(LOCAL_REFERENCE_SCOPE_CANDIDATE_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_local_reference_scope_candidate_visit() {}

#[cfg(test)]
fn record_local_reference_binding_candidate_comparison() {
    LOCAL_REFERENCE_BINDING_CANDIDATE_COMPARISONS
        .set(LOCAL_REFERENCE_BINDING_CANDIDATE_COMPARISONS.get() + 1);
}

#[cfg(not(test))]
fn record_local_reference_binding_candidate_comparison() {}

#[cfg(test)]
pub(crate) fn reset_local_reference_scope_candidate_visits() {
    LOCAL_REFERENCE_SCOPE_CANDIDATE_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn local_reference_scope_candidate_visits() -> usize {
    LOCAL_REFERENCE_SCOPE_CANDIDATE_VISITS.get()
}

#[cfg(test)]
pub(crate) fn reset_local_reference_binding_candidate_comparisons() {
    LOCAL_REFERENCE_BINDING_CANDIDATE_COMPARISONS.set(0);
}

#[cfg(test)]
pub(crate) fn local_reference_binding_candidate_comparisons() -> usize {
    LOCAL_REFERENCE_BINDING_CANDIDATE_COMPARISONS.get()
}

#[cfg(test)]
fn record_local_binding_scope_token_visit() {
    LOCAL_BINDING_SCOPE_TOKEN_VISITS.set(LOCAL_BINDING_SCOPE_TOKEN_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_local_binding_scope_token_visit() {}

#[cfg(test)]
fn record_handler_clause_scope_token_visit() {
    HANDLER_CLAUSE_SCOPE_TOKEN_VISITS.set(HANDLER_CLAUSE_SCOPE_TOKEN_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_handler_clause_scope_token_visit() {}

#[cfg(test)]
fn record_handler_clause_binding_token_visits(count: usize) {
    HANDLER_CLAUSE_BINDING_TOKEN_VISITS.set(HANDLER_CLAUSE_BINDING_TOKEN_VISITS.get() + count);
}

#[cfg(not(test))]
fn record_handler_clause_binding_token_visits(_count: usize) {}

#[cfg(test)]
pub(crate) fn reset_handler_clause_binding_token_visits() {
    HANDLER_CLAUSE_BINDING_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn handler_clause_binding_token_visits() -> usize {
    HANDLER_CLAUSE_BINDING_TOKEN_VISITS.get()
}

#[cfg(test)]
pub(crate) fn reset_handler_clause_scope_token_visits() {
    HANDLER_CLAUSE_SCOPE_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn handler_clause_scope_token_visits() -> usize {
    HANDLER_CLAUSE_SCOPE_TOKEN_VISITS.get()
}

#[cfg(test)]
pub(crate) fn reset_local_binding_scope_token_visits() {
    LOCAL_BINDING_SCOPE_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn local_binding_scope_token_visits() -> usize {
    LOCAL_BINDING_SCOPE_TOKEN_VISITS.get()
}

#[cfg(test)]
pub(crate) fn reset_function_scope_collections() {
    FUNCTION_SCOPE_COLLECTIONS.set(0);
}

#[cfg(test)]
pub(crate) fn function_scope_collections() -> usize {
    FUNCTION_SCOPE_COLLECTIONS.get()
}

#[cfg(test)]
fn record_type_reference_collection() {
    TYPE_REFERENCE_COLLECTIONS.set(TYPE_REFERENCE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
fn record_type_reference_token_visit() {
    TYPE_REFERENCE_TOKEN_VISITS.set(TYPE_REFERENCE_TOKEN_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_type_reference_token_visit() {}

#[cfg(test)]
pub(crate) fn reset_type_reference_collections() {
    TYPE_REFERENCE_COLLECTIONS.set(0);
    TYPE_REFERENCE_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn type_reference_collections() -> usize {
    TYPE_REFERENCE_COLLECTIONS.get()
}

#[cfg(test)]
pub(crate) fn type_reference_token_visits() -> usize {
    TYPE_REFERENCE_TOKEN_VISITS.get()
}

#[cfg(test)]
fn record_constructor_reference_collection() {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.set(CONSTRUCTOR_REFERENCE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_constructor_reference_collections() {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.set(0);
}

#[cfg(test)]
pub(crate) fn constructor_reference_collections() -> usize {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.get()
}

#[cfg(test)]
fn record_dependency_source_index() {
    DEPENDENCY_SOURCE_INDEXES.set(DEPENDENCY_SOURCE_INDEXES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_dependency_source_indexes() {
    DEPENDENCY_SOURCE_INDEXES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_source_indexes() -> usize {
    DEPENDENCY_SOURCE_INDEXES.get()
}

#[cfg(test)]
fn record_dependency_source_parse() {
    DEPENDENCY_SOURCE_PARSES.set(DEPENDENCY_SOURCE_PARSES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_dependency_source_parses() {
    DEPENDENCY_SOURCE_PARSES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_source_parses() -> usize {
    DEPENDENCY_SOURCE_PARSES.get()
}

#[cfg(test)]
fn record_workspace_source_parse() {
    WORKSPACE_SOURCE_PARSES.set(WORKSPACE_SOURCE_PARSES.get() + 1);
}

#[cfg(test)]
fn reset_workspace_source_parses() {
    WORKSPACE_SOURCE_PARSES.set(0);
}

#[cfg(test)]
fn workspace_source_parses() -> usize {
    WORKSPACE_SOURCE_PARSES.get()
}

#[cfg(test)]
fn record_dependency_path_classifications(count: usize) {
    DEPENDENCY_PATH_CLASSIFICATIONS.set(DEPENDENCY_PATH_CLASSIFICATIONS.get() + count);
}

#[cfg(test)]
pub(crate) fn reset_dependency_path_classifications() {
    DEPENDENCY_PATH_CLASSIFICATIONS.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_path_classifications() -> usize {
    DEPENDENCY_PATH_CLASSIFICATIONS.get()
}

#[cfg(test)]
fn record_schema_alias_import_index_entries(count: usize) {
    SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.set(SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.get() + count);
}

#[cfg(test)]
fn record_schema_alias_import_route_lookup() {
    SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.set(SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_import_work() {
    SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.set(0);
    SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_import_work() -> (usize, usize) {
    (
        SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.get(),
        SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.get(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_file_supplies_every_declaration_kind() {
        let (file, declarations, _) = index_workspace_source(SourceFile::new(
            "main.veln",
            concat!(
                "pub type Item\n",
                "  pub Value(value: Int)\n",
                "end\n\n",
                "pub type Exported = Item\n\n",
                "pub fn identity(value: Item) -> Item\n",
                "  value\n",
                "end\n",
            ),
        ));

        assert!(!file.tokens.is_empty());
        assert_eq!(declarations.functions[0].name, "identity");
        assert_eq!(declarations.types[0].name, "Item");
        assert_eq!(declarations.constructors[0].name, "Value");
        assert_eq!(declarations.type_aliases[0].name, "Exported");
    }

    #[test]
    fn package_declarations_share_dependency_origin_metadata() {
        let dependency = crate::tests::dependency_snapshot(
            "example/pkg",
            &[(
                "prelude.veln",
                concat!(
                    "pub type Item\n",
                    "  pub Value(value: Int)\n",
                    "end\n\n",
                    "pub type Exported = Item\n\n",
                    "pub fn identity(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            )],
            ["prelude.veln"],
        );
        let (source, entry) = dependency.indexed_sources().next().unwrap();
        let (file, parsed) = indexed_dependency_source(&dependency, source, entry.uri());
        let declarations = file_declarations(&file, &parsed.tree);

        fn assert_dependency_origin(
            package: &Option<String>,
            origin: Option<PackageOrigin>,
            source: &NavigationSource,
        ) {
            assert_eq!(package.as_deref(), Some("example/pkg"));
            assert_eq!(origin, Some(PackageOrigin::DirectDependency));
            assert!(matches!(source, NavigationSource::Package { .. }));
        }

        let function = &declarations.functions[0];
        assert_dependency_origin(
            &function.package,
            function.package_origin,
            &function.declaration.source,
        );
        let symbol_type = &declarations.types[0];
        assert_dependency_origin(
            &symbol_type.package,
            symbol_type.package_origin,
            &symbol_type.declaration.source,
        );
        let constructor = &declarations.constructors[0];
        assert_dependency_origin(
            &constructor.package,
            constructor.package_origin,
            &constructor.declaration.source,
        );
        let alias = &declarations.type_aliases[0];
        assert_dependency_origin(
            &alias.package,
            alias.package_origin,
            &alias.declaration.source,
        );
    }

    #[test]
    fn workspace_index_parses_each_source_once() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            SourceFile::new("main.veln", "fn main() -> Int\n  helper()\nend\n"),
            SourceFile::new("helper.veln", "fn helper() -> Int\n  1\nend\n"),
        ]);
        reset_workspace_source_parses();

        snapshot.navigation_index();

        assert_eq!(workspace_source_parses(), 2);
    }
}
