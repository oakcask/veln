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
    Pub,
    Fn,
    Type,
    Schema,
    Format,
    Where,
    Test,
    Effects,
    Let,
    End,
    Require,
    Ensure,
    Invariant,
    Mod,
    Use,
    From,
    Match,
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
    Colon,
    Dot,
    DoubleColon,
    Arrow,
    FatArrow,
    PipeGreater,
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
        Self::Pub,
        Self::Fn,
        Self::Type,
        Self::Schema,
        Self::Format,
        Self::Where,
        Self::Test,
        Self::Effects,
        Self::Let,
        Self::End,
        Self::Require,
        Self::Ensure,
        Self::Invariant,
        Self::Mod,
        Self::Use,
        Self::From,
        Self::Match,
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
        Self::Colon,
        Self::Dot,
        Self::DoubleColon,
        Self::Arrow,
        Self::FatArrow,
        Self::PipeGreater,
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
    "pub",
    "fn",
    "type",
    "schema",
    "format",
    "where",
    "test",
    "effects",
    "let",
    "end",
    "require",
    "ensure",
    "invariant",
    "mod",
    "use",
    "from",
    "match",
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
    ":",
    ".",
    "::",
    "->",
    "=>",
    "|>",
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
