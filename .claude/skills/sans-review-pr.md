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
