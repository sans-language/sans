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
