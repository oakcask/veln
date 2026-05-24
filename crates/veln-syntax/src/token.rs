use veln_source::TextRange;

#[derive(Clone, Debug, PartialEq, Eq)]
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
    Test,
    Effects,
    Let,
    End,
    Require,
    Ensure,
    Mod,
    Use,
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
    pub fn label(&self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Comment => "comment",
            Self::Ident => "identifier",
            Self::Hole => "hole",
            Self::String => "string",
            Self::Int => "integer",
            Self::Float => "float",
            Self::Newline => "newline",
            Self::Eof => "end of file",
            Self::Invalid => "invalid token",
            Self::Pub => "pub",
            Self::Fn => "fn",
            Self::Test => "test",
            Self::Effects => "effects",
            Self::Let => "let",
            Self::End => "end",
            Self::Require => "require",
            Self::Ensure => "ensure",
            Self::Mod => "mod",
            Self::Use => "use",
            Self::Match => "match",
            Self::Or => "or",
            Self::And => "and",
            Self::Not => "not",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::Comma => ",",
            Self::Colon => ":",
            Self::Dot => ".",
            Self::DoubleColon => "::",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::PipeGreater => "|>",
            Self::Question => "?",
            Self::Underscore => "_",
            Self::Equal => "=",
            Self::EqualEqual => "==",
            Self::BangEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
        }
    }
}

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
