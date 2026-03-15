/// Byte offset in source code.
pub type Span = std::ops::Range<usize>;
#[derive(Debug, Clone, PartialEq)]
pub struct Token { pub kind: TokenKind, pub span: Span }
/// A segment of an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart { Literal(String), Ident(String), Expr(String) }
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    IntLiteral(i64), FloatLiteral(f64), StringLiteral(String), InterpolatedString(Vec<StringPart>), Identifier(String),
    Fn, Let, Plus, Minus, Star, Slash, Percent, Eq,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Colon,
    True, False, If, Else, While, Return, Mut, Struct, Enum, Match,
    Trait, Impl, For, SelfValue, SelfType, Spawn, Channel, Mutex, Array, In, Import, Global, Break, Continue,
    Dot, ColonColon, ColonEq, FatArrow, EqEq, NotEq, Lt, Gt, LtEq, GtEq, And, Or, Bang, Question, Pipe, Eof,
}
