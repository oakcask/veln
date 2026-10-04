//! Editor-facing classification for Veln source.

mod semantic_tokens;

pub use semantic_tokens::{
    LspSemanticToken, SemanticToken, SemanticTokenKind, SemanticTokenModifier,
    SemanticTokenModifiers, SemanticTokenType, collect_semantic_tokens,
    custom_semantic_token_modifiers, custom_semantic_token_types, encode_lsp_semantic_tokens,
    semantic_token_legend,
};
