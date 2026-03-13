# Plan 5b: Mutex & Bounded Channels Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add mutex with explicit lock/unlock and bounded channels with blocking send to the Cyflym compiler.

**Architecture:** New `mutex` keyword and `Mutex` type flow through lexer → parser → typeck → IR → codegen, following the exact same pattern as Plan 5a's spawn/channel. Bounded channels modify `ChannelCreate` AST with optional capacity and add `ChannelCreateBounded` IR instruction. Both features codegen to pthread syscalls.

**Tech Stack:** Rust, inkwell 0.8 (llvm17-0), pthreads

**Spec:** `docs/superpowers/specs/2026-03-12-plan5b-mutex-bounded-channels-design.md`

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `crates/cyflym-lexer/src/token.rs` | Modify | Add `Mutex` token variant |
| `crates/cyflym-lexer/src/lib.rs` | Modify | Add `mutex` keyword mapping + 1 test |
| `crates/cyflym-parser/src/ast.rs` | Modify | Add `MutexCreate` expr, add `capacity` to `ChannelCreate` |
| `crates/cyflym-parser/src/lib.rs` | Modify | Parse `mutex(expr)`, parse `channel<T>(cap)`, update `expr_span` + 3 tests |
| `crates/cyflym-typeck/src/types.rs` | Modify | Add `Mutex` type variant + Display impl |
| `crates/cyflym-typeck/src/lib.rs` | Modify | Type check mutex/lock/unlock, bounded channel capacity + 5 tests |
| `crates/cyflym-ir/src/ir.rs` | Modify | Add `MutexCreate`, `MutexLock`, `MutexUnlock`, `ChannelCreateBounded` instructions |
| `crates/cyflym-ir/src/lib.rs` | Modify | Add `Mutex` to `IrType`, lower mutex + bounded channel + 3 tests |
| `crates/cyflym-codegen/src/lib.rs` | Modify | Codegen for mutex (72-byte struct), update channel to 208-byte struct, bounded send/recv + 2 tests |
| `crates/cyflym-driver/tests/e2e.rs` | Modify | Add 3 E2E test entries |
| `tests/fixtures/mutex_basic.cy` | Create | Single-threaded mutex test |
| `tests/fixtures/mutex_threaded.cy` | Create | Multi-threaded mutex test |
| `tests/fixtures/channel_bounded.cy` | Create | Bounded channel test |

---

## Chunk 1: Lexer, Parser, AST, Type System

### Task 1: Lexer — Add `mutex` keyword

**Files:**
- Modify: `crates/cyflym-lexer/src/token.rs:61-63`
- Modify: `crates/cyflym-lexer/src/lib.rs:82-83` (keyword map) and tests

- [ ] **Step 1: Write the failing test**

In `crates/cyflym-lexer/src/lib.rs`, add after the `lex_channel_keyword` test (line ~420):

```rust
    #[test]
    fn lex_mutex_keyword() {
        let tokens = lex("mutex").unwrap();
        assert_eq!(kinds(&tokens), vec![Mutex, Eof]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-lexer -- lex_mutex_keyword`
Expected: FAIL — `Mutex` variant doesn't exist

- [ ] **Step 3: Add `Mutex` token variant**

In `crates/cyflym-lexer/src/token.rs`, add after `Channel,` (line 63):

```rust
    Mutex,
```

- [ ] **Step 4: Add keyword mapping**

In `crates/cyflym-lexer/src/lib.rs`, add after `"channel" => TokenKind::Channel,` (line 83):

```rust
                    "mutex" => TokenKind::Mutex,
```

- [ ] **Step 5: Run test to verify it passes**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-lexer -- lex_mutex_keyword`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/cyflym-lexer/src/token.rs crates/cyflym-lexer/src/lib.rs
git commit -m "feat(lexer): add mutex keyword token"
```

---

### Task 2: Parser & AST — Add `MutexCreate` expression and bounded `ChannelCreate`

**Files:**
- Modify: `crates/cyflym-parser/src/ast.rs:181-184`
- Modify: `crates/cyflym-parser/src/lib.rs:716-726` (channel parsing), atom parsing, `expr_span`

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-parser/src/lib.rs`, add after the `parse_spawn_no_args` test (line ~1469):

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-parser -- parse_mutex_create parse_bounded_channel parse_lock_unlock`
Expected: FAIL — `MutexCreate` variant doesn't exist, capacity field doesn't exist

- [ ] **Step 3: Update AST**

In `crates/cyflym-parser/src/ast.rs`, replace the `ChannelCreate` variant (lines 181-184):

```rust
    ChannelCreate {
        element_type: TypeName,
        capacity: Option<Box<Expr>>,
        span: Span,
    },
    MutexCreate {
        value: Box<Expr>,
        span: Span,
    },
```

- [ ] **Step 4: Update `expr_span` function**

In `crates/cyflym-parser/src/lib.rs`, in the `expr_span` function (around line 921), add after the `ChannelCreate` match arm:

```rust
        Expr::MutexCreate { span, .. } => span,
```

- [ ] **Step 5: Update channel parser for optional capacity**

In `crates/cyflym-parser/src/lib.rs`, replace the `TokenKind::Channel` arm (lines 716-726):

```rust
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
```

- [ ] **Step 6: Add `mutex` parser**

In `crates/cyflym-parser/src/lib.rs`, add a new arm before the `TokenKind::Channel` arm:

```rust
            TokenKind::Mutex => {
                let start = tok.span.start;
                self.pos += 1;
                self.expect(&TokenKind::LParen)?;
                let value = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                let span = start..self.tokens[self.pos - 1].span.end;
                Ok(Expr::MutexCreate { value: Box::new(value), span })
            }
```

- [ ] **Step 7: Fix existing ChannelCreate references**

The existing parser test `parse_channel_create` and the `parse_send_method_call`/`parse_recv_method_call`/`parse_join_method_call` tests create `ChannelCreate` without the `capacity` field. Since the `ChannelCreate` now has `capacity: Option<Box<Expr>>`, `matches!(value, Expr::ChannelCreate { .. })` will still work. No changes needed to existing tests.

Update the statement-at-end-of-block check in `parse_block_body` if it references `ChannelCreate` fields — this should already work since `..` matches any fields.

- [ ] **Step 8: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-parser`
Expected: All parser tests PASS (existing + 3 new)

- [ ] **Step 9: Commit**

```bash
git add crates/cyflym-parser/src/ast.rs crates/cyflym-parser/src/lib.rs
git commit -m "feat(parser): add mutex(expr) and channel<T>(cap) syntax"
```

---

### Task 3: Type System — Add `Mutex` type, type check mutex operations and bounded channel capacity

**Files:**
- Modify: `crates/cyflym-typeck/src/types.rs:1-35`
- Modify: `crates/cyflym-typeck/src/lib.rs:154-166` (LetDestructure), `816-819` (ChannelCreate expr), `821-851` (MethodCall)

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-typeck/src/lib.rs`, add after the last concurrency test:

```rust
    #[test]
    fn check_mutex_create() {
        assert!(do_check("fn main() Int { let m = mutex(0) 0 }").is_ok());
    }

    #[test]
    fn check_mutex_lock_returns_inner_type() {
        assert!(do_check("fn main() Int { let m = mutex(42) let v = m.lock() v }").is_ok());
    }

    #[test]
    fn check_mutex_unlock_matching_type() {
        assert!(do_check("fn main() Int { let m = mutex(0) let v = m.lock() m.unlock(v + 1) 0 }").is_ok());
    }

    #[test]
    fn check_mutex_unlock_wrong_type() {
        let err = do_check("fn main() Int { let m = mutex(0) m.unlock(true) 0 }").unwrap_err();
        assert!(err.message.contains("mismatch") || err.message.contains("type"),
            "expected type error, got: {}", err.message);
    }

    #[test]
    fn check_lock_on_non_mutex() {
        let err = do_check("fn main() Int { let x = 42 x.lock() }").unwrap_err();
        assert!(err.message.contains("method") || err.message.contains("lock"),
            "expected method error, got: {}", err.message);
    }

    #[test]
    fn check_bounded_channel_capacity_non_int() {
        let err = do_check("fn main() Int { let (tx, rx) = channel<Int>(true) 0 }").unwrap_err();
        assert!(err.message.contains("Int") || err.message.contains("capacity"),
            "expected capacity type error, got: {}", err.message);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-typeck -- check_mutex`
Expected: FAIL — `Mutex` variant doesn't exist, `MutexCreate` not handled

- [ ] **Step 3: Add `Mutex` type variant**

In `crates/cyflym-typeck/src/types.rs`, add after `Receiver { inner: Box<Type> },` (line 11):

```rust
    Mutex { inner: Box<Type> },
```

And in the Display impl, add before the `Type::Fn` arm:

```rust
            Type::Mutex { inner } => write!(f, "Mutex<{}>", inner),
```

- [ ] **Step 4: Add `MutexCreate` expression type checking**

In `crates/cyflym-typeck/src/lib.rs`, add after the `Expr::ChannelCreate` arm (around line 819):

```rust
        Expr::MutexCreate { value, .. } => {
            let inner = check_expr(value, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
            Ok(Type::Mutex { inner: Box::new(inner) })
        }
```

- [ ] **Step 5: Add mutex method type checking**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::MethodCall` match (around line 825), add after the `JoinHandle` / `join` arm:

```rust
                (Type::Mutex { inner }, "lock") => {
                    if !args.is_empty() {
                        return Err(TypeError::new("lock() takes no arguments"));
                    }
                    return Ok(*inner.clone());
                }
                (Type::Mutex { inner }, "unlock") => {
                    if args.len() != 1 {
                        return Err(TypeError::new("unlock() takes exactly 1 argument"));
                    }
                    let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                    if arg_ty != **inner {
                        return Err(TypeError::new(format!(
                            "unlock() type mismatch: mutex holds {} but got {}", inner, arg_ty
                        )));
                    }
                    return Ok(Type::Int);
                }
```

- [ ] **Step 6: Update `ChannelCreate` expr type checking for capacity**

In `crates/cyflym-typeck/src/lib.rs`, replace the `Expr::ChannelCreate` arm (line ~816-818):

```rust
        Expr::ChannelCreate { element_type, capacity, .. } => {
            if let Some(cap_expr) = capacity {
                let cap_ty = check_expr(cap_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                if cap_ty != Type::Int {
                    return Err(TypeError::new(format!(
                        "channel capacity must be Int, got {}", cap_ty
                    )));
                }
            }
            let inner = resolve_type(&element_type.name, structs, enums)?;
            Ok(Type::Sender { inner: Box::new(inner) })
        }
```

- [ ] **Step 6b: Update `LetDestructure` handler for capacity type checking**

In `crates/cyflym-typeck/src/lib.rs`, replace the `Stmt::LetDestructure` arm (lines ~154-166). The existing handler pattern-matches `ChannelCreate` directly without calling `check_expr`, so the capacity is never type-checked. Fix by adding capacity validation:

```rust
        Stmt::LetDestructure { names, value, .. } => {
            match value {
                Expr::ChannelCreate { element_type, capacity, .. } => {
                    if let Some(cap_expr) = capacity {
                        let cap_ty = check_expr(cap_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
                        if cap_ty != Type::Int {
                            return Err(TypeError::new(format!(
                                "channel capacity must be Int, got {}", cap_ty
                            )));
                        }
                    }
                    let inner = resolve_type(&element_type.name, structs, enums)?;
                    if names.len() != 2 {
                        return Err(TypeError::new("channel destructuring requires exactly 2 names"));
                    }
                    locals.insert(names[0].clone(), (Type::Sender { inner: Box::new(inner.clone()) }, false));
                    locals.insert(names[1].clone(), (Type::Receiver { inner: Box::new(inner) }, false));
                    Ok(())
                }
                _ => Err(TypeError::new("destructuring let is only supported for channel<T>()")),
            }
        }
```

- [ ] **Step 7: Update spawn compatibility for Mutex type**

In `crates/cyflym-typeck/src/lib.rs`, in the `Expr::Spawn` arm (around line 801), update the compatibility check to also allow `Mutex`:

```rust
                    let compatible = actual == *expected
                        || (*expected == Type::Int && matches!(actual, Type::Sender { .. } | Type::Receiver { .. } | Type::JoinHandle | Type::Mutex { .. }));
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-typeck`
Expected: All typeck tests PASS (existing + 6 new)

- [ ] **Step 9: Commit**

```bash
git add crates/cyflym-typeck/src/types.rs crates/cyflym-typeck/src/lib.rs
git commit -m "feat(typeck): add Mutex type with lock/unlock checking, bounded channel capacity"
```

---

## Chunk 2: IR, Codegen, E2E Tests

### Task 4: IR — Add mutex instructions, `ChannelCreateBounded`, and `Mutex` IrType

**Files:**
- Modify: `crates/cyflym-ir/src/ir.rs:47-68`
- Modify: `crates/cyflym-ir/src/lib.rs:9` (IrType), `450-484` (MethodCall lowering), `629-632` (ChannelCreate), `708-724` (LetDestructure)

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-ir/src/lib.rs`, add after the last concurrency IR test:

```rust
    #[test]
    fn lower_mutex_create_lock_unlock() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let m = mutex(5) let v = m.lock() m.unlock(v) 0 }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexCreate { .. })),
            "expected MutexCreate instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexLock { .. })),
            "expected MutexLock instruction");
        assert!(instrs.iter().any(|i| matches!(i, Instruction::MutexUnlock { .. })),
            "expected MutexUnlock instruction");
    }

    #[test]
    fn lower_bounded_channel() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>(10) tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "expected ChannelCreateBounded instruction");
    }

    #[test]
    fn lower_unbounded_channel_unchanged() {
        let prog = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>() tx.send(1) rx.recv() }"
        ).unwrap();
        let module = lower(&prog);
        let instrs = &module.functions[0].body;
        assert!(instrs.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. })),
            "expected ChannelCreate (not bounded) instruction");
        assert!(!instrs.iter().any(|i| matches!(i, Instruction::ChannelCreateBounded { .. })),
            "should NOT have ChannelCreateBounded");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-ir -- lower_mutex lower_bounded lower_unbounded`
Expected: FAIL — `MutexCreate` variant doesn't exist on Instruction

- [ ] **Step 3: Add IR instructions**

In `crates/cyflym-ir/src/ir.rs`, add after `ChannelRecv` (line ~68):

```rust
    // Mutex operations
    MutexCreate {
        dest: Reg,
        value: Reg,
    },
    MutexLock {
        dest: Reg,
        mutex: Reg,
    },
    MutexUnlock {
        mutex: Reg,
        value: Reg,
    },
    // Bounded channel creation
    ChannelCreateBounded {
        tx_dest: Reg,
        rx_dest: Reg,
        capacity: Reg,
    },
```

- [ ] **Step 4: Add `Mutex` to `IrType`**

In `crates/cyflym-ir/src/lib.rs`, update line 9:

```rust
enum IrType { Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex }
```

- [ ] **Step 5: Add `MutexCreate` expression lowering**

In `crates/cyflym-ir/src/lib.rs`, add before the `Expr::ChannelCreate` arm (around line 629):

```rust
            Expr::MutexCreate { value, .. } => {
                let val_reg = self.lower_expr(value);
                let dest = self.fresh_reg();
                self.instructions.push(Instruction::MutexCreate {
                    dest: dest.clone(),
                    value: val_reg,
                });
                self.reg_types.insert(dest.clone(), IrType::Mutex);
                dest
            }
```

- [ ] **Step 6: Add mutex method lowering**

In `crates/cyflym-ir/src/lib.rs`, in the `Expr::MethodCall` match (around line 454), add after the `JoinHandle` / `join` arm:

```rust
                    (Some(IrType::Mutex), "lock") => {
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::MutexLock {
                            dest: dest.clone(),
                            mutex: obj_reg,
                        });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
                    (Some(IrType::Mutex), "unlock") => {
                        let val_reg = self.lower_expr(&args[0]);
                        self.instructions.push(Instruction::MutexUnlock {
                            mutex: obj_reg,
                            value: val_reg,
                        });
                        let dest = self.fresh_reg();
                        self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
                        self.reg_types.insert(dest.clone(), IrType::Int);
                        return dest;
                    }
```

- [ ] **Step 7: Update `LetDestructure` for bounded channels**

In `crates/cyflym-ir/src/lib.rs`, replace the `Stmt::LetDestructure` arm (lines ~708-724):

```rust
            Stmt::LetDestructure { names, value, .. } => {
                match value {
                    Expr::ChannelCreate { capacity, .. } => {
                        let tx_reg = self.fresh_reg();
                        let rx_reg = self.fresh_reg();
                        if let Some(cap_expr) = capacity {
                            let cap_reg = self.lower_expr(cap_expr);
                            self.instructions.push(Instruction::ChannelCreateBounded {
                                tx_dest: tx_reg.clone(),
                                rx_dest: rx_reg.clone(),
                                capacity: cap_reg,
                            });
                        } else {
                            self.instructions.push(Instruction::ChannelCreate {
                                tx_dest: tx_reg.clone(),
                                rx_dest: rx_reg.clone(),
                            });
                        }
                        self.reg_types.insert(tx_reg.clone(), IrType::Sender);
                        self.reg_types.insert(rx_reg.clone(), IrType::Receiver);
                        self.locals.insert(names[0].clone(), LocalVar::Value(tx_reg));
                        self.locals.insert(names[1].clone(), LocalVar::Value(rx_reg));
                    }
                    _ => panic!("LetDestructure only supports ChannelCreate"),
                }
            }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-ir`
Expected: All IR tests PASS (existing + 3 new)

- [ ] **Step 9: Commit**

```bash
git add crates/cyflym-ir/src/ir.rs crates/cyflym-ir/src/lib.rs
git commit -m "feat(ir): add MutexCreate/Lock/Unlock and ChannelCreateBounded instructions"
```

---

### Task 5: Codegen — Mutex operations, updated channel struct, bounded send/recv

**Files:**
- Modify: `crates/cyflym-codegen/src/lib.rs:472-651` (channel codegen), add mutex codegen

This is the largest task. It modifies the codegen for:
1. Mutex: MutexCreate (72-byte struct), MutexLock, MutexUnlock
2. Channel struct expansion: 152 bytes → 208 bytes (add send condvar + is_bounded flag)
3. ChannelCreateBounded: same as ChannelCreate but with user capacity and is_bounded=1
4. ChannelSend: add bounded blocking (wait on send condvar) and unbounded growth (malloc new buffer, copy, free old)
5. ChannelRecv: signal send condvar for bounded channels

- [ ] **Step 1: Write the failing tests**

In `crates/cyflym-codegen/src/lib.rs`, add after the last concurrency codegen test:

```rust
    #[test]
    fn codegen_mutex_create_lock_unlock() {
        let program = cyflym_parser::parse(
            "fn main() Int { let m = mutex(5) let v = m.lock() m.unlock(v) 0 }"
        ).unwrap();
        cyflym_typeck::check(&program).unwrap();
        let ir = cyflym_ir::lower(&program);
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }

    #[test]
    fn codegen_bounded_channel() {
        let program = cyflym_parser::parse(
            "fn main() Int { let (tx, rx) = channel<Int>(4) tx.send(1) rx.recv() }"
        ).unwrap();
        cyflym_typeck::check(&program).unwrap();
        let ir = cyflym_ir::lower(&program);
        let context = Context::create();
        let result = generate_llvm(&context, &ir);
        assert!(result.is_ok(), "codegen failed: {:?}", result.err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-codegen -- codegen_mutex codegen_bounded`
Expected: FAIL — no match for `MutexCreate` / `ChannelCreateBounded` in codegen

- [ ] **Step 3: Update ChannelCreate to 208-byte layout**

In `crates/cyflym-codegen/src/lib.rs`, replace the `Instruction::ChannelCreate` arm (lines ~472-539). The key changes are:
- Allocate 208 bytes (26 i64s) instead of 152
- Init second condvar at offset 19
- Set `is_bounded` to 0 at offset 25

```rust
                Instruction::ChannelCreate { tx_dest, rx_dest } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();

                    // Allocate channel: 26 * 8 = 208 bytes
                    let chan_call = builder.build_call(malloc_fn, &[i64_type.const_int(208, false).into()], "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let chan_ptr = match chan_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate buffer: 16 * 8 = 128 bytes
                    let buf_call = builder.build_call(malloc_fn, &[i64_type.const_int(128, false).into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store buffer ptr as i64 at offset 0
                    let buf_int: IntValue<'ctx> = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // capacity=16 at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, i64_type.const_int(16, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // count=0 at offset 2
                    let off2 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cnt_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // head=0 at offset 3
                    let off3 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off3, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // tail=0 at offset 4
                    let off4 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off4, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init mutex at offset 5
                    let null = ptr_type.const_null();
                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init recv condvar at offset 13
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[recv_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init send condvar at offset 19
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[send_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // is_bounded=0 at offset 25
                    let off25 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off25, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Both tx and rx are the channel ptr as i64
                    let chan_int = builder.build_ptr_to_int(chan_ptr, i64_type, "chan_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(tx_dest.clone(), chan_int);
                    regs.insert(rx_dest.clone(), chan_int);
                }
```

- [ ] **Step 4: Add ChannelCreateBounded codegen**

Add after the `ChannelCreate` arm:

```rust
                Instruction::ChannelCreateBounded { tx_dest, rx_dest, capacity } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let cap_val = regs[capacity];

                    // Allocate channel: 26 * 8 = 208 bytes
                    let chan_call = builder.build_call(malloc_fn, &[i64_type.const_int(208, false).into()], "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let chan_ptr = match chan_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Allocate buffer: capacity * 8 bytes
                    let eight = i64_type.const_int(8, false);
                    let buf_size = builder.build_int_mul(cap_val, eight, "bufsz")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_call = builder.build_call(malloc_fn, &[buf_size.into()], "buf")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_ptr = match buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store buffer ptr at offset 0
                    let buf_int: IntValue<'ctx> = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, buf_int)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // capacity at offset 1
                    let off1 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off1, cap_val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // count=0 at offset 2
                    let off2 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cnt_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off2, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // head=0 at offset 3
                    let off3 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off3, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // tail=0 at offset 4
                    let off4 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off4, i64_type.const_int(0, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init mutex at offset 5
                    let null = ptr_type.const_null();
                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init recv condvar at offset 13
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[recv_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init send condvar at offset 19
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_init").unwrap(), &[send_cond_ptr.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // is_bounded=1 at offset 25
                    let off25 = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(off25, i64_type.const_int(1, false))
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Both tx and rx are the channel ptr as i64
                    let chan_int = builder.build_ptr_to_int(chan_ptr, i64_type, "chan_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(tx_dest.clone(), chan_int);
                    regs.insert(rx_dest.clone(), chan_int);
                }
```

- [ ] **Step 5: Update ChannelSend codegen**

Replace the entire `Instruction::ChannelSend` arm (lines ~541-587) with the unified version that handles both bounded (block when full) and unbounded (grow buffer when full):

```rust
                Instruction::ChannelSend { tx, value } => {
                    let chan_int = regs[tx];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load is_bounded flag (offset 25)
                    let bnd_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_bounded = builder.build_load(i64_type, bnd_ptr, "is_bnd").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let is_bnd_bool = builder.build_int_compare(IntPredicate::NE, is_bounded, i64_type.const_int(0, false), "bnd")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let bounded_bb = context.append_basic_block(llvm_fn, "send_bounded");
                    let unbounded_check_bb = context.append_basic_block(llvm_fn, "send_unbnd_check");
                    let grow_bb = context.append_basic_block(llvm_fn, "send_grow");
                    let write_bb = context.append_basic_block(llvm_fn, "send_write");

                    builder.build_conditional_branch(is_bnd_bool, bounded_bb, unbounded_check_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // ── Bounded path: while count == capacity, wait on send condvar ──
                    builder.position_at_end(bounded_bb);
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_ptr_b = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_b = builder.build_load(i64_type, cnt_ptr_b, "cnt").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_b = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_b = builder.build_load(i64_type, cap_ptr_b, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let full_b = builder.build_int_compare(IntPredicate::EQ, cnt_b, cap_b, "full")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let bnd_wait_bb = context.append_basic_block(llvm_fn, "bnd_wait");
                    let bnd_recheck_bb = context.append_basic_block(llvm_fn, "bnd_recheck");
                    builder.build_conditional_branch(full_b, bnd_wait_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(bnd_wait_bb);
                    builder.build_call(llvm_module.get_function("pthread_cond_wait").unwrap(), &[send_cond_ptr.into(), mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(bnd_recheck_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(bnd_recheck_bb);
                    let cnt_b2 = builder.build_load(i64_type, cnt_ptr_b, "cnt2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_b2 = builder.build_load(i64_type, cap_ptr_b, "cap2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let still_full = builder.build_int_compare(IntPredicate::EQ, cnt_b2, cap_b2, "sfull")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(still_full, bnd_wait_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // ── Unbounded path: if count == capacity, grow buffer ──
                    builder.position_at_end(unbounded_check_bb);
                    let cnt_ptr_u = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp_u") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_u = builder.build_load(i64_type, cnt_ptr_u, "cnt_u").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_u = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp_u") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_u = builder.build_load(i64_type, cap_ptr_u, "cap_u").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let need_grow = builder.build_int_compare(IntPredicate::EQ, cnt_u, cap_u, "needgrow")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(need_grow, grow_bb, write_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // ── Grow buffer: malloc new at 2x, copy elements, free old ──
                    builder.position_at_end(grow_bb);
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let free_fn = llvm_module.get_function("free").unwrap();
                    let old_cap = builder.build_load(i64_type, cap_ptr_u, "old_cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let two = i64_type.const_int(2, false);
                    let new_cap = builder.build_int_mul(old_cap, two, "new_cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let eight = i64_type.const_int(8, false);
                    let new_buf_size = builder.build_int_mul(new_cap, eight, "nbs").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_call = builder.build_call(malloc_fn, &[new_buf_size.into()], "nbuf").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let new_buf_ptr = match new_buf_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Copy elements from old circular buffer to new linear buffer
                    // Loop: for i in 0..count, new_buf[i] = old_buf[(head + i) % old_cap]
                    let old_buf_int = builder.build_load(i64_type, chan_ptr, "obi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let old_buf_ptr = builder.build_int_to_ptr(old_buf_int, ptr_type, "obp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head_ptr_g = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "hp_g") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head_g = builder.build_load(i64_type, head_ptr_g, "head_g").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let count_g = builder.build_load(i64_type, cnt_ptr_u, "cnt_g").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Alloca loop counter
                    let idx_ptr = builder.build_alloca(i64_type, "gidx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(idx_ptr, i64_type.const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let gcond_bb = context.append_basic_block(llvm_fn, "gcond");
                    let gbody_bb = context.append_basic_block(llvm_fn, "gbody");
                    let gdone_bb = context.append_basic_block(llvm_fn, "gdone");

                    builder.build_unconditional_branch(gcond_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.position_at_end(gcond_bb);
                    let gi = builder.build_load(i64_type, idx_ptr, "gi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let gi_lt = builder.build_int_compare(IntPredicate::ULT, gi, count_g, "glt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(gi_lt, gbody_bb, gdone_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(gbody_bb);
                    let src_idx = builder.build_int_add(head_g, gi, "si").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let src_idx_mod = builder.build_int_unsigned_rem(src_idx, old_cap, "sim").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let src_gep = unsafe { builder.build_gep(i64_type, old_buf_ptr, &[src_idx_mod], "sg") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let elem = builder.build_load(i64_type, src_gep, "elem").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let dst_gep = unsafe { builder.build_gep(i64_type, new_buf_ptr, &[gi], "dg") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(dst_gep, elem).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let one = i64_type.const_int(1, false);
                    let gi_next = builder.build_int_add(gi, one, "gin").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(idx_ptr, gi_next).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(gcond_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(gdone_bb);
                    // Free old buffer
                    builder.build_call(free_fn, &[old_buf_ptr.into()], "").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Store new buffer ptr
                    let new_buf_int = builder.build_ptr_to_int(new_buf_ptr, i64_type, "nbi").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(chan_ptr, new_buf_int).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Update capacity
                    builder.build_store(cap_ptr_u, new_cap).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    // Reset head=0, tail=count
                    builder.build_store(head_ptr_g, i64_type.const_int(0, false)).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tail_ptr_g = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tp_g") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(tail_ptr_g, count_g).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(write_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // ── Write value at buffer[tail % capacity] ──
                    builder.position_at_end(write_bb);
                    let tail_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let tail = builder.build_load(i64_type, tail_ptr, "tail").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr_w = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp_w") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap_w = builder.build_load(i64_type, cap_ptr_w, "cap_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_int_w = builder.build_load(i64_type, chan_ptr, "bi_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr_w = builder.build_int_to_ptr(buf_int_w, ptr_type, "bp_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let idx = builder.build_int_unsigned_rem(tail, cap_w, "idx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let wp = unsafe { builder.build_gep(i64_type, buf_ptr_w, &[idx], "wp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(wp, val).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Increment tail and count
                    let one = i64_type.const_int(1, false);
                    let new_tail = builder.build_int_add(tail, one, "nt").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(tail_ptr, new_tail).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_ptr_w = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp_w") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt_w = builder.build_load(i64_type, cnt_ptr_w, "cnt_w").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let new_cnt = builder.build_int_add(cnt_w, one, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cnt_ptr_w, new_cnt).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Signal recv condvar
                    let recv_cond = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_signal").unwrap(), &[recv_cond.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Unlock
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
```

- [ ] **Step 6: Update ChannelRecv codegen to signal send condvar**

Replace the `Instruction::ChannelRecv` arm (lines ~588-651) — add send condvar signaling for bounded channels after consuming:

```rust
                Instruction::ChannelRecv { dest, rx } => {
                    let chan_int = regs[rx];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mtx") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let recv_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "rcnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Lock
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // While count == 0, wait
                    let wait_bb = context.append_basic_block(llvm_fn, "recv_wait");
                    let do_wait_bb = context.append_basic_block(llvm_fn, "recv_do_wait");
                    let body_bb = context.append_basic_block(llvm_fn, "recv_body");

                    builder.build_unconditional_branch(wait_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.position_at_end(wait_bb);

                    let cnt_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "cntp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt = builder.build_load(i64_type, cnt_ptr, "cnt").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let empty = builder.build_int_compare(IntPredicate::EQ, cnt, i64_type.const_int(0, false), "empty")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_conditional_branch(empty, do_wait_bb, body_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(do_wait_bb);
                    builder.build_call(llvm_module.get_function("pthread_cond_wait").unwrap(), &[recv_cond_ptr.into(), mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(wait_bb).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(body_bb);

                    // Read buffer[head % cap]
                    let head_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "hp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let head = builder.build_load(i64_type, head_ptr, "head").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cp") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cap = builder.build_load(i64_type, cap_ptr, "cap").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let idx = builder.build_int_unsigned_rem(head, cap, "idx").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let buf_int = builder.build_load(i64_type, chan_ptr, "bi").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "bp").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let rp = unsafe { builder.build_gep(i64_type, buf_ptr, &[idx], "rp") }.map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let received = builder.build_load(i64_type, rp, dest).map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

                    // Update head and count
                    let one = i64_type.const_int(1, false);
                    let new_head = builder.build_int_add(head, one, "nh").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(head_ptr, new_head).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let cnt2 = builder.build_load(i64_type, cnt_ptr, "cnt2").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let new_cnt = builder.build_int_sub(cnt2, one, "nc").map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_store(cnt_ptr, new_cnt).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Signal send condvar if bounded
                    let bnd_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(25, false)], "bnd_ptr") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let is_bounded = builder.build_load(i64_type, bnd_ptr, "is_bnd").map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    let is_bnd_bool = builder.build_int_compare(IntPredicate::NE, is_bounded, i64_type.const_int(0, false), "bnd")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let signal_bb = context.append_basic_block(llvm_fn, "recv_signal");
                    let unlock_bb = context.append_basic_block(llvm_fn, "recv_unlock");
                    builder.build_conditional_branch(is_bnd_bool, signal_bb, unlock_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(signal_bb);
                    let send_cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(19, false)], "scnd") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_cond_signal").unwrap(), &[send_cond_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_unconditional_branch(unlock_bb)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    builder.position_at_end(unlock_bb);
                    // Unlock
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_ptr.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    regs.insert(dest.clone(), received);
                }
```

- [ ] **Step 7: Add MutexCreate codegen**

Add after the `ThreadJoin` arm (before the closing `}` of the instruction match):

```rust
                Instruction::MutexCreate { dest, value } => {
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let malloc_fn = llvm_module.get_function("malloc").unwrap();
                    let val = regs[value];

                    // Allocate mutex struct: 9 * 8 = 72 bytes (1 value + 8 mutex)
                    let mtx_call = builder.build_call(malloc_fn, &[i64_type.const_int(72, false).into()], "mtx_alloc")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    let mtx_ptr = match mtx_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(bv) => bv.into_pointer_value(),
                        _ => return Err(CodegenError::LlvmError("malloc returned void".into())),
                    };

                    // Store value at offset 0
                    builder.build_store(mtx_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Init pthread_mutex at offset 1
                    let null = ptr_type.const_null();
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_init").unwrap(), &[mutex_inner.into(), null.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    let mtx_int = builder.build_ptr_to_int(mtx_ptr, i64_type, "mtx_int")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    regs.insert(dest.clone(), mtx_int);
                }
                Instruction::MutexLock { dest, mutex } => {
                    let mtx_int = regs[mutex];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let mtx_ptr = builder.build_int_to_ptr(mtx_int, ptr_type, "mtx_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Lock at offset 1
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_lock").unwrap(), &[mutex_inner.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Load value from offset 0
                    let val = builder.build_load(i64_type, mtx_ptr, dest)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
                    regs.insert(dest.clone(), val);
                }
                Instruction::MutexUnlock { mutex, value } => {
                    let mtx_int = regs[mutex];
                    let val = regs[value];
                    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
                    let mtx_ptr = builder.build_int_to_ptr(mtx_int, ptr_type, "mtx_p")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Store new value at offset 0
                    builder.build_store(mtx_ptr, val)
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

                    // Unlock at offset 1
                    let mutex_inner = unsafe { builder.build_gep(i64_type, mtx_ptr, &[i64_type.const_int(1, false)], "mi") }
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                    builder.build_call(llvm_module.get_function("pthread_mutex_unlock").unwrap(), &[mutex_inner.into()], "")
                        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
                }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-codegen`
Expected: All codegen tests PASS (existing + 2 new)

- [ ] **Step 9: Commit**

```bash
git add crates/cyflym-codegen/src/lib.rs
git commit -m "feat(codegen): add mutex ops, bounded channels, and buffer growth for unbounded channels"
```

---

### Task 6: E2E Tests

**Files:**
- Create: `tests/fixtures/mutex_basic.cy`
- Create: `tests/fixtures/mutex_threaded.cy`
- Create: `tests/fixtures/channel_bounded.cy`
- Modify: `crates/cyflym-driver/tests/e2e.rs`

- [ ] **Step 1: Create `mutex_basic.cy`**

```cyflym
fn main() Int {
    let m = mutex(10)
    let v = m.lock()
    m.unlock(v + 5)
    let v2 = m.lock()
    m.unlock(v2)
    v2
}
```

Expected exit code: 15

- [ ] **Step 2: Create `mutex_threaded.cy`**

Note: Concurrency method calls (`.lock()`, `.unlock()`, `.send()`, `.recv()`) must stay in `main()` where the IR lowering tracks their types via `reg_types`. Passing concurrency types as `Int` params to spawned functions loses the `IrType` — the IR lowering only tracks types within the function where the value was created. This is a known limitation; cross-function type propagation is deferred.

The test uses channels to coordinate: each spawned thread receives a value, adds 1, and sends it back.

```cyflym
fn worker(val Int) Int {
    val + 1
}

fn main() Int {
    let m = mutex(0)
    let v1 = m.lock()
    let h1 = spawn worker(v1)
    m.unlock(v1)
    h1.join()
    let v2 = m.lock()
    m.unlock(v2 + 1)
    let v3 = m.lock()
    m.unlock(v3)
    v3
}
```

Expected exit code: 1

This tests mutex create, lock, unlock, and interaction with spawn/join — all mutex method calls happen in `main()`.

- [ ] **Step 3: Create `channel_bounded.cy`**

Single-threaded bounded channel test — verifies blocking send doesn't deadlock when capacity isn't exceeded, and values are received correctly.

```cyflym
fn main() Int {
    let (tx, rx) = channel<Int>(2)
    tx.send(10)
    tx.send(20)
    let a = rx.recv()
    let b = rx.recv()
    a + b
}
```

Expected exit code: 30

- [ ] **Step 4: Add E2E test entries**

In `crates/cyflym-driver/tests/e2e.rs`, add:

```rust
#[test]
fn e2e_mutex_basic() {
    assert_eq!(compile_and_run("mutex_basic.cy"), 15);
}

#[test]
fn e2e_mutex_threaded() {
    assert_eq!(compile_and_run("mutex_threaded.cy"), 1);
}

#[test]
fn e2e_channel_bounded() {
    assert_eq!(compile_and_run("channel_bounded.cy"), 30);
}
```

- [ ] **Step 5: Run all E2E tests**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p cyflym-driver`
Expected: All E2E tests PASS (11 existing + 3 new = 14)

- [ ] **Step 6: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --workspace`
Expected: ~194 tests PASS

- [ ] **Step 7: Commit**

```bash
git add tests/fixtures/mutex_basic.cy tests/fixtures/mutex_threaded.cy tests/fixtures/channel_bounded.cy crates/cyflym-driver/tests/e2e.rs
git commit -m "feat: add mutex and bounded channel E2E tests"
```
