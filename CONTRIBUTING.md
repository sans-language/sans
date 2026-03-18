# Contributing to Sans

Sans is an AI-first compiled language — contributions from both humans and AI agents are welcome. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

## Quick Setup

macOS is the primary development platform. Linux contributors should adapt the LLVM path (e.g., `apt install llvm-17-dev` and set `LLVM_SYS_170_PREFIX=/usr/lib/llvm-17`). Windows is not currently supported for development.

**Prerequisites:** Rust (stable), LLVM 17, Xcode Command Line Tools (macOS).

```sh
# Install dependencies (macOS)
brew install llvm@17
xcode-select --install

# Build
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build

# Test
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test
```

## Architecture Overview

The compiler is a pipeline of 6 Rust crates, each handling one stage:

| Crate | Location | Purpose |
|-------|----------|---------|
| **sans-lexer** | `crates/sans-lexer/` | Tokenizes source code into a token stream |
| **sans-parser** | `crates/sans-parser/` | Parses tokens into an Abstract Syntax Tree (AST) |
| **sans-typeck** | `crates/sans-typeck/` | Type-checks the AST — resolves types, validates expressions, infers return types |
| **sans-ir** | `crates/sans-ir/` | Lowers the AST to a flat intermediate representation with opaque type tracking |
| **sans-codegen** | `crates/sans-codegen/` | Compiles IR to LLVM IR — register allocation, instruction emission |
| **sans-driver** | `crates/sans-driver/` | CLI entry point — orchestrates the pipeline, links with the runtime, produces the final binary |

**Self-hosted compiler:** The `compiler/` directory contains a separate Sans compiler written in Sans itself (~11,600 LOC across 7 modules). Feature additions target the Rust compiler pipeline. The self-hosted compiler is maintained in parallel — changes there are not required unless explicitly noted.

**Self-hosted runtime:** The `runtime/` directory contains 13 `.sans` modules (zero C files) implementing the standard library: arrays, strings, maps, JSON, HTTP, file I/O, logging, concurrency, and more. These are compiled from source on every build.

**Tests:** Unit tests live in each crate. End-to-end tests are in `crates/sans-driver/tests/e2e.rs` with fixtures in `tests/fixtures/`. Each fixture is a `.sans` file with an expected output comment.

## How to Add a Feature

Here's what happens when you add a new built-in function — say, `abs(x)` returning the absolute value of an integer.

**1. Type checking** (`crates/sans-typeck/src/lib.rs`) — Add the function's type signature to the `Expr::Call` match. This tells the compiler that `abs` takes an `Int` and returns an `Int`. If you skip this step, the compiler reports "unknown function."

**2. IR lowering** (`crates/sans-ir/`) — Add an IR instruction variant (e.g., `Abs`) and lower the `abs()` call to it. If you skip this step, typeck passes but codegen has no instruction to emit.

**3. Code generation** (`crates/sans-codegen/src/lib.rs`) — Emit the LLVM IR for the new instruction. If you skip this step, the IR instruction is silently dropped and the function returns garbage.

**4. Runtime** (`runtime/*.sans`) — If the function needs low-level backing (memory management, syscalls, etc.), implement it in the appropriate runtime module. Many built-ins are pure LLVM and don't need a runtime function.

**5. Driver** (`crates/sans-driver/`) — The driver links runtime modules automatically. Usually no changes needed here.

**6. Documentation + version bump** — Update all 8 documentation targets and bump the version number. If you skip this, the PR will be rejected. See the [Pull Request Process](#pull-request-process) section.

The self-hosted compiler in `compiler/` is a separate codebase. Unless the feature involves the self-hosted compiler directly, you only need to modify the Rust crates.

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
3. **Self-review is required** — run the code-review skill before opening a PR. Human review is the final gate.
4. **AI-optimized syntax** — all new features must use the fewest tokens possible. Ask: "Can this be expressed in fewer tokens?" See the syntax rules in CLAUDE.md.

## Pull Request Process

1. **Fork** the repository and create a feature branch.
2. **Implement** the feature following the pipeline steps above.
3. **Self-review** your changes (AI contributors: use the code-review skill).
4. **Open a PR** — the template checklist will remind you of all mandatory steps.

**Mandatory in every PR:**
- Documentation updates in all 8 targets — see [CLAUDE.md Documentation Update Checklist](CLAUDE.md#documentation-update-checklist).
- Tests pass.
- No compiled binaries (.o files, executables) committed.
- **Do not manually bump version numbers.** Version is managed by CI when the maintainer pushes a release tag. See [CLAUDE.md Versioning](CLAUDE.md#versioning).

## Common Gotchas

- **Do not manually bump version numbers.** Version is managed by CI when the maintainer pushes a release tag (`git tag v0.3.45 && git push origin v0.3.45`). The workflow updates all version files automatically.
- **Documentation updates span 8 places** (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README). Miss any one and the PR will be rejected.
- **`!` is bitwise NOT**, not logical NOT. `!1` is `-2` (truthy). Use `== 0` for logical negation.
- **No garbage collector** — heap allocations are leaked. Use `arena_begin()`/`arena_alloc(n)`/`arena_end()` for phase-based bulk deallocation.
- **E2E test fixtures** must use unique temp filenames (e.g., `/tmp/sans_test_<fixture_name>`) to prevent parallel test races.

## Getting Help

Open a GitHub issue using the [bug report](https://github.com/sans-language/sans/issues/new?template=bug_report.md) or [feature request](https://github.com/sans-language/sans/issues/new?template=feature_request.md) template. For questions about the codebase, open a discussion or issue.
