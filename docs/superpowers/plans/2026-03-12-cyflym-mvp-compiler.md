# Cyflym MVP Compiler Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an end-to-end compiler that can lex, parse, type-check, and compile a minimal Cyflym program to a native binary via LLVM.

**Architecture:** Rust workspace with separate crates for each compiler phase: lexer, parser, type checker, IR, codegen, and driver. Each crate has a clean public API and is independently testable. The compilation pipeline is: Source → Tokens → AST → Typed AST → Cyflym IR → LLVM IR → Native Binary.

**Tech Stack:** Rust (compiler implementation), LLVM 17+ (code generation via `inkwell` crate), Cargo workspace (build system)

**Spec:** `docs/superpowers/specs/2026-03-12-cyflym-language-design.md`

---

## Series Context

This is **Plan 1 of 7** in the Cyflym compiler series:

1. **MVP Compiler** (this plan) — lex, parse, type-check, codegen for basic programs
2. **Functions & Control Flow** — function calls, if/else, while, return, closures
3. **Structs, Enums & Pattern Matching** — data types, match expressions, destructuring
4. **Traits & Generics** — trait system, generic functions/structs, type constraints
5. **Concurrency** — green thread runtime, channels, spawn, select
6. **Standard Library** — http, json, log, crypto, io, testing
7. **Toolchain** — `cyflym` CLI, formatter, linter, test runner, package manager, LSP

---

## MVP Scope

The MVP compiler handles this subset of Cyflym:

```
// MVP target program
fn main() Int {
    let x Int = 42
    let y Int = x + 8
    let z Int = add(x, y)
    z
}

fn add(a Int, b Int) Int {
    a + b
}
```

**Supported in MVP:**
- Integer literals, arithmetic operators (`+`, `-`, `*`, `/`)
- `let` bindings (immutable, with explicit type annotations)
- Function declarations with typed parameters and return types
- Function calls
- Implicit return (last expression)
- `Int` type only (64-bit signed integer)
- Entry point: `fn main() Int` returns exit code

**Not in MVP** (deferred to later plans):
- Type inference, `mut`, strings, booleans, floats
- `if`/`else`, `while`, `loop`, `for`, `match`
- Structs, enums, traits, generics
- `Result`, `Option`, `?` operator
- Modules, imports, visibility
- GC, green threads, channels
- All stdlib packages

---

## Prerequisites

Before starting, the developer must install:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install LLVM 17 (macOS)
brew install llvm@17
echo 'export LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)' >> ~/.zshrc
source ~/.zshrc
```

---

## File Structure

```
cyflym/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── cyflym-lexer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Lexer: source → tokens
│   │       └── token.rs                # Token enum and Span type
│   ├── cyflym-parser/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Parser: tokens → AST
│   │       └── ast.rs                  # AST node types
│   ├── cyflym-typeck/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Type checker: AST → Typed AST
│   │       └── types.rs                # Type representations
│   ├── cyflym-ir/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # IR generation: Typed AST → Cyflym IR
│   │       └── ir.rs                   # IR instruction types
│   ├── cyflym-codegen/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # LLVM codegen: Cyflym IR → LLVM IR → binary
│   └── cyflym-driver/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                 # CLI: orchestrates pipeline
├── tests/
│   └── fixtures/
│       ├── minimal.cy                  # fn main() Int { 42 }
│       ├── let_binding.cy             # let bindings + arithmetic
│       ├── function_call.cy           # function calls
│       └── full_mvp.cy               # full MVP program
└── docs/
    └── superpowers/
        ├── specs/
        │   └── 2026-03-12-cyflym-language-design.md
        └── plans/
            └── 2026-03-12-cyflym-mvp-compiler.md  (this file)
```

---

## Chunk 1: Project Setup & Lexer

### Task 1: Initialize Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `crates/cyflym-lexer/Cargo.toml`
- Create: `crates/cyflym-lexer/src/lib.rs`
- Create: `crates/cyflym-lexer/src/token.rs`

- [ ] **Step 1: Verify Rust toolchain is installed**

Run: `rustc --version && cargo --version`
Expected: Version output for both (any stable Rust 1.70+)

If not installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

- [ ] **Step 2: Create workspace Cargo.toml**

Create `Cargo.toml` at the project root:

```toml
[workspace]
resolver = "2"
members = [
    "crates/cyflym-lexer",
    "crates/cyflym-parser",
    "crates/cyflym-typeck",
    "crates/cyflym-ir",
    "crates/cyflym-codegen",
    "crates/cyflym-driver",
]
```

- [ ] **Step 3: Create lexer crate skeleton**

Create `crates/cyflym-lexer/Cargo.toml`:

```toml
[package]
name = "cyflym-lexer"
version = "0.1.0"
edition = "2021"
```

Create `crates/cyflym-lexer/src/token.rs`:

```rust
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
```

Create `crates/cyflym-lexer/src/lib.rs`:

```rust
pub mod token;

pub use token::{Token, TokenKind, Span};
```

- [ ] **Step 4: Verify workspace compiles**

Run: `cargo build`
Expected: Compiles successfully (warnings about unused code are fine)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cyflym-lexer/
git commit -m "feat: initialize workspace and lexer crate skeleton"
```

---

### Task 2: Implement Lexer

**Files:**
- Modify: `crates/cyflym-lexer/src/lib.rs`

- [ ] **Step 1: Write failing tests for lexer**

Add to `crates/cyflym-lexer/src/lib.rs`:

```rust
pub mod token;

pub use token::{Token, TokenKind, Span};

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_integer_literal() {
        let tokens = lex("42").unwrap();
        assert_eq!(tokens.len(), 2); // IntLiteral + Eof
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn lex_identifier() {
        let tokens = lex("foo").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier("foo".to_string()));
    }

    #[test]
    fn lex_fn_keyword() {
        let tokens = lex("fn").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
    }

    #[test]
    fn lex_let_keyword() {
        let tokens = lex("let").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Let);
    }

    #[test]
    fn lex_operators() {
        let tokens = lex("+ - * /").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
    }

    #[test]
    fn lex_delimiters_and_punctuation() {
        let tokens = lex("( ) { } , :").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert_eq!(tokens[1].kind, TokenKind::RParen);
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
        assert_eq!(tokens[3].kind, TokenKind::RBrace);
        assert_eq!(tokens[4].kind, TokenKind::Comma);
        assert_eq!(tokens[5].kind, TokenKind::Colon);
    }

    #[test]
    fn lex_simple_function() {
        let source = "fn main() Int { 42 }";
        let tokens = lex(source).unwrap();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(kinds, vec![
            &TokenKind::Fn,
            &TokenKind::Identifier("main".to_string()),
            &TokenKind::LParen,
            &TokenKind::RParen,
            &TokenKind::Identifier("Int".to_string()),
            &TokenKind::LBrace,
            &TokenKind::IntLiteral(42),
            &TokenKind::RBrace,
            &TokenKind::Eof,
        ]);
    }

    #[test]
    fn lex_skips_whitespace_and_comments() {
        let source = "fn // this is a comment\nmain";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
    }

    #[test]
    fn lex_records_spans() {
        let tokens = lex("fn main").unwrap();
        assert_eq!(tokens[0].span, 0..2);
        assert_eq!(tokens[1].span, 3..7);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p cyflym-lexer`
Expected: All tests FAIL with "not yet implemented"

- [ ] **Step 3: Implement the lexer**

Replace the `todo!()` in `lex()` with the full implementation in `crates/cyflym-lexer/src/lib.rs`:

```rust
pub mod token;

pub use token::{Token, TokenKind, Span};

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        // Skip whitespace
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        // Skip line comments
        if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'/' {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let start = pos;

        // Integer literals
        if bytes[pos].is_ascii_digit() {
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let text = &source[start..pos];
            let value = text.parse::<i64>().map_err(|_| LexError {
                message: format!("invalid integer literal: {}", text),
                position: start,
            })?;
            tokens.push(Token {
                kind: TokenKind::IntLiteral(value),
                span: start..pos,
            });
            continue;
        }

        // Identifiers and keywords
        if bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_' {
            while pos < bytes.len()
                && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
            {
                pos += 1;
            }
            let text = &source[start..pos];
            let kind = match text {
                "fn" => TokenKind::Fn,
                "let" => TokenKind::Let,
                _ => TokenKind::Identifier(text.to_string()),
            };
            tokens.push(Token {
                kind,
                span: start..pos,
            });
            continue;
        }

        // Single-character tokens
        let kind = match bytes[pos] {
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            ch => {
                return Err(LexError {
                    message: format!("unexpected character: '{}'", ch as char),
                    position: pos,
                });
            }
        };
        pos += 1;
        tokens.push(Token {
            kind,
            span: start..pos,
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: pos..pos,
    });

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_integer_literal() {
        let tokens = lex("42").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn lex_identifier() {
        let tokens = lex("foo").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier("foo".to_string()));
    }

    #[test]
    fn lex_fn_keyword() {
        let tokens = lex("fn").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
    }

    #[test]
    fn lex_let_keyword() {
        let tokens = lex("let").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Let);
    }

    #[test]
    fn lex_operators() {
        let tokens = lex("+ - * /").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
    }

    #[test]
    fn lex_delimiters_and_punctuation() {
        let tokens = lex("( ) { } , :").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert_eq!(tokens[1].kind, TokenKind::RParen);
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
        assert_eq!(tokens[3].kind, TokenKind::RBrace);
        assert_eq!(tokens[4].kind, TokenKind::Comma);
        assert_eq!(tokens[5].kind, TokenKind::Colon);
    }

    #[test]
    fn lex_simple_function() {
        let source = "fn main() Int { 42 }";
        let tokens = lex(source).unwrap();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(kinds, vec![
            &TokenKind::Fn,
            &TokenKind::Identifier("main".to_string()),
            &TokenKind::LParen,
            &TokenKind::RParen,
            &TokenKind::Identifier("Int".to_string()),
            &TokenKind::LBrace,
            &TokenKind::IntLiteral(42),
            &TokenKind::RBrace,
            &TokenKind::Eof,
        ]);
    }

    #[test]
    fn lex_skips_whitespace_and_comments() {
        let source = "fn // this is a comment\nmain";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
    }

    #[test]
    fn lex_records_spans() {
        let tokens = lex("fn main").unwrap();
        assert_eq!(tokens[0].span, 0..2);
        assert_eq!(tokens[1].span, 3..7);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p cyflym-lexer`
Expected: All 8 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cyflym-lexer/
git commit -m "feat: implement lexer with tokens for MVP subset"
```

---

## Chunk 2: Parser

### Task 3: Define AST Types

**Files:**
- Create: `crates/cyflym-parser/Cargo.toml`
- Create: `crates/cyflym-parser/src/ast.rs`
- Create: `crates/cyflym-parser/src/lib.rs`

- [ ] **Step 1: Create parser crate with AST types**

Create `crates/cyflym-parser/Cargo.toml`:

```toml
[package]
name = "cyflym-parser"
version = "0.1.0"
edition = "2021"

[dependencies]
cyflym-lexer = { path = "../cyflym-lexer" }
```

Create `crates/cyflym-parser/src/ast.rs`:

```rust
use cyflym_lexer::Span;

/// A complete source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

/// A function declaration: `fn name(params) ReturnType { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeName,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// A function parameter: `name Type`
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_name: TypeName,
    pub span: Span,
}

/// A type reference (just a name for now).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeName {
    pub name: String,
    pub span: Span,
}

/// A statement in a function body.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name Type = expr`
    Let {
        name: String,
        type_name: TypeName,
        value: Expr,
        span: Span,
    },
    /// A bare expression (used as implicit return when last in block).
    Expr(Expr),
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal: `42`
    IntLiteral {
        value: i64,
        span: Span,
    },
    /// Variable reference: `x`
    Identifier {
        name: String,
        span: Span,
    },
    /// Binary operation: `a + b`
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// Function call: `add(x, y)`
    Call {
        function: String,
        args: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}
```

Create `crates/cyflym-parser/src/lib.rs`:

```rust
pub mod ast;

pub use ast::*;
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p cyflym-parser`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/cyflym-parser/
git commit -m "feat: define AST types for MVP subset"
```

---

### Task 4: Implement Parser

**Files:**
- Modify: `crates/cyflym-parser/src/lib.rs`

- [ ] **Step 1: Add `Eq` token to lexer**

The parser needs an `=` token for let bindings. Add it to the lexer first.

In `crates/cyflym-lexer/src/token.rs`, add `Eq` to the `TokenKind` enum:

```rust
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,        // =
```

In `crates/cyflym-lexer/src/lib.rs`, add to the single-character match in `lex()`:

```rust
            b'=' => TokenKind::Eq,
```

Run: `cargo test -p cyflym-lexer`
Expected: All existing tests still PASS

- [ ] **Step 2: Write failing tests for the parser**

Add to `crates/cyflym-parser/src/lib.rs`:

```rust
pub mod ast;

pub use ast::*;
use cyflym_lexer::{Token, TokenKind, lex};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: cyflym_lexer::Span,
}

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lex(source).map_err(|e| ParseError {
        message: e.message,
        span: e.position..e.position + 1,
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
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_function() {
        let program = parse("fn main() Int { 42 }").unwrap();
        assert_eq!(program.functions.len(), 1);
        let f = &program.functions[0];
        assert_eq!(f.name, "main");
        assert_eq!(f.params.len(), 0);
        assert_eq!(f.return_type.name, "Int");
        assert_eq!(f.body.len(), 1);
        match &f.body[0] {
            Stmt::Expr(Expr::IntLiteral { value, .. }) => assert_eq!(*value, 42),
            other => panic!("expected int literal, got {:?}", other),
        }
    }

    #[test]
    fn parse_let_binding() {
        let program = parse("fn main() Int { let x Int = 42 x }").unwrap();
        let f = &program.functions[0];
        assert_eq!(f.body.len(), 2);
        match &f.body[0] {
            Stmt::Let { name, type_name, .. } => {
                assert_eq!(name, "x");
                assert_eq!(type_name.name, "Int");
            }
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn parse_binary_expression() {
        let program = parse("fn main() Int { 1 + 2 }").unwrap();
        let f = &program.functions[0];
        match &f.body[0] {
            Stmt::Expr(Expr::BinaryOp { op, .. }) => assert_eq!(*op, BinOp::Add),
            other => panic!("expected binary op, got {:?}", other),
        }
    }

    #[test]
    fn parse_operator_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let program = parse("fn main() Int { 1 + 2 * 3 }").unwrap();
        let f = &program.functions[0];
        match &f.body[0] {
            Stmt::Expr(Expr::BinaryOp { op, right, .. }) => {
                assert_eq!(*op, BinOp::Add);
                match right.as_ref() {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Mul),
                    other => panic!("expected mul on right, got {:?}", other),
                }
            }
            other => panic!("expected binary op, got {:?}", other),
        }
    }

    #[test]
    fn parse_function_call() {
        let program = parse("fn main() Int { add(1, 2) }").unwrap();
        let f = &program.functions[0];
        match &f.body[0] {
            Stmt::Expr(Expr::Call { function, args, .. }) => {
                assert_eq!(function, "add");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected call, got {:?}", other),
        }
    }

    #[test]
    fn parse_function_with_params() {
        let program = parse("fn add(a Int, b Int) Int { a + b }").unwrap();
        let f = &program.functions[0];
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].type_name.name, "Int");
        assert_eq!(f.params[1].name, "b");
    }

    #[test]
    fn parse_multiple_functions() {
        let source = "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1, 2) }";
        let program = parse(source).unwrap();
        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].name, "add");
        assert_eq!(program.functions[1].name, "main");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p cyflym-parser`
Expected: All tests FAIL with "not yet implemented"

- [ ] **Step 4: Implement the parser**

Replace the `Parser` implementation in `crates/cyflym-parser/src/lib.rs` (keep tests and the `parse()` function as-is, replace only the `Parser` struct impl):

```rust
impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> cyflym_lexer::Span {
        self.tokens[self.pos].span.clone()
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<&Token, ParseError> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {:?}, got {:?}", expected, self.peek()),
                span: self.current_span(),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<(String, cyflym_lexer::Span), ParseError> {
        match self.peek().clone() {
            TokenKind::Identifier(name) => {
                let span = self.current_span();
                self.advance();
                Ok((name, span))
            }
            other => Err(ParseError {
                message: format!("expected identifier, got {:?}", other),
                span: self.current_span(),
            }),
        }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while *self.peek() != TokenKind::Eof {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Fn)?;
        let (name, _) = self.expect_identifier()?;
        self.expect(&TokenKind::LParen)?;

        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(&TokenKind::Comma)?;
            }
            let param_start = self.current_span().start;
            let (param_name, _) = self.expect_identifier()?;
            let (type_name_str, type_span) = self.expect_identifier()?;
            let param_end = type_span.end;
            params.push(Param {
                name: param_name,
                type_name: TypeName {
                    name: type_name_str,
                    span: type_span,
                },
                span: param_start..param_end,
            });
        }
        self.expect(&TokenKind::RParen)?;

        let (ret_type_name, ret_type_span) = self.expect_identifier()?;
        let return_type = TypeName {
            name: ret_type_name,
            span: ret_type_span,
        };

        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_body()?;
        let end = self.current_span().end;
        self.expect(&TokenKind::RBrace)?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span: start..end,
        })
    }

    fn parse_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if *self.peek() == TokenKind::Let {
            self.parse_let()
        } else {
            Ok(Stmt::Expr(self.parse_expr()?))
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Let)?;
        let (name, _) = self.expect_identifier()?;
        let (type_name_str, type_span) = self.expect_identifier()?;
        let type_name = TypeName {
            name: type_name_str,
            span: type_span,
        };

        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;

        let end = match &value {
            Expr::IntLiteral { span, .. }
            | Expr::Identifier { span, .. }
            | Expr::BinaryOp { span, .. }
            | Expr::Call { span, .. } => span.end,
        };

        Ok(Stmt::Let {
            name,
            type_name,
            value,
            span: start..end,
        })
    }

    /// Parse expression with operator precedence (Pratt parsing).
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;

        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };

            let (left_bp, right_bp) = infix_binding_power(op);
            if left_bp < min_bp {
                break;
            }

            self.advance(); // consume operator

            let right = self.parse_expr_bp(right_bp)?;
            let span = match &left {
                Expr::IntLiteral { span, .. }
                | Expr::Identifier { span, .. }
                | Expr::BinaryOp { span, .. }
                | Expr::Call { span, .. } => span.start,
            }..match &right {
                Expr::IntLiteral { span, .. }
                | Expr::Identifier { span, .. }
                | Expr::BinaryOp { span, .. }
                | Expr::Call { span, .. } => span.end,
            };

            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            TokenKind::IntLiteral(value) => {
                let span = self.current_span();
                self.advance();
                Ok(Expr::IntLiteral { value, span })
            }
            TokenKind::Identifier(name) => {
                let span = self.current_span();
                self.advance();

                // Check for function call: identifier followed by '('
                if *self.peek() == TokenKind::LParen {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    while *self.peek() != TokenKind::RParen {
                        if !args.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        args.push(self.parse_expr()?);
                    }
                    let end = self.current_span().end;
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Call {
                        function: name,
                        args,
                        span: span.start..end,
                    })
                } else {
                    Ok(Expr::Identifier { name, span })
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(ParseError {
                message: format!("expected expression, got {:?}", other),
                span: self.current_span(),
            }),
        }
    }
}

fn infix_binding_power(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Add | BinOp::Sub => (1, 2),
        BinOp::Mul | BinOp::Div => (3, 4),
    }
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p cyflym-lexer && cargo test -p cyflym-parser`
Expected: All lexer tests PASS, all 7 parser tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/cyflym-lexer/ crates/cyflym-parser/
git commit -m "feat: implement parser with Pratt precedence climbing"
```

---

## Chunk 3: Type Checker

### Task 5: Implement Type Checker

**Files:**
- Create: `crates/cyflym-typeck/Cargo.toml`
- Create: `crates/cyflym-typeck/src/types.rs`
- Create: `crates/cyflym-typeck/src/lib.rs`

- [ ] **Step 1: Create type checker crate with type definitions**

Create `crates/cyflym-typeck/Cargo.toml`:

```toml
[package]
name = "cyflym-typeck"
version = "0.1.0"
edition = "2021"

[dependencies]
cyflym-parser = { path = "../cyflym-parser" }
cyflym-lexer = { path = "../cyflym-lexer" }
```

Create `crates/cyflym-typeck/src/types.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    /// Function type: (param types) -> return type
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
    },
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Fn { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") {}", ret)
            }
        }
    }
}
```

- [ ] **Step 2: Write failing tests for type checker**

Create `crates/cyflym-typeck/src/lib.rs`:

```rust
pub mod types;

pub use types::Type;
use cyflym_parser::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub span: cyflym_lexer::Span,
}

pub fn check(program: &Program) -> Result<(), TypeError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_parser::parse;

    #[test]
    fn check_valid_minimal() {
        let program = parse("fn main() Int { 42 }").unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn check_valid_let_binding() {
        let program = parse("fn main() Int { let x Int = 42 x }").unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn check_valid_arithmetic() {
        let program = parse("fn main() Int { let x Int = 1 + 2 x }").unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn check_valid_function_call() {
        let source = "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1, 2) }";
        let program = parse(source).unwrap();
        check(&program).unwrap();
    }

    #[test]
    fn check_undefined_variable() {
        let program = parse("fn main() Int { x }").unwrap();
        let err = check(&program).unwrap_err();
        assert!(err.message.contains("undefined variable"));
    }

    #[test]
    fn check_undefined_function() {
        let program = parse("fn main() Int { foo(1) }").unwrap();
        let err = check(&program).unwrap_err();
        assert!(err.message.contains("undefined function"));
    }

    #[test]
    fn check_wrong_arg_count() {
        let source = "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1) }";
        let program = parse(source).unwrap();
        let err = check(&program).unwrap_err();
        assert!(err.message.contains("argument"));
    }

    #[test]
    fn check_requires_main() {
        let program = parse("fn add(a Int, b Int) Int { a + b }").unwrap();
        let err = check(&program).unwrap_err();
        assert!(err.message.contains("main"));
    }

    #[test]
    fn check_missing_return_expression() {
        let program = parse("fn main() Int { let x Int = 42 }").unwrap();
        let err = check(&program).unwrap_err();
        assert!(err.message.contains("missing return"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p cyflym-typeck`
Expected: All tests FAIL with "not yet implemented"

- [ ] **Step 4: Implement the type checker**

Replace `todo!()` and add the implementation in `crates/cyflym-typeck/src/lib.rs`. Keep tests, replace the `check` function and add supporting code:

```rust
pub fn check(program: &Program) -> Result<(), TypeError> {
    let mut checker = Checker::new();
    checker.check_program(program)
}

struct Checker {
    /// Map of function name -> (param types, return type)
    functions: HashMap<String, (Vec<Type>, Type)>,
}

impl Checker {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    fn resolve_type(&self, type_name: &TypeName) -> Result<Type, TypeError> {
        match type_name.name.as_str() {
            "Int" => Ok(Type::Int),
            other => Err(TypeError {
                message: format!("unknown type: {}", other),
                span: type_name.span.clone(),
            }),
        }
    }

    fn check_program(&mut self, program: &Program) -> Result<(), TypeError> {
        // First pass: register all function signatures
        for func in &program.functions {
            let params: Vec<Type> = func
                .params
                .iter()
                .map(|p| self.resolve_type(&p.type_name))
                .collect::<Result<_, _>>()?;
            let ret = self.resolve_type(&func.return_type)?;
            self.functions
                .insert(func.name.clone(), (params, ret));
        }

        // Check that main exists
        if !self.functions.contains_key("main") {
            return Err(TypeError {
                message: "program must have a main function".to_string(),
                span: 0..0,
            });
        }

        // Second pass: check function bodies
        for func in &program.functions {
            self.check_function(func)?;
        }

        Ok(())
    }

    fn check_function(&self, func: &Function) -> Result<(), TypeError> {
        let mut locals: HashMap<String, Type> = HashMap::new();

        // Add params to locals
        for param in &func.params {
            let ty = self.resolve_type(&param.type_name)?;
            locals.insert(param.name.clone(), ty);
        }

        let expected_ret = self.resolve_type(&func.return_type)?;

        // Check each statement
        let mut last_type = None;
        for stmt in &func.body {
            match stmt {
                Stmt::Let {
                    name,
                    type_name,
                    value,
                    span,
                } => {
                    let declared = self.resolve_type(type_name)?;
                    let actual = self.check_expr(value, &locals)?;
                    if declared != actual {
                        return Err(TypeError {
                            message: format!(
                                "type mismatch: declared {}, got {}",
                                declared, actual
                            ),
                            span: span.clone(),
                        });
                    }
                    locals.insert(name.clone(), declared);
                    last_type = None; // let doesn't produce a value
                }
                Stmt::Expr(expr) => {
                    let ty = self.check_expr(expr, &locals)?;
                    last_type = Some(ty);
                }
            }
        }

        // Check implicit return type
        match last_type {
            Some(last) => {
                if last != expected_ret {
                    return Err(TypeError {
                        message: format!(
                            "function {} returns {}, but body evaluates to {}",
                            func.name, expected_ret, last
                        ),
                        span: func.span.clone(),
                    });
                }
            }
            None => {
                return Err(TypeError {
                    message: format!(
                        "function {} missing return expression of type {}",
                        func.name, expected_ret
                    ),
                    span: func.span.clone(),
                });
            }
        }

        Ok(())
    }

    fn check_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<Type, TypeError> {
        match expr {
            Expr::IntLiteral { .. } => Ok(Type::Int),
            Expr::Identifier { name, span } => {
                locals.get(name).cloned().ok_or_else(|| TypeError {
                    message: format!("undefined variable: {}", name),
                    span: span.clone(),
                })
            }
            Expr::BinaryOp {
                left,
                op: _,
                right,
                span,
            } => {
                let left_ty = self.check_expr(left, locals)?;
                let right_ty = self.check_expr(right, locals)?;
                if left_ty != Type::Int || right_ty != Type::Int {
                    return Err(TypeError {
                        message: format!(
                            "arithmetic requires Int operands, got {} and {}",
                            left_ty, right_ty
                        ),
                        span: span.clone(),
                    });
                }
                Ok(Type::Int)
            }
            Expr::Call {
                function,
                args,
                span,
            } => {
                let (param_types, ret_type) =
                    self.functions.get(function).ok_or_else(|| TypeError {
                        message: format!("undefined function: {}", function),
                        span: span.clone(),
                    })?;

                if args.len() != param_types.len() {
                    return Err(TypeError {
                        message: format!(
                            "function {} expects {} arguments, got {}",
                            function,
                            param_types.len(),
                            args.len()
                        ),
                        span: span.clone(),
                    });
                }

                for (arg, expected) in args.iter().zip(param_types.iter()) {
                    let actual = self.check_expr(arg, locals)?;
                    if actual != *expected {
                        return Err(TypeError {
                            message: format!(
                                "argument type mismatch: expected {}, got {}",
                                expected, actual
                            ),
                            span: span.clone(),
                        });
                    }
                }

                Ok(ret_type.clone())
            }
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p cyflym-typeck`
Expected: All 9 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/cyflym-typeck/
git commit -m "feat: implement type checker with function signatures and locals"
```

---

## Chunk 4: Cyflym IR

### Task 6: Implement Cyflym IR

**Files:**
- Create: `crates/cyflym-ir/Cargo.toml`
- Create: `crates/cyflym-ir/src/ir.rs`
- Create: `crates/cyflym-ir/src/lib.rs`

- [ ] **Step 1: Create IR crate with instruction types**

Create `crates/cyflym-ir/Cargo.toml`:

```toml
[package]
name = "cyflym-ir"
version = "0.1.0"
edition = "2021"

[dependencies]
cyflym-parser = { path = "../cyflym-parser" }
cyflym-lexer = { path = "../cyflym-lexer" }
```

Create `crates/cyflym-ir/src/ir.rs`:

```rust
/// A complete IR module (corresponds to one source file).
#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<IrFunction>,
}

/// An IR function.
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Instruction>,
}

/// A register name (e.g., "%0", "%1").
pub type Reg = String;

/// IR instructions — SSA-style, flat, no nesting.
#[derive(Debug, Clone)]
pub enum Instruction {
    /// %reg = const <value>
    Const {
        dest: Reg,
        value: i64,
    },
    /// %reg = add %a, %b  (or sub, mul, div)
    BinOp {
        dest: Reg,
        op: IrBinOp,
        left: Reg,
        right: Reg,
    },
    /// %reg = copy %src  (variable load)
    Copy {
        dest: Reg,
        src: Reg,
    },
    /// %reg = call <function>(%args...)
    Call {
        dest: Reg,
        function: String,
        args: Vec<Reg>,
    },
    /// ret %reg
    Ret {
        value: Reg,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
}
```

- [ ] **Step 2: Write failing tests for IR generation**

Create `crates/cyflym-ir/src/lib.rs`:

```rust
pub mod ir;

pub use ir::*;
use cyflym_parser::*;

pub fn lower(program: &Program) -> Module {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_parser::parse;

    #[test]
    fn lower_minimal() {
        let program = parse("fn main() Int { 42 }").unwrap();
        let module = lower(&program);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
        // Should have: const %0 = 42, ret %0
        assert!(module.functions[0].body.len() >= 2);
        match &module.functions[0].body[0] {
            Instruction::Const { value, .. } => assert_eq!(*value, 42),
            other => panic!("expected Const, got {:?}", other),
        }
    }

    #[test]
    fn lower_let_and_arithmetic() {
        let program = parse("fn main() Int { let x Int = 1 + 2 x }").unwrap();
        let module = lower(&program);
        let body = &module.functions[0].body;
        // Should have instructions for: const 1, const 2, add, copy, ret
        let has_binop = body.iter().any(|i| matches!(i, Instruction::BinOp { .. }));
        assert!(has_binop, "expected a BinOp instruction");
    }

    #[test]
    fn lower_function_call() {
        let source = "fn add(a Int, b Int) Int { a + b } fn main() Int { add(1, 2) }";
        let program = parse(source).unwrap();
        let module = lower(&program);
        assert_eq!(module.functions.len(), 2);
        let main = &module.functions[1];
        let has_call = main.body.iter().any(|i| matches!(i, Instruction::Call { .. }));
        assert!(has_call, "expected a Call instruction");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p cyflym-ir`
Expected: All tests FAIL with "not yet implemented"

- [ ] **Step 4: Implement IR lowering**

Replace `todo!()` with the implementation in `crates/cyflym-ir/src/lib.rs`:

```rust
pub fn lower(program: &Program) -> Module {
    let mut functions = Vec::new();
    for func in &program.functions {
        functions.push(lower_function(func));
    }
    Module { functions }
}

struct IrBuilder {
    instructions: Vec<Instruction>,
    next_reg: usize,
    /// Maps variable names to their register.
    locals: std::collections::HashMap<String, Reg>,
}

impl IrBuilder {
    fn new(params: &[Param]) -> Self {
        let mut locals = std::collections::HashMap::new();
        for (i, param) in params.iter().enumerate() {
            locals.insert(param.name.clone(), format!("arg{}", i));
        }
        Self {
            instructions: Vec::new(),
            next_reg: 0,
            locals,
        }
    }

    fn fresh_reg(&mut self) -> Reg {
        let reg = format!("%{}", self.next_reg);
        self.next_reg += 1;
        reg
    }

    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    fn lower_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::IntLiteral { value, .. } => {
                let dest = self.fresh_reg();
                self.emit(Instruction::Const {
                    dest: dest.clone(),
                    value: *value,
                });
                dest
            }
            Expr::Identifier { name, .. } => {
                // Return the register holding this variable
                self.locals.get(name).cloned().unwrap_or_else(|| {
                    panic!("ICE: undefined variable {} in IR lowering", name)
                })
            }
            Expr::BinaryOp {
                left, op, right, ..
            } => {
                let left_reg = self.lower_expr(left);
                let right_reg = self.lower_expr(right);
                let dest = self.fresh_reg();
                let ir_op = match op {
                    BinOp::Add => IrBinOp::Add,
                    BinOp::Sub => IrBinOp::Sub,
                    BinOp::Mul => IrBinOp::Mul,
                    BinOp::Div => IrBinOp::Div,
                };
                self.emit(Instruction::BinOp {
                    dest: dest.clone(),
                    op: ir_op,
                    left: left_reg,
                    right: right_reg,
                });
                dest
            }
            Expr::Call {
                function, args, ..
            } => {
                let arg_regs: Vec<Reg> =
                    args.iter().map(|a| self.lower_expr(a)).collect();
                let dest = self.fresh_reg();
                self.emit(Instruction::Call {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_regs,
                });
                dest
            }
        }
    }
}

fn lower_function(func: &Function) -> IrFunction {
    let mut builder = IrBuilder::new(&func.params);
    let mut last_reg = None;

    for stmt in &func.body {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let reg = builder.lower_expr(value);
                builder.locals.insert(name.clone(), reg);
                last_reg = None;
            }
            Stmt::Expr(expr) => {
                let reg = builder.lower_expr(expr);
                last_reg = Some(reg);
            }
        }
    }

    // Implicit return of last expression
    if let Some(reg) = last_reg {
        builder.emit(Instruction::Ret { value: reg });
    }

    IrFunction {
        name: func.name.clone(),
        params: func.params.iter().enumerate().map(|(i, _)| format!("arg{}", i)).collect(),
        body: builder.instructions,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p cyflym-ir`
Expected: All 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/cyflym-ir/
git commit -m "feat: implement IR lowering from AST to flat SSA instructions"
```

---

## Chunk 5: LLVM Codegen

### Task 7: Implement LLVM Code Generation

**Files:**
- Create: `crates/cyflym-codegen/Cargo.toml`
- Create: `crates/cyflym-codegen/src/lib.rs`

- [ ] **Step 1: Verify LLVM is installed**

Run: `brew --prefix llvm@17`
Expected: Path output (e.g., `/Users/sgordon/homebrew/opt/llvm@17`)

If not installed:
```bash
brew install llvm@17
echo 'export LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)' >> ~/.zshrc
source ~/.zshrc
```

- [ ] **Step 2: Create codegen crate**

Create `crates/cyflym-codegen/Cargo.toml`:

```toml
[package]
name = "cyflym-codegen"
version = "0.1.0"
edition = "2021"

[dependencies]
cyflym-ir = { path = "../cyflym-ir" }
inkwell = { version = "0.5", features = ["llvm17-0"] }
```

- [ ] **Step 3: Write failing tests for codegen**

Create `crates/cyflym-codegen/src/lib.rs`:

```rust
use cyflym_ir::*;
use inkwell::context::Context;

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

/// Compile an IR module to an LLVM object file at the given path.
pub fn compile_to_object(module: &Module, output_path: &str) -> Result<(), CodegenError> {
    todo!()
}

/// Compile an IR module to LLVM IR string (for testing).
pub fn compile_to_llvm_ir(module: &Module) -> Result<String, CodegenError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_ir::{Module, IrFunction, Instruction, IrBinOp};

    fn minimal_module() -> Module {
        Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 42,
                    },
                    Instruction::Ret {
                        value: "%0".to_string(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn codegen_produces_llvm_ir() {
        let module = minimal_module();
        let ir = compile_to_llvm_ir(&module).unwrap();
        assert!(ir.contains("define i64 @main()"));
        assert!(ir.contains("ret i64 42"));
    }

    #[test]
    fn codegen_arithmetic() {
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 1,
                    },
                    Instruction::Const {
                        dest: "%1".to_string(),
                        value: 2,
                    },
                    Instruction::BinOp {
                        dest: "%2".to_string(),
                        op: IrBinOp::Add,
                        left: "%0".to_string(),
                        right: "%1".to_string(),
                    },
                    Instruction::Ret {
                        value: "%2".to_string(),
                    },
                ],
            }],
        };
        let ir = compile_to_llvm_ir(&module).unwrap();
        assert!(ir.contains("add i64"));
        assert!(ir.contains("ret i64"));
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p cyflym-codegen`
Expected: Tests FAIL with "not yet implemented"

- [ ] **Step 5: Implement LLVM codegen**

Replace the `todo!()` implementations in `crates/cyflym-codegen/src/lib.rs`:

```rust
use cyflym_ir::*;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::builder::Builder;
use inkwell::values::{IntValue, FunctionValue};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::OptimizationLevel;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

pub fn compile_to_object(module: &Module, output_path: &str) -> Result<(), CodegenError> {
    let context = Context::create();
    let llvm_module = generate_llvm(&context, module)?;

    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| CodegenError { message: e.to_string() })?;

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| CodegenError { message: e.to_string() })?;
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| CodegenError {
            message: "failed to create target machine".to_string(),
        })?;

    machine
        .write_to_file(&llvm_module, FileType::Object, output_path.as_ref())
        .map_err(|e| CodegenError { message: e.to_string() })?;

    Ok(())
}

pub fn compile_to_llvm_ir(module: &Module) -> Result<String, CodegenError> {
    let context = Context::create();
    let llvm_module = generate_llvm(&context, module)?;
    Ok(llvm_module.print_to_string().to_string())
}

fn generate_llvm<'ctx>(
    context: &'ctx Context,
    module: &Module,
) -> Result<LlvmModule<'ctx>, CodegenError> {
    let llvm_module = context.create_module("cyflym");
    let builder = context.create_builder();
    let i64_type = context.i64_type();

    // First pass: declare all functions
    let mut fn_values: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
    for func in &module.functions {
        let param_types: Vec<_> = func.params.iter().map(|_| i64_type.into()).collect();
        let fn_type = i64_type.fn_type(&param_types, false);
        let fn_value = llvm_module.add_function(&func.name, fn_type, None);
        fn_values.insert(func.name.clone(), fn_value);
    }

    // Second pass: generate function bodies
    for func in &module.functions {
        let fn_value = fn_values[&func.name];
        let entry = context.append_basic_block(fn_value, "entry");
        builder.position_at_end(entry);

        let mut regs: HashMap<String, IntValue<'ctx>> = HashMap::new();

        // Map param names to LLVM param values
        for (i, param_name) in func.params.iter().enumerate() {
            let param_val = fn_value
                .get_nth_param(i as u32)
                .unwrap()
                .into_int_value();
            regs.insert(param_name.clone(), param_val);
        }

        for inst in &func.body {
            match inst {
                Instruction::Const { dest, value } => {
                    let val = i64_type.const_int(*value as u64, true);
                    regs.insert(dest.clone(), val);
                }
                Instruction::BinOp {
                    dest,
                    op,
                    left,
                    right,
                } => {
                    let lhs = regs[left];
                    let rhs = regs[right];
                    let result = match op {
                        IrBinOp::Add => builder.build_int_add(lhs, rhs, dest)
                            .map_err(|e| CodegenError { message: e.to_string() })?,
                        IrBinOp::Sub => builder.build_int_sub(lhs, rhs, dest)
                            .map_err(|e| CodegenError { message: e.to_string() })?,
                        IrBinOp::Mul => builder.build_int_mul(lhs, rhs, dest)
                            .map_err(|e| CodegenError { message: e.to_string() })?,
                        IrBinOp::Div => builder.build_int_signed_div(lhs, rhs, dest)
                            .map_err(|e| CodegenError { message: e.to_string() })?,
                    };
                    regs.insert(dest.clone(), result);
                }
                Instruction::Copy { dest, src } => {
                    let val = regs[src];
                    regs.insert(dest.clone(), val);
                }
                Instruction::Call {
                    dest,
                    function,
                    args,
                } => {
                    let callee = fn_values.get(function).ok_or_else(|| CodegenError {
                        message: format!("undefined function in codegen: {}", function),
                    })?;
                    let arg_vals: Vec<_> = args
                        .iter()
                        .map(|a| regs[a].into())
                        .collect();
                    let call_val = builder
                        .build_call(*callee, &arg_vals, dest)
                        .map_err(|e| CodegenError { message: e.to_string() })?;
                    let result = call_val
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();
                    regs.insert(dest.clone(), result);
                }
                Instruction::Ret { value } => {
                    let val = regs[value];
                    builder.build_return(Some(&val))
                        .map_err(|e| CodegenError { message: e.to_string() })?;
                }
            }
        }
    }

    Ok(llvm_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyflym_ir::{Module, IrFunction, Instruction, IrBinOp};

    fn minimal_module() -> Module {
        Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 42,
                    },
                    Instruction::Ret {
                        value: "%0".to_string(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn codegen_produces_llvm_ir() {
        let module = minimal_module();
        let ir = compile_to_llvm_ir(&module).unwrap();
        assert!(ir.contains("define i64 @main()"));
        assert!(ir.contains("ret i64 42"));
    }

    #[test]
    fn codegen_arithmetic() {
        let module = Module {
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![],
                body: vec![
                    Instruction::Const {
                        dest: "%0".to_string(),
                        value: 1,
                    },
                    Instruction::Const {
                        dest: "%1".to_string(),
                        value: 2,
                    },
                    Instruction::BinOp {
                        dest: "%2".to_string(),
                        op: IrBinOp::Add,
                        left: "%0".to_string(),
                        right: "%1".to_string(),
                    },
                    Instruction::Ret {
                        value: "%2".to_string(),
                    },
                ],
            }],
        };
        let ir = compile_to_llvm_ir(&module).unwrap();
        assert!(ir.contains("add i64"));
        assert!(ir.contains("ret i64"));
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p cyflym-codegen`
Expected: Both tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/cyflym-codegen/
git commit -m "feat: implement LLVM codegen from Cyflym IR"
```

---

## Chunk 6: Driver & End-to-End

### Task 8: Implement Driver CLI

**Files:**
- Create: `crates/cyflym-driver/Cargo.toml`
- Create: `crates/cyflym-driver/src/main.rs`

- [ ] **Step 1: Create driver crate**

Create `crates/cyflym-driver/Cargo.toml`:

```toml
[package]
name = "cyflym"
version = "0.1.0"
edition = "2021"

[dependencies]
cyflym-lexer = { path = "../cyflym-lexer" }
cyflym-parser = { path = "../cyflym-parser" }
cyflym-typeck = { path = "../cyflym-typeck" }
cyflym-ir = { path = "../cyflym-ir" }
cyflym-codegen = { path = "../cyflym-codegen" }
```

Create `crates/cyflym-driver/src/main.rs`:

```rust
use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: cyflym build <file.cy>");
        std::process::exit(1);
    }

    let command = &args[1];
    let input_path = &args[2];

    match command.as_str() {
        "build" => {
            if let Err(e) = build(input_path) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("unknown command: {}", other);
            std::process::exit(1);
        }
    }
}

fn build(input_path: &str) -> Result<(), String> {
    // Read source
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("cannot read {}: {}", input_path, e))?;

    // Parse
    let program = cyflym_parser::parse(&source)
        .map_err(|e| format!("parse error at {:?}: {}", e.span, e.message))?;

    // Type check
    cyflym_typeck::check(&program)
        .map_err(|e| format!("type error at {:?}: {}", e.span, e.message))?;

    // Lower to IR
    let ir_module = cyflym_ir::lower(&program);

    // Codegen to object file
    let obj_path = input_path.replace(".cy", ".o");
    cyflym_codegen::compile_to_object(&ir_module, &obj_path)
        .map_err(|e| format!("codegen error: {}", e.message))?;

    // Link with system linker
    let output_path = input_path.replace(".cy", "");
    let status = Command::new("cc")
        .args([&obj_path, "-o", &output_path])
        .status()
        .map_err(|e| format!("linker failed: {}", e))?;

    if !status.success() {
        return Err("linking failed".to_string());
    }

    // Clean up object file
    let _ = fs::remove_file(&obj_path);

    println!("Built: {}", output_path);
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p cyflym`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/cyflym-driver/
git commit -m "feat: implement driver CLI with build command"
```

---

### Task 9: End-to-End Test

**Files:**
- Create: `tests/fixtures/minimal.cy`
- Create: `tests/fixtures/let_binding.cy`
- Create: `tests/fixtures/function_call.cy`
- Create: `tests/fixtures/full_mvp.cy`

- [ ] **Step 1: Create test fixture files**

Create `tests/fixtures/minimal.cy`:

```
fn main() Int {
    42
}
```

Create `tests/fixtures/let_binding.cy`:

```
fn main() Int {
    let x Int = 10
    let y Int = x + 32
    y
}
```

Create `tests/fixtures/function_call.cy`:

```
fn add(a Int, b Int) Int {
    a + b
}

fn main() Int {
    add(20, 22)
}
```

Create `tests/fixtures/full_mvp.cy`:

```
fn add(a Int, b Int) Int {
    a + b
}

fn main() Int {
    let x Int = 42
    let y Int = x + 8
    let z Int = add(x, y)
    z
}
```

- [ ] **Step 2: Build the compiler**

Run: `cargo build --release -p cyflym`
Expected: Compiles successfully

- [ ] **Step 3: Compile and run minimal.cy**

Run:
```bash
./target/release/cyflym build tests/fixtures/minimal.cy
./tests/fixtures/minimal
echo $?
```
Expected: Exit code `42`

- [ ] **Step 4: Compile and run let_binding.cy**

Run:
```bash
./target/release/cyflym build tests/fixtures/let_binding.cy
./tests/fixtures/let_binding
echo $?
```
Expected: Exit code `42` (10 + 32 = 42)

- [ ] **Step 5: Compile and run function_call.cy**

Run:
```bash
./target/release/cyflym build tests/fixtures/function_call.cy
./tests/fixtures/function_call
echo $?
```
Expected: Exit code `42` (20 + 22 = 42)

- [ ] **Step 6: Compile and run full_mvp.cy**

Run:
```bash
./target/release/cyflym build tests/fixtures/full_mvp.cy
./tests/fixtures/full_mvp
echo $?
```
Expected: Exit code `92` (42 + 8 = 50, add(42, 50) = 92)

- [ ] **Step 7: Commit test fixtures and celebrate**

```bash
git add tests/
git commit -m "feat: add end-to-end test fixtures — MVP compiler works!"
```

---

## Summary

After completing all tasks, you will have:

1. **Lexer** (`cyflym-lexer`) — tokenizes `.cy` source into tokens
2. **Parser** (`cyflym-parser`) — parses tokens into an AST with operator precedence
3. **Type Checker** (`cyflym-typeck`) — validates types, catches undefined vars/functions
4. **IR** (`cyflym-ir`) — lowers AST to flat SSA-style intermediate representation
5. **Codegen** (`cyflym-codegen`) — compiles Cyflym IR to native code via LLVM
6. **Driver** (`cyflym-driver`) — `cyflym build` command that orchestrates the pipeline

The Cyflym compiler can compile programs with integer arithmetic, let bindings, functions, and function calls to native binaries. This is the foundation for all subsequent plans.

**Next plan:** Plan 2 will add strings, booleans, `if`/`else`, `while`, `return`, `mut`, type inference, and `print()`.
