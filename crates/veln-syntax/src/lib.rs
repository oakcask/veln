//! Lexer, parser, lossless tree, and formatting input.

mod ast;
mod documentation;
mod format;
mod lexer;
mod parser;
mod token;
mod tree;

pub use ast::*;
pub use documentation::*;
pub use format::{canonical_type_text, format_tree};
pub use lexer::lex;
pub use parser::*;
pub use token::*;
pub use tree::*;

/// Maximum number of generic argument boundaries that may contain a variant
/// refinement. This bounds recursive lowering and wire processing.
pub const MAX_VARIANT_REFINEMENT_NESTING: usize = 256;

#[cfg(test)]
mod tests;
