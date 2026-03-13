pub mod ast;

use cyflym_lexer::token::{Span, Token, TokenKind};
use ast::*;

/// An error produced during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        ParseError { message: message.into(), span }
    }
}

/// Parse the given source string into a `Program`.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = cyflym_lexer::lex(source).map_err(|e| ParseError {
        message: e.message,
        span: e.span,
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    /// Peek at the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Expect the current token to match `kind`, consume it, and return it.
    fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(kind) {
            self.pos += 1;
            Ok(tok)
        } else {
            Err(ParseError::new(
                format!("expected {:?}, got {:?}", kind, tok.kind),
                tok.span,
            ))
        }
    }

    /// Expect an `Identifier` token and return its text.
    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let tok = self.peek().clone();
        if let TokenKind::Identifier(name) = &tok.kind {
            let name = name.clone();
            let span = tok.span.clone();
            self.pos += 1;
            Ok((name, span))
        } else {
            Err(ParseError::new(
                format!("expected identifier, got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    // ─── Top-level ────────────────────────────────────────────────────────────

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    // ─── Function ─────────────────────────────────────────────────────────────

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let fn_tok = self.expect(&TokenKind::Fn)?;
        let fn_start = fn_tok.span.start;

        let (name, _) = self.expect_ident()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = self.parse_type_name()?;

        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_body()?;
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let fn_end = rbrace.span.end;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span: fn_start..fn_end,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        // Empty param list
        if self.peek().kind == TokenKind::RParen {
            return Ok(params);
        }
        loop {
            let (name, name_span) = self.expect_ident()?;
            let type_name = self.parse_type_name()?;
            let end = type_name.span.end;
            params.push(Param {
                name,
                type_name,
                span: name_span.start..end,
            });
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1; // consume comma
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        let (name, span) = self.expect_ident()?;
        Ok(TypeName { name, span })
    }

    // ─── Body / Statements ────────────────────────────────────────────────────

    fn parse_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.peek().kind == TokenKind::Let {
            self.parse_let()
        } else if self.peek().kind == TokenKind::While {
            self.parse_while()
        } else if self.peek().kind == TokenKind::Return {
            self.parse_return()
        } else if self.peek().kind == TokenKind::If {
            self.parse_if_or_if_else()
        } else {
            // Could be an expression OR an assignment (name = expr)
            let expr = self.parse_expr(0)?;
            // Check if this is an assignment: identifier followed by =
            if self.peek().kind == TokenKind::Eq {
                if let Expr::Identifier { name, span: id_span } = expr {
                    self.pos += 1; // consume =
                    let value = self.parse_expr(0)?;
                    let end = expr_span(&value).end;
                    return Ok(Stmt::Assign {
                        name,
                        value,
                        span: id_span.start..end,
                    });
                } else {
                    return Err(ParseError::new(
                        "left side of assignment must be a variable name",
                        expr_span(&expr).clone(),
                    ));
                }
            }
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, ParseError> {
        let let_tok = self.expect(&TokenKind::Let)?;
        let let_start = let_tok.span.start;

        // Check for `mut` keyword
        let mutable = if self.peek().kind == TokenKind::Mut {
            self.pos += 1;
            true
        } else {
            false
        };

        let (name, _) = self.expect_ident()?;
        let type_name = if self.peek().kind != TokenKind::Eq {
            Some(self.parse_type_name()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr(0)?;
        let end = expr_span(&value).end;

        Ok(Stmt::Let {
            name,
            mutable,
            type_name,
            value,
            span: let_start..end,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        let while_tok = self.expect(&TokenKind::While)?;
        let start = while_tok.span.start;

        let condition = self.parse_expr(0)?;

        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_body()?;
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end;

        Ok(Stmt::While {
            condition,
            body,
            span: start..end,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let ret_tok = self.expect(&TokenKind::Return)?;
        let start = ret_tok.span.start;
        let value = self.parse_expr(0)?;
        let end = expr_span(&value).end;

        Ok(Stmt::Return {
            value,
            span: start..end,
        })
    }

    /// Parse `if condition { ... }` — if followed by `else`, returns Expr::If
    /// wrapped in Stmt::Expr; otherwise returns Stmt::If (no else branch).
    fn parse_if_or_if_else(&mut self) -> Result<Stmt, ParseError> {
        let if_tok = self.expect(&TokenKind::If)?;
        let start = if_tok.span.start;

        let condition = self.parse_expr(0)?;

        self.expect(&TokenKind::LBrace)?;

        // Check if this is an if/else expression or an if statement
        if self.peek_has_else_after_block() {
            // if/else expression: parse body as block_body (stmts + final expr)
            let (then_body, then_expr) = self.parse_block_body()?;
            self.expect(&TokenKind::RBrace)?;

            self.expect(&TokenKind::Else)?;
            self.expect(&TokenKind::LBrace)?;
            let (else_body, else_expr) = self.parse_block_body()?;
            let rbrace = self.expect(&TokenKind::RBrace)?;
            let end = rbrace.span.end;

            Ok(Stmt::Expr(Expr::If {
                condition: Box::new(condition),
                then_body,
                then_expr: Box::new(then_expr),
                else_body,
                else_expr: Box::new(else_expr),
                span: start..end,
            }))
        } else {
            // if statement (no else): parse body as statements
            let body = self.parse_body()?;
            let rbrace = self.expect(&TokenKind::RBrace)?;
            let end = rbrace.span.end;

            Ok(Stmt::If {
                condition,
                body,
                span: start..end,
            })
        }
    }

    /// Look ahead to determine if after the current block `{ ... }` there is
    /// an `else` keyword. This doesn't consume any tokens.
    fn peek_has_else_after_block(&self) -> bool {
        // Scan forward to find the matching `}`, then check if `else` follows.
        let mut depth = 1; // we've already consumed the opening `{`
        let mut pos = self.pos;
        while pos < self.tokens.len() {
            match &self.tokens[pos].kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        // Check the token after the matching `}`
                        let next = pos + 1;
                        return next < self.tokens.len()
                            && self.tokens[next].kind == TokenKind::Else;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            pos += 1;
        }
        false
    }

    // ─── Expressions (Pratt / precedence climbing) ────────────────────────────

    /// Parse an expression with the given minimum binding power.
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        // Parse left-hand side (prefix / atom)
        let mut lhs = self.parse_atom()?;

        loop {
            let tok = self.peek().clone();
            let Some((left_bp, right_bp, op)) = infix_binding_power(&tok.kind) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.pos += 1; // consume operator
            let rhs = self.parse_expr(right_bp)?;
            let span_start = expr_span(&lhs).start;
            let span_end = expr_span(&rhs).end;
            lhs = Expr::BinaryOp {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
                span: span_start..span_end,
            };
        }

        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::IntLiteral(v) => {
                let value = *v;
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::IntLiteral { value, span })
            }
            TokenKind::Identifier(_) => {
                let (name, name_span) = self.expect_ident()?;
                // Check for function call: identifier followed by `(`
                if self.peek().kind == TokenKind::LParen {
                    self.pos += 1; // consume `(`
                    let args = self.parse_call_args()?;
                    let rparen = self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Call {
                        function: name,
                        args,
                        span: name_span.start..rparen.span.end,
                    })
                } else {
                    Ok(Expr::Identifier { name, span: name_span })
                }
            }
            TokenKind::True => {
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::BoolLiteral { value: true, span })
            }
            TokenKind::False => {
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::BoolLiteral { value: false, span })
            }
            TokenKind::If => {
                self.parse_if_expr()
            }
            TokenKind::Bang => {
                let start = tok.span.start;
                self.pos += 1; // consume !
                let operand = self.parse_expr(13)?; // prefix binding power 13 (tightest)
                let end = expr_span(&operand).end;
                Ok(Expr::UnaryOp {
                    op: ast::UnaryOp::Not,
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            TokenKind::StringLiteral(s) => {
                let value = s.clone();
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::StringLiteral { value, span })
            }
            TokenKind::LParen => {
                self.pos += 1;
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(ParseError::new(
                format!("unexpected token in expression: {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let if_tok = self.expect(&TokenKind::If)?;
        let start = if_tok.span.start;

        // Parse condition (no braces around it)
        let condition = self.parse_expr(0)?;

        // Parse then branch: { body... expr }
        self.expect(&TokenKind::LBrace)?;
        let (then_body, then_expr) = self.parse_block_body()?;
        self.expect(&TokenKind::RBrace)?;

        // Parse else branch (required)
        self.expect(&TokenKind::Else)?;
        self.expect(&TokenKind::LBrace)?;
        let (else_body, else_expr) = self.parse_block_body()?;
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end;

        Ok(Expr::If {
            condition: Box::new(condition),
            then_body,
            then_expr: Box::new(then_expr),
            else_body,
            else_expr: Box::new(else_expr),
            span: start..end,
        })
    }

    /// Parse the inside of a `{ ... }` block: zero or more statements followed
    /// by a final expression.
    fn parse_block_body(&mut self) -> Result<(Vec<Stmt>, Expr), ParseError> {
        let mut stmts = Vec::new();
        loop {
            if self.peek().kind == TokenKind::RBrace || self.peek().kind == TokenKind::Eof {
                // No more tokens — error: empty block
                return Err(ParseError::new(
                    "expected expression in block",
                    self.peek().span.clone(),
                ));
            }
            let stmt = self.parse_stmt()?;
            // If next token is `}`, this stmt should be the final expression
            if self.peek().kind == TokenKind::RBrace {
                match stmt {
                    Stmt::Expr(expr) => return Ok((stmts, expr)),
                    Stmt::Let { span, .. }
                    | Stmt::While { span, .. }
                    | Stmt::Return { span, .. }
                    | Stmt::Assign { span, .. }
                    | Stmt::If { span, .. } => {
                        return Err(ParseError::new(
                            "block must end with an expression, not a statement",
                            span,
                        ));
                    }
                }
            }
            stmts.push(stmt);
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.peek().kind == TokenKind::RParen {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr(0)?);
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(args)
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns `(left_bp, right_bp, op)` for infix operators, or `None`.
fn infix_binding_power(kind: &TokenKind) -> Option<(u8, u8, BinOp)> {
    match kind {
        TokenKind::Or     => Some((1, 2, BinOp::Or)),
        TokenKind::And    => Some((3, 4, BinOp::And)),
        TokenKind::EqEq   => Some((5, 6, BinOp::Eq)),
        TokenKind::NotEq  => Some((5, 6, BinOp::NotEq)),
        TokenKind::Lt     => Some((7, 8, BinOp::Lt)),
        TokenKind::Gt     => Some((7, 8, BinOp::Gt)),
        TokenKind::LtEq   => Some((7, 8, BinOp::LtEq)),
        TokenKind::GtEq   => Some((7, 8, BinOp::GtEq)),
        TokenKind::Plus   => Some((9, 10, BinOp::Add)),
        TokenKind::Minus  => Some((9, 10, BinOp::Sub)),
        TokenKind::Star   => Some((11, 12, BinOp::Mul)),
        TokenKind::Slash  => Some((11, 12, BinOp::Div)),
        _ => None,
    }
}

fn expr_span(expr: &Expr) -> &Span {
    match expr {
        Expr::IntLiteral { span, .. } => span,
        Expr::BoolLiteral { span, .. } => span,
        Expr::StringLiteral { span, .. } => span,
        Expr::Identifier { span, .. } => span,
        Expr::BinaryOp { span, .. } => span,
        Expr::Call { span, .. } => span,
        Expr::If { span, .. } => span,
        Expr::UnaryOp { span, .. } => span,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_function() {
        let prog = parse("fn main() Int { 42 }").unwrap();
        assert_eq!(prog.functions.len(), 1);
        let func = &prog.functions[0];
        assert_eq!(func.name, "main");
        assert_eq!(func.params.len(), 0);
        assert_eq!(func.return_type.name, "Int");
        assert_eq!(func.body.len(), 1);
        matches!(&func.body[0], Stmt::Expr(Expr::IntLiteral { value: 42, .. }));
        if let Stmt::Expr(Expr::IntLiteral { value, .. }) = &func.body[0] {
            assert_eq!(*value, 42);
        } else {
            panic!("expected IntLiteral(42)");
        }
    }

    #[test]
    fn parse_let_binding() {
        let prog = parse("fn main() Int { let x Int = 42 x }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Stmt::Let { name, .. } if name == "x"));
        assert!(matches!(&body[1], Stmt::Expr(Expr::Identifier { name, .. }) if name == "x"));
    }

    #[test]
    fn parse_binary_expression() {
        let prog = parse("fn main() Int { 1 + 2 }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 1);
        if let Stmt::Expr(Expr::BinaryOp { op, left, right, .. }) = &body[0] {
            assert_eq!(*op, BinOp::Add);
            assert!(matches!(left.as_ref(), Expr::IntLiteral { value: 1, .. }));
            assert!(matches!(right.as_ref(), Expr::IntLiteral { value: 2, .. }));
        } else {
            panic!("expected BinaryOp");
        }
    }

    #[test]
    fn parse_operator_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let prog = parse("fn main() Int { 1 + 2 * 3 }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Expr(Expr::BinaryOp { op, left, right, .. }) = &body[0] {
            assert_eq!(*op, BinOp::Add);
            assert!(matches!(left.as_ref(), Expr::IntLiteral { value: 1, .. }));
            if let Expr::BinaryOp { op: inner_op, left: il, right: ir, .. } = right.as_ref() {
                assert_eq!(*inner_op, BinOp::Mul);
                assert!(matches!(il.as_ref(), Expr::IntLiteral { value: 2, .. }));
                assert!(matches!(ir.as_ref(), Expr::IntLiteral { value: 3, .. }));
            } else {
                panic!("expected inner BinaryOp(Mul)");
            }
        } else {
            panic!("expected outer BinaryOp(Add)");
        }
    }

    #[test]
    fn parse_function_call() {
        let prog = parse("fn main() Int { add(1, 2) }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 1);
        if let Stmt::Expr(Expr::Call { function, args, .. }) = &body[0] {
            assert_eq!(function, "add");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::IntLiteral { value: 1, .. }));
            assert!(matches!(&args[1], Expr::IntLiteral { value: 2, .. }));
        } else {
            panic!("expected Call");
        }
    }

    #[test]
    fn parse_function_with_params() {
        let prog = parse("fn add(a Int, b Int) Int { a + b }").unwrap();
        let func = &prog.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "a");
        assert_eq!(func.params[0].type_name.name, "Int");
        assert_eq!(func.params[1].name, "b");
        assert_eq!(func.params[1].type_name.name, "Int");
    }

    #[test]
    fn parse_multiple_functions() {
        let src = "fn foo() Int { 1 } fn bar() Int { 2 }";
        let prog = parse(src).unwrap();
        assert_eq!(prog.functions.len(), 2);
        assert_eq!(prog.functions[0].name, "foo");
        assert_eq!(prog.functions[1].name, "bar");
    }

    #[test]
    fn parse_bool_literal() {
        let prog = parse("fn main() Bool { true }").unwrap();
        if let Stmt::Expr(Expr::BoolLiteral { value, .. }) = &prog.functions[0].body[0] {
            assert_eq!(*value, true);
        } else {
            panic!("expected BoolLiteral(true)");
        }
    }

    #[test]
    fn parse_comparison() {
        let prog = parse("fn main() Bool { 1 == 2 }").unwrap();
        if let Stmt::Expr(Expr::BinaryOp { op, .. }) = &prog.functions[0].body[0] {
            assert_eq!(*op, BinOp::Eq);
        } else {
            panic!("expected BinaryOp(Eq)");
        }
    }

    #[test]
    fn parse_if_else_expr() {
        let prog = parse("fn main() Int { if true { 1 } else { 2 } }").unwrap();
        if let Stmt::Expr(Expr::If { then_expr, else_expr, .. }) = &prog.functions[0].body[0] {
            assert!(matches!(then_expr.as_ref(), Expr::IntLiteral { value: 1, .. }));
            assert!(matches!(else_expr.as_ref(), Expr::IntLiteral { value: 2, .. }));
        } else {
            panic!("expected If expression");
        }
    }

    #[test]
    fn parse_unary_not() {
        let prog = parse("fn main() Bool { !true }").unwrap();
        if let Stmt::Expr(Expr::UnaryOp { op, operand, .. }) = &prog.functions[0].body[0] {
            assert_eq!(*op, ast::UnaryOp::Not);
            assert!(matches!(operand.as_ref(), Expr::BoolLiteral { value: true, .. }));
        } else {
            panic!("expected UnaryOp(Not)");
        }
    }

    #[test]
    fn parse_precedence_comparison_vs_arithmetic() {
        // 1 + 2 == 3 should parse as (1 + 2) == 3
        let prog = parse("fn main() Bool { 1 + 2 == 3 }").unwrap();
        if let Stmt::Expr(Expr::BinaryOp { op, left, .. }) = &prog.functions[0].body[0] {
            assert_eq!(*op, BinOp::Eq);
            assert!(matches!(left.as_ref(), Expr::BinaryOp { op: BinOp::Add, .. }));
        } else {
            panic!("expected BinaryOp(Eq) at top level");
        }
    }

    #[test]
    fn parse_boolean_operators() {
        // true && false || true should parse as (true && false) || true
        let prog = parse("fn main() Bool { true && false || true }").unwrap();
        if let Stmt::Expr(Expr::BinaryOp { op, left, .. }) = &prog.functions[0].body[0] {
            assert_eq!(*op, BinOp::Or);
            assert!(matches!(left.as_ref(), Expr::BinaryOp { op: BinOp::And, .. }));
        } else {
            panic!("expected BinaryOp(Or) at top level");
        }
    }

    #[test]
    fn parse_if_else_with_let() {
        let prog = parse("fn main() Int { if true { let x Int = 1 x } else { 2 } }").unwrap();
        if let Stmt::Expr(Expr::If { then_body, then_expr, .. }) = &prog.functions[0].body[0] {
            assert_eq!(then_body.len(), 1); // let x = 1
            assert!(matches!(then_expr.as_ref(), Expr::Identifier { name, .. } if name == "x"));
        } else {
            panic!("expected If expression");
        }
    }

    #[test]
    fn parse_while_loop() {
        let prog = parse("fn main() Int { while true { 1 } 0 }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Stmt::While { .. }));
    }

    #[test]
    fn parse_return_stmt() {
        let prog = parse("fn main() Int { return 42 }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 1);
        if let Stmt::Return { value, .. } = &body[0] {
            assert!(matches!(value, Expr::IntLiteral { value: 42, .. }));
        } else {
            panic!("expected Return statement");
        }
    }

    #[test]
    fn parse_mutable_let() {
        let prog = parse("fn main() Int { let mut x Int = 0 x }").unwrap();
        if let Stmt::Let { name, mutable, .. } = &prog.functions[0].body[0] {
            assert_eq!(name, "x");
            assert!(*mutable);
        } else {
            panic!("expected mutable Let");
        }
    }

    #[test]
    fn parse_assignment() {
        let prog = parse("fn main() Int { let mut x Int = 0 x = 42 x }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 3);
        if let Stmt::Assign { name, .. } = &body[1] {
            assert_eq!(name, "x");
        } else {
            panic!("expected Assign statement");
        }
    }

    #[test]
    fn parse_let_inferred_type() {
        let prog = parse("fn main() Int { let x = 42 x }").unwrap();
        if let Stmt::Let { name, type_name, .. } = &prog.functions[0].body[0] {
            assert_eq!(name, "x");
            assert!(type_name.is_none());
        } else {
            panic!("expected Let");
        }
    }

    #[test]
    fn parse_let_mut_inferred_type() {
        let prog = parse("fn main() Int { let mut x = 0 x }").unwrap();
        if let Stmt::Let { name, mutable, type_name, .. } = &prog.functions[0].body[0] {
            assert_eq!(name, "x");
            assert!(*mutable);
            assert!(type_name.is_none());
        } else {
            panic!("expected mutable Let");
        }
    }

    #[test]
    fn parse_string_literal() {
        let prog = parse(r#"fn main() Int { print("hello") }"#).unwrap();
        if let Stmt::Expr(Expr::Call { function, args, .. }) = &prog.functions[0].body[0] {
            assert_eq!(function, "print");
            assert!(matches!(&args[0], Expr::StringLiteral { value, .. } if value == "hello"));
        } else {
            panic!("expected print call");
        }
    }

    #[test]
    fn parse_immutable_let() {
        let prog = parse("fn main() Int { let x Int = 42 x }").unwrap();
        if let Stmt::Let { mutable, .. } = &prog.functions[0].body[0] {
            assert!(!*mutable);
        } else {
            panic!("expected Let");
        }
    }
}
