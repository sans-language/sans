# Contribution Documentation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create contribution documentation (CONTRIBUTING.md, CODE_OF_CONDUCT.md, PR template, issue templates) and fix stale references in CLAUDE.md.

**Architecture:** CONTRIBUTING.md is the narrative entry point for humans; CLAUDE.md remains the machine-readable authority. No duplication — CONTRIBUTING.md links to CLAUDE.md for checklists. GitHub templates enforce mandatory steps at PR/issue creation time.

**Tech Stack:** Markdown files only. No code changes.

**Spec:** `docs/superpowers/specs/2026-03-17-contribution-docs-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `CLAUDE.md` | Modify | Fix stale C runtime references (prerequisite) |
| `CONTRIBUTING.md` | Create | Human-friendly contribution guide (8 sections) |
| `CODE_OF_CONDUCT.md` | Create | Contributor Covenant v2.1 |
| `.github/PULL_REQUEST_TEMPLATE.md` | Create | PR checklist enforcing mandatory steps |
| `.github/ISSUE_TEMPLATE/bug_report.md` | Create | Structured bug report template |
| `.github/ISSUE_TEMPLATE/feature_request.md` | Create | Structured feature request template |
| `README.md` | Modify | Add "Contributing" section |

---

## Chunk 1: Prerequisites and Templates

### Task 1: Fix stale C runtime references in CLAUDE.md

This is a blocking prerequisite. CONTRIBUTING.md links to CLAUDE.md as the "authoritative rule set" — it must be accurate.

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Fix Architecture section**

Find this line in `CLAUDE.md`:
```
- 8 C runtime files under `runtime/`: json.c, http.c, log.c, result.c, string_ext.c, array_ext.c, functional.c, server.c
```

Replace with:
```
- 13 Sans runtime modules under `runtime/`: arena.sans, array_ext.sans, curl.sans, functional.sans, http.sans, json.sans, log.sans, map.sans, result.sans, server.sans, sock.sans, ssl.sans, string_ext.sans
```

- [ ] **Step 2: Fix "Adding a New Built-in Function" step 5**

Find this line in `CLAUDE.md`:
```
5. If backed by C: add function to appropriate `runtime/*.c` file
```

Replace with:
```
5. If backed by runtime: add function to appropriate `runtime/*.sans` file
```

- [ ] **Step 3: Fix "Adding a New Type" step 7**

Find this line in `CLAUDE.md`:
```
7. If opaque: add C runtime backing
```

Replace with:
```
7. If opaque: add Sans runtime backing in `runtime/*.sans`
```

- [ ] **Step 4: Fix Conventions section**

Find this line in `CLAUDE.md`:
```
- Opaque types (JsonValue, HttpResponse, Result, etc.) backed by C runtime with `cy_` prefix
```

Replace with:
```
- Opaque types (JsonValue, HttpResponse, Result, etc.) backed by Sans runtime with `sans_` prefix
```

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: fix stale C runtime references in CLAUDE.md — runtime is 100% Sans"
```

---

### Task 2: Create PR template

**Files:**
- Create: `.github/PULL_REQUEST_TEMPLATE.md`

- [ ] **Step 1: Create the PR template**

Write `.github/PULL_REQUEST_TEMPLATE.md` with this exact content:

```markdown
## Summary
<!-- What does this PR do and why? -->

## Checklist
- [ ] Tests pass (`LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test`)
- [ ] Version bumped in all required files (see CLAUDE.md Versioning)
- [ ] Docs updated per CLAUDE.md Documentation Update Checklist (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README)
- [ ] Code self-reviewed (AI contributors: use superpowers:requesting-code-review)
- [ ] No compiled binaries committed

## Test plan
<!-- How did you verify this works? -->
```

- [ ] **Step 2: Commit**

```bash
git add .github/PULL_REQUEST_TEMPLATE.md
git commit -m "docs: add pull request template with mandatory checklist"
```

---

### Task 3: Create issue templates

**Files:**
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`

- [ ] **Step 0: Create directory**

```bash
mkdir -p .github/ISSUE_TEMPLATE
```

- [ ] **Step 1: Create bug report template**

Create `.github/ISSUE_TEMPLATE/bug_report.md` with this content:

```markdown
---
name: Bug Report
about: Report a bug in the Sans compiler or runtime
labels: bug
---

### Sans version
<!-- Output of `sans --version` -->

### OS / Architecture
<!-- e.g., macOS 15.3, ARM64 -->

### Code to reproduce
<!-- Minimal .sans file that triggers the bug -->
```sans
```

### Expected behavior
<!-- What should happen? -->

### Actual behavior
<!-- What happens instead? Include compiler output or error messages. -->
```

- [ ] **Step 2: Create feature request template**

Create `.github/ISSUE_TEMPLATE/feature_request.md` with this content:

```markdown
---
name: Feature Request
about: Suggest a new feature for Sans
labels: enhancement
---

### What the feature does
<!-- One sentence. -->

### Proposed syntax
<!-- Show the Sans syntax you'd like. Remember: "Can this be expressed in fewer tokens?" -->
```sans
```

### Example code
<!-- A short .sans program showing the feature in use. -->
```sans
```

### Pipeline stages affected
<!-- Optional. Which compiler stages would this touch? -->
<!-- typeck / IR / codegen / runtime -->

### Documentation impact
<!-- Which of the 8 doc targets would this affect? -->
<!-- reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README -->
```

- [ ] **Step 3: Commit**

```bash
git add .github/ISSUE_TEMPLATE/bug_report.md .github/ISSUE_TEMPLATE/feature_request.md
git commit -m "docs: add bug report and feature request issue templates"
```

---

## Chunk 2: CONTRIBUTING.md, CODE_OF_CONDUCT.md, README

### Task 4: Create CODE_OF_CONDUCT.md

**Files:**
- Create: `CODE_OF_CONDUCT.md`

- [ ] **Step 1: Create CODE_OF_CONDUCT.md**

Fetch the Contributor Covenant v2.1 text using WebFetch from `https://www.contributor-covenant.org/version/2/1/code_of_conduct/code_of_conduct.md`, then write it to `CODE_OF_CONDUCT.md` with this project-specific enforcement contact replacing the placeholder:

- **Enforcement contact:** "Open a GitHub issue on [sans-language/sans](https://github.com/sans-language/sans/issues) with the label `conduct`."

- [ ] **Step 2: Commit**

```bash
git add CODE_OF_CONDUCT.md
git commit -m "docs: add Contributor Covenant Code of Conduct v2.1"
```

---

### Task 5: Create CONTRIBUTING.md

**Files:**
- Create: `CONTRIBUTING.md`

- [ ] **Step 1: Write CONTRIBUTING.md**

Write `CONTRIBUTING.md` with this exact content:

````markdown
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
- Version bump (patch minimum, e.g., `0.3.44` -> `0.3.45`) across all required files — see [CLAUDE.md Versioning](CLAUDE.md#versioning-mandatory--do-not-skip).
- Documentation updates in all 8 targets — see [CLAUDE.md Documentation Update Checklist](CLAUDE.md#documentation-update-checklist).
- Tests pass.
- No compiled binaries (.o files, executables) committed.

Version bump and documentation updates must ship **in the same commit** as the feature — not as separate follow-up commits.

## Common Gotchas

- **Version bump is mandatory** on every code change, every time, across 6+ files. This is the most common mistake.
- **Documentation updates span 8 places** (reference.md, ai-reference.md, docs.html, HOVER_DATA, syntax highlighting, test fixtures, examples, README). Miss any one and the PR will be rejected.
- **`!` is bitwise NOT**, not logical NOT. `!1` is `-2` (truthy). Use `== 0` for logical negation.
- **No garbage collector** — heap allocations are leaked. Use `arena_begin()`/`arena_alloc(n)`/`arena_end()` for phase-based bulk deallocation.
- **E2E test fixtures** must use unique temp filenames (e.g., `/tmp/sans_test_<fixture_name>`) to prevent parallel test races.

## Getting Help

Open a GitHub issue using the [bug report](https://github.com/sans-language/sans/issues/new?template=bug_report.md) or [feature request](https://github.com/sans-language/sans/issues/new?template=feature_request.md) template. For questions about the codebase, open a discussion or issue.
````

- [ ] **Step 2: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "docs: add CONTRIBUTING.md — contributor guide for humans and AI agents"
```

---

### Task 6: Add Contributing section to README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add Contributing section**

Find this line in `README.md`:
```
## Known Limitations
```

Insert before it:
```markdown
## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to set up the development environment, add features, and submit pull requests. AI agents: read [CLAUDE.md](CLAUDE.md) for the complete rule set.

```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add Contributing section to README linking to CONTRIBUTING.md"
```
