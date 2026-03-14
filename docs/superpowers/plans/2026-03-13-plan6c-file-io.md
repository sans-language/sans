# Plan 6c: File I/O Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add file I/O built-in functions (`file_read`, `file_write`, `file_append`, `file_exists`) to the Sans compiler.

**Architecture:** Four new built-in functions follow the existing pattern: type checker recognizes function name → IR instruction → codegen emits C stdlib calls. No new types. Error handling via sentinel values. Codegen uses inkwell basic-block splits for fopen null checks.

**Tech Stack:** Rust, inkwell 0.8 (LLVM 17), C stdlib (fopen/fread/fwrite/fclose/ftell/fseek/access)

**Build requirement:** `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)` must be set for all cargo commands.

**Codegen API patterns (IMPORTANT — read before implementing Task 3):**
- `builder.build_call(fn, args, name)` returns a `Result` — always use `.map_err(|e| CodegenError::LlvmError(e.to_string()))?`
- To extract a return value: `match call_site.try_as_basic_value() { inkwell::values::ValueKind::Basic(bv) => bv.into_int_value(), _ => return Err(...) }`
- For pointer returns: `bv.into_pointer_value()` instead of `into_int_value()`
- When return value is not needed, just discard: `builder.build_call(...).map_err(...)?;`
- All other builder calls (build_store, build_int_compare, etc.) also use `.map_err(...)?` pattern
- **Never use `.unwrap()` on builder calls** — always propagate errors
- For phi nodes: save the current block BEFORE the conditional branch with `let pre_branch_bb = builder.get_insert_block().unwrap();`, then use `pre_branch_bb` as the incoming block for the error path

---

## Chunk 1: Type Checker + IR

### Task 1: Type Checker — File I/O built-in function checks

**Files:**
- Modify: `crates/sans-typeck/src/lib.rs`

- [ ] **Step 1: Write the failing test for file_read type check**

In `crates/sans-typeck/src/lib.rs`, add at the end of the `mod tests` block (before the final `}`):

```rust
#[test]
fn check_file_read_builtin() {
    do_check("fn main() Int { let s = file_read(\"test.txt\") 0 }").unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p sans-typeck check_file_read_builtin`
Expected: FAIL — `file_read` is not recognized as a function

- [ ] **Step 3: Write the failing test for file_write type check**

```rust
#[test]
fn check_file_write_builtin() {
    do_check("fn main() Int { file_write(\"test.txt\", \"hello\") }").unwrap();
}
```

- [ ] **Step 4: Write the failing test for file_exists type check**

```rust
#[test]
fn check_file_exists_builtin() {
    do_check("fn main() Int { if file_exists(\"test.txt\") { 1 } else { 0 } }").unwrap();
}
```

- [ ] **Step 5: Write the failing test for wrong argument type**

```rust
#[test]
fn check_file_read_wrong_type() {
    let err = do_check("fn main() Int { let s = file_read(42) 0 }").unwrap_err();
    assert!(err.message.contains("String"),
        "expected type error mentioning String, got: {}", err.message);
}
```

- [ ] **Step 6: Implement file I/O built-in checks**

In `crates/sans-typeck/src/lib.rs`, find the `Expr::Call` arm in `check_expr` (around line 599). After the `string_to_int` check (around line 633), add:

```rust
} else if function == "file_read" {
    if args.len() != 1 {
        return Err(TypeError::new(format!(
            "'file_read' expects 1 argument, got {}", args.len()
        )));
    }
    let arg_type = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if arg_type != Type::String {
        return Err(TypeError::new(format!(
            "'file_read' expects argument of type String, got {}", arg_type
        )));
    }
    return Ok(Type::String);
} else if function == "file_write" || function == "file_append" {
    if args.len() != 2 {
        return Err(TypeError::new(format!(
            "'{}' expects 2 arguments, got {}", function, args.len()
        )));
    }
    let path_type = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if path_type != Type::String {
        return Err(TypeError::new(format!(
            "'{}' expects first argument of type String, got {}", function, path_type
        )));
    }
    let content_type = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if content_type != Type::String {
        return Err(TypeError::new(format!(
            "'{}' expects second argument of type String, got {}", function, content_type
        )));
    }
    return Ok(Type::Int);
} else if function == "file_exists" {
    if args.len() != 1 {
        return Err(TypeError::new(format!(
            "'file_exists' expects 1 argument, got {}", args.len()
        )));
    }
    let arg_type = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if arg_type != Type::String {
        return Err(TypeError::new(format!(
            "'file_exists' expects argument of type String, got {}", arg_type
        )));
    }
    return Ok(Type::Bool);
```

- [ ] **Step 7: Run all type checker tests**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p sans-typeck`
Expected: All 89 tests pass (85 existing + 4 new)

- [ ] **Step 8: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`
Expected: All tests pass

- [ ] **Step 9: Commit**

```bash
git add crates/sans-typeck/src/lib.rs
git commit -m "feat(typeck): add file I/O built-in function type checks"
```

---

### Task 2: IR — File I/O instructions and lowering

**Files:**
- Modify: `crates/sans-ir/src/ir.rs`
- Modify: `crates/sans-ir/src/lib.rs`

- [ ] **Step 1: Write the failing test for file_read IR lowering**

In `crates/sans-ir/src/lib.rs`, add at the end of the `mod tests` block:

```rust
#[test]
fn lower_file_read_instruction() {
    let program = parse("fn main() Int { let s = file_read(\"test.txt\") 0 }");
    let module = lower(&program, None, &HashMap::new());
    let func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_file_read = func.body.iter().any(|i| {
        matches!(i, Instruction::FileRead { .. })
    });
    assert!(has_file_read, "expected FileRead instruction, got: {:?}", func.body);
}
```

- [ ] **Step 2: Write the failing test for file_write IR lowering**

```rust
#[test]
fn lower_file_write_instruction() {
    let program = parse("fn main() Int { file_write(\"test.txt\", \"hello\") }");
    let module = lower(&program, None, &HashMap::new());
    let func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_file_write = func.body.iter().any(|i| {
        matches!(i, Instruction::FileWrite { .. })
    });
    assert!(has_file_write, "expected FileWrite instruction, got: {:?}", func.body);
}
```

- [ ] **Step 3: Add instruction variants to ir.rs**

In `crates/sans-ir/src/ir.rs`, add these variants to the `Instruction` enum (after `StringToInt`, around line 133):

```rust
// File I/O
FileRead {
    dest: Reg,
    path: Reg,
},
FileWrite {
    dest: Reg,
    path: Reg,
    content: Reg,
},
FileAppend {
    dest: Reg,
    path: Reg,
    content: Reg,
},
FileExists {
    dest: Reg,
    path: Reg,
},
```

- [ ] **Step 4: Add IR lowering for file I/O calls**

In `crates/sans-ir/src/lib.rs`, find where `Expr::Call` is lowered (around line 412). After the `string_to_int` lowering (around line 450), add:

```rust
} else if function == "file_read" {
    let path_reg = self.lower_expr(&args[0]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::FileRead {
        dest: dest.clone(),
        path: path_reg,
    });
    self.reg_types.insert(dest.clone(), IrType::Str);
    return dest;
} else if function == "file_write" {
    let path_reg = self.lower_expr(&args[0]);
    let content_reg = self.lower_expr(&args[1]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::FileWrite {
        dest: dest.clone(),
        path: path_reg,
        content: content_reg,
    });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "file_append" {
    let path_reg = self.lower_expr(&args[0]);
    let content_reg = self.lower_expr(&args[1]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::FileAppend {
        dest: dest.clone(),
        path: path_reg,
        content: content_reg,
    });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "file_exists" {
    let path_reg = self.lower_expr(&args[0]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::FileExists {
        dest: dest.clone(),
        path: path_reg,
    });
    self.reg_types.insert(dest.clone(), IrType::Bool);
    return dest;
```

- [ ] **Step 5: Run IR tests**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p sans-ir`
Expected: All 34 tests pass (32 existing + 2 new)

- [ ] **Step 6: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/sans-ir/src/ir.rs crates/sans-ir/src/lib.rs
git commit -m "feat(ir): add file I/O instructions and lowering"
```

---

## Chunk 2: Codegen + E2E

### Task 3: Codegen — File I/O instruction compilation

**Files:**
- Modify: `crates/sans-codegen/src/lib.rs`

This is the largest task. Each file operation needs C stdlib function declarations and LLVM IR emission with basic-block splits for error handling.

- [ ] **Step 1: Add C stdlib function declarations**

In `crates/sans-codegen/src/lib.rs`, find the function declarations section (around lines 79-123). After the existing declarations (strtol, around line 123), add:

```rust
// File I/O functions
let fopen_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
llvm_module.add_function("fopen", fopen_type, Some(Linkage::External));

let fclose_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
llvm_module.add_function("fclose", fclose_type, Some(Linkage::External));

let fread_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
llvm_module.add_function("fread", fread_type, Some(Linkage::External));

let fwrite_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
llvm_module.add_function("fwrite", fwrite_type, Some(Linkage::External));

let fseek_type = i32_type.fn_type(&[i8_ptr_type.into(), i64_type.into(), i32_type.into()], false);
llvm_module.add_function("fseek", fseek_type, Some(Linkage::External));

let ftell_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
llvm_module.add_function("ftell", ftell_type, Some(Linkage::External));

let access_type = i32_type.fn_type(&[i8_ptr_type.into(), i32_type.into()], false);
llvm_module.add_function("access", access_type, Some(Linkage::External));
```

Note: `i8_ptr_type`, `i32_type`, `i64_type` should already be defined earlier in the function. If `i32_type` isn't defined, add `let i32_type = context.i32_type();` near the other type definitions.

- [ ] **Step 2: Implement FileExists codegen**

Start with the simplest instruction. In the main instruction match (find where `IntToString` and `StringToInt` are handled), add after the last existing instruction handler.

**IMPORTANT:** The code below shows the ALGORITHM, not copy-paste-ready code. You MUST adapt all `builder.build_*` and `build_call` calls to use the project's error handling pattern (see "Codegen API patterns" at the top of this plan). Read existing codegen handlers (e.g., `IntToString` around line 1408, `StringToInt` around line 1436) and match their exact style.

```rust
Instruction::FileExists { dest, path } => {
    let access_fn = llvm_module.get_function("access").unwrap();

    // Get path pointer
    let path_ptr = if let Some(p) = ptrs.get(path) {
        *p
    } else {
        let path_int = *regs.get(path).unwrap();
        builder.build_int_to_ptr(path_int, i8_ptr_type, "path_ptr").unwrap()
    };

    // access(path, 0) — F_OK = 0
    let f_ok = i32_type.const_int(0, false);
    let result = builder.build_call(access_fn, &[path_ptr.into(), f_ok.into()], "access_result").unwrap();
    let result_int = result.try_as_basic_value().left().unwrap().into_int_value();

    // Compare result == 0
    let is_zero = builder.build_int_compare(
        inkwell::IntPredicate::EQ, result_int, i32_type.const_int(0, false), "exists"
    ).unwrap();

    // Zero-extend i1 to i64
    let exists_i64 = builder.build_int_z_extend(is_zero, i64_type, "exists_i64").unwrap();
    regs.insert(dest.clone(), exists_i64);
}
```

- [ ] **Step 3: Implement FileRead codegen**

```rust
Instruction::FileRead { dest, path } => {
    let fopen_fn = llvm_module.get_function("fopen").unwrap();
    let fclose_fn = llvm_module.get_function("fclose").unwrap();
    let fseek_fn = llvm_module.get_function("fseek").unwrap();
    let ftell_fn = llvm_module.get_function("ftell").unwrap();
    let fread_fn = llvm_module.get_function("fread").unwrap();
    let malloc_fn = llvm_module.get_function("malloc").unwrap();

    // Get path pointer
    let path_ptr = if let Some(p) = ptrs.get(path) {
        *p
    } else {
        let path_int = *regs.get(path).unwrap();
        builder.build_int_to_ptr(path_int, i8_ptr_type, "path_ptr").unwrap()
    };

    // Build mode string "r"
    let mode_str = builder.build_global_string_ptr("r", "read_mode").unwrap();

    // fopen(path, "r")
    let file_ptr = builder.build_call(fopen_fn, &[path_ptr.into(), mode_str.as_pointer_value().into()], "file_ptr").unwrap();
    let file_ptr = file_ptr.try_as_basic_value().left().unwrap().into_pointer_value();

    // Check if null
    let null_ptr = i8_ptr_type.const_null();
    let is_null = builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        builder.build_ptr_to_int(file_ptr, i64_type, "file_int").unwrap(),
        builder.build_ptr_to_int(null_ptr, i64_type, "null_int").unwrap(),
        "is_null"
    ).unwrap();

    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();
    let then_bb = context.append_basic_block(current_fn, "fread_ok");
    let error_bb = context.append_basic_block(current_fn, "fread_err");
    let merge_bb = context.append_basic_block(current_fn, "fread_merge");

    builder.build_conditional_branch(is_null, error_bb, then_bb).unwrap();

    // Error path: return empty string
    builder.position_at_end(error_bb);
    let one = i64_type.const_int(1, false);
    let empty_buf = builder.build_call(malloc_fn, &[one.into()], "empty_buf").unwrap();
    let empty_buf = empty_buf.try_as_basic_value().left().unwrap().into_pointer_value();
    let zero_byte = context.i8_type().const_int(0, false);
    builder.build_store(empty_buf, zero_byte).unwrap();
    let empty_int = builder.build_ptr_to_int(empty_buf, i64_type, "empty_int").unwrap();
    builder.build_unconditional_branch(merge_bb).unwrap();

    // Success path: read file
    builder.position_at_end(then_bb);
    // fseek(file, 0, SEEK_END=2)
    let seek_end = i32_type.const_int(2, false);
    let zero_i64 = i64_type.const_int(0, false);
    builder.build_call(fseek_fn, &[file_ptr.into(), zero_i64.into(), seek_end.into()], "").unwrap();
    // size = ftell(file)
    let size = builder.build_call(ftell_fn, &[file_ptr.into()], "size").unwrap();
    let size = size.try_as_basic_value().left().unwrap().into_int_value();
    // fseek(file, 0, SEEK_SET=0)
    let seek_set = i32_type.const_int(0, false);
    builder.build_call(fseek_fn, &[file_ptr.into(), zero_i64.into(), seek_set.into()], "").unwrap();
    // buf = malloc(size + 1)
    let size_plus_one = builder.build_int_add(size, i64_type.const_int(1, false), "size_plus_one").unwrap();
    let buf = builder.build_call(malloc_fn, &[size_plus_one.into()], "buf").unwrap();
    let buf = buf.try_as_basic_value().left().unwrap().into_pointer_value();
    // fread(buf, 1, size, file)
    let one_i64 = i64_type.const_int(1, false);
    builder.build_call(fread_fn, &[buf.into(), one_i64.into(), size.into(), file_ptr.into()], "").unwrap();
    // null-terminate: buf[size] = 0
    let end_ptr = unsafe { builder.build_gep(context.i8_type(), buf, &[size], "end_ptr").unwrap() };
    builder.build_store(end_ptr, zero_byte).unwrap();
    // fclose(file)
    builder.build_call(fclose_fn, &[file_ptr.into()], "").unwrap();
    let buf_int = builder.build_ptr_to_int(buf, i64_type, "buf_int").unwrap();
    builder.build_unconditional_branch(merge_bb).unwrap();

    // Merge: phi node
    builder.position_at_end(merge_bb);
    let phi = builder.build_phi(i64_type, "read_result").unwrap();
    phi.add_incoming(&[(&empty_int, error_bb), (&buf_int, then_bb)]);
    let result_int = phi.as_basic_value().into_int_value();
    regs.insert(dest.clone(), result_int);
    let result_ptr = builder.build_int_to_ptr(result_int, i8_ptr_type, "read_ptr").unwrap();
    ptrs.insert(dest.clone(), result_ptr);
}
```

- [ ] **Step 4: Implement FileWrite codegen**

```rust
Instruction::FileWrite { dest, path, content } => {
    let fopen_fn = llvm_module.get_function("fopen").unwrap();
    let fclose_fn = llvm_module.get_function("fclose").unwrap();
    let fwrite_fn = llvm_module.get_function("fwrite").unwrap();
    let strlen_fn = llvm_module.get_function("strlen").unwrap();

    // Get path pointer
    let path_ptr = if let Some(p) = ptrs.get(path) {
        *p
    } else {
        let path_int = *regs.get(path).unwrap();
        builder.build_int_to_ptr(path_int, i8_ptr_type, "path_ptr").unwrap()
    };

    // Get content pointer
    let content_ptr = if let Some(p) = ptrs.get(content) {
        *p
    } else {
        let content_int = *regs.get(content).unwrap();
        builder.build_int_to_ptr(content_int, i8_ptr_type, "content_ptr").unwrap()
    };

    // strlen(content)
    let len = builder.build_call(strlen_fn, &[content_ptr.into()], "len").unwrap();
    let len = len.try_as_basic_value().left().unwrap().into_int_value();

    // fopen(path, "w")
    let mode_str = builder.build_global_string_ptr("w", "write_mode").unwrap();
    let file_ptr = builder.build_call(fopen_fn, &[path_ptr.into(), mode_str.as_pointer_value().into()], "file_ptr").unwrap();
    let file_ptr = file_ptr.try_as_basic_value().left().unwrap().into_pointer_value();

    // Check if null
    let null_ptr = i8_ptr_type.const_null();
    let is_null = builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        builder.build_ptr_to_int(file_ptr, i64_type, "file_int").unwrap(),
        builder.build_ptr_to_int(null_ptr, i64_type, "null_int").unwrap(),
        "is_null"
    ).unwrap();

    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();
    let then_bb = context.append_basic_block(current_fn, "fwrite_ok");
    let merge_bb = context.append_basic_block(current_fn, "fwrite_merge");

    builder.build_conditional_branch(is_null, merge_bb, then_bb).unwrap();

    // Success path
    builder.position_at_end(then_bb);
    let one_i64 = i64_type.const_int(1, false);
    builder.build_call(fwrite_fn, &[content_ptr.into(), one_i64.into(), len.into(), file_ptr.into()], "").unwrap();
    builder.build_call(fclose_fn, &[file_ptr.into()], "").unwrap();
    builder.build_unconditional_branch(merge_bb).unwrap();

    // Merge: phi for 0/1
    builder.position_at_end(merge_bb);
    let phi = builder.build_phi(i64_type, "write_result").unwrap();
    let zero = i64_type.const_int(0, false);
    let one_result = i64_type.const_int(1, false);
    let entry_bb = is_null.get_parent(); // block before branch
    // Note: the incoming for error is the block that had the conditional branch
    phi.add_incoming(&[(&zero, entry_bb), (&one_result, then_bb)]);
    let result = phi.as_basic_value().into_int_value();
    regs.insert(dest.clone(), result);
}
```

**Phi node pattern:** Save the current block BEFORE the conditional branch: `let pre_branch_bb = builder.get_insert_block().unwrap();`. Then use `pre_branch_bb` as the incoming block for the error/null path in the phi node. The success path's incoming block is `then_bb`.

- [ ] **Step 5: Implement FileAppend codegen**

Same as FileWrite but with mode `"a"` instead of `"w"`. Copy the FileWrite handler and change:
- The mode string: `builder.build_global_string_ptr("a", "append_mode")`
- The block names: `"fappend_ok"`, `"fappend_merge"`

- [ ] **Step 6: Run compilation check**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build`
Expected: Compiles successfully

- [ ] **Step 7: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`
Expected: All existing tests pass (codegen handles new instructions but no existing tests use them)

- [ ] **Step 8: Commit**

```bash
git add crates/sans-codegen/src/lib.rs
git commit -m "feat(codegen): add file I/O instruction compilation with fopen null checks"
```

---

### Task 4: E2E Tests — File I/O integration

**Files:**
- Modify: `crates/sans-driver/tests/e2e.rs`
- Create: `tests/fixtures/file_write_read.sans`
- Create: `tests/fixtures/file_exists_check.sans`

- [ ] **Step 1: Create `file_write_read.sans` fixture**

Create `tests/fixtures/file_write_read.sans`:

```sans
fn main() Int {
    let ok = file_write("/tmp/sans_test_write.txt", "hello world")
    let content = file_read("/tmp/sans_test_write.txt")
    content.len()
}
```

Expected exit code: **11** (length of "hello world")

- [ ] **Step 2: Create `file_exists_check.sans` fixture**

Create `tests/fixtures/file_exists_check.sans`:

```sans
fn main() Int {
    file_write("/tmp/sans_test_exists.txt", "test")
    let a = file_exists("/tmp/sans_test_exists.txt")
    let b = file_exists("/tmp/sans_nonexistent_file_xyz.txt")
    let result = 0
    if a {
        let result = 1
    }
    if b {
        let result = result + 10
    }
    result
}
```

Wait — Sans uses `let` for new bindings, not reassignment. And `if` without `else` is a statement, not an expression. Let me reconsider. The simplest approach:

```sans
fn main() Int {
    file_write("/tmp/sans_test_exists.txt", "test")
    let a = file_exists("/tmp/sans_test_exists.txt")
    let b = file_exists("/tmp/sans_nonexistent_file_xyz.txt")
    if a {
        if b {
            11
        } else {
            1
        }
    } else {
        if b {
            10
        } else {
            0
        }
    }
}
```

Expected exit code: **1** (first file exists = true, second doesn't = false → 1)

- [ ] **Step 3: Add E2E test functions**

In `crates/sans-driver/tests/e2e.rs`, add after the last test:

```rust
#[test]
fn e2e_file_write_read() {
    assert_eq!(compile_and_run("file_write_read.sans"), 11);
}

#[test]
fn e2e_file_exists_check() {
    assert_eq!(compile_and_run("file_exists_check.sans"), 1);
}
```

- [ ] **Step 4: Run E2E tests**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test -p sans e2e_file`
Expected: Both new tests pass

- [ ] **Step 5: Run full test suite**

Run: `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`
Expected: All ~249 tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/sans-driver/tests/e2e.rs tests/fixtures/file_write_read.sans tests/fixtures/file_exists_check.sans
git commit -m "feat(e2e): add file I/O end-to-end tests"
```
