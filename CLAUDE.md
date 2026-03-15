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
7. **Update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist))

## Adding a New Method on a Type
Same pipeline but use `Expr::MethodCall` match in typeck and method dispatch in IR lowering.
After implementation + tests, **update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist)).

## Adding a New Type
1. Add variant to `Type` enum in `crates/sans-typeck/src/types.rs`
2. Add `Display` impl
3. Add to `resolve_type()` in `typeck/lib.rs` if it has a name (like "Float")
4. Add `IrType` variant in `crates/sans-ir/src/lib.rs`
5. Add `ir_type_for_return` mapping
6. Add print guard in IR lowering
7. If opaque: add C runtime backing
8. **Update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist))

## AI-Optimized Syntax (MANDATORY)
Sans is designed for AI generation, not human readability. All new features, syntax additions, and examples MUST use the fewest tokens possible.

**Preferred syntax (use these, not the verbose alternatives):**
- `x = 42` not `let x = 42` (bare assignment)
- `x := 0` not `let mut x = 0` (mutable)
- `f(x:I) = x*2` not `fn f(x Int) Int { x * 2 }` (short types, expression body, no fn)
- `a[0]` not `a.get(0)` (index syntax)
- `x += 1` not `x = x + 1` (compound assignment)
- `r!` not `r.unwrap()` (unwrap shorthand)
- `r = f()?` not `r = f(); if r.is_err() { return r }; r = r!` (try operator)
- `cond ? a : b` not `if cond { a } else { b }` (ternary)
- `[1 2 3]` not `[1, 2, 3]` (no commas)
- `I/S/B/F` not `Int/String/Bool/Float` (short type names)
- `R<I>` not `Result<Int>` (short Result)

**File extension:** `.sans`

**Rule:** When adding any new feature or syntax, always ask: "Can this be expressed in fewer tokens?" If yes, implement the shorter form.

## Documentation Update Checklist
**Every new feature, built-in function, method, type, or syntax change MUST update all of the following before the work is considered complete:**

1. **`docs/reference.md`** — Add/update the human-readable reference with full explanation and examples
2. **`docs/ai-reference.md`** — Add/update the compact AI reference (short-form syntax)
3. **`website/static/docs.html`** — Add/update the website documentation to match `docs/reference.md`. Every section in reference.md MUST have a corresponding section in docs.html.
4. **`editors/vscode-sans/src/extension.ts`** — Add hover documentation entry to `HOVER_DATA` for any new keyword, function, method, or alias
5. **`editors/vscode-sans/syntaxes/sans.tmLanguage.json`** — Add syntax highlighting patterns for new keywords, builtins, or operators
6. **`tests/fixtures/`** — Add at least one E2E test fixture (`.sans` file) demonstrating the feature
7. **`examples/`** — Update existing examples or add a new one if the feature is significant enough to showcase
8. **`README.md`** — Update feature list if the feature is user-facing and notable

**If a short alias is added** (e.g., `p` for `print`), it must appear in all of: `ai-reference.md`, `reference.md`, and `HOVER_DATA`.

**Rule: A feature is not done until docs, website docs, hover docs, syntax highlighting, and examples are updated.** Do not split docs into a separate PR — ship them with the feature.

## Versioning (MANDATORY — DO NOT SKIP)
All version numbers must stay in sync and follow semver (`x.y.z`).

**CRITICAL: Every commit that adds a feature, fixes a bug, or changes behavior MUST include a version bump.** This is not optional. Increment **patch** minimum (`x.x.+1`). Include the version bump in the SAME commit as the change — not a separate commit later.

**Before every `git commit`:** check if the version needs bumping. If the commit adds/changes/fixes anything beyond docs-only changes, bump the version. When in doubt, bump.

**Files that must ALL be updated together:**
- `crates/sans-driver/Cargo.toml` (and all other `crates/*/Cargo.toml`)
- `editors/vscode-sans/package.json`
- `website/static/index.html` (footer)
- `website/static/docs.html` (footer)
- `website/static/benchmarks.html` (footer)
- `CLAUDE.md` (this file, "Current version" below)

The CLI `sans --version` reads from `Cargo.toml` automatically via `env!("CARGO_PKG_VERSION")`.

**Current version: `0.3.19`**

**Checklist before committing:**
1. Does this commit change code? → Bump version
2. Did I update ALL version files? → Check each one
3. Did I update `Current version` in this file? → Update it

## Rules
- **NEVER commit compiled binaries** (.o files, executables, Mach-O binaries). Use .gitignore to prevent this.
- **All new syntax/features must be AI-optimized** — use the fewest tokens possible.
- **All new features must include documentation updates** — see [Documentation Update Checklist](#documentation-update-checklist).
- **Version numbers must stay in sync** — see [Versioning](#versioning). Always increment patch (`x.x.+1`) minimum.

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
