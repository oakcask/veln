use crate::{
    BinaryOp, BodyLine, ContractKind, Expr, ExprKind, FunctionDecl, FunctionKind, HandleBody,
    HandlerDecl, HandlerEntry, PrefixOp, SchemaDecl, SchemaValidationClause, SyntaxItem,
    SyntaxTree, TokenKind, TypeDecl, TypeVariantDecl, TypeVariantFieldDelimiter, Visibility,
};
use veln_literals::parse_integer_literal;

mod cleanup;
mod commented_match_rewrite;

use commented_match_rewrite::tree_has_commented_match_rewrite;
mod declarations;
mod expressions;
mod match_formatting;
mod patterns;
mod source_layout;
mod structured_type_text;
mod type_text;

pub use declarations::format_tree;
pub use type_text::canonical_type_text;

use expressions::{format_defer_statement_with_comments, format_expr_at_indent_with_comments};
use match_formatting::{bool_match_rewrite, literal_match_rewrite};
use patterns::format_pattern;
use source_layout::*;
pub(crate) use structured_type_text::canonical_structured_type_text;
use structured_type_text::prepare_structured_type_text;
use type_text::{canonical_predicate_text, canonical_schema_field_type_text};

#[cfg(test)]
thread_local! {
    static COMMENT_RANGE_QUERIES: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
    static COMMENT_RANGE_KEYS_INSPECTED: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
    static MIXED_HANDLE_CHAIN_CANDIDATES: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
}

#[cfg(test)]
pub(crate) fn reset_comment_range_queries() {
    COMMENT_RANGE_QUERIES.set(0);
    COMMENT_RANGE_KEYS_INSPECTED.set(0);
}

#[cfg(test)]
pub(crate) fn comment_range_queries() -> usize {
    COMMENT_RANGE_QUERIES.get()
}

#[cfg(test)]
pub(crate) fn comment_range_keys_inspected() -> usize {
    COMMENT_RANGE_KEYS_INSPECTED.get()
}

#[cfg(test)]
fn record_comment_range_query() {
    COMMENT_RANGE_QUERIES.set(COMMENT_RANGE_QUERIES.get().saturating_add(1));
}

#[cfg(test)]
fn record_comment_range_key_inspected() {
    COMMENT_RANGE_KEYS_INSPECTED.set(COMMENT_RANGE_KEYS_INSPECTED.get().saturating_add(1));
}

#[cfg(test)]
pub(crate) fn reset_mixed_handle_chain_candidates() {
    MIXED_HANDLE_CHAIN_CANDIDATES.set(0);
}

#[cfg(test)]
pub(crate) fn mixed_handle_chain_candidates() -> usize {
    MIXED_HANDLE_CHAIN_CANDIDATES.get()
}

#[cfg(test)]
fn record_mixed_handle_chain_candidate() {
    MIXED_HANDLE_CHAIN_CANDIDATES.set(MIXED_HANDLE_CHAIN_CANDIDATES.get().saturating_add(1));
}
