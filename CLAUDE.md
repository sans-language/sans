# Sans Compiler - Project Rules

## Build
- `LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build`
- `cargo test` to run all tests

## Architecture
- 6 Rust crates under `crates/`: sans-lexer, sans-parser, sans-typeck, sans-ir, sans-codegen, sans-driver
- 8 C runtime files under `runtime/`: json.c, http.c, log.c, result.c, string_ext.c, array_ext.c, functional.c, server.c
- Tests: unit tests in each crate, E2E tests in `crates/sans-driver/tests/e2e.rs`
- Test fixtures in `tests/fixtures/` (.sans files and directories for multi-module)

## Adding a New Built-in Function
Pipeline: typeck (type check) -> IR (instruction + lowering) -> codegen (LLVM compilation) -> driver (link)
1. Add type checking in `crates/sans-typeck/src/lib.rs` (in the `Expr::Call` match)
2. Add IR instruction in `crates/sans-ir/src/ir.rs`
3. Add IR lowering in `crates/sans-ir/src/lib.rs`
4. Add codegen in `crates/sans-codegen/src/lib.rs` (declare external fn + compile instruction)
5. If backed by C: add function to appropriate `runtime/*.c` file
6. Add tests in each crate

## Adding a New Method on a Type
Same pipeline but use `Expr::MethodCall` match in typeck and method dispatch in IR lowering.

## Adding a New Type
1. Add variant to `Type` enum in `crates/sans-typeck/src/types.rs`
2. Add `Display` impl
3. Add to `resolve_type()` in `typeck/lib.rs` if it has a name (like "Float")
4. Add `IrType` variant in `crates/sans-ir/src/lib.rs`
5. Add `ir_type_for_return` mapping
6. Add print guard in IR lowering
7. If opaque: add C runtime backing

## AI-Optimized Syntax (MANDATORY)
Sans is designed for AI generation, not human readability. All new features, syntax additions, and examples MUST use the fewest tokens possible.

**Preferred syntax (use these, not the verbose alternatives):**
- `x = 42` not `let x = 42` (bare assignment)
- `x := 0` not `let mut x = 0` (mutable)
- `f(x:I) = x*2` not `fn f(x Int) Int { x * 2 }` (short types, expression body, no fn)
- `a[0]` not `a.get(0)` (index syntax)
- `x += 1` not `x = x + 1` (compound assignment)
- `r!` not `r.unwrap()` (unwrap shorthand)
- `cond ? a : b` not `if cond { a } else { b }` (ternary)
- `[1 2 3]` not `[1, 2, 3]` (no commas)
- `I/S/B/F` not `Int/String/Bool/Float` (short type names)
- `R<I>` not `Result<Int>` (short Result)

**File extension:** `.sans`

**Rule:** When adding any new feature or syntax, always ask: "Can this be expressed in fewer tokens?" If yes, implement the shorter form.

## Rules
- **NEVER commit compiled binaries** (.o files, executables, Mach-O binaries). Use .gitignore to prevent this.
- **All new syntax/features must be AI-optimized** — use the fewest tokens possible.

## Conventions
- All values are stored as i64 in the IR/codegen register map
- Pointers (strings, opaque types) stored in both regs (as i64 via ptr_to_int) and ptrs (as PointerValue)
- Opaque types (JsonValue, HttpResponse, Result, etc.) backed by C runtime with `cy_` prefix
- Type checker uses `types_compatible()` for return type checks (allows ResultErr to match Result<T>)
- E2E test helpers use unique temp filenames per fixture to prevent parallel test races

## Known Limitations
- IR type tracking is per-function: opaque types lose type info when passed cross-function as i64
- Multiple opaque method calls in complex expressions can crash codegen
- No GC - all heap allocations leaked
- Float stored as i64 via bitcast in register map
