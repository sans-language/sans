# Contributing to Sans

Sans is an AI-first compiled language — contributions from both humans and AI agents are welcome. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

## Quick Setup

macOS is the primary development platform. Linux contributors should adapt the LLVM path (e.g., `apt install llvm-17`). Windows is not currently supported for development.

**Prerequisites:** LLVM 17, Xcode Command Line Tools (macOS), a previous `sans` binary (for bootstrapping).

```sh
# Install dependencies (macOS)
brew install llvm@17
xcode-select --install

# Get the latest sans binary (needed to build the self-hosted compiler)
curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-macos-arm64.tar.gz | tar xz
sudo mv sans /usr/local/bin/

# Build the compiler from source
git clone https://github.com/sans-language/sans && cd sans
sans build compiler/main.sans

# Run tests
bash tests/run_tests.sh ./compiler/main
```

## Architecture Overview

Sans is **fully self-hosted** — the compiler is written in Sans itself. There is no Rust code.

| Module | Location | Purpose |
|--------|----------|---------|
| **lexer** | `compiler/lexer.sans` | Tokenizes source code |
| **parser** | `compiler/parser.sans` | Parses tokens into an AST |
| **typeck** | `compiler/typeck.sans` | Type-checks the AST — resolves types, validates expressions |
| **constants** | `compiler/constants.sans` | IR instruction opcodes, type tags, statement tags |
| **ir** | `compiler/ir.sans` | Lowers the AST to a flat intermediate representation |
| **codegen** | `compiler/codegen.sans` | Compiles IR to LLVM IR text |
| **main** | `compiler/main.sans` | CLI entry point — orchestrates pipeline, links runtime, produces binary |

**Runtime:** The `runtime/` directory contains 13+ `.sans` modules (zero C files) implementing the standard library: arrays, strings, maps, JSON, HTTP, file I/O, logging, concurrency, memory management, and more.

**Tests:** End-to-end tests in `tests/fixtures/` — each is a `.sans` file compiled and run, with the exit code checked against an expected value. Run with `bash tests/run_tests.sh ./compiler/main`.

## How to Add a Feature

All features follow the same compiler pipeline: **typeck → IR → codegen → (optional) runtime**.

### Adding a New Built-in Function

Example: adding `abs(x)` returning the absolute value of an integer.

1. **Type checking** (`compiler/typeck.sans`) — Add the function's type signature in the builtin call dispatch. This tells the compiler that `abs` takes `I` and returns `I`.

2. **IR constant** (`compiler/constants.sans`) — Add a new `IR_ABS` instruction constant.

3. **IR lowering** (`compiler/ir.sans`) — Lower the `abs()` call to the new IR instruction.

4. **Code generation** (`compiler/codegen.sans`) — Emit LLVM IR for the instruction. For simple operations, emit inline LLVM. For complex ones, call a runtime function.

5. **Runtime** (`runtime/*.sans`) — If the function needs a backing implementation (e.g., `sans_abs`), add it to the appropriate runtime module.

6. **Test** — Add an E2E fixture in `tests/fixtures/` and register it in `tests/run_tests.sh`.

7. **Documentation** — Update all targets per the [Documentation Update Checklist](CLAUDE.md#documentation-update-checklist).

Sans has three feature-addition pipelines — see [CLAUDE.md](CLAUDE.md) for the precise checklists:
- **Adding a New Built-in Function** (like `abs`)
- **Adding a New Method on a Type** (like `arr.reverse()`)
- **Adding a New Type** (like adding a `Set` type)

## AI Agent Contributors

If you are an AI agent (or a human directing one):

1. **Read [CLAUDE.md](CLAUDE.md) fully** — it is the authoritative rule set for this project. It contains the exact pipeline steps, mandatory checklists, and conventions.
2. **Use Claude Code skills** — the `docs/superpowers/` directory contains structured development workflows:
   - **brainstorming** — design exploration before implementation
   - **writing-plans** — create detailed implementation plans
   - **subagent-driven-development** — execute plans with parallel task dispatch
   - **requesting-code-review** — self-review before requesting human review
3. **Self-review is required** — run the code-review skill before opening a PR.
4. **AI-optimized syntax** — all new features must use the fewest tokens possible. Ask: "Can this be expressed in fewer tokens?" See the syntax rules in CLAUDE.md.

## Pull Request Process

1. **Fork** the repository and create a feature branch.
2. **Implement** the feature following the pipeline steps above.
3. **Test** — `bash tests/run_tests.sh ./compiler/main` must show 0 failures.
4. **Self-review** your changes (AI contributors: use the code-review skill).
5. **Open a PR** — the template checklist will remind you of all mandatory steps.

**Mandatory in every PR:**
- Documentation updates in all targets — see [CLAUDE.md Documentation Update Checklist](CLAUDE.md#documentation-update-checklist).
- All tests pass.
- No compiled binaries (.o files, executables) committed.
- **Do not manually bump version numbers.** Version is managed by CI when the maintainer pushes a release tag. See [CLAUDE.md Versioning](CLAUDE.md#versioning).

## Common Gotchas

- **Do not manually bump version numbers.** Version is managed by CI on tag push (`git tag v0.5.3 && git push origin v0.5.3`). The workflow updates all version files automatically.
- **Documentation updates span 8 places** (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README). Miss any one and the PR will be rejected.
- **`!` is postfix unwrap** for Result types, not logical NOT. Use `== 0` for logical negation.
- **Scope GC** manages heap memory automatically — allocations are freed on function return. Use `arena_begin()`/`arena_end()` for hot paths that need manual control.
- **E2E test fixtures** must use unique temp filenames to prevent parallel test races.
- **Bootstrap binary required** — the compiler compiles itself, so you need an existing `sans` binary to build from source.

## Getting Help

Open a GitHub issue using the [bug report](https://github.com/sans-language/sans/issues/new?template=bug_report.md) or [feature request](https://github.com/sans-language/sans/issues/new?template=feature_request.md) template.
