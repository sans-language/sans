# `sans lint` — Static Analysis Design

**Date:** 2026-04-08
**Status:** Approved
**Goal:** Add a `sans lint` command that catches common code issues beyond what the compiler errors on, without requiring a full build.

---

## 1. Command Interface

```
sans lint foo.sans                        # lint single file
sans lint compiler/                       # lint all .sans files recursively
sans lint .                               # lint current directory
sans lint --error=unused-imports foo.sans  # promote a rule to error severity
sans lint --quiet foo.sans                # suppress warnings, show only errors
```

**Exit codes:**
- `0` — no error-severity diagnostics (warnings are OK)
- `1` — one or more error-severity diagnostics found

**Directory mode:** When given a directory, recursively find all `*.sans` files and lint each. Report diagnostics grouped by file.

---

## 2. Architecture

`sans lint` reuses the existing compiler pipeline up through type checking, then runs a dedicated lint pass. No IR generation, no codegen, no linking.

```
sans build:  lexer -> parser -> typeck (with warnings) -> IR -> codegen -> link
sans lint:   lexer -> parser -> typeck (with warnings) -> lint pass -> report
```

### New module: `compiler/lint.sans`

The lint pass receives the parsed + type-checked AST and the diagnostic list. It runs each enabled rule and appends `DIAG_WARN` or `DIAG_ERROR` diagnostics through the existing diagnostic system (`tc_warn_at`, `tc_error_at`).

### Shared infrastructure

- **Diagnostics:** Reuses the existing `DIAG_ERROR`/`DIAG_WARN` system in typeck.sans with severity rendering, source context, and caret display.
- **LSP integration:** The LSP server can call the lint pass after its analysis to surface lint diagnostics in-editor alongside type errors.
- **Existing warnings stay in `sans build`:** Unused variable and unreachable code warnings (already in typeck) continue to appear during `sans build`. `sans build --quiet` suppresses them.

### CLI wiring in `main.sans`

Add a `lint` subcommand alongside `build`, `fmt`, `test`:
1. Parse CLI args for `lint` subcommand and flags (`--error=<rule>`, `--quiet`)
2. For each target file (single file or recursive directory scan):
   a. Run lexer + parser + typeck (reuse existing `parse` and `check_module`)
   b. Call `lint_check(prog, config)` from `lint.sans`
3. Render all collected diagnostics
4. Exit 1 if any error-severity diagnostics, else exit 0

---

## 3. Lint Rules

Five initial rules:

| Rule ID | Default Severity | Description |
|---|---|---|
| `unused-imports` | warn | Imported module is never referenced in the file |
| `unreachable-code` | warn | Code after a `return` statement (already detected in typeck — shared, not duplicated) |
| `empty-catch` | warn | `?` result is silently discarded without handling the error case |
| `shadowed-vars` | warn | Variable in an inner scope redeclares a name from an outer scope |
| `unnecessary-mut` | warn | Variable declared with `:=` (mutable) but never reassigned |

### Rule implementation notes

**`unused-imports`:** After typeck, walk the AST for all `import` statements. Check whether any function or type from the imported module is referenced. Requires tracking which module each called function belongs to.

**`unreachable-code`:** Already implemented in `typeck.sans` (lines 928, 4130). The lint pass does not duplicate this — it remains in typeck and fires during both `sans build` and `sans lint`.

**`empty-catch`:** Detect patterns where a `?` (try operator) result is used in a statement context but the error path is never handled — e.g., the result of `f()?` is discarded without binding or propagation.

**`shadowed-vars`:** Maintain a scope stack during AST walk. When a variable declaration is encountered, check if the name exists in any outer scope. Emit warning with the location of both the shadow and the original.

**`unnecessary-mut`:** Track all `:=` declarations. After processing the function body, check which mutable variables were never the target of a reassignment (`=`, `+=`, `-=`, etc.). Suggest using `=` instead.

---

## 4. Severity Configuration

### CLI flags

```
--error=<rule-id>    # promote rule to error severity
--quiet              # suppress warnings, show only errors
```

Multiple `--error` flags allowed: `sans lint --error=unused-imports --error=shadowed-vars .`

### Config file

Optional `lint` section in `sans.json` (the existing package manager config):

```json
{
  "lint": {
    "unused-imports": "error",
    "shadowed-vars": "off"
  }
}
```

Valid values: `"error"`, `"warn"`, `"off"`.

### Precedence

CLI flags override `sans.json`. Absence of configuration uses the default severity from the rule table above.

---

## 5. Integration with `sans build`

- `sans build` continues to show existing typeck warnings (unused vars, unreachable code) by default.
- `sans build --quiet` suppresses all warnings, showing only errors.
- Lint-only rules (`unused-imports`, `empty-catch`, `shadowed-vars`, `unnecessary-mut`) only run via `sans lint`, not during `sans build`.

---

## 6. Output Format

Standard diagnostic format matching the compiler's existing output:

```
compiler/main.sans:42:3: warning: unused import 'math' [unused-imports]
  42 |   import "math"
        ^
compiler/main.sans:87:5: warning: variable 'x' shadows outer declaration at line 12 [shadowed-vars]
  87 |     x := 0
          ^
```

Each diagnostic includes the rule ID in brackets for easy filtering and configuration.

---

## 7. Success Criteria

- [ ] `sans lint foo.sans` runs parse + typeck + lint without generating a build artifact
- [ ] `sans lint .` recursively lints all `.sans` files in a directory
- [ ] All 5 lint rules implemented and producing correct diagnostics
- [ ] Zero false positives on the existing compiler source and test fixtures
- [ ] `--error=<rule>` promotes a rule to error severity with exit code 1
- [ ] `sans.json` lint configuration is respected, CLI flags override it
- [ ] `sans build --quiet` suppresses warnings
- [ ] LSP can invoke the lint pass for in-editor diagnostics
- [ ] Rule ID shown in diagnostic output (e.g., `[unused-imports]`)
