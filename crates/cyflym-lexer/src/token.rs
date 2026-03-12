/// Byte offset in source code.
pub type Span = std::ops::Range<usize>;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    Identifier(String),

    // Keywords
    Fn,
    Let,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,        // =

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,

    // Punctuation
    Comma,
    Colon,

    // Special
    Eof,
}
