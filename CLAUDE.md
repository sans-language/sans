# Sans Compiler - Project Rules

## Build
- `sans build compiler/main.sans` to build the compiler
- `bash tests/run_tests.sh` to run all E2E tests
- Requires: LLVM 17 (`brew install llvm@17` on macOS)

## Architecture
- Self-hosted compiler in `compiler/`: lexer.sans, parser.sans, typeck.sans, constants.sans, ir.sans, codegen.sans, main.sans (~11,600 LOC)
- 13 Sans runtime modules under `runtime/`: arena.sans, array_ext.sans, curl.sans, functional.sans, http.sans, json.sans, log.sans, map.sans, result.sans, server.sans, sock.sans, ssl.sans, string_ext.sans
- Tests: E2E tests via `tests/run_tests.sh`, fixtures in `tests/fixtures/`
- The compiler is fully self-hosted — it compiles itself. No Rust code.

## Adding a New Built-in Function
Pipeline: typeck (type check) -> IR (instruction + lowering) -> codegen (LLVM compilation) -> link
1. Add type checking in `compiler/typeck.sans`
2. Add IR instruction constant in `compiler/constants.sans`
3. Add IR lowering in `compiler/ir.sans`
4. Add codegen in `compiler/codegen.sans`
5. If backed by runtime: add function to appropriate `runtime/*.sans` file
6. Add test fixture in `tests/fixtures/`
7. **Update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist))

## Adding a New Method on a Type
Same pipeline but use method dispatch in typeck and IR lowering.
After implementation + tests, **update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist)).

## Adding a New Type
1. Add type resolution in `compiler/typeck.sans`
2. Add IR type handling in `compiler/ir.sans`
3. Add codegen support in `compiler/codegen.sans`
4. If opaque: add Sans runtime backing in `runtime/*.sans`
5. Add test fixture in `tests/fixtures/`
6. **Update docs & tooling** (see [Documentation Update Checklist](#documentation-update-checklist))

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
3. **`website/docs/index.html`** — Add/update the website documentation to match `docs/reference.md`
4. **`editors/vscode-sans/src/extension.ts`** — Add hover documentation entry to `HOVER_DATA` for any new keyword, function, method, or alias
5. **`editors/vscode-sans/syntaxes/sans.tmLanguage.json`** — Add syntax highlighting patterns for new keywords, builtins, or operators
6. **`tests/fixtures/`** — Add at least one E2E test fixture (`.sans` file) demonstrating the feature
7. **`examples/`** — Update existing examples or add a new one if the feature is significant enough to showcase
8. **`README.md`** — Update feature list if the feature is user-facing and notable

**If a short alias is added** (e.g., `p` for `print`), it must appear in all of: `ai-reference.md`, `reference.md`, and `HOVER_DATA`.

**Rule: A feature is not done until docs, website docs, hover docs, syntax highlighting, and examples are updated.** Do not split docs into a separate PR — ship them with the feature.

## Versioning
All version numbers follow semver (`x.y.z`) and are managed automatically by CI.

**Do NOT manually bump version numbers.** Version is set by pushing a git tag:

```sh
git tag v0.4.1
git push origin v0.4.1
```

The release workflow (`.github/workflows/release.yml`) automatically:
1. Updates all version files (package.json, website meta tags, CLAUDE.md, compiler/main.sans)
2. Commits and pushes to main
3. Downloads the previous release binary as a bootstrap compiler
4. Builds the self-hosted compiler and creates a GitHub release

**Files managed by CI (do not edit versions manually):**
- `editors/vscode-sans/package.json`
- `website/index.html` (meta tag)
- `website/docs/index.html` (meta tag)
- `website/benchmarks/index.html` (meta tag)
- `website/download/index.html` (meta tag)
- `CLAUDE.md` (this file, "Current version" below)
- `compiler/main.sans` (self-hosted compiler version string)

The CLI `sans --version` reads the hardcoded version string in `compiler/main.sans`.

**Current version: `0.5.4`**

## Rules
- **NEVER commit compiled binaries** (.o files, executables, Mach-O binaries). Use .gitignore to prevent this.
- **All new syntax/features must be AI-optimized** — use the fewest tokens possible.
- **All new features must include documentation updates** — see [Documentation Update Checklist](#documentation-update-checklist).

## Conventions
- All values are stored as i64 in the IR/codegen register map
- Pointers (strings, opaque types) stored in both regs (as i64 via ptr_to_int) and ptrs (as PointerValue)
- Opaque types (JsonValue, HttpResponse, Result, etc.) backed by Sans runtime with `sans_` prefix
- E2E test fixtures must use unique temp filenames to prevent parallel test races

## Known Limitations
- **Scope GC**: User programs get automatic scope-based memory management (alloc/array/map/JSON/Result/string tracked per-function, freed on return, return values promoted to caller including nested container contents). Known gaps:
  - The compiler itself must be built from the bootstrap binary (not self-hosted with scope GC) because compiler internals use deeply nested data structures that scope_exit destroys
  - Thread safety — `rc_alloc_head`/`rc_scope_head` globals are not thread-safe
