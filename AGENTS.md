# Sans Compiler — AI Agent Rules

These rules apply to all AI coding assistants (Claude Code, Copilot, Cursor, Gemini, etc.) working on this project. For Claude Code-specific rules, see `CLAUDE.md`.

## Quick Start

```sh
# Prerequisites
brew install llvm@17   # macOS

# Bootstrap (first time — downloads a release binary and builds from source)
bash scripts/bootstrap.sh

# Build the compiler (after bootstrap)
sans build compiler/main.sans

# Run all tests
bash tests/run_tests.sh

# Install git hooks (optional, warns about missing editor updates)
bash scripts/setup-hooks.sh
```

## Project Overview

Sans is a self-hosted programming language designed for AI code generation. The compiler and all runtime modules are written in Sans itself — no C, Rust, or other languages.

- **Compiler** (~11,600 LOC): `compiler/` — lexer, parser, typeck, constants, ir, codegen, main
- **Runtime** (13 modules): `runtime/*.sans` — arena, array_ext, curl, functional, http, json, log, map, result, server, sock, ssl, string_ext
- **Tests**: E2E fixtures in `tests/fixtures/`, run via `bash tests/run_tests.sh`
- **Docs**: `docs/reference.md`, `docs/ai-reference.md`, `website/docs/index.html`
- **Editor**: VS Code extension in `editors/vscode-sans/`

## Mandatory Rules

### 1. AI-Optimized Syntax
Sans is designed for minimal token usage. Always use the shortest form:

| Use this | Not this | Why |
|----------|----------|-----|
| `x = 42` | `let x = 42` | bare assignment |
| `x := 0` | `let mut x = 0` | mutable |
| `f(x:I) = x*2` | `fn f(x Int) Int { x * 2 }` | short types, expression body |
| `I/S/B/F` | `Int/String/Bool/Float` | short type aliases |
| `R<I>` | `Result<Int>` | short Result |
| `r!` | `r.unwrap()` | unwrap shorthand |
| `r = f()?` | manual error check | try operator |
| `[1 2 3]` | `[1, 2, 3]` | no commas in array literals |
| `cond ? a : b` | `if cond { a } else { b }` | ternary |

### 2. Never Commit Binaries
No `.o` files, executables, or Mach-O binaries. The `.gitignore` handles most cases, but verify before committing.

### 3. Never Manually Bump Versions
Versions are managed by CI via git tags. Do not edit version strings in any file. To release: `git tag v0.x.y && git push origin v0.x.y`.

### 4. Documentation Ships With Features
Every new feature, builtin, method, type, or syntax change must update **all** of these before the work is complete:

1. `docs/reference.md` — human-readable reference
2. `docs/ai-reference.md` — compact AI reference
3. `website/docs/index.html` — website docs
4. **`editors/vscode-sans/src/extension.ts`** — hover docs (`HOVER_DATA`) **(enforced by CI)**
5. **`editors/vscode-sans/syntaxes/sans.tmLanguage.json`** — syntax highlighting **(enforced by CI)**
6. `tests/fixtures/` — E2E test fixture
7. `sans-language/examples` repo — update or add example if significant
8. `README.md` — update feature list if user-facing

**Enforcement:** The `editor-guard` CI workflow warns on PRs that change compiler, runtime, or docs files without updating the VSCode extension. A pre-commit hook (`scripts/setup-hooks.sh`) provides the same warning locally. Add the `skip-editor-guard` label to suppress the CI warning when no editor changes are needed.

### 5. Test Fixtures Use Unique Temp Files
E2E fixtures that write to disk must use unique `/tmp/` filenames (include the test name) to prevent parallel test races.

## Adding Features

### New Built-in Function
Pipeline: typeck → constants → IR → codegen → runtime → tests → docs

1. Type checking in `compiler/typeck.sans`
2. IR instruction constant in `compiler/constants.sans`
3. IR lowering in `compiler/ir.sans`
4. Codegen in `compiler/codegen.sans`
5. Runtime backing (if needed) in `runtime/*.sans`
6. Test fixture in `tests/fixtures/`
7. All documentation (see rule 4 above)

### New Method on a Type
Same pipeline, using method dispatch in typeck and IR lowering.

### New Type
1. Type resolution in `compiler/typeck.sans`
2. IR type handling in `compiler/ir.sans`
3. Codegen support in `compiler/codegen.sans`
4. Runtime backing (if opaque) in `runtime/*.sans`
5. Test fixture + all documentation

## Conventions

- Values stored as `i64` in IR/codegen register map
- Pointers stored in both `regs` (as i64 via ptr_to_int) and `ptrs` (as PointerValue)
- Opaque types (JsonValue, HttpResponse, Result, etc.) backed by runtime functions with `sans_` prefix
- File extension: `.sans`

## Known Limitations

- **Scope GC gaps**: The compiler itself must be built from the bootstrap binary (not self-hosted with scope GC) because compiler internals use deeply nested data structures that scope_exit destroys
- **Thread safety**: `rc_alloc_head`/`rc_scope_head` globals are not thread-safe

## Versioning Enforcement

Version numbers are managed exclusively by the organization via CI.

- **CI guard:** The `version-guard` workflow rejects PRs that modify version-managed files. If your PR is blocked, remove version changes from your diff.
- **Tag protection:** Only organization members can push `v*` tags to trigger releases.
- **Bypass:** If a maintainer needs to edit version-adjacent files, they apply the `skip-version-guard` label.
- **Do not** edit version strings in: `package.json`, `website/*.html` meta tags, `CLAUDE.md` version line, or `compiler/main.sans` version string.

## Workflows

Step-by-step guides for common development workflows. Claude Code users: use the corresponding skills in `.claude/skills/` instead.

| Workflow | Doc | Claude Code Skill |
|----------|-----|-------------------|
| Planning a feature | [docs/workflows/planning.md](docs/workflows/planning.md) | `sans-plan` |
| Architecture review | [docs/workflows/architecture-review.md](docs/workflows/architecture-review.md) | `sans-architect` |
| Skeptic review | [docs/workflows/skeptic-review.md](docs/workflows/skeptic-review.md) | `sans-skeptic` |
| PR review | [docs/workflows/pr-review.md](docs/workflows/pr-review.md) | `sans-review-pr` |
| Writing tests | [docs/workflows/testing.md](docs/workflows/testing.md) | `sans-test` |

**Workflow sequence:** Planning → Architecture Review → Implementation → Testing → PR Review
