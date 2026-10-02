use crate::{
    BinaryOp, BodyLine, ContractKind, Expr, ExprKind, FunctionDecl, FunctionKind, HandlerDecl,
    PrefixOp, SchemaDecl, SchemaValidationClause, SyntaxItem, SyntaxTree, TokenKind, TypeDecl,
    TypeVariantDecl, TypeVariantFieldDelimiter, Visibility,
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
use structured_type_text::prepare_structured_type_text;
use type_text::{canonical_predicate_text, canonical_schema_field_type_text};
