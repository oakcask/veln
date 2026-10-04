//! Public lexical facts shared by the lexer, parser, and editor asset projection.

use crate::TokenKind;

pub const LINE_COMMENT_START: char = '#';
pub const STRING_DELIMITER: char = '"';
pub const STRING_ESCAPE: char = '\\';
pub const TRUE_LITERAL: &str = "true";
pub const FALSE_LITERAL: &str = "false";
pub const SATISFY_MARKER: &str = "satisfy";
pub const VALIDATE_MARKER: &str = "validate";

pub const DELIMITER_PAIRS: &[(TokenKind, TokenKind)] = &[
    (TokenKind::LParen, TokenKind::RParen),
    (TokenKind::LBracket, TokenKind::RBracket),
    (TokenKind::LBrace, TokenKind::RBrace),
];
