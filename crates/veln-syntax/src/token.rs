use veln_source::TextRange;

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    Comment,
    Ident,
    Hole,
    String,
    Int,
    Float,
    Newline,
    Eof,
    Invalid,
    MalformedInt,
    Pub,
    Fn,
    Type,
    Schema,
    Codec,
    For,
    Decode,
    Encode,
    Derive,
    With,
    Format,
    Where,
    Test,
    Effect,
    Effects,
    Callsite,
    Perform,
    Handler,
    Handle,
    Let,
    Defer,
    Begin,
    End,
    Require,
    Ensure,
    Invariant,
    Mod,
    Use,
    From,
    At,
    Match,
    If,
    Else,
    Or,
    And,
    Not,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Dot,
    DoubleColon,
    Arrow,
    FatArrow,
    PipeGreater,
    Pipe,
    Ampersand,
    Caret,
    Tilde,
    ShiftLeft,
    ShiftRight,
    ShiftRightLogical,
    Question,
    Underscore,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
}

impl TokenKind {
    pub fn is_contextual_identifier(self) -> bool {
        matches!(
            self,
            Self::Ident | Self::Callsite | Self::Handle | Self::Handler
        )
    }

    pub fn is_binding_identifier(self) -> bool {
        matches!(self, Self::Ident | Self::Callsite)
    }

    pub fn is_bare_expression_identifier(self) -> bool {
        matches!(self, Self::Ident | Self::Callsite | Self::Handler)
    }

    pub const ALL: &'static [Self] = &[
        Self::Whitespace,
        Self::Comment,
        Self::Ident,
        Self::Hole,
        Self::String,
        Self::Int,
        Self::Float,
        Self::Newline,
        Self::Eof,
        Self::Invalid,
        Self::MalformedInt,
        Self::Pub,
        Self::Fn,
        Self::Type,
        Self::Schema,
        Self::Codec,
        Self::For,
        Self::Decode,
        Self::Encode,
        Self::Derive,
        Self::With,
        Self::Format,
        Self::Where,
        Self::Test,
        Self::Effect,
        Self::Effects,
        Self::Callsite,
        Self::Perform,
        Self::Handler,
        Self::Handle,
        Self::Let,
        Self::Defer,
        Self::Begin,
        Self::End,
        Self::Require,
        Self::Ensure,
        Self::Invariant,
        Self::Mod,
        Self::Use,
        Self::From,
        Self::At,
        Self::Match,
        Self::If,
        Self::Else,
        Self::Or,
        Self::And,
        Self::Not,
        Self::LParen,
        Self::RParen,
        Self::LBracket,
        Self::RBracket,
        Self::LBrace,
        Self::RBrace,
        Self::Comma,
        Self::Semicolon,
        Self::Colon,
        Self::Dot,
        Self::DoubleColon,
        Self::Arrow,
        Self::FatArrow,
        Self::PipeGreater,
        Self::Pipe,
        Self::Ampersand,
        Self::Caret,
        Self::Tilde,
        Self::ShiftLeft,
        Self::ShiftRight,
        Self::ShiftRightLogical,
        Self::Question,
        Self::Underscore,
        Self::Equal,
        Self::EqualEqual,
        Self::BangEqual,
        Self::Less,
        Self::LessEqual,
        Self::Greater,
        Self::GreaterEqual,
        Self::Plus,
        Self::Minus,
        Self::Star,
        Self::Slash,
    ];

    pub fn label(&self) -> &'static str {
        TOKEN_LABELS[*self as usize]
    }
}

/// Maximum structural expression work accepted before an editor-facing
/// operation declines to invoke the recursive parser on mutable source.
pub const PRESENTATION_PARSE_STRUCTURE_LIMIT: usize = 256;

/// Returns whether mutable source is bounded enough for presentation paths to
/// invoke the recursive parser without trusting that delimiters are complete.
pub fn presentation_parse_structure_is_bounded(tokens: &[Token]) -> bool {
    let mut structure = 0usize;
    let mut delimiter_depth = 0usize;
    let mut block_depth = 0usize;
    for token in tokens {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                delimiter_depth += 1;
                structure += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                delimiter_depth = delimiter_depth.saturating_sub(1);
            }
            TokenKind::If | TokenKind::Match | TokenKind::Begin => {
                block_depth += 1;
                structure += 1;
            }
            TokenKind::End => block_depth = block_depth.saturating_sub(1),
            TokenKind::Not
            | TokenKind::Minus
            | TokenKind::Tilde
            | TokenKind::Handle
            | TokenKind::Decode
            | TokenKind::Encode
            | TokenKind::Dot
            | TokenKind::Question
            | TokenKind::PipeGreater
            | TokenKind::Or
            | TokenKind::And
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::Ampersand
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::ShiftRightLogical
            | TokenKind::Plus
            | TokenKind::Star
            | TokenKind::Slash => structure += 1,
            TokenKind::Newline if delimiter_depth == 0 && block_depth == 0 => structure = 0,
            _ => {}
        }
        if structure > PRESENTATION_PARSE_STRUCTURE_LIMIT {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicToken {
    pub kind: TokenKind,
    pub spelling: &'static str,
}

pub const PUBLIC_FIXED_SPELLING_TOKENS: &[PublicToken] = public_fixed_spelling_tokens::TOKENS;

pub const PUBLIC_KEYWORDS: &[PublicToken] = &[
    PublicToken {
        kind: TokenKind::Pub,
        spelling: "pub",
    },
    PublicToken {
        kind: TokenKind::Fn,
        spelling: "fn",
    },
    PublicToken {
        kind: TokenKind::Type,
        spelling: "type",
    },
    PublicToken {
        kind: TokenKind::Schema,
        spelling: "schema",
    },
    PublicToken {
        kind: TokenKind::Codec,
        spelling: "codec",
    },
    PublicToken {
        kind: TokenKind::For,
        spelling: "for",
    },
    PublicToken {
        kind: TokenKind::Decode,
        spelling: "decode",
    },
    PublicToken {
        kind: TokenKind::Encode,
        spelling: "encode",
    },
    PublicToken {
        kind: TokenKind::Derive,
        spelling: "derive",
    },
    PublicToken {
        kind: TokenKind::With,
        spelling: "with",
    },
    PublicToken {
        kind: TokenKind::Format,
        spelling: "format",
    },
    PublicToken {
        kind: TokenKind::Where,
        spelling: "where",
    },
    PublicToken {
        kind: TokenKind::Test,
        spelling: "test",
    },
    PublicToken {
        kind: TokenKind::Effect,
        spelling: "effect",
    },
    PublicToken {
        kind: TokenKind::Effects,
        spelling: "effects",
    },
    PublicToken {
        kind: TokenKind::Callsite,
        spelling: "callsite",
    },
    PublicToken {
        kind: TokenKind::Perform,
        spelling: "perform",
    },
    PublicToken {
        kind: TokenKind::Handler,
        spelling: "handler",
    },
    PublicToken {
        kind: TokenKind::Handle,
        spelling: "handle",
    },
    PublicToken {
        kind: TokenKind::Let,
        spelling: "let",
    },
    PublicToken {
        kind: TokenKind::Defer,
        spelling: "defer",
    },
    PublicToken {
        kind: TokenKind::Begin,
        spelling: "begin",
    },
    PublicToken {
        kind: TokenKind::End,
        spelling: "end",
    },
    PublicToken {
        kind: TokenKind::Require,
        spelling: "require",
    },
    PublicToken {
        kind: TokenKind::Ensure,
        spelling: "ensure",
    },
    PublicToken {
        kind: TokenKind::Invariant,
        spelling: "invariant",
    },
    PublicToken {
        kind: TokenKind::Mod,
        spelling: "mod",
    },
    PublicToken {
        kind: TokenKind::Use,
        spelling: "use",
    },
    PublicToken {
        kind: TokenKind::From,
        spelling: "from",
    },
    PublicToken {
        kind: TokenKind::At,
        spelling: "at",
    },
    PublicToken {
        kind: TokenKind::Match,
        spelling: "match",
    },
    PublicToken {
        kind: TokenKind::If,
        spelling: "if",
    },
    PublicToken {
        kind: TokenKind::Else,
        spelling: "else",
    },
    PublicToken {
        kind: TokenKind::Or,
        spelling: "or",
    },
    PublicToken {
        kind: TokenKind::And,
        spelling: "and",
    },
    PublicToken {
        kind: TokenKind::Not,
        spelling: "not",
    },
];

pub const PUBLIC_PUNCTUATION: &[PublicToken] = &[
    PublicToken {
        kind: TokenKind::LParen,
        spelling: "(",
    },
    PublicToken {
        kind: TokenKind::RParen,
        spelling: ")",
    },
    PublicToken {
        kind: TokenKind::LBracket,
        spelling: "[",
    },
    PublicToken {
        kind: TokenKind::RBracket,
        spelling: "]",
    },
    PublicToken {
        kind: TokenKind::LBrace,
        spelling: "{",
    },
    PublicToken {
        kind: TokenKind::RBrace,
        spelling: "}",
    },
    PublicToken {
        kind: TokenKind::Comma,
        spelling: ",",
    },
    PublicToken {
        kind: TokenKind::Semicolon,
        spelling: ";",
    },
    PublicToken {
        kind: TokenKind::Colon,
        spelling: ":",
    },
    PublicToken {
        kind: TokenKind::Dot,
        spelling: ".",
    },
    PublicToken {
        kind: TokenKind::DoubleColon,
        spelling: "::",
    },
    PublicToken {
        kind: TokenKind::Arrow,
        spelling: "->",
    },
    PublicToken {
        kind: TokenKind::FatArrow,
        spelling: "=>",
    },
    PublicToken {
        kind: TokenKind::PipeGreater,
        spelling: "|>",
    },
    PublicToken {
        kind: TokenKind::Pipe,
        spelling: "|",
    },
    PublicToken {
        kind: TokenKind::Ampersand,
        spelling: "&",
    },
    PublicToken {
        kind: TokenKind::Caret,
        spelling: "^",
    },
    PublicToken {
        kind: TokenKind::Tilde,
        spelling: "~",
    },
    PublicToken {
        kind: TokenKind::ShiftLeft,
        spelling: "<<",
    },
    PublicToken {
        kind: TokenKind::ShiftRight,
        spelling: ">>",
    },
    PublicToken {
        kind: TokenKind::ShiftRightLogical,
        spelling: ">>>",
    },
    PublicToken {
        kind: TokenKind::Question,
        spelling: "?",
    },
    PublicToken {
        kind: TokenKind::Underscore,
        spelling: "_",
    },
    PublicToken {
        kind: TokenKind::Equal,
        spelling: "=",
    },
    PublicToken {
        kind: TokenKind::EqualEqual,
        spelling: "==",
    },
    PublicToken {
        kind: TokenKind::BangEqual,
        spelling: "!=",
    },
    PublicToken {
        kind: TokenKind::Less,
        spelling: "<",
    },
    PublicToken {
        kind: TokenKind::LessEqual,
        spelling: "<=",
    },
    PublicToken {
        kind: TokenKind::Greater,
        spelling: ">",
    },
    PublicToken {
        kind: TokenKind::GreaterEqual,
        spelling: ">=",
    },
    PublicToken {
        kind: TokenKind::Plus,
        spelling: "+",
    },
    PublicToken {
        kind: TokenKind::Minus,
        spelling: "-",
    },
    PublicToken {
        kind: TokenKind::Star,
        spelling: "*",
    },
    PublicToken {
        kind: TokenKind::Slash,
        spelling: "/",
    },
];

mod public_fixed_spelling_tokens {
    use super::{PUBLIC_KEYWORDS, PUBLIC_PUNCTUATION, PublicToken};

    pub const TOKENS: &[PublicToken] = &tokens();

    const fn tokens() -> [PublicToken; PUBLIC_KEYWORDS.len() + PUBLIC_PUNCTUATION.len()] {
        let mut tokens = [PUBLIC_KEYWORDS[0]; PUBLIC_KEYWORDS.len() + PUBLIC_PUNCTUATION.len()];
        let mut index = 0;
        while index < PUBLIC_KEYWORDS.len() {
            tokens[index] = PUBLIC_KEYWORDS[index];
            index += 1;
        }
        let mut punctuation_index = 0;
        while punctuation_index < PUBLIC_PUNCTUATION.len() {
            tokens[index] = PUBLIC_PUNCTUATION[punctuation_index];
            index += 1;
            punctuation_index += 1;
        }
        tokens
    }
}

const TOKEN_LABELS: &[&str] = &[
    "whitespace",
    "comment",
    "identifier",
    "hole",
    "string",
    "integer",
    "float",
    "newline",
    "end of file",
    "invalid token",
    "malformed integer",
    "pub",
    "fn",
    "type",
    "schema",
    "codec",
    "for",
    "decode",
    "encode",
    "derive",
    "with",
    "format",
    "where",
    "test",
    "effect",
    "effects",
    "callsite",
    "perform",
    "handler",
    "handle",
    "let",
    "defer",
    "begin",
    "end",
    "require",
    "ensure",
    "invariant",
    "mod",
    "use",
    "from",
    "at",
    "match",
    "if",
    "else",
    "or",
    "and",
    "not",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    ";",
    ":",
    ".",
    "::",
    "->",
    "=>",
    "|>",
    "|",
    "&",
    "^",
    "~",
    "<<",
    ">>",
    ">>>",
    "?",
    "_",
    "=",
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "+",
    "-",
    "*",
    "/",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub range: TextRange,
}

impl Token {
    pub(crate) fn eof(offset: usize) -> Self {
        Self {
            kind: TokenKind::Eof,
            text: String::new(),
            range: TextRange::at(offset),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
}

impl TokenKind {
    pub(crate) fn is_trivia(&self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

#[cfg(test)]
#[path = "token/tests.rs"]
mod tests;
