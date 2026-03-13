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
    StringLiteral(String),
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

    // Boolean literals
    True,
    False,

    // Control flow
    If,
    Else,
    While,
    Return,
    Mut,

    // Data types
    Struct,
    Enum,
    Match,

    // Trait / impl
    Trait,
    Impl,
    For,
    SelfValue, // `self` (value, lowercase)
    SelfType,  // `Self` (type, uppercase)

    // Member access
    Dot,

    // Path / match tokens
    ColonColon, // ::
    FatArrow,   // =>

    // Comparison operators
    EqEq,    // ==
    NotEq,   // !=
    Lt,      // <
    Gt,      // >
    LtEq,    // <=
    GtEq,    // >=

    // Boolean operators
    And,     // &&
    Or,      // ||
    Bang,    // !

    // Special
    Eof,
}
