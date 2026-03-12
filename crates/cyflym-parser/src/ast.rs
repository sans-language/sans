use cyflym_lexer::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeName,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_name: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeName {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        type_name: TypeName,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral { value: i64, span: Span },
    Identifier { name: String, span: Span },
    BinaryOp { left: Box<Expr>, op: BinOp, right: Box<Expr>, span: Span },
    Call { function: String, args: Vec<Expr>, span: Span },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div }
