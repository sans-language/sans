# Contributing to Sans

Sans is an AI-first compiled language — contributions from both humans and AI agents are welcome. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

## Quick Setup

Sans runs on macOS, Linux, and Windows (via MSYS2/MinGW).

**Prerequisites:** LLVM 17 and a previous `sans` binary (for bootstrapping).

### macOS

```sh
brew install llvm@17
xcode-select --install

curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-macos-arm64.tar.gz | tar xz
sudo mv sans /usr/local/bin/
```

### Linux (Ubuntu/Debian)

```sh
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 17

curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-linux-x86_64.tar.gz | tar xz
sudo mv sans /usr/local/bin/
```

### Windows (MSYS2)

Install [MSYS2](https://www.msys2.org/), then from a MINGW64 shell:

```sh
pacman -S mingw-w64-x86_64-llvm mingw-w64-x86_64-clang mingw-w64-x86_64-curl mingw-w64-x86_64-openssl mingw-w64-x86_64-zlib

curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-windows-x86_64.tar.gz | tar xz
mv sans.exe /usr/local/bin/
```

### Build & Test

```sh
git clone https://github.com/sans-language/sans && cd sans
sans build compiler/main.sans

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

1. **Read [CLAUDE.md](CLAUDE.md) fully** — it is the authoritative rule set for this project.
2. **Follow the development workflows** in `docs/workflows/`:
   - [Planning](docs/workflows/planning.md) — identify pipeline stages, map changes, build doc checklist
   - [Architecture Review](docs/workflows/architecture-review.md) — evaluate against Sans conventions
   - [Skeptic Review](docs/workflows/skeptic-review.md) — challenge the design before implementing
   - [Testing](docs/workflows/testing.md) — write comprehensive E2E fixtures
   - [PR Review](docs/workflows/pr-review.md) — full review before submission
3. **Claude Code users:** Use the project skills in `.claude/skills/` (`sans-plan`, `sans-architect`, `sans-skeptic`, `sans-test`, `sans-review-pr`) — they inject Sans-specific context into the workflow. The general superpowers skills (brainstorming, writing-plans, etc.) are also available as the underlying workflow engine.
4. **Self-review is required** — run the PR review workflow or `sans-review-pr` skill before opening a PR.
5. **AI-optimized syntax** — all new features must use the fewest tokens possible. See CLAUDE.md.

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

## Development Workflows

These guides walk through common development tasks step by step. Useful for both human and AI contributors.

| Workflow | Guide |
|----------|-------|
| Planning a feature | [docs/workflows/planning.md](docs/workflows/planning.md) |
| Architecture review | [docs/workflows/architecture-review.md](docs/workflows/architecture-review.md) |
| Skeptic review | [docs/workflows/skeptic-review.md](docs/workflows/skeptic-review.md) |
| Writing tests | [docs/workflows/testing.md](docs/workflows/testing.md) |
| PR review | [docs/workflows/pr-review.md](docs/workflows/pr-review.md) |

**Recommended sequence:** Planning → Architecture Review → Implementation → Testing → PR Review

## Common Gotchas

- **Do not manually bump version numbers.** Version is managed by CI on tag push (`git tag v0.5.3 && git push origin v0.5.3`). The workflow updates all version files automatically.
- **Documentation updates span 8 places** (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README). Miss any one and the PR will be rejected.
- **`!` is postfix unwrap** for Result types, not logical NOT. Use `== 0` for logical negation.
- **Scope GC** manages heap memory automatically — allocations are freed on function return. Use `arena_begin()`/`arena_end()` for hot paths that need manual control.
- **E2E test fixtures** must use unique temp filenames to prevent parallel test races.
- **CI blocks version changes in PRs.** The `version-guard` workflow will fail your PR if it modifies version-managed files. Remove version changes from your diff. Only maintainers can release via git tags.
- **Bootstrap binary required** — the compiler compiles itself, so you need an existing `sans` binary to build from source.

## Getting Help

Open a GitHub issue using the [bug report](https://github.com/sans-language/sans/issues/new?template=bug_report.md) or [feature request](https://github.com/sans-language/sans/issues/new?template=feature_request.md) template.
