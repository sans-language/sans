pub mod ast;

use sans_lexer::token::{Span, Token, TokenKind};
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
    let tokens = sans_lexer::lex(source).map_err(|e| ParseError {
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
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut impls = Vec::new();
        let mut has_declarations = false;

        while self.peek().kind != TokenKind::Eof {
            if self.peek().kind == TokenKind::Import {
                if has_declarations {
                    return Err(ParseError::new(
                        "imports must appear before all declarations",
                        self.peek().span.clone(),
                    ));
                }
                imports.push(self.parse_import()?);
            } else {
                has_declarations = true;
                if self.peek().kind == TokenKind::Enum {
                    enums.push(self.parse_enum_def()?);
                } else if self.peek().kind == TokenKind::Struct {
                    structs.push(self.parse_struct_def()?);
                } else if self.peek().kind == TokenKind::Trait {
                    traits.push(self.parse_trait_def()?);
                } else if self.peek().kind == TokenKind::Impl {
                    impls.push(self.parse_impl_block()?);
                } else {
                    functions.push(self.parse_function()?);
                }
            }
        }
        Ok(Program { imports, functions, structs, enums, traits, impls })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        let import_tok = self.expect(&TokenKind::Import)?;
        let start = import_tok.span.start;

        let path_tok = self.peek().clone();
        let path = if let TokenKind::StringLiteral(s) = &path_tok.kind {
            let s = s.clone();
            self.pos += 1;
            s
        } else {
            return Err(ParseError::new(
                format!("expected string literal after 'import', got {:?}", path_tok.kind),
                path_tok.span,
            ));
        };

        let module_name = path.rsplit('/').next().unwrap_or(&path).to_string();

        if module_name.is_empty() {
            return Err(ParseError::new(
                "import path must not be empty".to_string(),
                path_tok.span.clone(),
            ));
        }

        let end = path_tok.span.end;
        Ok(Import {
            path,
            module_name,
            span: start..end,
        })
    }

    // ─── Function ─────────────────────────────────────────────────────────────

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, ParseError> {
        let mut type_params = Vec::new();
        if self.peek().kind != TokenKind::Lt {
            return Ok(type_params);
        }
        self.pos += 1; // consume <
        loop {
            if self.peek().kind == TokenKind::Gt {
                break;
            }
            let (name, name_span) = self.expect_ident()?;
            let bound = if self.peek().kind == TokenKind::Colon {
                self.pos += 1; // consume :
                let (bound_name, _) = self.expect_ident()?;
                Some(bound_name)
            } else {
                None
            };
            let end = if bound.is_some() {
                self.tokens[self.pos - 1].span.end
            } else {
                name_span.end
            };
            type_params.push(TypeParam { name, bound, span: name_span.start..end });
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.expect(&TokenKind::Gt)?;
        Ok(type_params)
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let fn_tok = self.expect(&TokenKind::Fn)?;
        let fn_start = fn_tok.span.start;

        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_params()?;

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
            type_params,
            params,
            return_type,
            body,
            span: fn_start..fn_end,
        })
    }

    fn parse_struct_def(&mut self) -> Result<StructDef, ParseError> {
        let struct_tok = self.expect(&TokenKind::Struct)?;
        let start = struct_tok.span.start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            let (field_name, field_span) = self.expect_ident()?;
            let type_name = self.parse_type_name()?;
            let end = type_name.span.end;
            fields.push(StructField {
                name: field_name,
                type_name,
                span: field_span.start..end,
            });
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1;
            }
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        Ok(StructDef { name, fields, span: start..rbrace.span.end })
    }

    fn parse_enum_def(&mut self) -> Result<EnumDef, ParseError> {
        let enum_tok = self.expect(&TokenKind::Enum)?;
        let start = enum_tok.span.start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            let (variant_name, variant_span) = self.expect_ident()?;
            let mut fields = Vec::new();
            let end;
            if self.peek().kind == TokenKind::LParen {
                self.pos += 1; // consume (
                while self.peek().kind != TokenKind::RParen {
                    fields.push(self.parse_type_name()?);
                    if self.peek().kind == TokenKind::Comma {
                        self.pos += 1;
                    }
                }
                let rparen = self.expect(&TokenKind::RParen)?;
                end = rparen.span.end;
            } else {
                end = variant_span.end;
            }
            variants.push(EnumVariant { name: variant_name, fields, span: variant_span.start..end });
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1;
            }
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        Ok(EnumDef { name, variants, span: start..rbrace.span.end })
    }

    fn parse_trait_def(&mut self) -> Result<TraitDef, ParseError> {
        let trait_tok = self.expect(&TokenKind::Trait)?;
        let start = trait_tok.span.start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            let method = self.parse_trait_method_sig()?;
            methods.push(method);
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        Ok(TraitDef { name, methods, span: start..rbrace.span.end })
    }

    fn parse_trait_method_sig(&mut self) -> Result<TraitMethodSig, ParseError> {
        let fn_tok = self.expect(&TokenKind::Fn)?;
        let start = fn_tok.span.start;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_params()?;
        self.expect(&TokenKind::LParen)?;
        // First param must be `self`
        self.expect(&TokenKind::SelfValue)?;
        // Parse remaining params (each preceded by comma)
        let mut params = Vec::new();
        while self.peek().kind == TokenKind::Comma {
            self.pos += 1; // consume comma
            if self.peek().kind == TokenKind::RParen {
                break;
            }
            let (param_name, param_span) = self.expect_ident()?;
            let type_name = self.parse_type_name()?;
            let end = type_name.span.end;
            params.push(Param { name: param_name, type_name, span: param_span.start..end });
        }
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_type_name()?;
        let end = return_type.span.end;
        Ok(TraitMethodSig { name, type_params, params, return_type, span: start..end })
    }

    fn parse_impl_block(&mut self) -> Result<ImplBlock, ParseError> {
        let impl_tok = self.expect(&TokenKind::Impl)?;
        let start = impl_tok.span.start;
        let (first_name, _) = self.expect_ident()?;

        let (trait_name, target_type) = if self.peek().kind == TokenKind::For {
            self.pos += 1; // consume `for`
            let (target, _) = self.expect_ident()?;
            (Some(first_name), target)
        } else {
            (None, first_name)
        };

        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            methods.push(self.parse_method(&target_type)?);
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        Ok(ImplBlock { trait_name, target_type, methods, span: start..rbrace.span.end })
    }

    fn parse_method(&mut self, target_type: &str) -> Result<Function, ParseError> {
        let fn_tok = self.expect(&TokenKind::Fn)?;
        let fn_start = fn_tok.span.start;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_params()?;
        self.expect(&TokenKind::LParen)?;

        // First param must be `self` — we translate it to a typed param
        self.expect(&TokenKind::SelfValue)?;
        let self_span = self.tokens[self.pos - 1].span.clone();
        let mut params = vec![Param {
            name: "self".to_string(),
            type_name: TypeName { name: target_type.to_string(), span: self_span.clone() },
            span: self_span,
        }];

        // Parse remaining params
        while self.peek().kind == TokenKind::Comma {
            self.pos += 1;
            if self.peek().kind == TokenKind::RParen {
                break;
            }
            let (param_name, param_span) = self.expect_ident()?;
            let type_name = self.parse_type_name()?;
            let end = type_name.span.end;
            params.push(Param { name: param_name, type_name, span: param_span.start..end });
        }
        self.expect(&TokenKind::RParen)?;

        let return_type = self.parse_type_name()?;
        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_body()?;
        let rbrace = self.expect(&TokenKind::RBrace)?;

        Ok(Function {
            name,
            type_params,
            params,
            return_type,
            body,
            span: fn_start..rbrace.span.end,
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
        // Handle parameterized types like Result<Int>, Result<String>
        if self.peek().kind == TokenKind::Lt {
            self.pos += 1; // consume <
            let inner = self.parse_type_name()?;
            self.expect(&TokenKind::Gt)?;
            let full_name = format!("{}<{}>", name, inner.name);
            Ok(TypeName { name: full_name, span })
        } else {
            Ok(TypeName { name, span })
        }
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
        } else if self.peek().kind == TokenKind::For {
            let start = self.peek().span.start;
            self.pos += 1;
            let var = if let TokenKind::Identifier(name) = &self.peek().kind {
                let name = name.clone();
                self.pos += 1;
                name
            } else {
                return Err(ParseError::new(
                    "expected variable name after 'for'",
                    self.peek().span.clone(),
                ));
            };
            self.expect(&TokenKind::In)?;
            let iterable = self.parse_expr(0)?;
            self.expect(&TokenKind::LBrace)?;
            let body = self.parse_body()?;
            self.expect(&TokenKind::RBrace)?;
            let span = start..self.tokens[self.pos - 1].span.end;
            Ok(Stmt::ForIn { var, iterable, body, span })
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

        // Check for destructuring: let (a, b) = expr
        if self.peek().kind == TokenKind::LParen {
            self.pos += 1; // consume '('
            let (name1, _) = self.expect_ident()?;
            self.expect(&TokenKind::Comma)?;
            let (name2, _) = self.expect_ident()?;
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::Eq)?;
            let value = self.parse_expr(0)?;
            let span = let_start..expr_span(&value).end;
            return Ok(Stmt::LetDestructure { names: vec![name1, name2], value, span });
        }

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
            // Check for field access / method call first (highest precedence)
            if self.peek().kind == TokenKind::Dot {
                self.pos += 1; // consume .
                let (field_or_method, ident_span) = self.expect_ident()?;
                let start = expr_span(&lhs).start;

                // Check if this is a method call: identifier followed by `(`
                if self.peek().kind == TokenKind::LParen {
                    self.pos += 1; // consume (
                    let args = self.parse_call_args()?;
                    let rparen = self.expect(&TokenKind::RParen)?;
                    lhs = Expr::MethodCall {
                        object: Box::new(lhs),
                        method: field_or_method,
                        args,
                        span: start..rparen.span.end,
                    };
                } else {
                    lhs = Expr::FieldAccess {
                        object: Box::new(lhs),
                        field: field_or_method,
                        span: start..ident_span.end,
                    };
                }
                continue;
            }

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
            TokenKind::FloatLiteral(v) => {
                let value = *v;
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::FloatLiteral { value, span })
            }
            TokenKind::Identifier(_) => {
                let (name, name_span) = self.expect_ident()?;
                // Check for enum variant: identifier followed by `::`
                if self.peek().kind == TokenKind::ColonColon {
                    self.pos += 1; // consume ::
                    let (variant_name, variant_span) = self.expect_ident()?;
                    let mut args = Vec::new();
                    let end;
                    if self.peek().kind == TokenKind::LParen {
                        self.pos += 1; // consume (
                        args = self.parse_call_args()?;
                        let rparen = self.expect(&TokenKind::RParen)?;
                        end = rparen.span.end;
                    } else {
                        end = variant_span.end;
                    }
                    return Ok(Expr::EnumVariant {
                        enum_name: name,
                        variant_name,
                        args,
                        span: name_span.start..end,
                    });
                }
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
                } else if self.peek().kind == TokenKind::LBrace && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    // Struct literal: Name { field: value, ... }
                    self.pos += 1; // consume {
                    let mut fields = Vec::new();
                    while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
                        let (field_name, _) = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let value = self.parse_expr(0)?;
                        fields.push((field_name, value));
                        if self.peek().kind == TokenKind::Comma {
                            self.pos += 1;
                        }
                    }
                    let rbrace = self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::StructLiteral {
                        name,
                        fields,
                        span: name_span.start..rbrace.span.end,
                    })
                } else {
                    Ok(Expr::Identifier { name, span: name_span })
                }
            }
            TokenKind::SelfValue => {
                let span = tok.span.clone();
                self.pos += 1;
                Ok(Expr::Identifier { name: "self".to_string(), span })
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
            TokenKind::Match => {
                self.parse_match_expr()
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
            }            TokenKind::Minus => {
                let start = tok.span.start;
                self.pos += 1; // consume -
                let operand = self.parse_expr(13)?; // prefix binding power 13 (tightest)
                let end = expr_span(&operand).end;
                Ok(Expr::UnaryOp {
                    op: ast::UnaryOp::Neg,
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
            TokenKind::InterpolatedString(parts) => {
                let parts = parts.clone();
                let span = tok.span.clone();
                self.pos += 1;
                self.desugar_interpolated_string(parts, span)
            }
            TokenKind::Spawn => {
                let start = tok.span.start;
                self.pos += 1;
                let (func_name, _) = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let mut args = Vec::new();
                while self.peek().kind != TokenKind::RParen {
                    args.push(self.parse_expr(0)?);
                    if self.peek().kind == TokenKind::Comma {
                        self.pos += 1;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                let span = start..self.tokens[self.pos - 1].span.end;
                Ok(Expr::Spawn { function: func_name, args, span })
            }
            TokenKind::Array => {
                let start = tok.span.start;
                self.pos += 1;
                self.expect(&TokenKind::Lt)?;
                let type_name = self.parse_type_name()?;
                self.expect(&TokenKind::Gt)?;
                self.expect(&TokenKind::LParen)?;
                self.expect(&TokenKind::RParen)?;
                let span = start..self.tokens[self.pos - 1].span.end;
                Ok(Expr::ArrayCreate { element_type: type_name, span })
            }
            TokenKind::Mutex => {
                let start = tok.span.start;
                self.pos += 1;
                self.expect(&TokenKind::LParen)?;
                let value = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                let span = start..self.tokens[self.pos - 1].span.end;
                Ok(Expr::MutexCreate { value: Box::new(value), span })
            }
            TokenKind::Channel => {
                let start = tok.span.start;
                self.pos += 1;
                self.expect(&TokenKind::Lt)?;
                let type_name = self.parse_type_name()?;
                self.expect(&TokenKind::Gt)?;
                self.expect(&TokenKind::LParen)?;
                let capacity = if self.peek().kind != TokenKind::RParen {
                    Some(Box::new(self.parse_expr(0)?))
                } else {
                    None
                };
                self.expect(&TokenKind::RParen)?;
                let span = start..self.tokens[self.pos - 1].span.end;
                Ok(Expr::ChannelCreate { element_type: type_name, capacity, span })
            }
            TokenKind::LParen => {
                self.pos += 1;
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                let start = tok.span.start;
                self.pos += 1;
                let mut elements = Vec::new();
                while self.peek().kind != TokenKind::RBracket && self.peek().kind != TokenKind::Eof {
                    elements.push(self.parse_expr(0)?);
                    if self.peek().kind == TokenKind::Comma {
                        self.pos += 1;
                    }
                }
                let rbracket = self.expect(&TokenKind::RBracket)?;
                if elements.is_empty() {
                    return Err(ParseError::new(
                        "empty array literal not allowed; use array<T>() for empty arrays",
                        start..rbracket.span.end,
                    ));
                }
                Ok(Expr::ArrayLiteral {
                    elements,
                    span: start..rbracket.span.end,
                })
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

    fn desugar_interpolated_string(&self, parts: Vec<sans_lexer::token::StringPart>, span: Span) -> Result<Expr, ParseError> {
        use sans_lexer::token::StringPart;
        let mut result: Option<Expr> = None;
        for part in parts {
            let expr = match part {
                StringPart::Literal(s) => Expr::StringLiteral { value: s, span: span.clone() },
                StringPart::Ident(name) => Expr::Identifier { name, span: span.clone() },
            };
            result = Some(match result {
                None => expr,
                Some(left) => Expr::BinaryOp {
                    left: Box::new(left),
                    op: BinOp::Add,
                    right: Box::new(expr),
                    span: span.clone(),
                },
            });
        }
        Ok(result.unwrap_or(Expr::StringLiteral { value: String::new(), span }))
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        let match_tok = self.expect(&TokenKind::Match)?;
        let start = match_tok.span.start;
        let scrutinee = self.parse_expr(0)?;
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while self.peek().kind != TokenKind::RBrace && self.peek().kind != TokenKind::Eof {
            let arm = self.parse_match_arm()?;
            arms.push(arm);
            if self.peek().kind == TokenKind::Comma {
                self.pos += 1;
            }
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start..rbrace.span.end,
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        let pattern_start = match &pattern {
            Pattern::EnumVariant { span, .. } => span.start,
        };
        self.expect(&TokenKind::FatArrow)?;
        let body = self.parse_expr(0)?;
        let end = expr_span(&body).end;
        Ok(MatchArm {
            pattern,
            body,
            span: pattern_start..end,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let (enum_name, enum_span) = self.expect_ident()?;
        self.expect(&TokenKind::ColonColon)?;
        let (variant_name, variant_span) = self.expect_ident()?;
        let mut bindings = Vec::new();
        let end;
        if self.peek().kind == TokenKind::LParen {
            self.pos += 1; // consume (
            while self.peek().kind != TokenKind::RParen {
                let (binding, _) = self.expect_ident()?;
                bindings.push(binding);
                if self.peek().kind == TokenKind::Comma {
                    self.pos += 1;
                }
            }
            let rparen = self.expect(&TokenKind::RParen)?;
            end = rparen.span.end;
        } else {
            end = variant_span.end;
        }
        Ok(Pattern::EnumVariant {
            enum_name,
            variant_name,
            bindings,
            span: enum_span.start..end,
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
                    | Stmt::If { span, .. }
                    | Stmt::LetDestructure { span, .. }
                    | Stmt::ForIn { span, .. } => {
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
        TokenKind::Slash   => Some((11, 12, BinOp::Div)),
        TokenKind::Percent => Some((11, 12, BinOp::Mod)),
        _ => None,
    }
}

fn expr_span(expr: &Expr) -> &Span {
    match expr {
        Expr::IntLiteral { span, .. } => span,
        Expr::FloatLiteral { span, .. } => span,
        Expr::BoolLiteral { span, .. } => span,
        Expr::StringLiteral { span, .. } => span,
        Expr::Identifier { span, .. } => span,
        Expr::BinaryOp { span, .. } => span,
        Expr::Call { span, .. } => span,
        Expr::If { span, .. } => span,
        Expr::UnaryOp { span, .. } => span,
        Expr::StructLiteral { span, .. } => span,
        Expr::FieldAccess { span, .. } => span,
        Expr::EnumVariant { span, .. } => span,
        Expr::Match { span, .. } => span,
        Expr::MethodCall { span, .. } => span,
        Expr::Spawn { span, .. } => span,
        Expr::ChannelCreate { span, .. } => span,
        Expr::MutexCreate { span, .. } => span,
        Expr::ArrayCreate { span, .. } => span,
        Expr::ArrayLiteral { span, .. } => span,
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
    fn parse_struct_def() {
        let prog = parse("struct Point { x Int, y Int, } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.structs.len(), 1);
        let s = &prog.structs[0];
        assert_eq!(s.name, "Point");
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].name, "x");
        assert_eq!(s.fields[0].type_name.name, "Int");
        assert_eq!(s.fields[1].name, "y");
        assert_eq!(s.fields[1].type_name.name, "Int");
    }

    #[test]
    fn parse_struct_literal() {
        let prog = parse("fn main() Int { let p = Point { x: 1, y: 2 } 0 }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Let { value, .. } = &body[0] {
            if let Expr::StructLiteral { name, fields, .. } = value {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[1].0, "y");
            } else {
                panic!("expected StructLiteral, got {:?}", value);
            }
        } else {
            panic!("expected Let");
        }
    }

    #[test]
    fn parse_field_access() {
        let prog = parse("fn main() Int { let p = Point { x: 1, y: 2 } p.x }").unwrap();
        let body = &prog.functions[0].body;
        assert_eq!(body.len(), 2);
        if let Stmt::Expr(Expr::FieldAccess { object, field, .. }) = &body[1] {
            assert_eq!(field, "x");
            assert!(matches!(object.as_ref(), Expr::Identifier { name, .. } if name == "p"));
        } else {
            panic!("expected FieldAccess, got {:?}", &body[1]);
        }
    }

    #[test]
    fn parse_enum_def() {
        let prog = parse("enum Color { Red, Green, Blue, } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.enums.len(), 1);
        let e = &prog.enums[0];
        assert_eq!(e.name, "Color");
        assert_eq!(e.variants.len(), 3);
        assert_eq!(e.variants[0].name, "Red");
        assert_eq!(e.variants[0].fields.len(), 0);
        assert_eq!(e.variants[1].name, "Green");
        assert_eq!(e.variants[2].name, "Blue");
    }

    #[test]
    fn parse_enum_with_data() {
        let prog = parse("enum Shape { Circle(Int), Rectangle(Int, Int), } fn main() Int { 0 }").unwrap();
        let e = &prog.enums[0];
        assert_eq!(e.name, "Shape");
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].name, "Circle");
        assert_eq!(e.variants[0].fields.len(), 1);
        assert_eq!(e.variants[0].fields[0].name, "Int");
        assert_eq!(e.variants[1].name, "Rectangle");
        assert_eq!(e.variants[1].fields.len(), 2);
    }

    #[test]
    fn parse_enum_variant_expr() {
        let prog = parse("fn main() Int { let c = Color::Red 0 }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Let { value, .. } = &body[0] {
            if let Expr::EnumVariant { enum_name, variant_name, args, .. } = value {
                assert_eq!(enum_name, "Color");
                assert_eq!(variant_name, "Red");
                assert_eq!(args.len(), 0);
            } else {
                panic!("expected EnumVariant, got {:?}", value);
            }
        } else {
            panic!("expected Let");
        }
    }

    #[test]
    fn parse_match_expr() {
        let prog = parse("fn main() Int { match Color::Red { Color::Red => 1, Color::Green => 2, } }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Expr(Expr::Match { arms, .. }) = &body[0] {
            assert_eq!(arms.len(), 2);
            if let Pattern::EnumVariant { enum_name, variant_name, .. } = &arms[0].pattern {
                assert_eq!(enum_name, "Color");
                assert_eq!(variant_name, "Red");
            } else {
                panic!("expected EnumVariant pattern");
            }
        } else {
            panic!("expected Match expression, got {:?}", &body[0]);
        }
    }

    #[test]
    fn parse_trait_def() {
        let prog = parse("trait Describable { fn describe(self) Int } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.traits.len(), 1);
        let t = &prog.traits[0];
        assert_eq!(t.name, "Describable");
        assert_eq!(t.methods.len(), 1);
        assert_eq!(t.methods[0].name, "describe");
        assert_eq!(t.methods[0].params.len(), 0); // self is not in params
        assert_eq!(t.methods[0].return_type.name, "Int");
    }

    #[test]
    fn parse_impl_block() {
        let prog = parse("struct Point { x Int, y Int, } impl Point { fn sum(self) Int { self.x + self.y } } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.impls.len(), 1);
        let imp = &prog.impls[0];
        assert_eq!(imp.trait_name, None);
        assert_eq!(imp.target_type, "Point");
        assert_eq!(imp.methods.len(), 1);
        assert_eq!(imp.methods[0].name, "sum");
        // First param is self with type Point
        assert_eq!(imp.methods[0].params[0].name, "self");
        assert_eq!(imp.methods[0].params[0].type_name.name, "Point");
    }

    #[test]
    fn parse_trait_impl() {
        let prog = parse("trait Describable { fn value(self) Int } struct Point { x Int, y Int, } impl Describable for Point { fn value(self) Int { self.x } } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.impls.len(), 1);
        let imp = &prog.impls[0];
        assert_eq!(imp.trait_name, Some("Describable".to_string()));
        assert_eq!(imp.target_type, "Point");
    }

    #[test]
    fn parse_method_call() {
        let prog = parse("fn main() Int { let p = Point { x: 1, y: 2 } p.sum() }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Expr(Expr::MethodCall { object, method, args, .. }) = &body[1] {
            assert_eq!(method, "sum");
            assert_eq!(args.len(), 0);
            assert!(matches!(object.as_ref(), Expr::Identifier { name, .. } if name == "p"));
        } else {
            panic!("expected MethodCall, got {:?}", &body[1]);
        }
    }

    #[test]
    fn parse_method_call_with_args() {
        let prog = parse("fn main() Int { let p = Point { x: 1, y: 2 } p.add(3) }").unwrap();
        let body = &prog.functions[0].body;
        if let Stmt::Expr(Expr::MethodCall { method, args, .. }) = &body[1] {
            assert_eq!(method, "add");
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected MethodCall, got {:?}", &body[1]);
        }
    }

    #[test]
    fn parse_trait_method_with_params() {
        let prog = parse("trait Math { fn add(self, n Int) Int } fn main() Int { 0 }").unwrap();
        let t = &prog.traits[0];
        assert_eq!(t.methods[0].name, "add");
        assert_eq!(t.methods[0].params.len(), 1);
        assert_eq!(t.methods[0].params[0].name, "n");
        assert_eq!(t.methods[0].params[0].type_name.name, "Int");
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

    #[test]
    fn parse_generic_function() {
        let prog = parse("fn identity<T>(x T) T { x } fn main() Int { identity(42) }").unwrap();
        let func = &prog.functions[0];
        assert_eq!(func.name, "identity");
        assert_eq!(func.type_params.len(), 1);
        assert_eq!(func.type_params[0].name, "T");
        assert_eq!(func.type_params[0].bound, None);
        assert_eq!(func.params[0].type_name.name, "T");
        assert_eq!(func.return_type.name, "T");
    }

    #[test]
    fn parse_generic_with_bound() {
        let prog = parse("fn get_sum<T: Summable>(x T) Int { x.sum() } fn main() Int { 0 }").unwrap();
        let func = &prog.functions[0];
        assert_eq!(func.type_params.len(), 1);
        assert_eq!(func.type_params[0].name, "T");
        assert_eq!(func.type_params[0].bound, Some("Summable".to_string()));
    }

    #[test]
    fn parse_generic_multiple_params() {
        let prog = parse("fn pair<A, B>(a A, b B) Int { 0 } fn main() Int { 0 }").unwrap();
        let func = &prog.functions[0];
        assert_eq!(func.type_params.len(), 2);
        assert_eq!(func.type_params[0].name, "A");
        assert_eq!(func.type_params[1].name, "B");
    }

    #[test]
    fn parse_non_generic_function_unchanged() {
        let prog = parse("fn add(a Int, b Int) Int { a + b } fn main() Int { 0 }").unwrap();
        assert_eq!(prog.functions[0].type_params.len(), 0);
    }

    #[test]
    fn parse_spawn_expr() {
        let program = parse("fn main() Int { spawn foo(1, 2) }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::Expr(Expr::Spawn { function, args, .. }) => {
                assert_eq!(function, "foo");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected Spawn, got {:?}", other),
        }
    }

    #[test]
    fn parse_channel_create() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::LetDestructure { names, value, .. } => {
                assert_eq!(names, &["tx".to_string(), "rx".to_string()]);
                assert!(matches!(value, Expr::ChannelCreate { .. }));
            }
            other => panic!("expected LetDestructure, got {:?}", other),
        }
    }

    #[test]
    fn parse_send_method_call() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) 0 }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
                assert_eq!(method, "send");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected send MethodCall, got {:?}", other),
        }
    }

    #[test]
    fn parse_recv_method_call() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>() let val = rx.recv() 0 }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::MethodCall { method, .. } if method == "recv"));
            }
            other => panic!("expected Let with recv, got {:?}", other),
        }
    }

    #[test]
    fn parse_join_method_call() {
        let program = parse("fn main() Int { let h = spawn foo() h.join() 0 }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::Expr(Expr::MethodCall { method, .. }) => {
                assert_eq!(method, "join");
            }
            other => panic!("expected join MethodCall, got {:?}", other),
        }
    }

    #[test]
    fn parse_spawn_no_args() {
        let program = parse("fn main() Int { spawn worker() }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::Expr(Expr::Spawn { function, args, .. }) => {
                assert_eq!(function, "worker");
                assert_eq!(args.len(), 0);
            }
            other => panic!("expected Spawn, got {:?}", other),
        }
    }

    #[test]
    fn parse_mutex_create() {
        let program = parse("fn main() Int { let m = mutex(0) 0 }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::MutexCreate { .. }));
            }
            other => panic!("expected Let with MutexCreate, got {:?}", other),
        }
    }

    #[test]
    fn parse_bounded_channel() {
        let program = parse("fn main() Int { let (tx, rx) = channel<Int>(10) 0 }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::LetDestructure { value, .. } => {
                match value {
                    Expr::ChannelCreate { capacity: Some(cap), .. } => {
                        assert!(matches!(**cap, Expr::IntLiteral { value: 10, .. }));
                    }
                    other => panic!("expected ChannelCreate with capacity, got {:?}", other),
                }
            }
            other => panic!("expected LetDestructure, got {:?}", other),
        }
    }

    #[test]
    fn parse_lock_unlock_method_calls() {
        let program = parse("fn main() Int { let m = mutex(0) let v = m.lock() m.unlock(v) 0 }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::MethodCall { method, .. } if method == "lock"));
            }
            other => panic!("expected Let with lock() call, got {:?}", other),
        }
        match &program.functions[0].body[2] {
            Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
                assert_eq!(method, "unlock");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected unlock() call, got {:?}", other),
        }
    }

    #[test]
    fn parse_array_create() {
        let program = parse("fn main() Int { let a = array<Int>() 0 }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::ArrayCreate { .. }));
            }
            other => panic!("expected Let with ArrayCreate, got {:?}", other),
        }
    }

    #[test]
    fn parse_for_in() {
        let program = parse("fn main() Int { let a = array<Int>() for x in a { print(x) } 0 }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::ForIn { var, .. } => {
                assert_eq!(var, "x");
            }
            other => panic!("expected ForIn, got {:?}", other),
        }
    }

    #[test]
    fn parse_array_push_get_set_len() {
        let program = parse("fn main() Int { let a = array<Int>() a.push(1) a.get(0) }").unwrap();
        match &program.functions[0].body[1] {
            Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
                assert_eq!(method, "push");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected push call, got {:?}", other),
        }
    }

    #[test]
    fn parse_int_to_string_call() {
        let program = parse("fn main() Int { let s = int_to_string(42) 0 }").unwrap();
        match &program.functions[0].body[0] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::Call { function, .. } if function == "int_to_string"));
            }
            other => panic!("expected Let with Call, got {:?}", other),
        }
    }

    #[test]
    fn parse_string_concat() {
        let program = parse(r#"fn main() Int { let s = "a" + "b" 0 }"#).unwrap();
        match &program.functions[0].body[0] {
            Stmt::Let { value, .. } => {
                assert!(matches!(value, Expr::BinaryOp { .. }));
            }
            other => panic!("expected Let with BinaryOp, got {:?}", other),
        }
    }

    #[test]
    fn parse_import_declaration() {
        let prog = parse("import \"utils\"\nfn main() Int { 0 }").unwrap();
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.imports[0].path, "utils");
        assert_eq!(prog.imports[0].module_name, "utils");
    }

    #[test]
    fn parse_multiple_imports() {
        let prog = parse("import \"utils\"\nimport \"models/user\"\nfn main() Int { 0 }").unwrap();
        assert_eq!(prog.imports.len(), 2);
        assert_eq!(prog.imports[0].path, "utils");
        assert_eq!(prog.imports[0].module_name, "utils");
        assert_eq!(prog.imports[1].path, "models/user");
        assert_eq!(prog.imports[1].module_name, "user");
    }

    #[test]
    fn parse_import_after_function_errors() {
        let result = parse("fn foo() Int { 0 }\nimport \"utils\"\nfn main() Int { 0 }");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("imports must appear before all declarations"),
            "expected import placement error, got: {}", err.message);
    }
}
