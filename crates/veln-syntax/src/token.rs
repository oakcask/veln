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
    Perform,
    Handler,
    Handles,
    Handle,
    Let,
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
        Self::Perform,
        Self::Handler,
        Self::Handles,
        Self::Handle,
        Self::Let,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicToken {
    pub kind: TokenKind,
    pub spelling: &'static str,
}

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
        kind: TokenKind::Perform,
        spelling: "perform",
    },
    PublicToken {
        kind: TokenKind::Handler,
        spelling: "handler",
    },
    PublicToken {
        kind: TokenKind::Handles,
        spelling: "handles",
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
    "perform",
    "handler",
    "handles",
    "handle",
    "let",
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
mod tests {
    use super::{TOKEN_LABELS, TokenKind};

    #[test]
    fn token_labels_cover_all_token_kinds() {
        assert_eq!(TOKEN_LABELS.len(), TokenKind::ALL.len());
        for (index, kind) in TokenKind::ALL.iter().enumerate() {
            assert_eq!(*kind as usize, index);
            assert!(!kind.label().is_empty());
        }
    }
}
