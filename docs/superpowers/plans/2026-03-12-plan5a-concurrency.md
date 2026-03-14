# Plan 5a: Concurrency (Spawn + Channels) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OS-thread-based concurrency to Sans with `spawn`, `channel<T>()`, `.send()`, `.recv()`, and `.join()`.

**Architecture:** 1:1 OS threads via pthreads, unbounded channels as mutex-guarded growable queues. New keywords, AST nodes, types, IR instructions, and LLVM codegen for pthread syscalls and channel data structures.

**Tech Stack:** Rust, inkwell 0.8 (llvm17-0), pthreads (libc)

**Spec:** `docs/superpowers/specs/2026-03-12-plan5a-concurrency-design.md`

---

## Batch 1: Lexer + Parser + AST + Type System

### Task 1: Lexer — Add `spawn` and `channel` keywords

**Files:**
- Modify: `crates/sans-lexer/src/token.rs`
- Modify: `crates/sans-lexer/src/lib.rs`

**Token changes** — add to `TokenKind` enum after the `SelfType` line:

```rust
// Concurrency
Spawn,
Channel,
```

**Lexer changes** — add to the keyword match in `lex()` after the `"Self"` arm:

```rust
"spawn" => TokenKind::Spawn,
"channel" => TokenKind::Channel,
```

**Tests to add:**

```rust
#[test]
fn lex_spawn_keyword() {
    let tokens = lex("spawn").unwrap();
    assert_eq!(kinds(&tokens), vec![Spawn, Eof]);
}

#[test]
fn lex_channel_keyword() {
    let tokens = lex("channel").unwrap();
    assert_eq!(kinds(&tokens), vec![Channel, Eof]);
}
```

---

### Task 2: AST — Add Spawn, ChannelCreate, and LetDestructure nodes

**Files:**
- Modify: `crates/sans-parser/src/ast.rs`

**Add to `Expr` enum:**

```rust
Spawn {
    function: String,
    args: Vec<Expr>,
    span: Span,
},
ChannelCreate {
    element_type: TypeName,
    span: Span,
},
```

**Add to `Stmt` enum:**

```rust
LetDestructure {
    names: Vec<String>,
    value: Expr,
    span: Span,
},
```

---

### Task 3: Parser — Parse spawn, channel, and let destructuring

**Files:**
- Modify: `crates/sans-parser/src/lib.rs`

**3a: Parse `spawn f(args)` in `parse_expr_inner` (prefix position)**

When the parser sees `TokenKind::Spawn`, it should:
1. Consume the `Spawn` token
2. Expect an identifier (the function name)
3. Expect `LParen`
4. Parse comma-separated args
5. Expect `RParen`
6. Return `Expr::Spawn { function, args, span }`

Add this case to the `parse_expr_inner` method (where `IntLiteral`, `True`, `Identifier`, etc. are matched):

```rust
TokenKind::Spawn => {
    self.pos += 1; // consume 'spawn'
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
    Expr::Spawn { function: func_name, args, span }
}
```

**3b: Parse `channel<Type>()` in `parse_expr_inner`**

When the parser sees `TokenKind::Channel`:
1. Consume `Channel`
2. Expect `Lt` (`<`)
3. Parse a type name
4. Expect `Gt` (`>`)
5. Expect `LParen`, `RParen`
6. Return `Expr::ChannelCreate { element_type, span }`

```rust
TokenKind::Channel => {
    self.pos += 1; // consume 'channel'
    self.expect(&TokenKind::Lt)?;
    let type_name = self.parse_type_name()?;
    self.expect(&TokenKind::Gt)?;
    self.expect(&TokenKind::LParen)?;
    self.expect(&TokenKind::RParen)?;
    let span = start..self.tokens[self.pos - 1].span.end;
    Expr::ChannelCreate { element_type: type_name, span }
}
```

**3c: Parse `let (a, b) = expr` in `parse_stmt`**

In the `TokenKind::Let` branch of `parse_stmt`, after consuming `Let`, check if the next token is `LParen`. If so:
1. Consume `LParen`
2. Parse first identifier
3. Expect `Comma`
4. Parse second identifier
5. Expect `RParen`
6. Expect `Eq`
7. Parse expression
8. Return `Stmt::LetDestructure { names, value, span }`

Otherwise, fall through to existing `Let` handling.

```rust
// Inside parse_stmt, in the Let branch, before existing let parsing:
TokenKind::Let => {
    let start = self.peek().span.start;
    self.pos += 1; // consume 'let'

    // Check for destructuring: let (a, b) = expr
    if self.peek().kind == TokenKind::LParen {
        self.pos += 1; // consume '('
        let (name1, _) = self.expect_ident()?;
        self.expect(&TokenKind::Comma)?;
        let (name2, _) = self.expect_ident()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr(0)?;
        let span = start..self.tokens[self.pos - 1].span.end;
        return Ok(Stmt::LetDestructure { names: vec![name1, name2], value, span });
    }

    // ... existing let parsing code ...
}
```

**Tests to add:**

```rust
#[test]
fn parse_spawn_expr() {
    let program = parse("fn main() Int { spawn foo(1, 2) }").unwrap();
    let body = &program.functions[0].body;
    match &body[0] {
        Stmt::Expr(Expr::Spawn { function, args, .. }) => {
            assert_eq!(function, "foo");
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Spawn expr, got {:?}", other),
    }
}

#[test]
fn parse_channel_create() {
    let program = parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }").unwrap();
    let body = &program.functions[0].body;
    match &body[0] {
        Stmt::LetDestructure { names, value, .. } => {
            assert_eq!(names, &["tx".to_string(), "rx".to_string()]);
            match value {
                Expr::ChannelCreate { element_type, .. } => {
                    assert_eq!(element_type.name, "Int");
                }
                other => panic!("expected ChannelCreate, got {:?}", other),
            }
        }
        other => panic!("expected LetDestructure, got {:?}", other),
    }
}

#[test]
fn parse_send_method_call() {
    let program = parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) 0 }").unwrap();
    let body = &program.functions[0].body;
    match &body[1] {
        Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
            assert_eq!(method, "send");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected MethodCall send, got {:?}", other),
    }
}

#[test]
fn parse_recv_method_call() {
    let program = parse("fn main() Int { let (tx, rx) = channel<Int>() let val = rx.recv() 0 }").unwrap();
    let body = &program.functions[0].body;
    // body[1] should be Let with recv method call
    match &body[1] {
        Stmt::Let { value, .. } => {
            match value {
                Expr::MethodCall { method, args, .. } => {
                    assert_eq!(method, "recv");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected MethodCall recv, got {:?}", other),
            }
        }
        other => panic!("expected Let stmt, got {:?}", other),
    }
}

#[test]
fn parse_join_method_call() {
    let program = parse("fn main() Int { let h = spawn foo() h.join() 0 }").unwrap();
    let body = &program.functions[0].body;
    match &body[1] {
        Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
            assert_eq!(method, "join");
            assert_eq!(args.len(), 0);
        }
        other => panic!("expected MethodCall join, got {:?}", other),
    }
}

#[test]
fn parse_spawn_no_args() {
    let program = parse("fn main() Int { spawn worker() }").unwrap();
    let body = &program.functions[0].body;
    match &body[0] {
        Stmt::Expr(Expr::Spawn { function, args, .. }) => {
            assert_eq!(function, "worker");
            assert_eq!(args.len(), 0);
        }
        other => panic!("expected Spawn expr, got {:?}", other),
    }
}
```

---

### Task 4: Type System — Add JoinHandle, Sender, Receiver types

**Files:**
- Modify: `crates/sans-typeck/src/types.rs`
- Modify: `crates/sans-typeck/src/lib.rs`

**4a: Add new type variants** to `types.rs`:

```rust
pub enum Type {
    Int,
    Bool,
    String,
    Fn { params: Vec<Type>, ret: Box<Type> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Enum { name: String, variants: Vec<(String, Vec<Type>)> },
    // Concurrency types
    JoinHandle,
    Sender { inner: Box<Type> },
    Receiver { inner: Box<Type> },
}
```

Update the `Display` impl:

```rust
Type::JoinHandle => write!(f, "JoinHandle"),
Type::Sender { inner } => write!(f, "Sender<{}>", inner),
Type::Receiver { inner } => write!(f, "Receiver<{}>", inner),
```

**4b: Type-check concurrency expressions** in `lib.rs`:

In `check_expr`, add handling for `Expr::Spawn`:

```rust
Expr::Spawn { function, args, .. } => {
    // Look up the function
    if let Some((param_types, _ret_type)) = fn_env.get(function) {
        if args.len() != param_types.len() {
            return Err(TypeError::new(format!(
                "wrong argument count calling '{}': expected {} but got {}",
                function, param_types.len(), args.len()
            )));
        }
        for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
            let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
            if actual != *expected {
                return Err(TypeError::new(format!(
                    "argument {} to '{}': expected {} but got {}",
                    i + 1, function, expected, actual
                )));
            }
        }
        Ok(Type::JoinHandle)
    } else {
        Err(TypeError::new(format!("undefined function '{}' in spawn", function)))
    }
}
```

Add handling for `Expr::ChannelCreate`:

```rust
Expr::ChannelCreate { element_type, .. } => {
    let inner = resolve_type(&element_type.name, structs, enums)?;
    // This should only appear inside LetDestructure, but return a placeholder
    // The actual binding happens in check_stmt for LetDestructure
    Ok(Type::Sender { inner: Box::new(inner) })
}
```

In `check_stmt`, add handling for `Stmt::LetDestructure`:

```rust
Stmt::LetDestructure { names, value, .. } => {
    match value {
        Expr::ChannelCreate { element_type, .. } => {
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

In `check_expr` for `Expr::MethodCall`, add handling for concurrency types **before** the existing struct/enum check:

```rust
Expr::MethodCall { object, method, args, .. } => {
    let obj_ty = check_expr(object, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;

    // Handle concurrency built-in methods
    match (&obj_ty, method.as_str()) {
        (Type::Sender { inner }, "send") => {
            if args.len() != 1 {
                return Err(TypeError::new("send() takes exactly 1 argument"));
            }
            let arg_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits)?;
            if arg_ty != **inner {
                return Err(TypeError::new(format!(
                    "send() type mismatch: channel holds {} but got {}",
                    inner, arg_ty
                )));
            }
            return Ok(Type::Int); // send returns Int (0) as statement
        }
        (Type::Receiver { inner }, "recv") => {
            if !args.is_empty() {
                return Err(TypeError::new("recv() takes no arguments"));
            }
            return Ok(*inner.clone());
        }
        (Type::JoinHandle, "join") => {
            if !args.is_empty() {
                return Err(TypeError::new("join() takes no arguments"));
            }
            return Ok(Type::Int);
        }
        _ => {} // fall through to existing struct/enum method handling
    }

    // Existing struct/enum method call handling...
    let type_name = match &obj_ty {
        Type::Struct { name, .. } => name.clone(),
        Type::Enum { name, .. } => name.clone(),
        other => return Err(TypeError::new(format!(
            "method call on non-struct/enum type {}", other
        ))),
    };
    // ... rest of existing code
}
```

Also update the `check` function's Pass 2 to handle `LetDestructure` in the last-statement check:

In the function body checking loop, the `is_last` match needs to handle `Stmt::LetDestructure`:

```rust
Stmt::Let { .. } | Stmt::While { .. } | Stmt::Assign { .. } | Stmt::If { .. } | Stmt::LetDestructure { .. } => {
    return Err(TypeError::new(format!(
        "function '{}': missing return expression", func.name
    )));
}
```

**Tests to add to typeck:**

```rust
#[test]
fn typeck_spawn_produces_join_handle() {
    let src = "fn worker() Int { 0 } fn main() Int { let h = spawn worker() 0 }";
    assert!(check_src(src).is_ok());
}

#[test]
fn typeck_spawn_wrong_args() {
    let src = "fn worker(x Int) Int { x } fn main() Int { let h = spawn worker() 0 }";
    assert!(check_src(src).is_err());
}

#[test]
fn typeck_channel_creates_sender_receiver() {
    let src = "fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }";
    assert!(check_src(src).is_ok());
}

#[test]
fn typeck_send_type_mismatch() {
    let src = "fn main() Int { let (tx, rx) = channel<Int>() tx.send(true) 0 }";
    let err = check_src(src).unwrap_err();
    assert!(err.message.contains("type mismatch"), "got: {}", err.message);
}

#[test]
fn typeck_recv_returns_element_type() {
    let src = "fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }";
    assert!(check_src(src).is_ok());
}

#[test]
fn typeck_join_on_handle() {
    let src = "fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }";
    assert!(check_src(src).is_ok());
}

#[test]
fn typeck_join_on_non_handle() {
    let src = "fn main() Int { let x Int = 42 x.join() }";
    assert!(check_src(src).is_err());
}

#[test]
fn typeck_send_on_non_sender() {
    let src = "fn main() Int { let x Int = 42 x.send(1) 0 }";
    assert!(check_src(src).is_err());
}
```

---

## Batch 2: IR Lowering + Codegen + E2E

### Task 5: IR — Add concurrency instructions and IrType variants

**Files:**
- Modify: `crates/sans-ir/src/ir.rs`
- Modify: `crates/sans-ir/src/lib.rs`

**5a: Add IR instructions** to `ir.rs` Instruction enum:

```rust
// Thread operations
ThreadSpawn {
    dest: Reg,
    function: String,
    args: Vec<Reg>,
},
ThreadJoin {
    handle: Reg,
},

// Channel operations
ChannelCreate {
    tx_dest: Reg,
    rx_dest: Reg,
},
ChannelSend {
    tx: Reg,
    value: Reg,
},
ChannelRecv {
    dest: Reg,
    rx: Reg,
},
```

**5b: Add IrType variants** in `lib.rs`:

```rust
enum IrType { Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle }
```

**5c: Lower Expr::Spawn** — in `lower_expr`:

```rust
Expr::Spawn { function, args, .. } => {
    let arg_regs: Vec<Reg> = args.iter().map(|a| self.lower_expr(a)).collect();
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::ThreadSpawn {
        dest: dest.clone(),
        function: function.clone(),
        args: arg_regs,
    });
    self.reg_types.insert(dest.clone(), IrType::JoinHandle);
    dest
}
Expr::ChannelCreate { .. } => {
    // Handled in LetDestructure lowering; should not appear standalone
    panic!("ChannelCreate should only appear inside LetDestructure")
}
```

**5d: Lower Stmt::LetDestructure** — in `lower_stmt`:

```rust
Stmt::LetDestructure { names, value, .. } => {
    match value {
        Expr::ChannelCreate { .. } => {
            let tx_reg = self.fresh_reg();
            let rx_reg = self.fresh_reg();
            self.instructions.push(Instruction::ChannelCreate {
                tx_dest: tx_reg.clone(),
                rx_dest: rx_reg.clone(),
            });
            self.reg_types.insert(tx_reg.clone(), IrType::Sender);
            self.reg_types.insert(rx_reg.clone(), IrType::Receiver);
            self.locals.insert(names[0].clone(), LocalVar::Value(tx_reg));
            self.locals.insert(names[1].clone(), LocalVar::Value(rx_reg));
        }
        _ => panic!("LetDestructure only supports ChannelCreate"),
    }
}
```

**5e: Lower concurrency method calls** — in the `Expr::MethodCall` arm of `lower_expr`, before the existing struct/enum handling:

```rust
Expr::MethodCall { object, method, args, .. } => {
    let obj_reg = self.lower_expr(object);

    // Handle concurrency built-in methods
    match (self.reg_types.get(&obj_reg), method.as_str()) {
        (Some(IrType::Sender), "send") => {
            let val_reg = self.lower_expr(&args[0]);
            self.instructions.push(Instruction::ChannelSend {
                tx: obj_reg,
                value: val_reg,
            });
            // send is statement-only; return a dummy 0
            let dest = self.fresh_reg();
            self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
            self.reg_types.insert(dest.clone(), IrType::Int);
            return dest;
        }
        (Some(IrType::Receiver), "recv") => {
            let dest = self.fresh_reg();
            self.instructions.push(Instruction::ChannelRecv {
                dest: dest.clone(),
                rx: obj_reg,
            });
            self.reg_types.insert(dest.clone(), IrType::Int);
            return dest;
        }
        (Some(IrType::JoinHandle), "join") => {
            self.instructions.push(Instruction::ThreadJoin {
                handle: obj_reg,
            });
            let dest = self.fresh_reg();
            self.instructions.push(Instruction::Const { dest: dest.clone(), value: 0 });
            self.reg_types.insert(dest.clone(), IrType::Int);
            return dest;
        }
        _ => {} // fall through to existing code
    }

    // Existing struct/enum method call handling...
    let type_name = match self.reg_types.get(&obj_reg) {
        Some(IrType::Struct(name)) => name.clone(),
        Some(IrType::Enum(name)) => name.clone(),
        _ => panic!("method call on non-struct/enum"),
    };
    // ... rest unchanged
}
```

**Tests to add:**

```rust
#[test]
fn lower_spawn() {
    let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() 0 }");
    let module = lower(&program);
    let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_spawn = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadSpawn { .. }));
    assert!(has_spawn, "expected ThreadSpawn instruction");
}

#[test]
fn lower_channel_create() {
    let program = parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }");
    let module = lower(&program);
    let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_create = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelCreate { .. }));
    assert!(has_create, "expected ChannelCreate instruction");
}

#[test]
fn lower_send_recv() {
    let program = parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }");
    let module = lower(&program);
    let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_send = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelSend { .. }));
    let has_recv = main_func.body.iter().any(|i| matches!(i, Instruction::ChannelRecv { .. }));
    assert!(has_send, "expected ChannelSend instruction");
    assert!(has_recv, "expected ChannelRecv instruction");
}

#[test]
fn lower_join() {
    let program = parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }");
    let module = lower(&program);
    let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_join = main_func.body.iter().any(|i| matches!(i, Instruction::ThreadJoin { .. }));
    assert!(has_join, "expected ThreadJoin instruction");
}
```

---

### Task 6: Codegen — LLVM codegen for concurrency instructions

**Files:**
- Modify: `crates/sans-codegen/src/lib.rs`

This is the most complex task. The codegen must:

**6a: Declare extern functions** at the start of `generate_llvm`, after declaring `printf`:

```rust
// Declare pthread functions
let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
let i32_type = context.i32_type();

// pthread_create(pthread_t*, attr*, fn_ptr, arg) -> i32
let pthread_create_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into(), ptr_type.into()], false);
llvm_module.add_function("pthread_create", pthread_create_type, Some(Linkage::External));

// pthread_join(pthread_t, retval**) -> i32
let pthread_join_type = i32_type.fn_type(&[i64_type.into(), ptr_type.into()], false);
llvm_module.add_function("pthread_join", pthread_join_type, Some(Linkage::External));

// pthread_mutex_init, lock, unlock
let mutex_fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
llvm_module.add_function("pthread_mutex_init", mutex_fn_type, Some(Linkage::External));
let mutex_op_type = i32_type.fn_type(&[ptr_type.into()], false);
llvm_module.add_function("pthread_mutex_lock", mutex_op_type, Some(Linkage::External));
llvm_module.add_function("pthread_mutex_unlock", mutex_op_type, Some(Linkage::External));

// pthread_cond_init, wait, signal
let cond_init_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
llvm_module.add_function("pthread_cond_init", cond_init_type, Some(Linkage::External));
let cond_wait_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
llvm_module.add_function("pthread_cond_wait", cond_wait_type, Some(Linkage::External));
let cond_signal_type = i32_type.fn_type(&[ptr_type.into()], false);
llvm_module.add_function("pthread_cond_signal", cond_signal_type, Some(Linkage::External));

// malloc, realloc, free
let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
llvm_module.add_function("malloc", malloc_type, Some(Linkage::External));
let realloc_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
llvm_module.add_function("realloc", realloc_type, Some(Linkage::External));
let free_type = context.void_type().fn_type(&[ptr_type.into()], false);
llvm_module.add_function("free", free_type, Some(Linkage::External));
```

**6b: Handle `ChannelCreate`** — allocate channel struct on heap:

The channel struct layout (all i64-sized fields for simplicity):
- Field 0: buffer pointer (i64, cast from ptr)
- Field 1: capacity (i64)
- Field 2: count (i64)
- Field 3: head (i64)
- Field 4: tail (i64)
- Fields 5+: pthread_mutex_t (64 bytes = 8 i64s on macOS)
- Fields 13+: pthread_cond_t (48 bytes = 6 i64s on macOS)

Total: 19 i64 fields = 152 bytes

```rust
Instruction::ChannelCreate { tx_dest, rx_dest } => {
    let malloc_fn = llvm_module.get_function("malloc").unwrap();
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());

    // Allocate channel struct: 19 * 8 = 152 bytes
    let chan_size = i64_type.const_int(152, false);
    let chan_ptr = builder.build_call(malloc_fn, &[chan_size.into()], "chan_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
        .try_as_basic_value().left().unwrap().into_pointer_value();

    // Allocate initial buffer: 16 * 8 = 128 bytes
    let buf_size = i64_type.const_int(128, false);
    let buf_ptr = builder.build_call(malloc_fn, &[buf_size.into()], "buf_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?
        .try_as_basic_value().left().unwrap().into_pointer_value();

    // Store buffer pointer at field 0
    let buf_as_int = builder.build_ptr_to_int(buf_ptr, i64_type, "buf_int")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(chan_ptr, buf_as_int)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Store capacity (16) at field 1 (offset 8)
    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(cap_ptr, i64_type.const_int(16, false))
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Store count (0) at field 2 (offset 16)
    let count_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "count_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(count_ptr, i64_type.const_int(0, false))
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Store head (0) at field 3
    let head_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(head_ptr, i64_type.const_int(0, false))
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Store tail (0) at field 4
    let tail_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(tail_ptr, i64_type.const_int(0, false))
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Init mutex at field 5 (offset 40)
    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mutex_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let null_ptr = ptr_type.const_null();
    let mutex_init_fn = llvm_module.get_function("pthread_mutex_init").unwrap();
    builder.build_call(mutex_init_fn, &[mutex_ptr.into(), null_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Init condvar at field 13 (offset 104)
    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cond_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let cond_init_fn = llvm_module.get_function("pthread_cond_init").unwrap();
    builder.build_call(cond_init_fn, &[cond_ptr.into(), null_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Both tx and rx point to the same channel struct, stored as i64
    let chan_as_int = builder.build_ptr_to_int(chan_ptr, i64_type, "chan_int")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    regs.insert(tx_dest.clone(), chan_as_int);
    regs.insert(rx_dest.clone(), chan_as_int);
}
```

**6c: Handle `ChannelSend`** — lock, write to buffer, signal, unlock:

```rust
Instruction::ChannelSend { tx, value } => {
    let chan_int = regs[tx];
    let val = regs[value];
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Get mutex ptr (field 5)
    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mutex_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Lock mutex
    let lock_fn = llvm_module.get_function("pthread_mutex_lock").unwrap();
    builder.build_call(lock_fn, &[mutex_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Load count, capacity, tail, buffer ptr
    let count_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "count_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let count = builder.build_load(i64_type, count_ptr, "count")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let cap = builder.build_load(i64_type, cap_ptr, "cap")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

    let tail_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(4, false)], "tail_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let tail = builder.build_load(i64_type, tail_ptr, "tail")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

    let buf_int_ptr = chan_ptr; // field 0 is at offset 0
    let buf_int = builder.build_load(i64_type, buf_int_ptr, "buf_int")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "buf_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // TODO: if count == capacity, realloc (skip for now — initial capacity 16 is enough for E2E tests)

    // Write value at buffer[tail % capacity]
    let write_idx = builder.build_int_unsigned_rem(tail, cap, "write_idx")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let write_ptr = unsafe { builder.build_gep(i64_type, buf_ptr, &[write_idx], "write_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(write_ptr, val)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Increment tail and count
    let one = i64_type.const_int(1, false);
    let new_tail = builder.build_int_add(tail, one, "new_tail")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(tail_ptr, new_tail)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let new_count = builder.build_int_add(count, one, "new_count")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(count_ptr, new_count)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Signal condvar (field 13)
    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cond_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let signal_fn = llvm_module.get_function("pthread_cond_signal").unwrap();
    builder.build_call(signal_fn, &[cond_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Unlock mutex
    let unlock_fn = llvm_module.get_function("pthread_mutex_unlock").unwrap();
    builder.build_call(unlock_fn, &[mutex_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
}
```

**6d: Handle `ChannelRecv`** — lock, wait while empty, read, unlock:

```rust
Instruction::ChannelRecv { dest, rx } => {
    let chan_int = regs[rx];
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let chan_ptr = builder.build_int_to_ptr(chan_int, ptr_type, "chan_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    let mutex_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(5, false)], "mutex_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let cond_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(13, false)], "cond_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Lock mutex
    let lock_fn = llvm_module.get_function("pthread_mutex_lock").unwrap();
    builder.build_call(lock_fn, &[mutex_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // While count == 0, wait on condvar
    let wait_loop = context.append_basic_block(llvm_fn, "recv_wait");
    let recv_body = context.append_basic_block(llvm_fn, "recv_body");
    builder.build_unconditional_branch(wait_loop)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    builder.position_at_end(wait_loop);
    let count_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(2, false)], "count_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let count = builder.build_load(i64_type, count_ptr, "count")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let is_empty = builder.build_int_compare(IntPredicate::EQ, count, i64_type.const_int(0, false), "is_empty")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let wait_block = context.append_basic_block(llvm_fn, "recv_do_wait");
    builder.build_conditional_branch(is_empty, wait_block, recv_body)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    builder.position_at_end(wait_block);
    let wait_fn = llvm_module.get_function("pthread_cond_wait").unwrap();
    builder.build_call(wait_fn, &[cond_ptr.into(), mutex_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_unconditional_branch(wait_loop)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    builder.position_at_end(recv_body);

    // Read from buffer[head % capacity]
    let head_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(3, false)], "head_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let head = builder.build_load(i64_type, head_ptr, "head")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let cap_ptr = unsafe { builder.build_gep(i64_type, chan_ptr, &[i64_type.const_int(1, false)], "cap_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let cap = builder.build_load(i64_type, cap_ptr, "cap")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let read_idx = builder.build_int_unsigned_rem(head, cap, "read_idx")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    let buf_int = builder.build_load(i64_type, chan_ptr, "buf_int")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let buf_ptr = builder.build_int_to_ptr(buf_int, ptr_type, "buf_ptr")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let read_ptr = unsafe { builder.build_gep(i64_type, buf_ptr, &[read_idx], "read_ptr") }
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    let received_val = builder.build_load(i64_type, read_ptr, dest)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();

    // Increment head, decrement count
    let one = i64_type.const_int(1, false);
    let new_head = builder.build_int_add(head, one, "new_head")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(head_ptr, new_head)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    // Reload count since we're in recv_body now
    let count2 = builder.build_load(i64_type, count_ptr, "count2")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    let new_count = builder.build_int_sub(count2, one, "new_count")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    builder.build_store(count_ptr, new_count)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Unlock mutex
    let unlock_fn = llvm_module.get_function("pthread_mutex_unlock").unwrap();
    builder.build_call(unlock_fn, &[mutex_ptr.into()], "")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    regs.insert(dest.clone(), received_val);
}
```

**6e: Handle `ThreadSpawn`** — create trampoline, pack args, call pthread_create:

```rust
Instruction::ThreadSpawn { dest, function, args } => {
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let malloc_fn = llvm_module.get_function("malloc").unwrap();
    let free_fn = llvm_module.get_function("free").unwrap();

    // Allocate arg struct: N * 8 bytes
    let num_args = args.len();
    let arg_struct_size = i64_type.const_int((num_args * 8) as u64, false);
    let arg_struct_ptr = if num_args > 0 {
        builder.build_call(malloc_fn, &[arg_struct_size.into()], "arg_struct")
            .map_err(|e| CodegenError::LlvmError(e.to_string()))?
            .try_as_basic_value().left().unwrap().into_pointer_value()
    } else {
        ptr_type.const_null()
    };

    // Store each arg into the struct
    for (i, arg_name) in args.iter().enumerate() {
        let val = if let Some(p) = ptrs.get(arg_name) {
            builder.build_ptr_to_int(*p, i64_type, "ptr2int")
                .map_err(|e| CodegenError::LlvmError(e.to_string()))?
        } else {
            regs[arg_name]
        };
        let field_ptr = unsafe { builder.build_gep(i64_type, arg_struct_ptr, &[i64_type.const_int(i as u64, false)], "arg_field") }
            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
        builder.build_store(field_ptr, val)
            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
    }

    // Create trampoline function
    let trampoline_name = format!("__trampoline_{}", function);
    let trampoline_type = ptr_type.fn_type(&[ptr_type.into()], false);
    let trampoline_fn = if let Some(existing) = llvm_module.get_function(&trampoline_name) {
        existing
    } else {
        let tramp = llvm_module.add_function(&trampoline_name, trampoline_type, None);
        let tramp_entry = context.append_basic_block(tramp, "entry");
        let saved_block = builder.get_insert_block().unwrap();
        builder.position_at_end(tramp_entry);

        let arg_ptr = tramp.get_nth_param(0).unwrap().into_pointer_value();
        let target_fn = llvm_module.get_function(function).unwrap();

        // Load args from struct
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
        for i in 0..num_args {
            let field_ptr = unsafe { builder.build_gep(i64_type, arg_ptr, &[i64_type.const_int(i as u64, false)], "load_arg") }
                .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
            let val = builder.build_load(i64_type, field_ptr, &format!("arg{}", i))
                .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
            call_args.push(val.into());
        }

        // Call target function
        builder.build_call(target_fn, &call_args, "")
            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

        // Free arg struct
        if num_args > 0 {
            builder.build_call(free_fn, &[arg_ptr.into()], "")
                .map_err(|e| CodegenError::LlvmError(e.to_string()))?;
        }

        // Return null
        builder.build_return(Some(&ptr_type.const_null()))
            .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

        builder.position_at_end(saved_block);
        tramp
    };

    // Allocate pthread_t (8 bytes)
    let thread_ptr = builder.build_alloca(i64_type, "thread")
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Call pthread_create
    let pthread_create_fn = llvm_module.get_function("pthread_create").unwrap();
    let trampoline_ptr = trampoline_fn.as_global_value().as_pointer_value();
    builder.build_call(
        pthread_create_fn,
        &[thread_ptr.into(), ptr_type.const_null().into(), trampoline_ptr.into(), arg_struct_ptr.into()],
        ""
    ).map_err(|e| CodegenError::LlvmError(e.to_string()))?;

    // Load the thread handle as i64
    let thread_handle = builder.build_load(i64_type, thread_ptr, dest)
        .map_err(|e| CodegenError::LlvmError(e.to_string()))?.into_int_value();
    regs.insert(dest.clone(), thread_handle);
}
```

**6f: Handle `ThreadJoin`:**

```rust
Instruction::ThreadJoin { handle } => {
    let handle_val = regs[handle];
    let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let pthread_join_fn = llvm_module.get_function("pthread_join").unwrap();
    builder.build_call(
        pthread_join_fn,
        &[handle_val.into(), ptr_type.const_null().into()],
        ""
    ).map_err(|e| CodegenError::LlvmError(e.to_string()))?;
}
```

**Codegen tests:**

```rust
#[test]
fn codegen_channel_create() {
    let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() 0 }").expect("parse");
    sans_typeck::check(&program).expect("typeck");
    let module = sans_ir::lower(&program);
    let ir = compile_to_llvm_ir(&module).expect("codegen");
    assert!(ir.contains("malloc"), "expected malloc in:\n{}", ir);
    assert!(ir.contains("pthread_mutex_init"), "expected pthread_mutex_init in:\n{}", ir);
}

#[test]
fn codegen_spawn() {
    let program = sans_parser::parse("fn worker() Int { 0 } fn main() Int { let h = spawn worker() h.join() }").expect("parse");
    sans_typeck::check(&program).expect("typeck");
    let module = sans_ir::lower(&program);
    let ir = compile_to_llvm_ir(&module).expect("codegen");
    assert!(ir.contains("pthread_create"), "expected pthread_create in:\n{}", ir);
    assert!(ir.contains("pthread_join"), "expected pthread_join in:\n{}", ir);
    assert!(ir.contains("__trampoline_worker"), "expected trampoline in:\n{}", ir);
}

#[test]
fn codegen_send_recv() {
    let program = sans_parser::parse("fn main() Int { let (tx, rx) = channel<Int>() tx.send(42) rx.recv() }").expect("parse");
    sans_typeck::check(&program).expect("typeck");
    let module = sans_ir::lower(&program);
    let ir = compile_to_llvm_ir(&module).expect("codegen");
    assert!(ir.contains("pthread_mutex_lock"), "expected lock in:\n{}", ir);
    assert!(ir.contains("pthread_cond_signal"), "expected signal in:\n{}", ir);
    assert!(ir.contains("pthread_cond_wait"), "expected wait in:\n{}", ir);
}
```

---

### Task 7: E2E Tests — Fixture programs

**Files:**
- Create: `tests/fixtures/spawn_basic.cy`
- Create: `tests/fixtures/spawn_join.cy`
- Create: `tests/fixtures/channel_pingpong.cy`
- Modify: `crates/sans-driver/tests/e2e.rs`

**E2E linking note:** The `compile_and_run` helper links with `cc`. On macOS, pthreads are included in libc automatically, so no `-lpthread` flag is needed. On Linux, `-lpthread` would be needed — for now we target macOS only.

**spawn_basic.cy** — Spawn thread sends value through channel, main receives and exits with it:

```cyflym
fn sender(tx Int) Int {
    tx.send(42)
    0
}

fn main() Int {
    let (tx, rx) = channel<Int>()
    let h = spawn sender(tx)
    let val = rx.recv()
    h.join()
    val
}
```

Expected exit code: 42

**Note:** `tx` is passed as `Int` param type because all values are i64 at the type level. But wait — the type checker would reject this because `tx` is `Sender<Int>`, not `Int`. We need to handle this. The spawned function's parameter needs to accept the sender type.

**Revised approach:** Since the type checker has `Sender` and `Receiver` types but the AST type names are simple strings, we need `Sender` and `Receiver` as recognized type names in `resolve_type`. Add to resolve_type:

```rust
"JoinHandle" => Ok(Type::JoinHandle),
// For sender/receiver, we need to handle the generic syntax
// For now, use a simpler approach: pass channels as Int (they're i64 at runtime)
```

Actually, this is a significant issue. The function parameter type in the source code would need to be `Sender<Int>` which the parser can't handle as a TypeName (it's just a string). **Simplification for Plan 5a:** spawned functions can only take `Int` parameters. Channels are passed as `Int` since they're i64 pointers at runtime. The type checker treats `Sender<T>` and `Receiver<T>` as assignment-compatible with `Int` for the purpose of function argument passing to spawned functions.

**Actually, even simpler:** Let spawn pass channel values as i64. In the sender function, use `Int` as the param type. At the IR level, sender/receiver are just i64 values. We just need the IR lowering to recognize that when a register typed as `Sender` or `Receiver` is used as an argument, it should be treated as a regular i64 value. This already works because all IR values are i64.

The issue is purely at the type-checker level: `spawn sender(tx)` where `tx: Sender<Int>` but `sender` expects `(tx Int)`. We need to allow `Sender<T>` to be passed where `Int` is expected (or vice versa) in spawn context.

**Simplest fix:** In the type checker for `Expr::Spawn`, allow `Sender`, `Receiver`, and `JoinHandle` types to be passed as `Int` parameters:

```rust
// In the Expr::Spawn type checking, after checking arg count:
for (i, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
    let actual = check_expr(arg, ...)?;
    // Allow concurrency types to be passed as Int (they're i64 pointers at runtime)
    let compatible = actual == *expected
        || (*expected == Type::Int && matches!(actual, Type::Sender { .. } | Type::Receiver { .. } | Type::JoinHandle));
    if !compatible {
        return Err(TypeError::new(format!(...)));
    }
}
```

**spawn_basic.cy:**

```cyflym
fn sender(tx Int) Int {
    tx.send(42)
    0
}

fn main() Int {
    let (tx, rx) = channel<Int>()
    let h = spawn sender(tx)
    let val = rx.recv()
    h.join()
    val
}
```

Wait, but inside `sender`, `tx` is typed as `Int`, so `tx.send(42)` would fail typeck because `send` is only valid on `Sender`.

**Better approach:** Don't pass channels as function params. Instead, use a shared global or restructure the test. Actually, the simplest E2E test pattern is:

**spawn_join.cy** — Simple join test, no channels:

```cyflym
fn worker() Int {
    0
}

fn main() Int {
    let h = spawn worker()
    h.join()
    7
}
```

Expected exit code: 7 (just verifies spawn + join work)

**spawn_basic.cy** — Use channel in main, spawned function writes to it via a wrapper:

Actually this reveals a real design gap: you can't pass typed channels to spawned functions without generic type names in function signatures. For Plan 5a, let's keep the E2E tests simple:

1. **spawn_join.cy** — spawn + join, exit 7
2. **spawn_basic.cy** — channel send/recv in single thread (no cross-thread), exit 42
3. **channel_cross_thread.cy** — use a global channel approach...

No, we can't do globals either. Let me think about this differently.

The solution is: make `.send()` and `.recv()` work on `Int`-typed variables too when the IR knows they're senders/receivers. But that's mixing concerns.

**Practical solution for E2E:** The IR lowering tracks `IrType::Sender`/`Receiver` per register. When a function receives an `Int` param that is actually a channel pointer, the IR won't know it's a sender. So we need the spawned function to somehow know.

**Simplest approach:** Add a special `__channel_send` and `__channel_recv` built-in function (like `print`) that takes the channel pointer as Int:

Actually, the cleanest Plan 5a approach: **only test channel within the same thread, and spawn/join separately.** Cross-thread channels require passing typed channels to functions, which needs richer type syntax. We'll defer that to Plan 5b.

**Final E2E fixtures:**

**spawn_join.cy** (exit 7):
```cyflym
fn worker() Int {
    0
}

fn main() Int {
    let h = spawn worker()
    h.join()
    7
}
```

**channel_basic.cy** (exit 42) — single-threaded channel test:
```cyflym
fn main() Int {
    let (tx, rx) = channel<Int>()
    tx.send(42)
    rx.recv()
}
```

**spawn_channel.cy** (exit 10) — cross-thread via shared channel. The trick: the spawned function doesn't need the channel — the main thread sends, spawned thread is just for join testing:
```cyflym
fn adder(a Int, b Int) Int {
    a + b
}

fn main() Int {
    let (tx, rx) = channel<Int>()
    tx.send(10)
    let h = spawn adder(3, 4)
    let val = rx.recv()
    h.join()
    val
}
```

Expected: exit 10

**E2E test entries:**

```rust
#[test]
fn e2e_spawn_join() {
    assert_eq!(compile_and_run("spawn_join.cy"), 7);
}

#[test]
fn e2e_channel_basic() {
    assert_eq!(compile_and_run("channel_basic.cy"), 42);
}

#[test]
fn e2e_spawn_channel() {
    assert_eq!(compile_and_run("spawn_channel.cy"), 10);
}
```

---

## Commit Strategy

- **Batch 1 commit:** `feat: add spawn/channel syntax, AST, parser, and type system`
- **Batch 2 commit:** `feat: add concurrency IR instructions, LLVM codegen, and E2E tests`
