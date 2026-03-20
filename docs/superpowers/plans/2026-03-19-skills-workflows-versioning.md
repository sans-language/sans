# Skills, Workflows & Versioning Enforcement — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add CI versioning enforcement, 5 Claude Code skills, 5 standalone workflow docs, and update AGENTS.md + CONTRIBUTING.md for multi-developer collaboration.

**Architecture:** 11 new files + 2 updated files. The CI workflow enforces versioning via pattern-level diff checks. Skills are markdown files in `.claude/skills/` with YAML frontmatter. Workflow docs in `docs/workflows/` mirror each skill for non-Claude-Code users.

**Tech Stack:** GitHub Actions (YAML), Claude Code skills (Markdown with frontmatter), Git

**Spec:** `docs/superpowers/specs/2026-03-19-skills-workflows-versioning-design.md`

---

## Chunk 1: CI Versioning Enforcement

### Task 1: Create version-guard GitHub Action

**Files:**
- Create: `.github/workflows/version-guard.yml`

- [ ] **Step 1: Create the workflow file**

```yaml
name: Version Guard

on:
  pull_request:
    branches: [main]

jobs:
  check-version-files:
    name: Block version file changes
    runs-on: ubuntu-latest
    if: "!contains(github.event.pull_request.labels.*.name, 'skip-version-guard')"
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Check for version changes
        run: |
          BASE="${{ github.event.pull_request.base.sha }}"
          HEAD="${{ github.event.pull_request.head.sha }}"
          VIOLATIONS=""

          # Helper: extract diff for a specific file
          file_diff() {
            git diff "$BASE...$HEAD" -U0 -- "$1"
          }

          # Check package.json version field
          if file_diff "editors/vscode-sans/package.json" | grep -qE '^\+.*"version"\s*:'; then
            VIOLATIONS="$VIOLATIONS\n  - editors/vscode-sans/package.json (version field)"
          fi

          # Check CLAUDE.md version line
          if file_diff "CLAUDE.md" | grep -qE '^\+.*\*\*Current version:'; then
            VIOLATIONS="$VIOLATIONS\n  - CLAUDE.md (Current version line)"
          fi

          # Check compiler/main.sans version string
          if file_diff "compiler/main.sans" | grep -qE '^\+.*"sans [0-9]+\.[0-9]+\.[0-9]+"'; then
            VIOLATIONS="$VIOLATIONS\n  - compiler/main.sans (version string)"
          fi

          # Check website HTML meta version tags
          for html in website/index.html website/docs/index.html website/benchmarks/index.html website/download/index.html; do
            if file_diff "$html" | grep -qE '^\+.*content="[0-9]+\.[0-9]+\.[0-9]+"'; then
              VIOLATIONS="$VIOLATIONS\n  - $html (version meta tag)"
            fi
          done

          if [ -n "$VIOLATIONS" ]; then
            echo "::error::Version files are managed by CI on tag push. Please remove version changes from your PR."
            echo ""
            echo "Detected version changes in:"
            echo -e "$VIOLATIONS"
            echo ""
            echo "See CLAUDE.md#versioning for details."
            echo "If this is intentional, ask a maintainer to add the 'skip-version-guard' label."
            exit 1
          fi

          echo "✓ No version file changes detected."
```

- [ ] **Step 2: Verify YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/version-guard.yml'))"`
Expected: No error (silent success)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/version-guard.yml
git commit -m "ci: add version-guard workflow to block version changes in PRs"
```

---

## Chunk 2: Claude Code Skills

### Task 2: Create sans-plan skill

**Files:**
- Create: `.claude/skills/sans-plan.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: sans-plan
type: skill
description: Plan a Sans compiler feature — maps compiler pipeline stages, doc checklist, and AI syntax rules before creating the implementation plan
---

# Sans Feature Planning

You are planning a feature for the Sans compiler. Before creating the implementation plan, gather project-specific context.

## Step 1: Identify the Feature Type

Determine which pipeline applies:

**New built-in function** (e.g., `abs`, `exit`):
1. `compiler/typeck.sans` — add type signature in builtin call dispatch
2. `compiler/constants.sans` — add IR instruction constant (e.g., `IR_ABS`)
3. `compiler/ir.sans` — lower the call to the new IR instruction
4. `compiler/codegen.sans` — emit LLVM IR for the instruction
5. `runtime/*.sans` — add backing function with `sans_` prefix (if needed)

**New method on a type** (e.g., `arr.reverse()`):
1. `compiler/typeck.sans` — add method in type method dispatch
2. `compiler/constants.sans` — add IR instruction constant
3. `compiler/ir.sans` — lower via method dispatch
4. `compiler/codegen.sans` — emit LLVM IR
5. `runtime/*.sans` — add backing function (if needed)

**New type** (e.g., `Set`):
1. `compiler/typeck.sans` — add type resolution
2. `compiler/ir.sans` — add IR type handling
3. `compiler/codegen.sans` — add codegen support
4. `runtime/*.sans` — add runtime backing with `sans_` prefix (if opaque)

## Step 2: Check Conventions

Before planning, verify the design follows Sans conventions:
- **i64 register model**: all values stored as i64 in IR/codegen register map
- **Pointer dual storage**: pointers in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue)
- **Opaque types**: backed by runtime functions with `sans_` prefix
- **Scope GC**: allocations tracked per-function, freed on return, return values promoted to caller
- **AI-optimized syntax**: can this be expressed in fewer tokens? Use short forms (`I/S/B/F`, bare assignment, no commas)

## Step 3: Pre-populate Documentation Checklist

Every feature plan MUST include tasks for ALL of these:
1. `docs/reference.md` — human-readable reference
2. `docs/ai-reference.md` — compact AI reference
3. `website/docs/index.html` — website documentation
4. `editors/vscode-sans/src/extension.ts` — hover docs in `HOVER_DATA`
5. `editors/vscode-sans/syntaxes/sans.tmLanguage.json` — syntax highlighting
6. `tests/fixtures/` — E2E test fixture(s)
7. `examples/` — update or add example if significant
8. `README.md` — update feature list if user-facing

## Step 4: Scope GC Implications

Consider:
- Does this feature allocate heap memory? If so, it must be tracked by scope GC.
- Does the return value need promotion to the caller's scope?
- Are there nested containers (e.g., array of arrays) that need recursive promotion?
- Does this interact with arenas?

## Step 5: Create the Plan

Now invoke the `superpowers:writing-plans` skill with all the context gathered above. The plan should include:
- The specific compiler files and line ranges to modify
- The exact pipeline steps for this feature type
- All 8 documentation update tasks
- Test fixtures with expected exit codes
- Scope GC considerations noted in relevant tasks
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/sans-plan.md
git commit -m "feat: add sans-plan skill for compiler feature planning"
```

### Task 3: Create sans-architect skill

**Files:**
- Create: `.claude/skills/sans-architect.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: sans-architect
type: skill
description: Evaluate a design against Sans compiler architecture — register model, opaque types, scope GC, and AI-optimized syntax conventions
---

# Sans Architecture Review

You are reviewing a proposed design for the Sans compiler. Evaluate it against the project's architecture and conventions.

## Sans Architecture Overview

**Compiler pipeline:** Source → Lexer → Parser → Typeck → IR → Codegen → LLVM IR → Binary

**Key files:**
| File | Purpose |
|------|---------|
| `compiler/lexer.sans` | Tokenizes source |
| `compiler/parser.sans` | Tokens → AST |
| `compiler/typeck.sans` | Type-checks AST, resolves types |
| `compiler/constants.sans` | IR opcodes, type/statement tags |
| `compiler/ir.sans` | AST → flat IR |
| `compiler/codegen.sans` | IR → LLVM IR text |
| `compiler/main.sans` | CLI, pipeline orchestration, linking |

**Runtime:** 13 `.sans` modules in `runtime/` — all self-hosted, no C.

**Register model:** All values stored as `i64`. Pointers stored in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue).

**Opaque types:** JsonValue, HttpResponse, Result, Map, etc. — backed by runtime with `sans_` prefix functions.

**Scope GC:** Allocations tracked per-function, freed on return. Return values promoted to caller scope (including nested container contents).

## Evaluation Checklist

For every proposed design, answer these questions:

### Fit with Register Model
- [ ] Does this use i64 for value storage?
- [ ] If it introduces pointers, are they stored in both `regs` and `ptrs`?
- [ ] Does it need a new type tag in `constants.sans`?

### IR Design
- [ ] Does this need a new IR instruction, or can it reuse an existing one?
- [ ] Is the operation compile-time (typeck/codegen) or runtime (needs `sans_` function)?
- [ ] If it adds an IR instruction, is the constant added to `constants.sans`?

### Scope GC Interaction
- [ ] Does this allocate heap memory?
- [ ] Will allocations be tracked by scope GC?
- [ ] What happens when this value crosses a function boundary?
- [ ] Are there nested containers that need recursive promotion?
- [ ] Does this need arena support for hot paths?

### AI-Optimized Syntax
- [ ] Can the syntax be expressed in fewer tokens?
- [ ] Are short type names used (`I/S/B/F/R<T>`)?
- [ ] Does it follow existing patterns (bare assignment, expression bodies, no commas)?

### Conflicts
- [ ] Does this conflict with existing builtins or methods?
- [ ] Does it introduce ambiguity in the parser?
- [ ] Does it break backward compatibility?

### Thread Safety
- [ ] Does this touch global state (`rc_alloc_head`, `rc_scope_head`)?
- [ ] If used from spawned threads, are there race conditions?
- (Note: thread safety is a known limitation — flag as "worth discussing", not blocking)

## Output Format

Present findings as:

### Architecture Review: [Feature Name]

**Verdict:** APPROVED / NEEDS CHANGES / BLOCKED

**Findings:**
- **Blocking:** [issues that must be resolved]
- **Worth discussing:** [design questions to consider]
- **Minor nits:** [suggestions, not required]

**Recommendations:** [specific suggestions for improvement]

## After Review

If the design needs iteration, work with the user to resolve issues. Once approved, invoke the `superpowers:brainstorming` skill to continue the design process with the architecture context established here.
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/sans-architect.md
git commit -m "feat: add sans-architect skill for architecture review"
```

### Task 4: Create sans-skeptic skill

**Files:**
- Create: `.claude/skills/sans-skeptic.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: sans-skeptic
type: skill
description: Devil's advocate — challenges designs before implementation and critiques code after, checking for over-engineering, missing edge cases, and deviation from Sans conventions
---

# Sans Skeptic Review

You are the skeptic. Your job is to challenge assumptions, find gaps, and prevent unnecessary complexity. You operate in two modes.

## Determine Mode

- **Pre-implementation:** You are given a spec or design document → challenge it before code is written
- **Post-implementation:** You are given a diff or changed files → critique code that has been written

## Pre-Implementation Mode

Read the spec/design, then systematically challenge it:

### Necessity
- Do we actually need this? What problem does it solve?
- Could an existing feature handle this with minor changes?
- Is the user asking for this, or are we guessing they'll want it?
- If we don't build this, what's the real cost?

### Edge Cases
- What happens with empty input? (empty string, empty array, zero, nil)
- What happens with huge input? (max i64, very long strings, large arrays)
- What happens with invalid input? (wrong types, malformed data)
- What happens at boundaries? (first element, last element, single element)

### Scope GC
- Does this allocate? If so, is it tracked?
- What if this is called in a loop — does memory accumulate?
- What if the return value is stored in a container that crosses scope boundaries?
- Does this interact with arenas? Should it?

### Concurrency
- What if this is called from a spawned thread?
- Does it touch global state?
- (Flag as "worth discussing" — thread safety is a known limitation, not blocking unless the PR claims safety)

### Syntax
- Is this the shortest possible syntax? Can it use fewer tokens?
- Does it follow existing patterns or introduce a new pattern?
- Will this confuse an AI generating code? (The primary user is AI, not humans)

### Complexity
- Is this over-engineered for the current use case?
- Are we designing for hypothetical future requirements?
- Could a simpler approach work for now?
- How many compiler files does this touch? Is that proportional to the value?

## Post-Implementation Mode

Read the diff/changed files, then critique:

### Over-engineering
- Are there abstractions that are only used once?
- Are there helper functions that could be inlined?
- Is there error handling for scenarios that can't happen?
- Are there configuration options nobody asked for?

### Scope Creep
- Does the diff change things beyond what the spec called for?
- Are there "while I'm here" improvements that weren't planned?
- Is the test coverage proportional or excessive?

### Convention Violations
- Is AI-optimized syntax used in all new code and examples?
- Are naming conventions followed (`sans_` prefix, `IR_` constants)?
- Are opaque types handled correctly (runtime backing)?
- Do test fixtures use unique `/tmp/` filenames?

### Consistency
- Do the changes follow existing patterns in the codebase?
- If typeck handles a new case, do IR and codegen handle it too?
- Are all 8 documentation targets updated?

## Output Format

```markdown
## Skeptic Review: [Feature/PR Name]

### Mode: Pre-implementation / Post-implementation

### Blocking (must address)
1. [issue] — [why this matters]

### Worth Discussing (should consider)
1. [issue] — [trade-off to weigh]

### Minor Nits (take it or leave it)
1. [suggestion]

### Summary
[1-2 sentence overall assessment]
```
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/sans-skeptic.md
git commit -m "feat: add sans-skeptic skill for design and code critique"
```

### Task 5: Create sans-review-pr skill

**Files:**
- Create: `.claude/skills/sans-review-pr.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: sans-review-pr
type: skill
description: Full PR review for Sans — checklist compliance, code review, compiler pipeline consistency, then build and run tests
---

# Sans PR Review

You are reviewing a pull request for the Sans compiler. This is a full review covering checklist compliance, code quality, architecture fit, and build verification.

## Input

The user provides a PR number, branch name, or asks to review current changes.

## Stage 1: Fetch the Diff

```bash
# For a PR number:
gh pr diff <number>

# For a branch:
git diff main...<branch>

# For current changes:
git diff
```

Read the full diff before proceeding. Identify all changed files.

## Stage 2: Checklist Compliance

Check each item. Mark PASS or FAIL with explanation.

### Documentation (all 8 targets)
If the PR adds a feature, builtin, method, type, or syntax change, ALL of these must be updated:
- [ ] `docs/reference.md`
- [ ] `docs/ai-reference.md`
- [ ] `website/docs/index.html`
- [ ] `editors/vscode-sans/src/extension.ts` (HOVER_DATA)
- [ ] `editors/vscode-sans/syntaxes/sans.tmLanguage.json`
- [ ] `tests/fixtures/` (at least one E2E fixture)
- [ ] `examples/` (if significant)
- [ ] `README.md` (if user-facing)

If the PR is a bug fix or refactor that doesn't add user-facing features, not all docs need updating — use judgment.

### Version Guard
- [ ] No changes to version-managed files: `editors/vscode-sans/package.json` (version field), `website/*.html` (meta tags), `CLAUDE.md` (version line), `compiler/main.sans` (version string)

### Binary Check
- [ ] No `.o` files, executables, or Mach-O binaries in the diff

### AI-Optimized Syntax
- [ ] New code and examples use short forms: `I/S/B/F`, bare assignment, expression bodies, no commas in array literals, `r!` for unwrap, `?` for try

### Test Fixtures
- [ ] New fixture(s) added to `tests/fixtures/`
- [ ] Fixture(s) registered in `tests/run_tests.sh` with correct expected exit code
- [ ] Fixture filenames are descriptive (`<feature>_<scenario>.sans`)
- [ ] Fixtures use unique `/tmp/` filenames if writing to disk

## Stage 3: Code Review

### Compiler Pipeline Consistency
If the PR modifies compiler files, verify the pipeline is consistent:
- If `typeck.sans` adds a new type/builtin/method → does `constants.sans` have the IR constant?
- If `constants.sans` adds a constant → does `ir.sans` lower it?
- If `ir.sans` lowers a new instruction → does `codegen.sans` emit LLVM IR for it?
- If `codegen.sans` calls a runtime function → does `runtime/*.sans` define it with `sans_` prefix?

### Runtime Safety
- Memory management: are allocations tracked by scope GC?
- Return value promotion: do returned values get promoted to caller scope?
- Nested containers: are nested allocations recursively promoted?
- Arena interaction: if arenas are used, are begin/end balanced?

### Edge Cases
- Empty/zero/null inputs handled?
- Boundary conditions covered?
- Error paths return meaningful results?

### Naming Conventions
- Runtime functions: `sans_` prefix
- IR constants: `IR_` prefix
- Test fixtures: `<feature>_<scenario>.sans`

## Stage 4: Build and Test

Run these commands and report results:

```bash
# If compiler files were modified:
sans build compiler/main.sans

# Compile new fixtures individually:
sans build tests/fixtures/<new_fixture>.sans

# Run the full test suite:
bash tests/run_tests.sh
```

Report: pass/fail counts, any regressions, any skipped tests.

## Stage 5: Verdict

Produce this structured output:

```markdown
## PR Review: #<number>

### Checklist
| Item | Status | Notes |
|------|--------|-------|
| Docs: reference.md | PASS/FAIL/N/A | |
| Docs: ai-reference.md | PASS/FAIL/N/A | |
| Docs: website | PASS/FAIL/N/A | |
| Docs: HOVER_DATA | PASS/FAIL/N/A | |
| Docs: syntax highlighting | PASS/FAIL/N/A | |
| Test fixture | PASS/FAIL | |
| Examples | PASS/FAIL/N/A | |
| README | PASS/FAIL/N/A | |
| No version changes | PASS/FAIL | |
| No binaries | PASS/FAIL | |
| AI-optimized syntax | PASS/FAIL | |

### Code Review
**Blocking:**
- [issues that must be fixed]

**Suggestions:**
- [improvements to consider]

**Nits:**
- [minor style/preference items]

### Build & Test
- Compiler build: PASS/FAIL
- Test suite: X/Y passed (Z skipped, W failed)
- New fixture: PASS/FAIL (exit code: expected N, got M)

### Verdict: APPROVE / REQUEST CHANGES / COMMENT
[1-2 sentence summary]
```
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/sans-review-pr.md
git commit -m "feat: add sans-review-pr skill for full PR review"
```

### Task 6: Create sans-test skill

**Files:**
- Create: `.claude/skills/sans-test.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: sans-test
type: skill
description: Write comprehensive Sans test fixtures — happy path, edge cases, and boundary conditions — then run the full suite and report results
---

# Sans Test Writing

You are writing comprehensive test fixtures for a Sans feature or fix. Tests are E2E — each fixture is a `.sans` file that is compiled, run, and its exit code checked.

## How Sans Tests Work

Each test fixture is a `.sans` file in `tests/fixtures/`. The test runner (`tests/run_tests.sh`) compiles it with `sans build`, runs the binary, and checks the exit code against an expected value.

```bash
# In run_tests.sh:
run_test "feature_name"  "$REPO_ROOT/tests/fixtures/feature_name.sans"  42
#        ^label           ^fixture path                                  ^expected exit code
```

**Exit code conventions:**
- Use the computed result as the exit code (e.g., a test for `abs(-42)` exits with `42`)
- Use `0` only for tests verifying side effects (printing, file I/O, logging)
- Exit codes must be 0-255 (unsigned byte)
- For tests that check multiple things, sum or combine results into the exit code

**File naming:** `<feature>_<scenario>.sans` (e.g., `array_sort_empty.sans`, `map_filter.sans`)

**Temp files:** If a fixture writes to disk, use a unique path: `/tmp/sans_test_<fixture_name>_<unique>`

## Step 1: Analyze the Change

Read the diff or description of what was added/modified. Identify:
- What feature or fix was implemented?
- What types/functions/methods are involved?
- What are the inputs and outputs?

## Step 2: Check Existing Coverage

Search `tests/fixtures/` and `tests/run_tests.sh` for existing tests covering this feature. Note gaps.

## Step 3: Write the Happy-Path Fixture

Write a test that exercises the primary use case. Use AI-optimized syntax:
- `I/S/B/F` for types, bare assignment, no commas in arrays
- `r!` for unwrap, `?` for try operator
- Expression bodies for simple functions

Example fixture structure:
```
// tests/fixtures/feature_basic.sans
main() = {
  result := 0
  // ... test the feature ...
  result += expected_value
  exit(result)
}
```

## Step 4: Identify Edge Cases

Systematically check each category:

### Empty/Zero/Null
- Empty string `""`
- Empty array `[]`
- Zero `0`
- What happens when the feature is called with "nothing"?

### Boundaries
- Single element arrays
- First/last element access
- Max values (large integers near i64 limits)
- Very long strings

### Error Paths
- Invalid input types (if the feature should reject them)
- Operations that should return `Result` — test both `ok` and `err` paths
- Division by zero, index out of bounds

### Type Interactions
- Does it work with `I`, `S`, `B`, `F`?
- Does it work with arrays of different types?
- Does it work with nested containers?

### Scope GC
- Call the feature in a function that returns — does memory get freed?
- Call it in a loop — does memory accumulate?
- Store the result in a container — is it promoted correctly?

### Cross-Feature
- Does it work inside a lambda?
- Does it work across module imports?
- Does it interact correctly with other features?

## Step 5: Write Edge Case Fixtures

One fixture per edge case (or group closely related edge cases). Keep each focused.

## Step 6: Register in run_tests.sh

Add entries to `tests/run_tests.sh` in the appropriate section (single-file or directory-based):

```bash
run_test "feature_basic"        "$REPO_ROOT/tests/fixtures/feature_basic.sans"        42
run_test "feature_empty"        "$REPO_ROOT/tests/fixtures/feature_empty.sans"         0
run_test "feature_boundary"     "$REPO_ROOT/tests/fixtures/feature_boundary.sans"      99
```

## Step 7: Build and Run

```bash
# Compile each new fixture:
sans build tests/fixtures/feature_basic.sans

# Run the full suite:
bash tests/run_tests.sh

# Verify: 0 failures, new tests show as passing
```

## Step 8: Report

```markdown
## Test Report: [Feature Name]

### Fixtures Written
| Fixture | Tests | Expected Exit |
|---------|-------|---------------|
| feature_basic.sans | happy path | 42 |
| feature_empty.sans | empty input | 0 |

### Edge Cases Covered
- [x] Empty input
- [x] Boundary values
- [ ] Error paths (not applicable — feature doesn't fail)

### Suite Results
- Total: X tests
- Passed: Y
- Failed: Z
- Skipped: W

### Concerns
- [any issues, gaps, or things to watch]
```
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/sans-test.md
git commit -m "feat: add sans-test skill for comprehensive test writing"
```

---

## Chunk 3: Standalone Workflow Docs

### Task 7: Create planning workflow doc

**Files:**
- Create: `docs/workflows/planning.md`

- [ ] **Step 1: Write the workflow doc**

```markdown
# Planning a Sans Feature

Step-by-step guide for planning a feature, fix, or change to the Sans compiler.

## 1. Identify the Feature Type

Determine which pipeline your change follows:

### New Built-in Function (e.g., `abs`, `strlen`)
| Step | File | What to do |
|------|------|------------|
| 1 | `compiler/typeck.sans` | Add type signature in builtin call dispatch |
| 2 | `compiler/constants.sans` | Add IR instruction constant (`IR_NAME`) |
| 3 | `compiler/ir.sans` | Lower the call to the IR instruction |
| 4 | `compiler/codegen.sans` | Emit LLVM IR for the instruction |
| 5 | `runtime/*.sans` | Add `sans_` runtime function (if needed) |
| 6 | `tests/fixtures/` | Add E2E test fixture |
| 7 | All docs | See Documentation Checklist below |

### New Method on a Type (e.g., `arr.reverse()`)
Same as above, but use method dispatch in typeck and IR lowering.

### New Type (e.g., `Set`, `Queue`)
| Step | File | What to do |
|------|------|------------|
| 1 | `compiler/typeck.sans` | Add type resolution |
| 2 | `compiler/ir.sans` | Add IR type handling |
| 3 | `compiler/codegen.sans` | Add codegen support |
| 4 | `runtime/*.sans` | Add runtime backing with `sans_` prefix (if opaque) |
| 5 | `tests/fixtures/` | Add E2E test fixture |
| 6 | All docs | See Documentation Checklist below |

## 2. Check Conventions

Before writing code, verify your design follows:
- **i64 register model** — all values stored as i64 in IR/codegen
- **Pointer dual storage** — pointers in both `regs` (i64) and `ptrs` (PointerValue)
- **Opaque types** — runtime-backed types use `sans_` prefix functions
- **Scope GC** — allocations tracked per-function, freed on return, return values promoted
- **AI-optimized syntax** — use the shortest form possible (`I/S/B/F`, bare assignment, no commas)

## 3. Documentation Checklist

Copy this into your plan — every feature must update ALL items:

```
- [ ] docs/reference.md — human-readable reference
- [ ] docs/ai-reference.md — compact AI reference
- [ ] website/docs/index.html — website documentation
- [ ] editors/vscode-sans/src/extension.ts — HOVER_DATA entry
- [ ] editors/vscode-sans/syntaxes/sans.tmLanguage.json — syntax highlighting
- [ ] tests/fixtures/ — E2E test fixture
- [ ] examples/ — update or add example (if significant)
- [ ] README.md — update feature list (if user-facing)
```

## 4. Scope GC Considerations

For every feature that allocates memory, answer:
- Is the allocation tracked by scope GC?
- Does the return value get promoted to the caller's scope?
- Are nested containers (array of arrays, map of strings) recursively promoted?
- Should this use arenas for hot paths?

## 5. Write Your Plan

Structure your plan as a series of tasks, each with:
- Exact file paths to create or modify
- The specific changes needed
- Test fixtures with expected exit codes
- Documentation updates

**Next step:** [Architecture Review](architecture-review.md)
```

- [ ] **Step 2: Commit**

```bash
git add docs/workflows/planning.md
git commit -m "docs: add planning workflow guide"
```

### Task 8: Create architecture-review workflow doc

**Files:**
- Create: `docs/workflows/architecture-review.md`

- [ ] **Step 1: Write the workflow doc**

```markdown
# Architecture Review for Sans

Step-by-step guide for evaluating a design against Sans compiler architecture.

## Sans Architecture Quick Reference

**Pipeline:** Source → Lexer → Parser → Typeck → IR → Codegen → LLVM IR → Binary

| File | Purpose |
|------|---------|
| `compiler/lexer.sans` | Tokenizes source |
| `compiler/parser.sans` | Tokens → AST |
| `compiler/typeck.sans` | Type-checks AST, resolves types |
| `compiler/constants.sans` | IR opcodes, type/statement tags |
| `compiler/ir.sans` | AST → flat IR |
| `compiler/codegen.sans` | IR → LLVM IR text |
| `compiler/main.sans` | CLI, orchestration, linking |

**Runtime:** 13 `.sans` modules in `runtime/` — fully self-hosted, zero C code.

**Memory:** Scope-based GC — allocations freed on function return, return values promoted to caller.

## Review Checklist

For every proposed design, work through each section:

### Does It Fit the Register Model?
- All values must be storable as i64
- Pointers go in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue)
- New types need a type tag in `constants.sans`

### IR Design Decision Tree
```
Is this a compile-time operation?
├── Yes → Handle in typeck/codegen only (no IR instruction needed)
└── No → Runtime operation
    ├── Simple (1-2 LLVM instructions) → Inline in codegen
    └── Complex → Add sans_ runtime function
        ├── Add IR instruction constant to constants.sans
        ├── Add lowering in ir.sans
        └── Add codegen emission in codegen.sans
```

### Scope GC Interaction
- Does it allocate heap memory? → Must be tracked
- Does it return allocated memory? → Must promote to caller scope
- Nested containers? → Recursive promotion needed
- Hot path? → Consider arena support

### AI-Optimized Syntax Check
- Can the syntax use fewer tokens?
- Short type names: `I/S/B/F/R<T>`
- Bare assignment: `x = 42` not `let x = 42`
- Expression bodies: `f(x:I) = x*2`
- No commas: `[1 2 3]`

### Conflict Check
- Does it clash with existing builtins or methods?
- Does it introduce parser ambiguity?
- Does it break backward compatibility?

### Thread Safety
- Does it touch global state (`rc_alloc_head`, `rc_scope_head`)?
- Thread safety is a known limitation — flag concerns as "worth discussing", not blocking

## Output Template

```
### Architecture Review: [Feature Name]

**Verdict:** APPROVED / NEEDS CHANGES / BLOCKED

**Findings:**
- Blocking: [must fix]
- Worth discussing: [design questions]
- Minor nits: [suggestions]

**Recommendations:** [specific improvements]
```

**Next step:** [Skeptic Review](skeptic-review.md) (challenge the design before implementing)
```

- [ ] **Step 2: Commit**

```bash
git add docs/workflows/architecture-review.md
git commit -m "docs: add architecture-review workflow guide"
```

### Task 9: Create skeptic-review workflow doc

**Files:**
- Create: `docs/workflows/skeptic-review.md`

- [ ] **Step 1: Write the workflow doc**

```markdown
# Skeptic Review for Sans

Challenge designs before implementation and critique code after. The goal is to prevent unnecessary complexity, catch missing edge cases, and enforce Sans conventions.

## When to Use

- **Before implementation:** After a design/spec is written, before code starts
- **After implementation:** After code is written, before PR submission

## Pre-Implementation Checklist

### Necessity
- [ ] Does this solve a real, current problem?
- [ ] Could an existing feature handle this with minor changes?
- [ ] What happens if we don't build this?

### Edge Cases
- [ ] Empty input (empty string, empty array, zero)
- [ ] Huge input (max i64, very long strings, large arrays)
- [ ] Invalid input (wrong types, malformed data)
- [ ] Boundary conditions (first/last element, single element)

### Scope GC
- [ ] If it allocates, is memory tracked?
- [ ] In a loop, does memory accumulate?
- [ ] Do return values promote correctly across scope boundaries?
- [ ] Nested container interaction?

### Concurrency
- [ ] What if called from a spawned thread?
- [ ] Does it touch global state?
- (Thread safety is a known limitation — flag as "worth discussing" unless the feature specifically claims thread-safety)

### Syntax
- [ ] Is this the shortest possible syntax?
- [ ] Does it follow existing patterns?
- [ ] Will AI code generators produce it correctly?

### Complexity
- [ ] Is this over-engineered?
- [ ] Are we designing for hypothetical future needs?
- [ ] How many compiler files does this touch — proportional to value?

## Post-Implementation Checklist

### Over-engineering
- [ ] Abstractions used only once?
- [ ] Helpers that could be inlined?
- [ ] Error handling for impossible scenarios?
- [ ] Configuration nobody asked for?

### Scope Creep
- [ ] Changes beyond what the spec called for?
- [ ] "While I'm here" improvements?
- [ ] Excessive test coverage?

### Convention Violations
- [ ] AI-optimized syntax in all new code and examples?
- [ ] `sans_` prefix on runtime functions?
- [ ] `IR_` prefix on constants?
- [ ] Unique `/tmp/` filenames in fixtures?
- [ ] All 8 documentation targets updated?

### Consistency
- [ ] Follows existing codebase patterns?
- [ ] Pipeline complete (typeck → constants → IR → codegen)?
- [ ] No dangling references?

## Severity Guide

| Severity | Meaning | Action |
|----------|---------|--------|
| **Blocking** | Will cause bugs, breaks conventions, missing required work | Must fix before proceeding |
| **Worth discussing** | Design trade-off, potential issue, debatable choice | Discuss and decide |
| **Minor nit** | Style preference, minor improvement | Take it or leave it |

## Output Template

```
## Skeptic Review: [Feature Name]

### Mode: Pre-implementation / Post-implementation

### Blocking
1. [issue] — [why]

### Worth Discussing
1. [issue] — [trade-off]

### Minor Nits
1. [suggestion]

### Summary
[1-2 sentence assessment]
```

**Next step (pre-impl):** Address blocking issues, then proceed to implementation
**Next step (post-impl):** [Testing](testing.md)
```

- [ ] **Step 2: Commit**

```bash
git add docs/workflows/skeptic-review.md
git commit -m "docs: add skeptic-review workflow guide"
```

### Task 10: Create pr-review workflow doc

**Files:**
- Create: `docs/workflows/pr-review.md`

- [ ] **Step 1: Write the workflow doc**

```markdown
# PR Review for Sans

Complete guide for reviewing a pull request to the Sans compiler.

## Step 1: Get the Diff

```bash
# From a PR number:
gh pr diff <number>

# From a branch:
git diff main...<branch>
```

Read the full diff. Identify all changed files.

## Step 2: Checklist Compliance

### Documentation (8 targets)
If the PR adds a feature, builtin, method, type, or syntax change:

| Target | File | Updated? |
|--------|------|----------|
| Reference docs | `docs/reference.md` | |
| AI reference | `docs/ai-reference.md` | |
| Website docs | `website/docs/index.html` | |
| Hover docs | `editors/vscode-sans/src/extension.ts` (HOVER_DATA) | |
| Syntax highlighting | `editors/vscode-sans/syntaxes/sans.tmLanguage.json` | |
| Test fixture | `tests/fixtures/*.sans` | |
| Examples | `examples/` | |
| README | `README.md` | |

Bug fixes and refactors may not need all 8 — use judgment.

### Version Guard
- No changes to `editors/vscode-sans/package.json` version field
- No changes to `website/*.html` version meta tags
- No changes to `CLAUDE.md` version line
- No changes to `compiler/main.sans` version string

### No Binaries
- No `.o` files, executables, or Mach-O binaries in the diff

### AI-Optimized Syntax
- Short type names: `I/S/B/F/R<T>`
- Bare assignment, expression bodies, no commas in arrays
- `r!` for unwrap, `?` for try

### Test Fixtures
- Fixture added and registered in `tests/run_tests.sh`
- Descriptive name: `<feature>_<scenario>.sans`
- Unique `/tmp/` filenames if writing to disk

## Step 3: Code Review

### Compiler Pipeline Consistency
If compiler files are modified:
- `typeck.sans` adds type/builtin → `constants.sans` has IR constant?
- `constants.sans` adds constant → `ir.sans` lowers it?
- `ir.sans` lowers instruction → `codegen.sans` emits LLVM IR?
- `codegen.sans` calls runtime → `runtime/*.sans` defines it with `sans_` prefix?

### Runtime Safety
- Allocations tracked by scope GC?
- Return values promoted to caller scope?
- Nested containers recursively promoted?
- Arena begin/end balanced?

### Naming Conventions
- Runtime functions: `sans_` prefix
- IR constants: `IR_` prefix
- Test fixtures: `<feature>_<scenario>.sans`

## Step 4: Build and Test

```bash
# If compiler files were modified:
sans build compiler/main.sans

# Compile new fixtures individually:
sans build tests/fixtures/<new_fixture>.sans

# Full test suite:
bash tests/run_tests.sh
```

## Step 5: Write the Verdict

```
## PR Review: #<number>

### Checklist
| Item | Status | Notes |
|------|--------|-------|
| Docs: reference.md | PASS/FAIL/N/A | |
| Docs: ai-reference.md | PASS/FAIL/N/A | |
| Docs: website | PASS/FAIL/N/A | |
| Docs: HOVER_DATA | PASS/FAIL/N/A | |
| Docs: syntax highlighting | PASS/FAIL/N/A | |
| Test fixture | PASS/FAIL | |
| Examples | PASS/FAIL/N/A | |
| README | PASS/FAIL/N/A | |
| No version changes | PASS/FAIL | |
| No binaries | PASS/FAIL | |
| AI-optimized syntax | PASS/FAIL | |

### Code Review
- Blocking: [must fix]
- Suggestions: [should consider]
- Nits: [optional]

### Build & Test
- Compiler build: PASS/FAIL
- Test suite: X/Y passed (Z skipped, W failed)

### Verdict: APPROVE / REQUEST CHANGES / COMMENT
```

**Next step:** Post the review on the PR via `gh pr review <number>`
```

- [ ] **Step 2: Commit**

```bash
git add docs/workflows/pr-review.md
git commit -m "docs: add PR review workflow guide"
```

### Task 11: Create testing workflow doc

**Files:**
- Create: `docs/workflows/testing.md`

- [ ] **Step 1: Write the workflow doc**

```markdown
# Writing Tests for Sans

Guide for writing comprehensive E2E test fixtures for the Sans compiler.

## How Tests Work

Each test is a `.sans` file in `tests/fixtures/`. The test runner compiles it, runs the binary, and checks the exit code.

```bash
# In tests/run_tests.sh:
run_test "label"  "$REPO_ROOT/tests/fixtures/label.sans"  42
#        ^name    ^path                                     ^expected exit code
```

**Build and run:**
```bash
# Compile a fixture:
sans build tests/fixtures/my_test.sans

# Run full suite:
bash tests/run_tests.sh
```

## Exit Code Conventions

- Use the **computed result** as the exit code (e.g., `abs(-42)` → exit `42`)
- Use `0` only for side-effect tests (printing, file I/O, logging)
- Exit codes are 0-255 (unsigned byte)
- For multi-check tests, sum or combine results

**Example:**
```
// tests/fixtures/array_basic.sans
main() = {
  a = [10 8 10]
  exit(a[0] + a[1] + a[2])  // exits 28
}
```

## Naming Convention

`<feature>_<scenario>.sans`

Examples:
- `array_sort.sans` — basic sort test
- `array_sort_empty.sans` — sort an empty array
- `map_filter.sans` — filter a map
- `lambda_capture_nested.sans` — nested lambda captures

## Temp Files

If a fixture writes to disk, use a unique path:
```
"/tmp/sans_test_<fixture_name>"
```
This prevents races when tests run in parallel.

## Edge Case Checklist

For every feature, write tests covering:

### Empty/Zero/Null
- [ ] Empty string `""`
- [ ] Empty array `[]`
- [ ] Zero `0`
- [ ] Feature called with minimal input

### Boundaries
- [ ] Single-element array
- [ ] First/last element access
- [ ] Large values

### Error Paths
- [ ] Invalid input (if feature should handle it)
- [ ] Result `ok` and `err` paths
- [ ] Graceful failure scenarios

### Scope GC
- [ ] Feature used inside a function (memory freed on return?)
- [ ] Feature used in a loop (memory accumulates?)
- [ ] Return value stored in container (promoted correctly?)

### Cross-Feature
- [ ] Works inside a lambda?
- [ ] Works across module imports?
- [ ] Interacts with other features correctly?

## Registering Tests

Add to `tests/run_tests.sh` in the appropriate section:

```bash
# Single-file tests (most common):
run_test "feature_basic"    "$REPO_ROOT/tests/fixtures/feature_basic.sans"    42

# Directory-based (multi-module) tests:
run_test "feature_import"   "$REPO_ROOT/tests/fixtures/feature_import/main.sans"   7
```

## Good Test Examples

Look at these existing fixtures for reference:
- `tests/fixtures/array_basic.sans` — simple computed exit code
- `tests/fixtures/file_write_read.sans` — side-effect test with temp file
- `tests/fixtures/try_operator.sans` — Result/error path testing
- `tests/fixtures/scope_basic.sans` — scope GC verification
- `tests/fixtures/import_basic/main.sans` — multi-module test

**Next step:** [PR Review](pr-review.md)
```

- [ ] **Step 2: Commit**

```bash
git add docs/workflows/testing.md
git commit -m "docs: add testing workflow guide"
```

---

## Chunk 4: Update Existing Files

### Task 12: Update AGENTS.md

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Add Workflows section and versioning enforcement note**

Add after the "Known Limitations" section:

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add AGENTS.md
git commit -m "docs: add workflows and versioning enforcement to AGENTS.md"
```

### Task 13: Update CONTRIBUTING.md

**Files:**
- Modify: `CONTRIBUTING.md`

- [ ] **Step 1: Update AI Agent Contributors section**

Replace the existing "AI Agent Contributors" section with:

```markdown
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
```

- [ ] **Step 2: Add standalone Workflows section**

Add a new section after "Pull Request Process" and before "Common Gotchas", visible to all contributors (not just AI agents):

```markdown
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
```

- [ ] **Step 3: Add versioning enforcement note to Common Gotchas**

Add to the Common Gotchas section:

```markdown
- **CI blocks version changes in PRs.** The `version-guard` workflow will fail your PR if it modifies version-managed files. Remove version changes from your diff. Only maintainers can release via git tags.
```

- [ ] **Step 4: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "docs: update CONTRIBUTING.md with new workflows and skills"
```
