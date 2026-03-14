# Plan 6b: Module/Import System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-file compilation via `import "path"` syntax with module-prefixed function calls, enabling code organization across files.

**Architecture:** The lexer gains an `Import` token. The parser parses `import "path"` declarations at the top of files into `Program.imports`. A new `imports.rs` module in the driver crate recursively resolves imports, detects cycles, and returns modules in topological order. The type checker gains a `module_exports` parameter to resolve cross-module function calls. IR lowering mangles non-main function names as `{module}__{function}`. All module IR is merged into one flat `Module` before codegen (unchanged).

**Tech Stack:** Rust, sans compiler workspace (lexer/parser/typeck/IR/codegen/driver crates), LLVM 17 via inkwell 0.8

**Spec:** `docs/superpowers/specs/2026-03-13-plan6b-module-import-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/sans-lexer/src/token.rs` | Modify | Add `Import` variant to `TokenKind` |
| `crates/sans-lexer/src/lib.rs` | Modify | Map `"import"` keyword, add test |
| `crates/sans-parser/src/ast.rs` | Modify | Add `Import` struct, add `imports` field to `Program` |
| `crates/sans-parser/src/lib.rs` | Modify | Parse `import "path"` declarations, enforce placement, add tests |
| `crates/sans-typeck/src/lib.rs` | Modify | Add `ModuleExports`/`FunctionSignature` types, update `check()` to accept module_exports, resolve cross-module calls in MethodCall/FieldAccess, add tests |
| `crates/sans-ir/src/lib.rs` | Modify | Update `lower()` to accept module name for name mangling, resolve cross-module calls to mangled names, add tests |
| `crates/sans-driver/src/imports.rs` | Create | `resolve_imports()` — recursive import resolution, cycle detection, topological sort |
| `crates/sans-driver/src/main.rs` | Modify | Update `build()` to use multi-module pipeline: resolve → parse → check → lower → merge → codegen |
| `crates/sans-driver/tests/e2e.rs` | Modify | Update `compile_and_run` for multi-file fixtures, add 4 E2E tests |
| `tests/fixtures/import_basic/` | Create | Basic import E2E fixture (main.sans + utils.sans) |
| `tests/fixtures/import_nested/` | Create | Nested path import E2E fixture (main.sans + models/user.sans) |
| `tests/fixtures/import_chain/` | Create | Transitive import E2E fixture (main.sans + a.sans + b.sans) |
| `tests/fixtures/import_struct/` | Create | Cross-module struct E2E fixture (main.sans + models.sans) |

---

## Chunk 1: Lexer + Parser + AST

### Task 1: Lexer — `import` keyword

**Files:**
- Modify: `crates/sans-lexer/src/token.rs`
- Modify: `crates/sans-lexer/src/lib.rs`

- [ ] **Step 1: Write the failing test**

In `crates/sans-lexer/src/lib.rs`, add at the end of the `mod tests` block:

```rust
#[test]
fn lex_import_keyword() {
    let tokens = lex("import").unwrap();
    assert_eq!(kinds(&tokens), vec![Import, Eof]);
}
```

Also add `Import` to the `use super::*;` line's available names (it's re-exported via `TokenKind::Import`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sans-lexer lex_import_keyword`
Expected: FAIL — `Import` variant does not exist on `TokenKind`

- [ ] **Step 3: Add `Import` token variant**

In `crates/sans-lexer/src/token.rs`, add `Import,` after `In,` in the `TokenKind` enum, with a section comment:

```rust
    Array,
    In,

    // Modules
    Import,
```

- [ ] **Step 4: Add keyword mapping**

In `crates/sans-lexer/src/lib.rs`, add `"import"` to the keyword match block, after the `"in"` arm:

```rust
                    "in" => TokenKind::In,
                    "import" => TokenKind::Import,
                    _ => TokenKind::Identifier(text.to_string()),
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sans-lexer lex_import_keyword`
Expected: PASS

- [ ] **Step 6: Run all lexer tests**

Run: `cargo test -p sans-lexer`
Expected: All 29 tests pass (28 existing + 1 new)

- [ ] **Step 7: Commit**

```bash
git add crates/sans-lexer/src/token.rs crates/sans-lexer/src/lib.rs
git commit -m "feat(lexer): add import keyword token"
```

---

### Task 2: AST — Import struct and Program.imports field

**Files:**
- Modify: `crates/sans-parser/src/ast.rs`

- [ ] **Step 1: Add the Import struct**

In `crates/sans-parser/src/ast.rs`, add the `Import` struct before the `Program` struct:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,        // e.g., "models/user"
    pub module_name: String, // last segment, e.g., "user"
    pub span: Span,
}
```

Add `use sans_lexer::token::Span;` at the top if not already present (it should already be there since `Span` is used in other AST nodes).

- [ ] **Step 2: Add `imports` field to `Program`**

Update the `Program` struct to include an `imports` field as the first field:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub traits: Vec<TraitDef>,
    pub impls: Vec<ImplBlock>,
}
```

- [ ] **Step 3: Fix all compilation errors from adding `imports` field**

The `Program` struct is constructed in `crates/sans-parser/src/lib.rs` line 95 in `parse_program()`. It currently returns:

```rust
Ok(Program { functions, structs, enums, traits, impls })
```

Update to:

```rust
Ok(Program { imports: Vec::new(), functions, structs, enums, traits, impls })
```

This is a temporary placeholder — Task 3 will add real import parsing.

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully. All downstream crates (typeck, IR, codegen, driver, tests) should still compile because the new `imports` field has a default empty vec.

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All 225 existing tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/sans-parser/src/ast.rs crates/sans-parser/src/lib.rs
git commit -m "feat(ast): add Import struct and Program.imports field"
```

---

### Task 3: Parser — Parse import declarations

**Files:**
- Modify: `crates/sans-parser/src/lib.rs`

- [ ] **Step 1: Write the failing test for single import**

In `crates/sans-parser/src/lib.rs`, add at the end of the `mod tests` block:

```rust
#[test]
fn parse_import_declaration() {
    let prog = parse("import \"utils\"\nfn main() Int { 0 }").unwrap();
    assert_eq!(prog.imports.len(), 1);
    assert_eq!(prog.imports[0].path, "utils");
    assert_eq!(prog.imports[0].module_name, "utils");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sans-parser parse_import_declaration`
Expected: FAIL — parser doesn't recognize `import` keyword at top level, returns parse error or empty imports

- [ ] **Step 3: Write the failing test for multiple imports**

```rust
#[test]
fn parse_multiple_imports() {
    let prog = parse("import \"utils\"\nimport \"models/user\"\nfn main() Int { 0 }").unwrap();
    assert_eq!(prog.imports.len(), 2);
    assert_eq!(prog.imports[0].path, "utils");
    assert_eq!(prog.imports[0].module_name, "utils");
    assert_eq!(prog.imports[1].path, "models/user");
    assert_eq!(prog.imports[1].module_name, "user");
}
```

- [ ] **Step 4: Write the failing test for import-after-declaration error**

```rust
#[test]
fn parse_import_after_function_errors() {
    let result = parse("fn foo() Int { 0 }\nimport \"utils\"\nfn main() Int { 0 }");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("imports must appear before all declarations"),
        "expected import placement error, got: {}", err.message);
}
```

- [ ] **Step 5: Implement import parsing in `parse_program()`**

In `crates/sans-parser/src/lib.rs`, update `parse_program()` to parse imports before other declarations:

```rust
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
```

- [ ] **Step 6: Implement `parse_import()` method**

Add this method to the `impl Parser` block, before `parse_program()`:

```rust
fn parse_import(&mut self) -> Result<Import, ParseError> {
    let import_tok = self.expect(&TokenKind::Import)?;
    let start = import_tok.span.start;

    // Expect a string literal for the path
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

    // Module name is the last segment of the path
    let module_name = path.rsplit('/').next().unwrap_or(&path).to_string();

    let end = path_tok.span.end;
    Ok(Import {
        path,
        module_name,
        span: start..end,
    })
}
```

Note: This uses `Import` from ast.rs — ensure `Import` is in scope. The `use ast::*;` at the top of lib.rs already covers this.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p sans-parser parse_import`
Expected: All 3 new import tests pass

- [ ] **Step 8: Run all parser tests**

Run: `cargo test -p sans-parser`
Expected: All 56 tests pass (53 existing + 3 new)

- [ ] **Step 9: Run full test suite**

Run: `cargo test`
Expected: All tests pass (existing tests unaffected since they produce programs with no imports)

- [ ] **Step 10: Commit**

```bash
git add crates/sans-parser/src/lib.rs
git commit -m "feat(parser): parse import declarations with placement validation"
```

---

## Chunk 2: Type Checker

### Task 4: Type Checker — Module exports types and cross-module resolution

**Files:**
- Modify: `crates/sans-typeck/src/lib.rs`

This is the largest task. The type checker needs:
1. New `ModuleExports` and `FunctionSignature` types
2. Updated `check()` signature to accept `module_exports`
3. Cross-module function call resolution in `check_expr` MethodCall arm
4. Error on field access on module name
5. Cross-module struct type resolution in `resolve_type`

- [ ] **Step 1: Write the failing test for cross-module function call**

In `crates/sans-typeck/src/lib.rs`, add at the end of the `mod tests` block:

```rust
#[test]
fn check_cross_module_function_call() {
    let prog = sans_parser::parse(
        "fn main() Int { utils.add(1, 2) }"
    ).expect("parse error");

    let mut module_exports = HashMap::new();
    let mut utils_fns = HashMap::new();
    utils_fns.insert("add".to_string(), FunctionSignature {
        params: vec![Type::Int, Type::Int],
        return_type: Type::Int,
    });
    module_exports.insert("utils".to_string(), ModuleExports {
        functions: utils_fns,
        structs: HashMap::new(),
        enums: HashMap::new(),
    });

    assert!(check(&prog, &module_exports).is_ok());
}
```

Note: `check()` returns `Result<ModuleExports, TypeError>` — `.is_ok()` works fine since we just need to know it didn't error.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sans-typeck check_cross_module_function_call`
Expected: FAIL — `ModuleExports`, `FunctionSignature` don't exist yet, `check()` doesn't accept `module_exports`

- [ ] **Step 3: Add `ModuleExports` and `FunctionSignature` types**

In `crates/sans-typeck/src/lib.rs`, add after the `GenericFnInfo` struct definition (around line 12):

```rust
/// Exported items from a module, available for cross-module resolution.
pub struct ModuleExports {
    pub functions: HashMap<String, FunctionSignature>,
    pub structs: HashMap<String, Vec<(String, Type)>>,
    pub enums: HashMap<String, Vec<(String, Vec<Type>)>>,
}

/// Signature of an exported function.
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}
```

- [ ] **Step 4: Update `check()` signature to accept `module_exports` and return `ModuleExports`**

Change the `check` function signature from:

```rust
pub fn check(program: &Program) -> Result<(), TypeError> {
```

to:

```rust
pub fn check(program: &Program, module_exports: &HashMap<String, ModuleExports>) -> Result<ModuleExports, TypeError> {
```

The function now returns the `ModuleExports` for the checked program. This lets the driver collect exports as a byproduct of type checking, avoiding duplicated type resolution logic.

At the end of `check()`, before the final `Ok(())`, build and return the exports:

```rust
    // Build module exports from this program's definitions
    let mut fn_exports = HashMap::new();
    for func in &program.functions {
        if func.type_params.is_empty() {
            let (param_types, ret_type) = fn_env.get(&func.name).unwrap();
            fn_exports.insert(func.name.clone(), FunctionSignature {
                params: param_types.clone(),
                return_type: ret_type.clone(),
            });
        }
    }

    Ok(ModuleExports {
        functions: fn_exports,
        structs: struct_registry,
        enums: enum_registry,
    })
```

Note: `struct_registry` and `enum_registry` are already `HashMap<String, Vec<(String, Type)>>` and `HashMap<String, Vec<(String, Vec<Type>)>>` respectively — they match the `ModuleExports` field types exactly.

- [ ] **Step 5: Thread `module_exports` through all internal functions**

Update the signature of every function that is called from `check()` to accept `module_exports`:

Add `module_exports: &HashMap<String, ModuleExports>,` as the last parameter to:
- `check_stmts()`
- `check_stmt()`
- `check_expr()`
- `resolve_type()`

Update all call sites of these functions throughout the file to pass `module_exports` (or `&module_exports`). There are approximately 70+ call sites to update:

- ~40 calls to `check_expr` (in check_stmt, check_expr recursive, and check function bodies)
- ~10 calls to `check_stmts`/`check_stmt`
- ~22 calls to `resolve_type` — **including the calls inside `check()` itself** (passes 0a through 0d at lines 202, 215, 229, 231, 253, 261, 273, 275, 291, 293, 310, 312, 334, 376, 380) — these are easy to miss since they're in the outer function, not the sub-functions

For `resolve_type`, update the signature:
```rust
fn resolve_type(
    name: &str,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<Type, TypeError> {
```

The body of `resolve_type` stays the same for now — module-qualified type resolution is handled in step 8.

- [ ] **Step 6: Fix all existing callers of `check()`**

The `check()` function is called from:
1. `crates/sans-driver/src/main.rs` line 42: `sans_typeck::check(&program)`
2. `crates/sans-driver/tests/e2e.rs` line 15: `sans_typeck::check(&program)`
3. All `do_check()` test helpers in `crates/sans-typeck/src/lib.rs`

Update all callers to pass an empty `HashMap`:

In `crates/sans-driver/src/main.rs`:
```rust
sans_typeck::check(&program, &std::collections::HashMap::new()).map_err(|e| format!("type error: {}", e.message))?;
```

In `crates/sans-driver/tests/e2e.rs`:
```rust
sans_typeck::check(&program, &std::collections::HashMap::new())
    .unwrap_or_else(|e| panic!("type error: {}", e));
```

Note: Both callers discard the `ModuleExports` return value for now — they'll use it in Task 6/7.

In `crates/sans-typeck/src/lib.rs`, update the `do_check` test helper to discard the return value:
```rust
fn do_check(src: &str) -> Result<(), TypeError> {
    let prog = sans_parser::parse(src)
        .expect("parse error in test input");
    check(&prog, &HashMap::new())?;
    Ok(())
}
```

- [ ] **Step 7: Verify compilation and all existing tests pass**

Run: `cargo test`
Expected: All 225 existing tests pass. The new test still fails because cross-module MethodCall resolution isn't implemented yet.

- [ ] **Step 8: Implement cross-module function call resolution in MethodCall**

In `check_expr()`, find the `Expr::MethodCall` arm. At the **very beginning** of this arm, before checking the object type, add module resolution:

```rust
Expr::MethodCall { object, method, args, .. } => {
    // Check if this is a cross-module function call: mod.func(args)
    if let Expr::Identifier { name, .. } = object.as_ref() {
        if let Some(mod_exports) = module_exports.get(name) {
            // This is a module-prefixed call
            let sig = mod_exports.functions.get(method)
                .ok_or_else(|| TypeError::new(format!(
                    "function '{}' not found in module '{}'", method, name
                )))?;
            // Check argument count
            if args.len() != sig.params.len() {
                return Err(TypeError::new(format!(
                    "function '{}' in module '{}' expects {} arguments, got {}",
                    method, name, sig.params.len(), args.len()
                )));
            }
            // Check argument types
            for (i, (arg, expected)) in args.iter().zip(sig.params.iter()).enumerate() {
                let actual = check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
                if actual != *expected {
                    return Err(TypeError::new(format!(
                        "argument {} to '{}.{}': expected {} but got {}",
                        i + 1, name, method, expected, actual
                    )));
                }
            }
            return Ok(sig.return_type.clone());
        }
    }

    // ... existing MethodCall type checking continues below ...
```

- [ ] **Step 9: Implement field access error on module name**

In `check_expr()`, find the `Expr::FieldAccess` arm. At the beginning, add a module check:

```rust
Expr::FieldAccess { object, field, .. } => {
    // Error if accessing a field on a module name
    if let Expr::Identifier { name, .. } = object.as_ref() {
        if module_exports.contains_key(name) {
            return Err(TypeError::new(format!(
                "cannot access field on module '{}' — did you mean to call a function?", name
            )));
        }
    }

    // ... existing FieldAccess type checking continues below ...
```

- [ ] **Step 10: Update `resolve_type` for cross-module struct/enum types**

In `resolve_type()`, after the existing `enums.get(other)` check, add a fallback that searches module exports:

```rust
fn resolve_type(
    name: &str,
    structs: &HashMap<String, Vec<(String, Type)>>,
    enums: &HashMap<String, Vec<(String, Vec<Type>)>>,
    module_exports: &HashMap<String, ModuleExports>,
) -> Result<Type, TypeError> {
    match name {
        "Int" => Ok(Type::Int),
        "Bool" => Ok(Type::Bool),
        "String" => Ok(Type::String),
        other => {
            if let Some(fields) = structs.get(other) {
                Ok(Type::Struct { name: other.to_string(), fields: fields.clone() })
            } else if let Some(variants) = enums.get(other) {
                Ok(Type::Enum { name: other.to_string(), variants: variants.clone() })
            } else {
                // Search module exports for the type
                for (_mod_name, exports) in module_exports {
                    if let Some(fields) = exports.structs.get(other) {
                        return Ok(Type::Struct { name: other.to_string(), fields: fields.clone() });
                    }
                    if let Some(variants) = exports.enums.get(other) {
                        return Ok(Type::Enum { name: other.to_string(), variants: variants.clone() });
                    }
                }
                Err(TypeError::new(format!("unknown type '{}'", other)))
            }
        }
    }
}
```

- [ ] **Step 11: Run the cross-module function call test**

Run: `cargo test -p sans-typeck check_cross_module_function_call`
Expected: PASS

- [ ] **Step 12: Write and run remaining type checker tests**

Add these tests to the `mod tests` block:

```rust
#[test]
fn check_cross_module_function_with_struct_return() {
    // Module "models" exports a struct User and a function create() -> User
    let prog = sans_parser::parse(
        "fn main() Int { let u = models.create() u.age }"
    ).expect("parse error");

    let mut module_exports = HashMap::new();
    let user_fields = vec![
        ("name".to_string(), Type::String),
        ("age".to_string(), Type::Int),
    ];
    let mut models_fns = HashMap::new();
    models_fns.insert("create".to_string(), FunctionSignature {
        params: vec![],
        return_type: Type::Struct {
            name: "User".to_string(),
            fields: user_fields.clone(),
        },
    });
    let mut models_structs = HashMap::new();
    models_structs.insert("User".to_string(), user_fields);
    module_exports.insert("models".to_string(), ModuleExports {
        functions: models_fns,
        structs: models_structs,
        enums: HashMap::new(),
    });

    assert!(check(&prog, &module_exports).is_ok());
}

#[test]
fn check_unknown_function_in_module() {
    let prog = sans_parser::parse(
        "fn main() Int { utils.nonexistent() }"
    ).expect("parse error");

    let mut module_exports = HashMap::new();
    module_exports.insert("utils".to_string(), ModuleExports {
        functions: HashMap::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
    });

    let err = check(&prog, &module_exports).unwrap_err();
    assert!(err.message.contains("not found in module"),
        "expected module function error, got: {}", err.message);
}

#[test]
fn check_unknown_module_prefix() {
    // "nomod" is not in module_exports, so it's treated as a variable.
    // Since "nomod" is not defined as a variable, this should produce
    // an "undefined variable" error.
    let prog = sans_parser::parse(
        "fn main() Int { nomod.func() }"
    ).expect("parse error");

    let err = check(&prog, &HashMap::new()).unwrap_err();
    assert!(err.message.contains("undefined") || err.message.contains("no method"),
        "expected undefined/no method error, got: {}", err.message);
}

#[test]
fn check_field_access_on_module_errors() {
    let prog = sans_parser::parse(
        "fn main() Int { utils.x }"
    ).expect("parse error");

    let mut module_exports = HashMap::new();
    module_exports.insert("utils".to_string(), ModuleExports {
        functions: HashMap::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
    });

    let err = check(&prog, &module_exports).unwrap_err();
    assert!(err.message.contains("cannot access field on module"),
        "expected module field access error, got: {}", err.message);
}

#[test]
fn check_duplicate_import_is_ok() {
    // Duplicate module entries in module_exports — this tests that the
    // type checker doesn't complain. (Deduplication is the driver's job.)
    let prog = sans_parser::parse(
        "fn main() Int { utils.add(1, 2) }"
    ).expect("parse error");

    let mut module_exports = HashMap::new();
    let mut utils_fns = HashMap::new();
    utils_fns.insert("add".to_string(), FunctionSignature {
        params: vec![Type::Int, Type::Int],
        return_type: Type::Int,
    });
    module_exports.insert("utils".to_string(), ModuleExports {
        functions: utils_fns,
        structs: HashMap::new(),
        enums: HashMap::new(),
    });

    assert!(check(&prog, &module_exports).is_ok());
}
```

Run: `cargo test -p sans-typeck`
Expected: All 85 tests pass (79 existing + 6 new)

- [ ] **Step 13: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 14: Commit**

```bash
git add crates/sans-typeck/src/lib.rs crates/sans-driver/src/main.rs crates/sans-driver/tests/e2e.rs
git commit -m "feat(typeck): add cross-module function call resolution and module exports"
```

---

## Chunk 3: IR + Driver + E2E

### Task 5: IR — Name mangling and cross-module call lowering

**Files:**
- Modify: `crates/sans-ir/src/lib.rs`

- [ ] **Step 1: Write the failing test for name mangling**

In `crates/sans-ir/src/lib.rs`, add at the end of the `mod tests` block:

```rust
#[test]
fn lower_with_module_name_mangles_functions() {
    let program = parse("fn add(a Int, b Int) Int { a + b }");
    let module = lower(&program, Some("utils"));
    let func = module.functions.iter().find(|f| f.name == "utils__add");
    assert!(func.is_some(), "expected mangled function name 'utils__add'");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sans-ir lower_with_module_name_mangles_functions`
Expected: FAIL — `lower()` doesn't accept a module name parameter

- [ ] **Step 3: Write the failing test for cross-module call lowering**

```rust
#[test]
fn lower_cross_module_call_uses_mangled_name() {
    let program = parse("import \"utils\"\nfn main() Int { utils.add(1, 2) }");
    let module = lower(&program, None);
    let func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_mangled_call = func.body.iter().any(|i| {
        matches!(i, Instruction::Call { function, .. } if function == "utils__add")
    });
    assert!(has_mangled_call, "expected call to 'utils__add', got: {:?}",
        func.body.iter().filter(|i| matches!(i, Instruction::Call { .. })).collect::<Vec<_>>());
}
```

Note: The source includes `import "utils"` so that `program.imports` is populated and `IrBuilder.module_names` contains `"utils"`.

- [ ] **Step 4: Update `lower()` signature to accept module name**

Change the `lower` function signature from:

```rust
pub fn lower(program: &Program) -> Module {
```

to:

```rust
pub fn lower(program: &Program, module_name: Option<&str>) -> Module {
```

When `module_name` is `Some("utils")`, all function names are mangled as `utils__funcname`. When `None` (main module), no mangling.

- [ ] **Step 5: Implement name mangling in `lower()`**

In the function body of `lower()`, where `IrFunction` is created from each AST function, apply mangling:

Find the line that creates the function name (it assigns `func.name` to `IrFunction.name`). Change it to:

```rust
let func_name = if let Some(mod_name) = module_name {
    format!("{}__{}", mod_name, func.name)
} else {
    func.name.clone()
};
```

Use `func_name` as the `IrFunction.name`.

Similarly, for impl method mangling (where function name is `format!("{}_{}", imp.target_type, method.name)`), prepend the module name if present:

```rust
let mangled = if let Some(mod_name) = module_name {
    format!("{}__{}__{}", mod_name, imp.target_type, method.name)
} else {
    format!("{}_{}", imp.target_type, method.name)
};
```

- [ ] **Step 6: Implement cross-module call resolution in MethodCall lowering**

In the `lower_expr` method (or wherever `Expr::MethodCall` is lowered), add a check at the top of the MethodCall handling:

When the object is `Expr::Identifier { name, .. }` and the name matches a known module (we need to know which identifiers are module names), resolve the call to `{name}__{method}`.

The simplest approach: add two fields to `IrBuilder`:
1. `module_names: Vec<String>` — to identify module-prefixed calls
2. `module_fn_ret_types: HashMap<(String, String), IrType>` — maps `(module_name, function_name)` to the return type's `IrType`, so that cross-module calls get the correct result register type (critical for struct-returning functions like `models.new_point()` where the result needs `IrType::Struct("Point")` for field access to work)

Add both fields to the `IrBuilder` struct:

```rust
struct IrBuilder {
    counter: usize,
    label_counter: usize,
    locals: HashMap<String, LocalVar>,
    instructions: Vec<Instruction>,
    reg_types: HashMap<Reg, IrType>,
    struct_defs: HashMap<String, Vec<String>>,
    enum_defs: HashMap<String, Vec<(String, usize, usize)>>,
    module_names: Vec<String>,
    module_fn_ret_types: HashMap<(String, String), IrType>,
}
```

Initialize `module_names` from `program.imports` in `lower()`:

```rust
let module_names: Vec<String> = program.imports.iter()
    .map(|imp| imp.module_name.clone())
    .collect();
```

Initialize `module_fn_ret_types` as empty — it will be populated by the driver before lowering (see Task 6). Update `lower()` to accept it:

```rust
pub fn lower(program: &Program, module_name: Option<&str>, module_fn_ret_types: HashMap<(String, String), IrType>) -> Module {
```

Wait — `IrType` is `pub(crate)`. To pass it from the driver, we need to either make it `pub` or add a helper. The simplest approach: make `IrType` `pub` and add a constructor helper.

Make `IrType` public by changing `enum IrType` to `pub enum IrType` in `lib.rs`.

Add a `pub fn ir_type_for_return` helper that converts from the type checker's `Type` to `IrType`. This is used by the driver to build `module_fn_ret_types`:

```rust
pub fn ir_type_for_return(ty: &sans_typeck::types::Type) -> IrType {
    use sans_typeck::types::Type;
    match ty {
        Type::Int => IrType::Int,
        Type::Bool => IrType::Bool,
        Type::String => IrType::Str,
        Type::Struct { name, .. } => IrType::Struct(name.clone()),
        Type::Enum { name, .. } => IrType::Enum(name.clone()),
        Type::Array { inner } => IrType::Array(Box::new(ir_type_for_return(inner))),
        _ => IrType::Int, // Fallback for types not expected as cross-module returns
    }
}
```

Note: This requires adding `sans-typeck` as a dependency of `sans-ir` in `Cargo.toml`. Add to `crates/sans-ir/Cargo.toml`:

```toml
[dependencies]
sans-parser = { path = "../sans-parser" }
sans-typeck = { path = "../sans-typeck" }
```

Update `lower()` signature:

```rust
pub fn lower(
    program: &Program,
    module_name: Option<&str>,
    module_fn_ret_types: &HashMap<(String, String), IrType>,
) -> Module {
```

Pass `module_fn_ret_types` when creating each `IrBuilder`.

In the MethodCall lowering, at the very beginning, add:

```rust
Expr::MethodCall { object, method, args, .. } => {
    // Check for cross-module function call
    if let Expr::Identifier { name, .. } = object.as_ref() {
        if self.module_names.contains(name) {
            // Cross-module call: mod.func(args) → Call("{mod}__{func}", args)
            let mangled_name = format!("{}__{}", name, method);
            let arg_regs: Vec<Reg> = args.iter()
                .map(|a| self.lower_expr(a))
                .collect();
            let dest = self.fresh_reg();
            // Look up the return type for this cross-module function
            let ret_type = self.module_fn_ret_types
                .get(&(name.clone(), method.clone()))
                .cloned()
                .unwrap_or(IrType::Int);
            self.reg_types.insert(dest.clone(), ret_type);
            self.instructions.push(Instruction::Call {
                dest: dest.clone(),
                function: mangled_name,
                args: arg_regs,
            });
            return dest;
        }
    }
    // ... existing MethodCall lowering continues ...
```

- [ ] **Step 7: Fix all existing callers of `lower()`**

The `lower()` function is called from:
1. `crates/sans-driver/src/main.rs` line 45: `sans_ir::lower(&program)`
2. `crates/sans-driver/tests/e2e.rs` line 19: `sans_ir::lower(&program)`
3. All `parse` + `lower` test patterns in `crates/sans-ir/src/lib.rs`

Update all callers to pass `None` and empty `HashMap`:

In `crates/sans-driver/src/main.rs`:
```rust
let ir_module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
```

In `crates/sans-driver/tests/e2e.rs`:
```rust
let ir_module = sans_ir::lower(&program, None, &std::collections::HashMap::new());
```

In `crates/sans-ir/src/lib.rs` tests, add a `lower_main` test helper:

```rust
fn lower_main(src: &str) -> Module {
    let program = parse(src);
    lower(&program, None, &HashMap::new())
}
```

Then replace all `lower(&program)` calls in existing tests with `lower(&program, None, &HashMap::new())` or use the helper.

- [ ] **Step 8: Run tests**

Run: `cargo test -p sans-ir`
Expected: All 32 tests pass (30 existing + 2 new)

- [ ] **Step 9: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 10: Commit**

```bash
git add crates/sans-ir/src/lib.rs crates/sans-driver/src/main.rs crates/sans-driver/tests/e2e.rs
git commit -m "feat(ir): add module name mangling and cross-module call lowering"
```

---

### Task 6: Driver — Import resolution module

**Files:**
- Create: `crates/sans-driver/src/imports.rs`
- Modify: `crates/sans-driver/src/main.rs`

- [ ] **Step 1: Create `imports.rs` with the `resolve_imports` function**

Create `crates/sans-driver/src/imports.rs`:

```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sans_parser::ast::Program;

/// A parsed module with its metadata.
pub struct ResolvedModule {
    pub name: String,      // module prefix, e.g., "user"
    pub path: PathBuf,     // absolute path to .sans file
    pub program: Program,  // parsed AST
}

/// Recursively resolves all imports starting from the entry point.
/// Returns modules in topological order (dependencies first, entry point last).
/// The entry point itself is NOT included in the returned vec — only imported modules.
pub fn resolve_imports(entry_point: &Path) -> Result<Vec<ResolvedModule>, String> {
    let base_dir = entry_point.parent()
        .ok_or_else(|| format!("cannot determine directory of '{}'", entry_point.display()))?;

    let mut resolved: Vec<ResolvedModule> = Vec::new();
    let mut visited: HashMap<PathBuf, String> = HashMap::new(); // path -> module_name
    let mut in_progress: HashSet<PathBuf> = HashSet::new(); // for cycle detection

    // Parse the entry point to get its imports
    let entry_source = std::fs::read_to_string(entry_point)
        .map_err(|e| format!("could not read '{}': {}", entry_point.display(), e))?;
    let entry_program = sans_parser::parse(&entry_source)
        .map_err(|e| format!("parse error in '{}' at {}..{}: {}", entry_point.display(), e.span.start, e.span.end, e.message))?;

    let entry_canonical = entry_point.canonicalize()
        .map_err(|e| format!("could not canonicalize '{}': {}", entry_point.display(), e))?;
    in_progress.insert(entry_canonical.clone());

    // Recursively resolve each import
    for import in &entry_program.imports {
        resolve_import(
            &import.path,
            &import.module_name,
            base_dir,
            &mut resolved,
            &mut visited,
            &mut in_progress,
            &entry_canonical,
        )?;
    }

    Ok(resolved)
}

fn resolve_import(
    import_path: &str,
    module_name: &str,
    base_dir: &Path,
    resolved: &mut Vec<ResolvedModule>,
    visited: &mut HashMap<PathBuf, String>,
    in_progress: &mut HashSet<PathBuf>,
    _from_file: &Path,
) -> Result<(), String> {
    // Resolve the file path
    let file_path = base_dir.join(format!("{}.sans", import_path));
    let canonical = file_path.canonicalize()
        .map_err(|_| format!("module not found: {}", import_path))?;

    // Check for duplicate (already resolved) — idempotent
    if visited.contains_key(&canonical) {
        return Ok(());
    }

    // Check for circular import
    if in_progress.contains(&canonical) {
        return Err(format!("circular import detected: {}", import_path));
    }

    // Mark as in-progress
    in_progress.insert(canonical.clone());

    // Read and parse the module
    let source = std::fs::read_to_string(&file_path)
        .map_err(|_| format!("module not found: {}", import_path))?;
    let program = sans_parser::parse(&source)
        .map_err(|e| format!(
            "parse error in '{}' at {}..{}: {}",
            file_path.display(), e.span.start, e.span.end, e.message
        ))?;

    // Recursively resolve this module's imports (transitive)
    // Per spec: ALL imports resolve relative to entry point's directory (base_dir)
    for sub_import in &program.imports {
        resolve_import(
            &sub_import.path,
            &sub_import.module_name,
            base_dir,
            resolved,
            visited,
            in_progress,
            &canonical,
        )?;
    }

    // Mark as visited and add to resolved list (dependencies first)
    in_progress.remove(&canonical);
    visited.insert(canonical.clone(), module_name.to_string());
    resolved.push(ResolvedModule {
        name: module_name.to_string(),
        path: canonical,
        program,
    });

    Ok(())
}
```

- [ ] **Step 2: Register the module in `main.rs`**

In `crates/sans-driver/src/main.rs`, add at the top:

```rust
mod imports;
```

- [ ] **Step 3: Update the `build()` function for multi-module compilation**

Replace the entire `build()` function body in `crates/sans-driver/src/main.rs`:

```rust
fn build(source_path: &PathBuf) -> Result<(), String> {
    // Validate extension
    if source_path.extension().and_then(|e| e.to_str()) != Some("sans") {
        return Err(format!(
            "expected a .sans source file, got: {}",
            source_path.display()
        ));
    }

    // Step 1: Resolve imports (recursive, topological order)
    let resolved_modules = imports::resolve_imports(source_path)?;

    // Step 2: Read and parse the entry point
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("could not read '{}': {}", source_path.display(), e))?;
    let main_program = sans_parser::parse(&source).map_err(|e| {
        format!(
            "parse error at {}..{}: {}",
            e.span.start, e.span.end, e.message
        )
    })?;

    // Step 3: Type-check in dependency order, collecting module exports
    // check() returns ModuleExports as a byproduct — no duplicated type resolution
    let mut module_exports: std::collections::HashMap<String, sans_typeck::ModuleExports> =
        std::collections::HashMap::new();

    for module in &resolved_modules {
        let exports = sans_typeck::check(&module.program, &module_exports)
            .map_err(|e| format!("type error in module '{}': {}", module.name, e.message))?;
        module_exports.insert(module.name.clone(), exports);
    }

    // Type-check main module
    sans_typeck::check(&main_program, &module_exports)
        .map_err(|e| format!("type error: {}", e.message))?;

    // Step 4: Build module_fn_ret_types for IR lowering (maps (module, func) → IrType)
    let mut module_fn_ret_types: std::collections::HashMap<(String, String), sans_ir::IrType> =
        std::collections::HashMap::new();
    for (mod_name, exports) in &module_exports {
        for (func_name, sig) in &exports.functions {
            let ir_type = sans_ir::ir_type_for_return(&sig.return_type);
            module_fn_ret_types.insert((mod_name.clone(), func_name.clone()), ir_type);
        }
    }

    // Step 5: Lower to IR with name mangling, then merge
    // Each module gets the accumulated module_fn_ret_types from its dependencies
    let mut all_ir_functions = Vec::new();

    for module in &resolved_modules {
        let ir = sans_ir::lower(&module.program, Some(&module.name), &module_fn_ret_types);
        all_ir_functions.extend(ir.functions);
    }

    let main_ir = sans_ir::lower(&main_program, None, &module_fn_ret_types);
    all_ir_functions.extend(main_ir.functions);

    let merged_module = sans_ir::ir::Module {
        functions: all_ir_functions,
    };

    // Step 6: Codegen to object file
    let obj_path = source_path.with_extension("o");
    let obj_path_str = obj_path
        .to_str()
        .ok_or_else(|| "object path contains invalid UTF-8".to_string())?;

    sans_codegen::compile_to_object(&merged_module, obj_path_str)
        .map_err(|e| format!("codegen error: {}", e))?;

    // Step 7: Link
    let output_path = source_path.with_extension("");
    let output_path_str = output_path
        .to_str()
        .ok_or_else(|| "output path contains invalid UTF-8".to_string())?;

    let link_status = process::Command::new("cc")
        .args([obj_path_str, "-o", output_path_str])
        .status()
        .map_err(|e| format!("failed to invoke linker: {}", e))?;

    if !link_status.success() {
        return Err(format!(
            "linker exited with status {}",
            link_status.code().unwrap_or(-1)
        ));
    }

    // Step 8: Clean up .o file
    std::fs::remove_file(&obj_path)
        .map_err(|e| format!("could not remove object file '{}': {}", obj_path.display(), e))?;

    // Step 9: Report success
    println!("Built: {}", output_path.display());

    Ok(())
}
```

Note: No `resolve_driver_type` helper needed — `check()` returns `ModuleExports` directly, eliminating duplicated type resolution logic. The `ir_type_for_return` helper (added in Task 5) converts `Type` to `IrType` for the IR lowering.

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Run existing tests**

Run: `cargo test`
Expected: All existing tests pass (single-file programs still work with empty imports)

- [ ] **Step 6: Commit**

```bash
git add crates/sans-driver/src/imports.rs crates/sans-driver/src/main.rs
git commit -m "feat(driver): add import resolution and multi-module compilation pipeline"
```

---

### Task 7: E2E Tests — Multi-file compilation

**Files:**
- Modify: `crates/sans-driver/tests/e2e.rs`
- Create: `tests/fixtures/import_basic/main.sans`
- Create: `tests/fixtures/import_basic/utils.sans`
- Create: `tests/fixtures/import_nested/main.sans`
- Create: `tests/fixtures/import_nested/models/user.sans`
- Create: `tests/fixtures/import_chain/main.sans`
- Create: `tests/fixtures/import_chain/a.sans`
- Create: `tests/fixtures/import_chain/b.sans`
- Create: `tests/fixtures/import_struct/main.sans`
- Create: `tests/fixtures/import_struct/models.sans`

- [ ] **Step 1: Create `crates/sans-driver/src/lib.rs` and update `main.rs`**

The driver is a binary crate. To make `imports` accessible from E2E tests, create a library companion:

Create `crates/sans-driver/src/lib.rs`:

```rust
pub mod imports;
```

In `crates/sans-driver/src/main.rs`, replace `mod imports;` (added in Task 6) with:

```rust
use sans::imports;
```

This lets `e2e.rs` tests use `sans::imports::resolve_imports`.

- [ ] **Step 2: Add `compile_and_run_dir` helper for multi-file fixtures**

The current `compile_and_run(fixture: &str)` handles single files. Add a new helper that replicates the multi-module pipeline:

```rust
/// Helper: compile a multi-file fixture directory and run main.sans, returning the exit code.
fn compile_and_run_dir(fixture_dir: &str) -> i32 {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir_path = format!("{}/../../tests/fixtures/{}", manifest_dir, fixture_dir);
    let main_path = std::path::PathBuf::from(format!("{}/main.sans", dir_path));

    // Resolve imports
    let resolved_modules = sans::imports::resolve_imports(&main_path)
        .unwrap_or_else(|e| panic!("import resolution error: {}", e));

    // Parse main
    let main_source = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|e| panic!("could not read main.sans: {}", e));
    let main_program = sans_parser::parse(&main_source)
        .unwrap_or_else(|e| panic!("parse error: {:?}", e));

    // Type-check in dependency order, collecting exports from check() return value
    let mut module_exports = std::collections::HashMap::new();
    for module in &resolved_modules {
        let exports = sans_typeck::check(&module.program, &module_exports)
            .unwrap_or_else(|e| panic!("type error in module '{}': {}", module.name, e));
        module_exports.insert(module.name.clone(), exports);
    }

    sans_typeck::check(&main_program, &module_exports)
        .unwrap_or_else(|e| panic!("type error: {}", e));

    // Build module_fn_ret_types for IR lowering
    let mut module_fn_ret_types: std::collections::HashMap<(String, String), sans_ir::IrType> =
        std::collections::HashMap::new();
    for (mod_name, exports) in &module_exports {
        for (func_name, sig) in &exports.functions {
            let ir_type = sans_ir::ir_type_for_return(&sig.return_type);
            module_fn_ret_types.insert((mod_name.clone(), func_name.clone()), ir_type);
        }
    }

    // Lower to IR with mangling, merge
    // All modules get the full module_fn_ret_types so transitive calls resolve correctly
    let mut all_ir_functions = Vec::new();
    for module in &resolved_modules {
        let ir = sans_ir::lower(&module.program, Some(&module.name), &module_fn_ret_types);
        all_ir_functions.extend(ir.functions);
    }
    let main_ir = sans_ir::lower(&main_program, None, &module_fn_ret_types);
    all_ir_functions.extend(main_ir.functions);

    let merged = sans_ir::ir::Module { functions: all_ir_functions };

    // Codegen, link, run
    let tmp_dir = std::env::temp_dir();
    let obj_path = tmp_dir.join(format!("{}.o", fixture_dir));
    let bin_path = tmp_dir.join(fixture_dir);

    sans_codegen::compile_to_object(&merged, obj_path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("codegen error: {}", e));

    let link_status = Command::new("cc")
        .args([obj_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
        .status()
        .expect("failed to invoke linker");
    assert!(link_status.success(), "linker failed");

    let run_status = Command::new(bin_path.to_str().unwrap())
        .status()
        .expect("failed to run compiled binary");

    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&bin_path);

    run_status.code().unwrap_or(-1)
}
```

No `simple_resolve_type` helper needed — `check()` returns `ModuleExports` directly.

```rust
use sans::imports;
```

Then in `e2e.rs`, the test can use `sans::imports::resolve_imports`.

- [ ] **Step 3: Create E2E fixture: `import_basic`**

Create directory `tests/fixtures/import_basic/`.

Create `tests/fixtures/import_basic/utils.sans`:
```
fn add(a Int, b Int) Int {
    a + b
}
```

Create `tests/fixtures/import_basic/main.sans`:
```
import "utils"

fn main() Int {
    utils.add(3, 4)
}
```

Expected exit code: **7**

- [ ] **Step 4: Create E2E fixture: `import_nested`**

Create directories `tests/fixtures/import_nested/` and `tests/fixtures/import_nested/models/`.

Create `tests/fixtures/import_nested/models/user.sans`:
```
struct User {
    name String,
    age Int,
}

fn create_age(age Int) Int {
    age + 5
}
```

Create `tests/fixtures/import_nested/main.sans`:
```
import "models/user"

fn main() Int {
    user.create_age(10)
}
```

Expected exit code: **15**

- [ ] **Step 5: Create E2E fixture: `import_chain`**

Create directory `tests/fixtures/import_chain/`.

Create `tests/fixtures/import_chain/b.sans`:
```
fn base_value() Int {
    10
}
```

Create `tests/fixtures/import_chain/a.sans`:
```
import "b"

fn compute() Int {
    b.base_value() + 3
}
```

Create `tests/fixtures/import_chain/main.sans`:
```
import "a"

fn main() Int {
    a.compute()
}
```

Expected exit code: **13**

- [ ] **Step 6: Create E2E fixture: `import_struct`**

Create directory `tests/fixtures/import_struct/`.

Create `tests/fixtures/import_struct/models.sans`:
```
struct Point {
    x Int,
    y Int,
}

fn new_point(x Int, y Int) Point {
    Point { x: x, y: y }
}
```

Create `tests/fixtures/import_struct/main.sans`:
```
import "models"

fn main() Int {
    let p = models.new_point(8, 14)
    p.x + p.y
}
```

Expected exit code: **22**

- [ ] **Step 7: Add E2E tests**

Add to `crates/sans-driver/tests/e2e.rs`:

```rust
#[test]
fn e2e_import_basic() {
    assert_eq!(compile_and_run_dir("import_basic"), 7);
}

#[test]
fn e2e_import_nested() {
    assert_eq!(compile_and_run_dir("import_nested"), 15);
}

#[test]
fn e2e_import_chain() {
    assert_eq!(compile_and_run_dir("import_chain"), 13);
}

#[test]
fn e2e_import_struct() {
    assert_eq!(compile_and_run_dir("import_struct"), 22);
}
```

- [ ] **Step 8: Run all E2E tests**

Run: `cargo test -p sans-driver`
Expected: All 22 tests pass (18 existing + 4 new)

- [ ] **Step 9: Run full test suite**

Run: `cargo test`
Expected: All ~243 tests pass

- [ ] **Step 10: Commit**

```bash
git add crates/sans-driver/src/lib.rs crates/sans-driver/src/imports.rs crates/sans-driver/src/main.rs crates/sans-driver/tests/e2e.rs tests/fixtures/import_basic/ tests/fixtures/import_nested/ tests/fixtures/import_chain/ tests/fixtures/import_struct/
git commit -m "feat(driver): multi-module compilation pipeline with E2E tests"
```
