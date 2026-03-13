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
        mutable: bool,
        type_name: TypeName,
        value: Expr,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Return {
        value: Expr,
        span: Span,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    /// An `if` used as a statement (no else branch required; body is
    /// a list of statements, not required to end with an expression).
    If {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral { value: i64, span: Span },
    BoolLiteral { value: bool, span: Span },
    Identifier { name: String, span: Span },
    BinaryOp { left: Box<Expr>, op: BinOp, right: Box<Expr>, span: Span },
    Call { function: String, args: Vec<Expr>, span: Span },
    If {
        condition: Box<Expr>,
        then_body: Vec<Stmt>,
        then_expr: Box<Expr>,
        else_body: Vec<Stmt>,
        else_expr: Box<Expr>,
        span: Span,
    },
    UnaryOp { op: UnaryOp, operand: Box<Expr>, span: Span },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div, Eq, NotEq, Lt, Gt, LtEq, GtEq, And, Or }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp { Not }
