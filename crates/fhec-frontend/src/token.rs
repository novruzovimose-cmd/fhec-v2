#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Fn,
    Let,
    If,
    Else,
    Return,

    Encrypted,
    Plaintext,
    Bool,
    True,
    False,

    UInt,
    Int,

    Identifier,
    Integer,

    Plus,
    Minus,
    Star,
    Slash,

    EqualEqual,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    Arrow,

    Equal,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Semicolon,

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub position: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, position: usize) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            position,
        }
    }
}
