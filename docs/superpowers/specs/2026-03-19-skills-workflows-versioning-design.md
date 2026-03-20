# Skills, Workflows & Versioning Enforcement — Design Spec

**Date:** 2026-03-19
**Status:** Draft

## Goal

Prepare the Sans project for multi-developer collaboration by:
1. Adding CI enforcement so only the organization can manage versioning
2. Creating 5 Claude Code skills for structured development workflows
3. Creating 5 standalone workflow docs for non-Claude-Code contributors
4. Updating AGENTS.md and CONTRIBUTING.md to reference the new workflows

## 1. CI Versioning Enforcement

### New workflow: `.github/workflows/version-guard.yml`

**Trigger:** `pull_request` targeting `main`

**Logic:** Check if the PR diff modifies any version-managed file. If so, fail with a clear message.

**Version-managed files (checked via `git diff`):**
- `editors/vscode-sans/package.json`
- `website/index.html`
- `website/docs/index.html`
- `website/benchmarks/index.html`
- `website/download/index.html`
- `CLAUDE.md` (only the "Current version" line)
- `compiler/main.sans` (only the version string)

**Failure message:**
> "Version files are managed by CI on tag push. Please remove version changes from your PR. See CLAUDE.md#versioning."

**Branch protection (manual GitHub setting):**
- Enable tag protection rules: only org members can push `v*` tags
- This is documented but not automated — requires a one-time GitHub settings change

### Pattern-level filtering

CLAUDE.md, compiler/main.sans, and the website HTML files have legitimate non-version edits (docs, rules, content). The guard must use pattern-level filtering to avoid blocking valid PRs.

**Detection patterns (applied to `git diff -U0` output):**
- `CLAUDE.md`: flag if diff contains a line matching `\*\*Current version:`
- `compiler/main.sans`: flag if diff contains a change to the line with the version string (grep for `"0\.[0-9]+\.[0-9]+"` pattern changes)
- `editors/vscode-sans/package.json`: flag if diff contains a change to `"version":` field
- `website/*.html`: flag if diff contains a change to `<meta` tags with `content="` followed by a semver pattern

**Bypass label:** PRs with the `skip-version-guard` label bypass the check. This allows maintainers to legitimately edit version-adjacent content when needed. Only org members can apply this label.

## 2. Claude Code Skills

All skills live in `.claude/skills/` and are checked into the repo.

Each skill file has YAML frontmatter with `name`, `description`, and `type` fields for discoverability in Claude Code's skill listing.

### Skill chaining mechanism

Skills that "chain to" a superpowers skill do so via a prompt instruction at the end of the skill file: e.g., "Now invoke the `superpowers:writing-plans` skill, passing the context gathered above." This tells Claude Code to use the Skill tool to load the next skill. Skills marked "Standalone" have no such instruction and complete independently.

### 2a. `sans-plan.md`

**Purpose:** Inject Sans-specific context into the planning process.

**Trigger:** Starting any feature or change that touches the compiler pipeline.

**Behavior:**
1. Identify which compiler stages are affected (typeck, constants, IR, codegen, runtime)
2. Map the change across all affected files
3. Pre-populate the plan with the 8-point doc checklist
4. Identify scope GC implications
5. Check if AI-optimized syntax rules apply (new syntax must be minimal)
6. Chain to `superpowers:writing-plans` with the accumulated context

**Key context injected:**
- Compiler file map (which file does what)
- Feature addition pipeline (builtin / method / type)
- Doc checklist (all 8 targets)
- AI-optimized syntax rules
- Scope GC interaction points
- Convention reminders (i64 registers, opaque types, sans_ prefix)

### 2b. `sans-architect.md`

**Purpose:** Evaluate designs against Sans architecture and conventions.

**Trigger:** Design decisions about new types, syntax, compiler internals, runtime modules.

**Behavior:**
1. Review the proposed design
2. Evaluate against Sans conventions:
   - Does it fit the i64 register model?
   - Does it need an opaque type with `sans_` runtime backing?
   - How does scope GC interact? Will allocations be tracked/freed correctly?
   - Can the syntax be expressed in fewer tokens?
   - Does it conflict with existing builtins or methods?
3. Flag conflicts and suggest alternatives
4. Chains to `superpowers:brainstorming` with Sans architecture knowledge

**Key questions the architect asks:**
- "Does this need a new IR instruction or can it reuse an existing one?"
- "Is this a compile-time or runtime operation?"
- "What happens when this value crosses a function boundary (scope GC)?"
- "Can this be expressed with fewer tokens?"

### 2c. `sans-skeptic.md`

**Purpose:** Devil's advocate — challenges designs before implementation, critiques code after.

**Trigger:** Invoked manually or by other skills at review points.

**Pre-implementation mode:**
1. Read the spec/design document
2. Challenge assumptions:
   - "Do we actually need this?"
   - "What happens when the input is empty/null/huge?"
   - "How does this interact with scope GC?"
   - "What if this is called from a spawned thread?" (flag as "worth discussing" unless the PR specifically claims thread-safety — thread safety is a known limitation)
   - "Is this over-engineering for the current use case?"
3. Check for missing edge cases
4. Question syntax choices against the AI-optimized mandate
5. Produce structured critique with severity: **blocking** / **worth discussing** / **minor nit**

**Post-implementation mode:**
1. Read the diff/changed files
2. Check for:
   - Over-engineering (abstractions that aren't needed yet)
   - Scope creep (changes beyond what was specified)
   - Deviation from AI-optimized syntax
   - Unnecessary complexity
   - Missing error handling at system boundaries
   - Inconsistency with existing patterns
3. Produce structured critique with same severity levels

**Standalone:** No superpowers dependency. Works independently.

### 2d. `sans-review-pr.md`

**Purpose:** Full PR review — checklist, code, build, test.

**Trigger:** `sans:review-pr <PR-number-or-branch>`

**Behavior (sequential stages):**

**Stage 1 — Checklist compliance:**
- Fetch PR diff via `gh pr diff`
- Check: all 8 doc locations updated?
- Check: no version-managed files modified?
- Check: no compiled binaries in the diff?
- Check: AI-optimized syntax used in new code?
- Check: test fixture added and registered in run_tests.sh?

**Stage 2 — Code review:**
- Review compiler pipeline consistency (if typeck adds a type, does IR handle it? does codegen emit it?)
- Review runtime safety (memory management, scope GC interaction)
- Review edge cases and error paths
- Review naming conventions (sans_ prefix for runtime, IR_ prefix for constants)
- Check for thread-safety issues if concurrency is involved

**Stage 3 — Build and test:**
- If the PR modifies compiler files: `sans build compiler/main.sans` — does the compiler still build?
- If the PR adds/modifies runtime or test fixtures: compile each new fixture individually (`sans build tests/fixtures/new_fixture.sans`) and verify it runs with the expected exit code
- `bash tests/run_tests.sh` — full suite passes, no regressions?

**Stage 4 — Verdict:**
Produce structured output:
```
## PR Review: #<number>

### Checklist: PASS/FAIL
- [x] or [ ] for each item

### Code Review
- Blocking issues (must fix)
- Suggestions (should consider)
- Nits (optional)

### Build & Test: PASS/FAIL
- Build output
- Test results (pass/fail/skip counts)

### Verdict: APPROVE / REQUEST CHANGES / COMMENT
```

**Standalone:** No superpowers dependency.

### 2e. `sans-test.md`

**Purpose:** Write comprehensive test fixtures, run the suite, identify missing coverage.

**Trigger:** After implementing a feature or fix.

**Behavior:**

1. **Analyze the change:** Read the diff to understand what was added/modified
2. **Check existing coverage:** Are there already fixtures for this feature?
3. **Write primary fixture:** Happy-path test with correct exit code. Exit codes are per-fixture — each test defines its own expected value (typically the computed result, e.g., `exit(42)` for a test that should produce 42). Use `0` only for tests that verify side effects (e.g., printing, file I/O). The expected exit code is the third argument in `run_test` in `run_tests.sh`.
4. **Identify edge cases:**
   - Empty/zero/null inputs
   - Boundary values (max int, empty string, empty array)
   - Error paths (what should fail gracefully?)
   - Type mismatches (if applicable)
   - Scope GC interaction (does memory get freed correctly?)
   - Interaction with other features (e.g., does it work inside a lambda? across modules?)
5. **Write edge case fixtures:** One per edge case, with descriptive names
6. **Register in run_tests.sh:** Add all new fixtures with correct expected exit codes
7. **Run full suite:** `bash tests/run_tests.sh` — verify all pass, no regressions
8. **Report:** List what was tested, what edge cases were covered, any concerns

**Naming convention for fixtures:** `<feature>_<scenario>.sans` (e.g., `array_sort_empty.sans`, `lambda_capture_nested.sans`)

**Standalone:** No superpowers dependency.

## 3. Standalone Workflow Docs

For contributors using Cursor, Copilot, Gemini, or working without Claude Code. Located in `docs/workflows/`.

Each doc includes a "Next step" link to the natural next workflow in the sequence: planning → architecture review → implementation → testing → PR review.

### 3a. `planning.md`
Step-by-step guide mirroring `sans:plan`:
- How to identify affected compiler stages
- Feature pipeline templates (builtin / method / type)
- Doc checklist template to copy into your plan
- AI-optimized syntax review checklist

### 3b. `architecture-review.md`
Step-by-step guide mirroring `sans:architect`:
- Sans architecture overview (register model, opaque types, scope GC)
- Questions to ask when evaluating a design
- Common patterns and anti-patterns
- Decision tree: compile-time vs runtime, inline vs runtime function, new IR instruction vs reuse

### 3c. `skeptic-review.md`
Step-by-step guide mirroring `sans:skeptic`:
- Pre-implementation challenge checklist
- Post-implementation critique checklist
- Severity classification guide
- Common Sans-specific pitfalls to watch for

### 3d. `pr-review.md`
Step-by-step guide mirroring `sans:review-pr`:
- Full checklist to verify manually
- Code review focus areas for Sans
- How to build and test locally
- Structured review output template

### 3e. `testing.md`
Step-by-step guide mirroring `sans:test`:
- How to write a fixture (exit code conventions, tmp file naming)
- How to register in run_tests.sh
- Edge case identification checklist
- Examples of good test fixtures

## 4. Updates to Existing Files

### AGENTS.md
Add a "Workflows" section linking to `docs/workflows/` for each workflow. Add a note that Claude Code users should use the corresponding skills instead.

### CONTRIBUTING.md
- Update "AI Agent Contributors" section to reference the new Sans-specific skills (`.claude/skills/`) alongside the existing superpowers references. The superpowers skills (brainstorming, writing-plans, etc.) remain as the general workflow engine; the new Sans skills layer project-specific context on top.
- Add a "Workflows" section pointing to `docs/workflows/`
- Add versioning enforcement note (CI will reject version changes)

## File Manifest

| File | Action |
|------|--------|
| `.github/workflows/version-guard.yml` | Create |
| `.claude/skills/sans-plan.md` | Create |
| `.claude/skills/sans-architect.md` | Create |
| `.claude/skills/sans-skeptic.md` | Create |
| `.claude/skills/sans-review-pr.md` | Create |
| `.claude/skills/sans-test.md` | Create |
| `docs/workflows/planning.md` | Create |
| `docs/workflows/architecture-review.md` | Create |
| `docs/workflows/skeptic-review.md` | Create |
| `docs/workflows/pr-review.md` | Create |
| `docs/workflows/testing.md` | Create |
| `AGENTS.md` | Update |
| `CONTRIBUTING.md` | Update |

**Total: 11 new files, 2 updated files**
