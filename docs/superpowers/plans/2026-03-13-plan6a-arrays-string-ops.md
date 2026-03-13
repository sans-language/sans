# Plan 6a: Arrays & String Operations Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add dynamically-sized `Array<T>` with push/get/set/len, `for..in` iteration, string methods (len, concat, substring), and int/string conversion built-ins.

**Architecture:** New `array` and `in` keywords flow through lexer → parser → typeck → IR → codegen, following the established pattern from Plans 5a/5b. `for..in` is a new statement type that lowers to a counted loop in IR. String `+` extends the existing binary operator. All operations use dedicated IR instructions that codegen lowers to heap operations and C stdlib calls.

**Tech Stack:** Rust, inkwell 0.8 (llvm17-0), C stdlib (strlen, memcpy, snprintf, strtol)

**Spec:** `docs/superpowers/specs/2026-03-13-plan6a-arrays-string-ops-design.md`

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `crates/cyflym-lexer/src/token.rs` | Modify | Add `Array` and `In` token variants |
| `crates/cyflym-lexer/src/lib.rs` | Modify | Add `array` and `in` keyword mappings + 2 tests |
| `crates/cyflym-parser/src/ast.rs` | Modify | Add `ArrayCreate` expr, `ForIn` stmt |
| `crates/cyflym-parser/src/lib.rs` | Modify | Parse `array<T>()`, `for x in expr { }`, update `expr_span`, update `parse_block_body` + 5 tests |
| `crates/cyflym-typeck/src/types.rs` | Modify | Add `Array` type variant + Display impl |
| `crates/cyflym-typeck/src/lib.rs` | Modify | Type check array ops, string ops, for-in, String+String, int_to_string/string_to_int + 10 tests |
| `crates/cyflym-ir/src/ir.rs` | Modify | Add 10 new instructions (5 array + 5 string) |
| `crates/cyflym-ir/src/lib.rs` | Modify | Add `Array(Box<IrType>)` to IrType, lower all new ops, lower ForIn to counted loop + 4 tests |
| `crates/cyflym-codegen/src/lib.rs` | Modify | Codegen for array (24-byte struct), string ops (strlen/memcpy/snprintf/strtol), declare C functions + 3 tests |
| `crates/cyflym-driver/tests/e2e.rs` | Modify | Add 4 E2E test entries |
| `tests/fixtures/array_basic.cy` | Create | Array create/push/get/set/len test |
| `tests/fixtures/array_for_in.cy` | Create | For-in iteration test |
| `tests/fixtures/string_ops.cy` | Create | String len/concat/substring test |
| `tests/fixtures/string_conversion.cy` | Create | int_to_string/string_to_int test |

---

## Chunk 1: Lexer, Parser, AST

### Task 1: Lexer — Add `array` and `in` keywords

**Files:**
- Modify: `crates/cyflym-lexer/src/token.rs:64`
- Modify: `crates/cyflym-lexer/src/lib.rs:84` (keyword map) and tests

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-lexer/src/lib.rs`, add after the `lex_mutex_keyword` test (line ~427):

```rust
    #[test]
    fn lex_array_keyword() {
        let tokens = lex("array").unwrap();
        assert_eq!(kinds(&tokens), vec![Array, Eof]);
    }

    #[test]
    fn lex_in_keyword() {
        let tokens = lex("in").unwrap();
        assert_eq!(kinds(&tokens), vec![In, Eof]);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-lexer -- lex_array_keyword lex_in_keyword`
Expected: FAIL — `Array` and `In` variants don't exist

- [ ] **Step 3: Add `Array` and `In` token variants**

In `crates/cyflym-lexer/src/token.rs`, add after `Mutex,` (line 64):

```rust
    Array,
    In,
```

- [ ] **Step 4: Add keyword mappings**

In `crates/cyflym-lexer/src/lib.rs`, add after `"mutex" => TokenKind::Mutex,` (line 84):

```rust
                    "array" => TokenKind::Array,
                    "in" => TokenKind::In,
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-lexer`
Expected: All lexer tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/cyflym-lexer/src/token.rs crates/cyflym-lexer/src/lib.rs
git commit -m "feat(lexer): add array and in keyword tokens"
```

---

### Task 2: Parser & AST — Add `ArrayCreate` expression, `ForIn` statement, parse `array<T>()` and `for x in expr { }`

**Files:**
- Modify: `crates/cyflym-parser/src/ast.rs:186-189` (Expr), `96-130` (Stmt)
- Modify: `crates/cyflym-parser/src/lib.rs:362-371` (parse_stmt), `716-739` (atom parsing), `919-937` (expr_span), `861-875` (parse_block_body)

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-parser/src/lib.rs`, add after the `parse_lock_unlock_method_calls` test (line ~1530):

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-parser -- parse_array_create parse_for_in parse_array_push parse_int_to_string parse_string_concat`
Expected: FAIL — `ArrayCreate` and `ForIn` variants don't exist

- [ ] **Step 3: Update AST — Add ArrayCreate and ForIn**

In `crates/cyflym-parser/src/ast.rs`, add after the `MutexCreate` variant (line ~189):

```rust
    ArrayCreate {
        element_type: TypeName,
        span: Span,
    },
```

And add to the `Stmt` enum after the `LetDestructure` variant (line ~128):

```rust
    ForIn {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
```

- [ ] **Step 4: Update `expr_span` function**

In `crates/cyflym-parser/src/lib.rs`, in the `expr_span` function (around line 935), add after the `MutexCreate` match arm:

```rust
        Expr::ArrayCreate { span, .. } => span,
```

- [ ] **Step 5: Add `array<T>()` parser**

In `crates/cyflym-parser/src/lib.rs`, add a new arm before the `TokenKind::Mutex` arm (around line 716):

```rust
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
```

- [ ] **Step 6: Add `for x in expr { body }` parser**

In `crates/cyflym-parser/src/lib.rs`, in the `parse_stmt` function (around line 362), add a new branch. The existing code checks for `If`, `While`, `Let`, `Return`. Add before the expression fallback:

```rust
        } else if self.peek().kind == TokenKind::For {
            let start = self.peek().span.start;
            self.pos += 1;
            let var_tok = self.peek().clone();
            if var_tok.kind != TokenKind::Ident {
                return Err(self.error("expected variable name after 'for'"));
            }
            let var = match &var_tok.kind {
                TokenKind::Ident => var_tok.text.clone(),
                _ => unreachable!(),
            };
            self.pos += 1;
            self.expect(&TokenKind::In)?;
            let iterable = self.parse_expr(0)?;
            self.expect(&TokenKind::LBrace)?;
            let body = self.parse_block_body()?;
            self.expect(&TokenKind::RBrace)?;
            let span = start..self.tokens[self.pos - 1].span.end;
            Ok(Stmt::ForIn { var, iterable, body, span })
```

- [ ] **Step 7: Update `parse_block_body` for ForIn**

In `crates/cyflym-parser/src/lib.rs`, the `parse_block_body` function (around line 861) checks if the last statement is a non-expression statement (Let, While, etc.) and rejects it. Add `ForIn` to this check alongside `While`. Find the match arm that lists statement variants and add `Stmt::ForIn { .. }`.

- [ ] **Step 8: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-parser`
Expected: All parser tests PASS (existing + 5 new)

- [ ] **Step 9: Commit**

```bash
git add crates/cyflym-parser/src/ast.rs crates/cyflym-parser/src/lib.rs
git commit -m "feat(parser): add array<T>() and for-in syntax"
```

---

## Chunk 2: Type System

### Task 3: Type System — Add `Array` type, type check array methods, string ops, for-in, String+String, built-in conversions

**Files:**
- Modify: `crates/cyflym-typeck/src/types.rs:12,26`
- Modify: `crates/cyflym-typeck/src/lib.rs:72-175` (check_stmt), `429-447` (BinaryOp), `528` (Call/print), `824-888` (ChannelCreate/MutexCreate/MethodCall), `809` (spawn compat)

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-typeck/src/lib.rs`, add after the last test (line ~1381):

```rust
    #[test]
    fn check_array_create() {
        assert!(do_check("fn main() Int { let a = array<Int>() 0 }").is_ok());
    }

    #[test]
    fn check_array_push_matching_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) 0 }").is_ok());
    }

    #[test]
    fn check_array_push_wrong_type() {
        let err = do_check("fn main() Int { let a = array<Int>() a.push(true) 0 }").unwrap_err();
        assert!(err.message.contains("mismatch") || err.message.contains("type"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_array_get_returns_element_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) a.get(0) }").is_ok());
    }

    #[test]
    fn check_array_len_returns_int() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.len() }").is_ok());
    }

    #[test]
    fn check_for_in_binds_element_type() {
        assert!(do_check("fn main() Int { let a = array<Int>() a.push(1) for x in a { print(x) } 0 }").is_ok());
    }

    #[test]
    fn check_for_in_non_array_error() {
        let err = do_check("fn main() Int { for x in 42 { print(x) } 0 }").unwrap_err();
        assert!(err.message.contains("Array") || err.message.contains("for"),
            "expected for-in type error, got: {}", err.message);
    }

    #[test]
    fn check_string_len() {
        assert!(do_check(r#"fn main() Int { let s = "hello" s.len() }"#).is_ok());
    }

    #[test]
    fn check_string_concat() {
        assert!(do_check(r#"fn main() Int { let s = "a" + "b" 0 }"#).is_ok());
    }

    #[test]
    fn check_string_plus_int_error() {
        let err = do_check(r#"fn main() Int { let s = "a" + 1 0 }"#).unwrap_err();
        assert!(err.message.contains("type") || err.message.contains("mismatch") || err.message.contains("operand"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_int_to_string() {
        assert!(do_check(r#"fn main() Int { let s = int_to_string(42) 0 }"#).is_ok());
    }

    #[test]
    fn check_string_to_int() {
        assert!(do_check(r#"fn main() Int { string_to_int("42") }"#).is_ok());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-typeck -- check_array check_for_in check_string check_int_to`
Expected: FAIL — `Array` variant doesn't exist, `ArrayCreate` not handled

- [ ] **Step 3: Add `Array` type variant**

In `crates/cyflym-typeck/src/types.rs`, add after `Mutex { inner: Box<Type> },` (line 12):

```rust
    Array { inner: Box<Type> },
```

And in the Display impl, add before the `Type::Fn` arm:

```rust
            Type::Array { inner } => write!(f, "Array<{}>", inner),
```

- [ ] **Step 4: Add `ArrayCreate` expression type checking**

In `crates/cyflym-typeck/src/lib.rs`, add after the `Expr::MutexCreate` arm (around line 840):

```rust
        Expr::ArrayCreate { element_type, .. } => {
            let inner = resolve_type(&element_type.name, structs, enums)?;
            Ok(Type::Array { inner: Box::new(inner) })
        }
```

- [ ] **Step 5: Add array method type checking**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::MethodCall` match (around line 877), add after the `Mutex` / `unlock` arm:

```rust
                (Type::Array { inner }, "push") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("push() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!(
                            "push() type mismatch: array holds {} but got {}", inner, arg_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { inner }, "get") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("get() takes exactly 1 argument"));
                    }
                    let idx_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if idx_ty != Type::Int {
                        return Err(TypeError::new(format!("get() index must be Int, got {}", idx_ty)));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Array { inner }, "set") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("set() takes exactly 2 arguments (index, value)"));
                    }
                    let idx_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if idx_ty != Type::Int {
                        return Err(TypeError::new(format!("set() index must be Int, got {}", idx_ty)));
                    }
                    let val_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if val_ty != **inner {
                        return Err(TypeError::new(format!(
                            "set() type mismatch: array holds {} but got {}", inner, val_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
                (Type::Array { .. }, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
```

- [ ] **Step 6: Add String method type checking**

In the same `Expr::MethodCall` match, add:

```rust
                (Type::String, "len") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("len() takes no arguments"));
                    }
                    return Ok(Type::Int);
                }
                (Type::String, "substring") => {
                    if args.len() != 2 {
                        return Err(TypeError::new("substring() takes exactly 2 arguments (start, end)"));
                    }
                    let start_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if start_ty != Type::Int {
                        return Err(TypeError::new(format!("substring() start must be Int, got {}", start_ty)));
                    }
                    let end_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if end_ty != Type::Int {
                        return Err(TypeError::new(format!("substring() end must be Int, got {}", end_ty)));
                    }
                    return Ok(Type::String);
                }
```

- [ ] **Step 7: Update BinaryOp::Add for String + String**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::BinaryOp` match for `BinOp::Add` (around line 436), the current code checks that both operands are Int. Replace the Add arm to also allow String + String:

```rust
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    if op == &BinOp::Add && left_type == Type::String && right_type == Type::String {
                        return Ok(Type::String);
                    }
                    if left_type != Type::Int || right_type != Type::Int {
                        return Err(TypeError::new(format!(
                            "arithmetic operator requires Int operands, got {} and {}", left_type, right_type
                        )));
                    }
                    Ok(Type::Int)
                }
```

- [ ] **Step 8: Add `int_to_string` and `string_to_int` built-in type checking**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::Call` match (around line 528), the existing code special-cases `print`. Add two more special cases after the print handling:

```rust
            "int_to_string" => {
                if args.len() != 1 {
                    return Err(TypeError::new("int_to_string() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                if arg_ty != Type::Int {
                    return Err(TypeError::new(format!("int_to_string() requires Int argument, got {}", arg_ty)));
                }
                return Ok(Type::String);
            }
            "string_to_int" => {
                if args.len() != 1 {
                    return Err(TypeError::new("string_to_int() takes exactly 1 argument"));
                }
                let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                if arg_ty != Type::String {
                    return Err(TypeError::new(format!("string_to_int() requires String argument, got {}", arg_ty)));
                }
                return Ok(Type::Int);
            }
```

- [ ] **Step 9: Add `ForIn` to `check_stmt`**

In `crates/cyflym-typeck/src/lib.rs`, in the `check_stmt` function match (around line 84), add a new arm for `ForIn`:

```rust
        Stmt::ForIn { var, iterable, body, .. } => {
            let iter_ty = check_expr(iterable, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
            match iter_ty {
                Type::Array { inner } => {
                    let mut loop_locals = locals.clone();
                    loop_locals.insert(var.clone(), (*inner, false));
                    for stmt in body {
                        check_stmt(stmt, &mut loop_locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    }
                    Ok(())
                }
                _ => Err(TypeError::new(format!("for-in requires Array, got {}", iter_ty))),
            }
        }
```

- [ ] **Step 10: Update spawn compatibility for Array type**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::Spawn` arm (around line 809), update the compatibility check to also allow `Array`:

```rust
                    let compatible = actual == *expected
                        || (*expected == Type::Int && matches!(actual, Type::Sender { .. } | Type::Receiver { .. } | Type::JoinHandle | Type::Mutex { .. } | Type::Array { .. }));
```

- [ ] **Step 11: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-typeck`
Expected: All typeck tests PASS (existing + 12 new)

- [ ] **Step 12: Commit**

```bash
git add crates/cyflym-typeck/src/types.rs crates/cyflym-typeck/src/lib.rs
git commit -m "feat(typeck): add Array type, for-in, string ops, String+String, int/string conversion"
```

---

## Chunk 3: IR

### Task 4: IR — Add array and string instructions, `Array(Box<IrType>)`, lower ForIn to counted loop

**Files:**
- Modify: `crates/cyflym-ir/src/ir.rs:83-87`
- Modify: `crates/cyflym-ir/src/lib.rs:9` (IrType), `183-278` (BinaryOp), `357` (Call/print), `450-503` (MethodCall), `653` (MutexCreate), `666-768` (lower_stmt), `85-105` (lower_function last stmt)

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-ir/src/lib.rs`, add after the last test (line ~1074):

```rust
    #[test]
    fn lower_array_create_push_get_len() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(5) a.get(0) }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayCreate { .. })),
            "expected ArrayCreate instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayPush { .. })),
            "expected ArrayPush instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayGet { .. })),
            "expected ArrayGet instruction");
    }

    #[test]
    fn lower_for_in_to_counted_loop() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(1) for x in a { print(x) } 0 }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayLen { .. })),
            "expected ArrayLen for for-in loop");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ArrayGet { .. })),
            "expected ArrayGet for for-in loop");
    }

    #[test]
    fn lower_string_concat() {
        let prog = cyflym_parser::parse(
            r#"fn main() Int { let s = "a" + "b" 0 }"#
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::StringConcat { .. })),
            "expected StringConcat instruction");
    }

    #[test]
    fn lower_int_to_string_and_string_to_int() {
        let prog = cyflym_parser::parse(
            r#"fn main() Int { let s = int_to_string(42) string_to_int(s) }"#
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::IntToString { .. })),
            "expected IntToString instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::StringToInt { .. })),
            "expected StringToInt instruction");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-ir -- lower_array lower_for_in lower_string lower_int_to`
Expected: FAIL — new instruction variants don't exist

- [ ] **Step 3: Add IR instructions**

In `crates/cyflym-ir/src/ir.rs`, add after `ChannelCreateBounded` (line ~87):

```rust
    // Array operations
    ArrayCreate {
        dest: Reg,
        element_type: IrType,
    },
    ArrayPush {
        array: Reg,
        value: Reg,
    },
    ArrayGet {
        dest: Reg,
        array: Reg,
        index: Reg,
    },
    ArraySet {
        array: Reg,
        index: Reg,
        value: Reg,
    },
    ArrayLen {
        dest: Reg,
        array: Reg,
    },
    // String operations
    StringLen {
        dest: Reg,
        string: Reg,
    },
    StringConcat {
        dest: Reg,
        left: Reg,
        right: Reg,
    },
    StringSubstring {
        dest: Reg,
        string: Reg,
        start: Reg,
        end: Reg,
    },
    IntToString {
        dest: Reg,
        value: Reg,
    },
    StringToInt {
        dest: Reg,
        string: Reg,
    },
```

Note: `IrType` must be made `pub` in `crates/cyflym-ir/src/lib.rs` (or moved to `ir.rs`) since `ArrayCreate` references it. The simplest approach: move the `IrType` enum definition from `lib.rs` to `ir.rs` and re-export it, or just make the existing `IrType` pub and import it in `ir.rs`. Check existing visibility — if `IrType` is already in `lib.rs` and `Instruction` is in `ir.rs`, the `ArrayCreate` field needs access. The pragmatic fix: change `ArrayCreate` to store a `String` element type name instead of `IrType`, or define `IrType` in `ir.rs`. Follow whichever pattern is simpler given the existing code.

**Alternative (recommended):** Store the element type as a simple boolean or enum variant flag in the instruction, or better yet, track it only in `reg_types` HashMap (not in the instruction itself). Change `ArrayCreate` to just `{ dest: Reg }` and have the IR lowering set `reg_types[dest] = IrType::Array(element_ir_type)`. This avoids cross-module type references. The `ArrayGet` lowering then looks up the array's `IrType::Array(inner)` to determine the result register's type.

Use this approach:

```rust
    ArrayCreate {
        dest: Reg,
    },
```

And in the IR lowering, track the element type via `reg_types`:
```rust
self.reg_types.insert(dest.clone(), IrType::Array(Box::new(element_ir_type)));
```

- [ ] **Step 4: Update `IrType`**

In `crates/cyflym-ir/src/lib.rs`, update line 9:

```rust
enum IrType { Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex, Array(Box<IrType>) }
```

- [ ] **Step 5: Add `ArrayCreate` expression lowering**

In `crates/cyflym-ir/src/lib.rs`, add after the `Expr::MutexCreate` arm (around line 662):

```rust
            Expr::ArrayCreate { element_type, .. } => {
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::ArrayCreate {
                    dest: dest.clone(),
                });
                let inner_ir_type = match element_type.name.as_str() {
                    "Int" => IrType::Int,
                    "Bool" => IrType::Bool,
                    "String" => IrType::Str,
                    other => IrType::Struct(other.to_string()),
                };
                self.reg_types.insert(dest.clone(), IrType::Array(Box::new(inner_ir_type)));
                dest
            }
```

- [ ] **Step 6: Add array and string method lowering**

In `crates/cyflym-ir/src/lib.rs`, in the `Expr::MethodCall` match (around line 493), add after the `Mutex`/`unlock` arm:

```rust
                    (Some(IrType::Array(inner)), "push") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::ArrayPush {
                            array: obj_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Array(inner)), "get") => {
                        let idx_reg = self.lower_expr(&args[0]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayGet {
                            dest: dest.clone(),
                            array: obj_reg,
                            index: idx_reg,
                        });
                        self.reg_types.insert(dest.clone(), inner.as_ref().clone());
                        return dest;
                    }
                    (Some(IrType::Array(_)), "set") => {
                        let idx_reg = self.lower_expr(&args[0]);
                        let val_reg = self.lower_expr(&args[1]);
                        self.instructions.push(Instruction::ArraySet {
                            array: obj_reg,
                            index: idx_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Array(_)), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::ArrayLen {
                            dest: dest.clone(),
                            array: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Str), "len") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringLen {
                            dest: dest.clone(),
                            string: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Str), "substring") => {
                        let start_reg = self.lower_expr(&args[0]);
                        let end_reg = self.lower_expr(&args[1]);
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::StringSubstring {
                            dest: dest.clone(),
                            string: obj_reg,
                            start: start_reg,
                            end: end_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Str);
                        return dest;
                    }
```

- [ ] **Step 7: Update BinaryOp::Add lowering for String concat**

In `crates/cyflym-ir/src/lib.rs`, in the `lower_expr` BinaryOp handling (around line 262), after lowering both operands, check if they are strings:

```rust
            Expr::BinaryOp { op, left, right, .. } => {
                let left_reg = self.lower_expr(left);
                let right_reg = self.lower_expr(right);
                // Check for String + String → StringConcat
                if matches!(op, BinOp::Add) && self.reg_types.get(&left_reg) == Some(&IrType::Str) {
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringConcat {
                        dest: dest.clone(),
                        left: left_reg,
                        right: right_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }
                let ir_op = match op {
                    // ... existing match arms ...
```

- [ ] **Step 8: Add `int_to_string` and `string_to_int` call lowering**

In `crates/cyflym-ir/src/lib.rs`, in the `Expr::Call` match (around line 357), after the `print` special case, add:

```rust
                "int_to_string" => {
                    let val_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::IntToString {
                        dest: dest.clone(),
                        value: val_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Str);
                    return dest;
                }
                "string_to_int" => {
                    let str_reg = self.lower_expr(&args[0]);
                    let dest = self.fresh_reg();
                    self.instructions.push(Instruction::StringToInt {
                        dest: dest.clone(),
                        string: str_reg,
                    });
                    self.reg_types.insert(dest.clone(), IrType::Int);
                    return dest;
                }
```

- [ ] **Step 9: Add `ForIn` to `lower_stmt`**

In `crates/cyflym-ir/src/lib.rs`, in the `lower_stmt` function match (around line 666), add:

```rust
            Stmt::ForIn { var, iterable, body, .. } => {
                let arr_reg = self.lower_expr(iterable);
                // len = ArrayLen(arr)
                let len_reg = self.fresh_reg();
                self.instructions.push(Instruction::ArrayLen {
                    dest: len_reg.clone(),
                    array: arr_reg.clone(),
                });
                self.reg_types.insert(len_reg.clone(), IrType::Int);
                // idx = 0
                let idx_reg = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: idx_reg.clone(), value: 0 });
                self.reg_types.insert(idx_reg.clone(), IrType::Int);
                // Determine element IrType from array's IrType
                let elem_ir_type = match self.reg_types.get(&arr_reg) {
                    Some(IrType::Array(inner)) => inner.as_ref().clone(),
                    _ => IrType::Int, // fallback
                };
                // Emit loop structure using existing While-like pattern
                // We use the same alloca-based loop pattern as ForIn spec:
                // Store idx to alloca, loop: load, compare, branch, body, increment, branch back
                // However, IR doesn't have alloca — we use a mutable register approach.
                // Actually, looking at the existing While lowering pattern, it uses
                // basic blocks with Branch instructions. Let's follow that pattern.
                let loop_label = format!("forin_loop_{}", self.fresh_reg().0.replace('%', ""));
                let body_label = format!("forin_body_{}", self.fresh_reg().0.replace('%', ""));
                let done_label = format!("forin_done_{}", self.fresh_reg().0.replace('%', ""));

                self.instructions.push(Instruction::Branch { label: loop_label.clone() });
                self.instructions.push(Instruction::Label { name: loop_label.clone() });

                // Compare idx < len
                let cmp_reg = self.fresh_reg();
                self.instructions.push(Instruction::Compare {
                    dest: cmp_reg.clone(),
                    op: CmpOp::Lt,
                    left: idx_reg.clone(),
                    right: len_reg.clone(),
                });

                self.instructions.push(Instruction::BranchIf {
                    cond: cmp_reg,
                    then_label: body_label.clone(),
                    else_label: done_label.clone(),
                });
                self.instructions.push(Instruction::Label { name: body_label.clone() });

                // x = ArrayGet(arr, idx)
                let elem_reg = self.fresh_reg();
                self.instructions.push(Instruction::ArrayGet {
                    dest: elem_reg.clone(),
                    array: arr_reg.clone(),
                    index: idx_reg.clone(),
                });
                self.reg_types.insert(elem_reg.clone(), elem_ir_type);
                self.locals.insert(var.clone(), LocalVar::Value(elem_reg));

                // Lower body
                for stmt in body {
                    self.lower_stmt(stmt);
                }

                // idx = idx + 1
                let one_reg = self.fresh_reg();
                self.instructions.push(Instruction::Const { dest: one_reg.clone(), value: 1 });
                let next_idx = self.fresh_reg();
                self.instructions.push(Instruction::BinOp {
                    dest: next_idx.clone(),
                    op: IrBinOp::Add,
                    left: idx_reg.clone(),
                    right: one_reg,
                });
                self.reg_types.insert(next_idx.clone(), IrType::Int);

                // Overwrite idx_reg with next value — but IR is SSA, so we need to
                // update the register used in the next loop iteration.
                // Since our IR uses string-named registers and the codegen handles them
                // via a HashMap, we can reuse the idx_reg name by emitting a Copy.
                self.instructions.push(Instruction::Copy {
                    dest: idx_reg.clone(),
                    src: next_idx,
                });

                self.instructions.push(Instruction::Branch { label: loop_label });
                self.instructions.push(Instruction::Label { name: done_label });
            }
```

**IMPORTANT NOTE:** The above ForIn lowering uses `Instruction::Copy`, `Instruction::Label`, `Instruction::Branch`, `Instruction::BranchIf`, and `Instruction::Compare` — these must already exist in the IR from the `While` loop implementation. Check `ir.rs` for these variants. If any are missing or named differently, adapt the code to match existing IR instruction names. Also check how `While` is lowered in `lower_stmt` and follow the exact same pattern for control flow.

- [ ] **Step 10: Update `lower_function` last-statement check for ForIn**

In `crates/cyflym-ir/src/lib.rs`, in the `lower_function` last-statement check (around line 85-105), add `Stmt::ForIn { .. }` alongside `Stmt::While { .. }` in whatever match arm handles non-expression statements.

- [ ] **Step 11: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-ir`
Expected: All IR tests PASS (existing + 4 new)

- [ ] **Step 12: Commit**

```bash
git add crates/cyflym-ir/src/ir.rs crates/cyflym-ir/src/lib.rs
git commit -m "feat(ir): add array/string instructions and ForIn lowering"
```

---

## Chunk 4: Codegen & E2E

### Task 5: Codegen — Array operations, string operations, C function declarations

**Files:**
- Modify: `crates/cyflym-codegen/src/lib.rs:82-110` (C declarations), `890-951` (after mutex codegen)

This is the largest task. It adds:
1. C function declarations: `strlen`, `memcpy`, `snprintf`, `strtol`
2. ArrayCreate (24-byte struct: data ptr, len, capacity)
3. ArrayPush (with grow: memcpy to 2× buffer)
4. ArrayGet, ArraySet, ArrayLen
5. StringLen, StringConcat, StringSubstring, IntToString, StringToInt

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-codegen/src/lib.rs`, add after the last codegen test:

```rust
    #[test]
    fn codegen_array_create_push_get_len() {
        let program = cyflym_parser::parse(
            "fn main() Int { let a = array<Int>() a.push(5) a.get(0) }"
        ).unwrap();
        cyflym_typeck::check(&program).unwrap();
        let ir = cyflym_ir::lower(&program);
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_string_concat() {
        let program = cyflym_parser::parse(
            r#"fn main() Int { let s = "hello" + " world" 0 }"#
        ).unwrap();
        cyflym_typeck::check(&program).unwrap();
        let ir = cyflym_ir::lower(&program);
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_int_to_string_and_string_to_int() {
        let program = cyflym_parser::parse(
            r#"fn main() Int { let s = int_to_string(42) string_to_int(s) }"#
        ).unwrap();
        cyflym_typeck::check(&program).unwrap();
        let ir = cyflym_ir::lower(&program);
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-codegen -- codegen_array codegen_string codegen_int_to`
Expected: FAIL — no match for `ArrayCreate` / `StringConcat` / etc in codegen

- [ ] **Step 3: Add C function declarations**

In `crates/cyflym-codegen/src/lib.rs`, after the existing `free` declaration (around line 110), add:

```rust
                // strlen
                let strlen_type = i64_type.fn_type(&[ptr_type.into()], false);
                llvm_module.add_function("strlen", strlen_type, None);

                // memcpy
                let memcpy_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
                llvm_module.add_function("memcpy", memcpy_type, None);

                // snprintf (variadic)
                let snprintf_type = i64_type.fn_type(&[ptr_type.into(), i64_type.into(), ptr_type.into()], true);
                llvm_module.add_function("snprintf", snprintf_type, None);

                // strtol
                let strtol_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
                llvm_module.add_function("strtol", strtol_type, None);
```

- [ ] **Step 4: Add ArrayCreate codegen**

Add a new match arm in the instruction codegen (after the MutexUnlock arm):

```rust
                Instruction::ArrayCreate { dest } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();

                    // Allocate array struct: 3 * 8 = 24 bytes (data, len, capacity)
                    let arr_call = builder.build_call(malloc_fn, &[i64_type.const_int(24, false).into()], "arr")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let arr_ptr = match arr_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate initial buffer: 8 * 8 = 64 bytes (capacity 8)
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(64, false).into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // data = buf_ptr at offset 0
                    let buf_int = builder.build_ptr_to_int(buf_ptr, i64_type, "bi")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(arr_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // len = 0 at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(1, false)], "len_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // capacity = 8 at offset 2
                    let off2 = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(2, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(8, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let arr_int = builder.build_ptr_to_int(arr_ptr, i64_type, "arr_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), arr_int);
                }
```

- [ ] **Step 5: Add ArrayPush codegen (with grow)**

```rust
                Instruction::ArrayPush { array, value } => {
                    let arr_int = regs[array];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "ap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load len and capacity
                    let len_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(1, false)], "lp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = builder.build_load(i64_type, len_ptr, "len").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(2, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap = builder.build_load(i64_type, cap_ptr, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Check if full
                    let full = builder.build_int_compare(IntPredicate::EQ, len, cap, "full")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let grow_bb = context.append_basic_block(llvm_fn, "arr_grow");
                    let write_bb = context.append_basic_block(llvm_fn, "arr_write");
                    builder.build_conditional_branch(full, grow_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Grow: malloc 2x, memcpy, free old
                    builder.position_at_end(grow_bb);
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();
                    let free_fn = llvm_module.get_function("free").unwrap();
                    let two = i64_type.const_int(2, false);
                    let new_cap = builder.build_int_mul(cap, two, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let eight = i64_type.const_int(8, false);
                    let new_size = builder.build_int_mul(new_cap, eight, "ns").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_call = builder.build_call(malloc_fn, &[new_size.into()], "nb").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_ptr = match new_buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };
                    // memcpy(new_buf, old_buf, len * 8)
                    let old_buf_int = builder.build_load(i64_type, arr_ptr, "obi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let old_buf_ptr = builder.build_int_to_ptr(old_buf_int, ptr_type, "obp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let copy_size = builder.build_int_mul(len, eight, "cs").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(memcpy_fn, &[new_buf_ptr.into(), old_buf_ptr.into(), copy_size.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Free old
                    builder.build_call(free_fn, &[old_buf_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Update data ptr and capacity
                    let new_buf_int = builder.build_ptr_to_int(new_buf_ptr, i64_type, "nbi").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(arr_ptr, new_buf_int).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cap_ptr, new_cap).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(write_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Write value at data[len]
                    builder.position_at_end(write_bb);
                    let cur_len = builder.build_load(i64_type, len_ptr, "cl").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cur_buf_int = builder.build_load(i64_type, arr_ptr, "cbi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cur_buf_ptr = builder.build_int_to_ptr(cur_buf_int, ptr_type, "cbp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let wp = unsafe { builder.build_gep(i64_type, cur_buf_ptr, &[cur_len], "wp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(wp, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Increment len
                    let one = i64_type.const_int(1, false);
                    let new_len = builder.build_int_add(cur_len, one, "nl").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(len_ptr, new_len).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
```

- [ ] **Step 6: Add ArrayGet, ArraySet, ArrayLen codegen**

```rust
                Instruction::ArrayGet { dest, array, index } => {
                    let arr_int = regs[array];
                    let idx = regs[index];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "ap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_int = builder.build_load(i64_type, arr_ptr, "bi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ep = unsafe { builder.build_gep(i64_type, buf_ptr, &[idx], "ep") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let val = builder.build_load(i64_type, ep, dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), val);
                }
                Instruction::ArraySet { array, index, value } => {
                    let arr_int = regs[array];
                    let idx = regs[index];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "ap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_int = builder.build_load(i64_type, arr_ptr, "bi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let ep = unsafe { builder.build_gep(i64_type, buf_ptr, &[idx], "ep") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(ep, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
                Instruction::ArrayLen { dest, array } => {
                    let arr_int = regs[array];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let arr_ptr = builder.build_int_to_ptr(arr_int, ptr_type, "ap")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len_ptr = unsafe { builder.build_gep(i64_type, arr_ptr, &[i64_type.const_int(1, false)], "lp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = builder.build_load(i64_type, len_ptr, dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), len);
                }
```

- [ ] **Step 7: Add StringLen, StringConcat, StringSubstring codegen**

```rust
                Instruction::StringLen { dest, string } => {
                    let str_int = regs[string];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = builder.build_int_to_ptr(str_int, ptr_type, "sp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();
                    let len_call = builder.build_call(strlen_fn, &[str_ptr.into()], "slen")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len = match len_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(),
                        _ => return Err(CodegenError::LlvmError("strlen returned void".into())),
                    };
                    regs.insert(dest.clone(), len);
                }
                Instruction::StringConcat { dest, left, right } => {
                    let left_int = regs[left];
                    let right_int = regs[right];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let left_ptr = builder.build_int_to_ptr(left_int, ptr_type, "lp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let right_ptr = builder.build_int_to_ptr(right_int, ptr_type, "rp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let strlen_fn = llvm_module.get_function("strlen").unwrap();
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();
                    // Get lengths
                    let len1_call = builder.build_call(strlen_fn, &[left_ptr.into()], "l1").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len1 = match len1_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(), _ => return Err(CodegenError::LlvmError("strlen void".into())) };
                    let len2_call = builder.build_call(strlen_fn, &[right_ptr.into()], "l2").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len2 = match len2_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(), _ => return Err(CodegenError::LlvmError("strlen void".into())) };
                    // malloc(len1 + len2 + 1)
                    let total = builder.build_int_add(len1, len2, "tl").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let total_plus1 = builder.build_int_add(total, i64_type.const_int(1, false), "tp1").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[total_plus1.into()], "cb").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(), _ => return Err(CodegenError::LlvmError("malloc void".into())) };
                    // memcpy(buf, left, len1)
                    builder.build_call(memcpy_fn, &[buf_ptr.into(), left_ptr.into(), len1.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // memcpy(buf + len1, right, len2)
                    let mid = unsafe { builder.build_gep(context.i8_type(), buf_ptr, &[len1], "mid") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(memcpy_fn, &[mid.into(), right_ptr.into(), len2.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // null terminate: buf[len1+len2] = 0
                    let end = unsafe { builder.build_gep(context.i8_type(), buf_ptr, &[total], "end") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(end, context.i8_type().const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_ptr_to_int(buf_ptr, i64_type, "ci").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::StringSubstring { dest, string, start, end } => {
                    let str_int = regs[string];
                    let start_val = regs[start];
                    let end_val = regs[end];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = builder.build_int_to_ptr(str_int, ptr_type, "sp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let memcpy_fn = llvm_module.get_function("memcpy").unwrap();
                    // length = end - start
                    let length = builder.build_int_sub(end_val, start_val, "slen").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let len_plus1 = builder.build_int_add(length, i64_type.const_int(1, false), "slp1").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // malloc(length + 1)
                    let buf_call = builder.build_call(malloc_fn, &[len_plus1.into()], "sb").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(), _ => return Err(CodegenError::LlvmError("malloc void".into())) };
                    // src = str_ptr + start
                    let src = unsafe { builder.build_gep(context.i8_type(), str_ptr, &[start_val], "src") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // memcpy(buf, src, length)
                    builder.build_call(memcpy_fn, &[buf_ptr.into(), src.into(), length.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // null terminate
                    let term = unsafe { builder.build_gep(context.i8_type(), buf_ptr, &[length], "term") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(term, context.i8_type().const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_ptr_to_int(buf_ptr, i64_type, "si").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
```

- [ ] **Step 8: Add IntToString and StringToInt codegen**

```rust
                Instruction::IntToString { dest, value } => {
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let snprintf_fn = llvm_module.get_function("snprintf").unwrap();
                    // malloc(21) — max i64 decimal digits + sign + null
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(21, false).into()], "ib")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(), _ => return Err(CodegenError::LlvmError("malloc void".into())) };
                    // Format string "%ld"
                    let fmt = builder.build_global_string_ptr("%ld", "itsfmt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // snprintf(buf, 21, "%ld", val)
                    builder.build_call(snprintf_fn, &[buf_ptr.into(), i64_type.const_int(21, false).into(), fmt.as_pointer_value().into(), val.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = builder.build_ptr_to_int(buf_ptr, i64_type, "its").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), result);
                }
                Instruction::StringToInt { dest, string } => {
                    let str_int = regs[string];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let str_ptr = builder.build_int_to_ptr(str_int, ptr_type, "sp")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let strtol_fn = llvm_module.get_function("strtol").unwrap();
                    let null = ptr_type.const_null();
                    let base = i64_type.const_int(10, false);
                    let result_call = builder.build_call(strtol_fn, &[str_ptr.into(), null.into(), base.into()], "sti")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let result = match result_call.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(), _ => return Err(CodegenError::LlvmError("strtol void".into())) };
                    regs.insert(dest.clone(), result);
                }
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-codegen`
Expected: All codegen tests PASS (existing + 3 new)

- [ ] **Step 10: Commit**

```bash
git add crates/cyflym-codegen/src/lib.rs
git commit -m "feat(codegen): add array ops, string ops, and C stdlib declarations"
```

---

### Task 6: E2E Tests

**Files:**
- Create: `tests/fixtures/array_basic.cy`
- Create: `tests/fixtures/array_for_in.cy`
- Create: `tests/fixtures/string_ops.cy`
- Create: `tests/fixtures/string_conversion.cy`
- Modify: `crates/cyflym-driver/tests/e2e.rs`

- [ ] **Step 1: Create `array_basic.cy`**

```cyflym
fn main() Int {
    let a = array<Int>()
    a.push(10)
    a.push(20)
    a.push(30)
    a.set(1, 25)
    let x = a.get(1)
    let n = a.len()
    x + n
}
```

Expected exit code: 28 (25 + 3)

- [ ] **Step 2: Create `array_for_in.cy`**

```cyflym
fn main() Int {
    let a = array<Int>()
    a.push(1)
    a.push(2)
    a.push(3)
    a.push(4)
    let sum = 0
    for x in a {
        sum = sum + x
    }
    sum
}
```

Expected exit code: 10 (1+2+3+4)

Note: `sum` must be declared as `let mut sum = 0` if the language requires `mut` for reassignment. Check whether `sum = sum + x` compiles — the typeck requires `mut` for `Assign`. Update to `let mut sum = 0` if needed.

- [ ] **Step 3: Create `string_ops.cy`**

```cyflym
fn main() Int {
    let s = "hello"
    let n = s.len()
    let s2 = s + " world"
    let n2 = s2.len()
    let sub = s.substring(1, 3)
    let n3 = sub.len()
    n + n2 + n3
}
```

Expected exit code: 18 (5 + 11 + 2)

- [ ] **Step 4: Create `string_conversion.cy`**

```cyflym
fn main() Int {
    let s = int_to_string(42)
    let n = string_to_int(s)
    n
}
```

Expected exit code: 42

- [ ] **Step 5: Add E2E test entries**

In `crates/cyflym-driver/tests/e2e.rs`, add:

```rust
#[test]
fn e2e_array_basic() {
    assert_eq!(compile_and_run("array_basic.cy"), 28);
}

#[test]
fn e2e_array_for_in() {
    assert_eq!(compile_and_run("array_for_in.cy"), 10);
}

#[test]
fn e2e_string_ops() {
    assert_eq!(compile_and_run("string_ops.cy"), 18);
}

#[test]
fn e2e_string_conversion() {
    assert_eq!(compile_and_run("string_conversion.cy"), 42);
}
```

- [ ] **Step 6: Run all E2E tests**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-driver`
Expected: All E2E tests PASS (14 existing + 4 new = 18)

- [ ] **Step 7: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --workspace`
Expected: ~220+ tests PASS

- [ ] **Step 8: Commit**

```bash
git add tests/fixtures/array_basic.cy tests/fixtures/array_for_in.cy tests/fixtures/string_ops.cy tests/fixtures/string_conversion.cy crates/cyflym-driver/tests/e2e.rs
git commit -m "feat: add array and string E2E tests"
```
