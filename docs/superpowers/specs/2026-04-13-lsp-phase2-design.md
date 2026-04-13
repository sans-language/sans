# LSP Phase 2 — Advanced IDE Features

**Date:** 2026-04-13
**Status:** Approved
**Scope:** Local variable completion, struct field/method completion on `.`, cross-file find references, expression type hover, scope-aware rename.

---

## 1. Core Data Structure: Scope-Aware Symbol Table

Extend `compiler/typeck.sans` to emit a rich symbol table during `collect_mode` (already used by the LSP). Three new collections populated during type checking:

### ScopeEntry (40 bytes)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| 0 | scope_id | I | Unique scope identifier |
| 8 | parent_scope_id | I | Parent scope (-1 for top-level) |
| 16 | start_line | I | Opening brace line |
| 24 | end_line | I | Closing brace line |
| 32 | depth | I | Nesting depth |

### BindingEntry (48 bytes)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| 0 | name | S | Variable name |
| 8 | type | I | Type pointer from typeck |
| 16 | line | I | Declaration line |
| 24 | col | I | Declaration column |
| 32 | scope_id | I | Enclosing scope |
| 40 | kind | I | 0=local, 1=param, 2=global |

### ExprTypeEntry (24 bytes)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| 0 | line | I | Expression line |
| 8 | col | I | Expression column |
| 16 | type | I | Inferred type pointer |

### Globals

```
g tc_scopes = 0         // array of ScopeEntry
g tc_bindings = 0       // array of BindingEntry
g tc_expr_types = 0     // array of ExprTypeEntry
g tc_scope_counter = 0  // incrementing scope ID
g tc_current_scope = 0  // current scope during walk
```

Initialized in `check_inner()` when `collect_mode = 1`. Normal compilation unaffected.

---

## 2. Compiler Hook Points (typeck.sans)

### Scope Push

At every point typeck enters a new scope (function body, if/else block, while body, for body, match arm):
- Allocate ScopeEntry with `scope_id = tc_scope_counter++`
- Set `parent_scope_id = tc_current_scope`
- Record `start_line` from the AST node
- Push to `tc_scopes`
- Set `tc_current_scope = new scope_id`

### Scope Pop

At every scope exit:
- Record `end_line` on the current ScopeEntry
- Restore `tc_current_scope = parent_scope_id`

### Variable Binding

At `let` statements and function parameter processing (where typeck does `mset(locals, name, type)`):
- Allocate BindingEntry with name, type, line, col from the AST node
- Set `scope_id = tc_current_scope`
- Set `kind` (0 for locals, 1 for params, 2 for globals)
- Push to `tc_bindings`

### Expression Type

At the end of `check_expr()` (which already returns the type):
- If `collect_mode` is set AND the expression is an identifier or method call:
  - Record (line, col, type) in `tc_expr_types`
- Skip for literals, operators, and other non-hoverable positions to avoid excessive entries

---

## 3. Feature A: Local Variable Completion

**In `completion.sans`:**

When completions are requested at a cursor position:
1. Call `visible_bindings_at(cursor_line)` — walks the scope chain from the innermost scope containing cursor_line, collecting all BindingEntries
2. For each binding, create a completion item: label=name, detail=type_to_string(type), kind=Variable (for locals/params) or kind=Field (for globals)
3. Merge with existing keyword/builtin/symbol completions
4. Deeper-scope bindings shadow outer ones with the same name

**New function in `analyzer.sans`:**

```
visible_bindings_at(line:I) → array of BindingEntry
```

Finds the innermost scope containing `line`, then walks parent chain collecting bindings from each scope. Filters to bindings declared before `line` (can't complete a variable before its declaration).

---

## 4. Feature B: Struct Field / Method Completion on `.`

**In `completion.sans`:**

When the trigger character is `.`:
1. Find the identifier before the dot: scan backward from cursor column to find the start of the identifier token, then extract it with `word_at_position(line, token_start_col)`
2. Look up its type:
   - Check `tc_bindings` for a binding with that name visible at the cursor line
   - OR check `tc_expr_types` for a recorded type at the identifier's position
3. Resolve the type:
   - If struct type → look up fields from `structs_map`, return as completion items
   - If builtin type (Array, String, Map, HttpRequest, etc.) → return from existing `get_method_completions`
   - If type has entries in `methods_map` → return those methods
4. Each completion item includes the field/method type as detail

**Improvement over current:** Currently `get_method_completions` guesses the type from the identifier text. Now it uses the actual inferred type.

---

## 5. Feature C: Cross-File Find References

**In `references.sans`:**

Extend `find_references()` with cross-file search:

1. Find references in the current file (existing token-stream scan)
2. Determine if the symbol is module-level (function, struct, enum) by checking `lsp_last_symbols`
3. If module-level:
   a. Get reverse dependencies from `mcache_rdeps` — files that import the current module
   b. For each importing file: parse (or use cached parse) and scan token stream for the identifier
   c. For project-wide search: discover all `.sans` files under workspace root, parse lazily, scan
   d. Cache parsed token streams in a new `g lsp_token_cache` map (file_path → token_array)
4. Invalidate cached token streams on `didChange` / `didSave` (hook into existing cache invalidation)
5. Return combined locations from all files

**New utility:**

```
find_sans_files(root:S) → array of file paths
```

Recursively lists `*.sans` files under the workspace root (using `listdir` + `is_dir`).

---

## 6. Feature D: Expression Type Hover

**In `analyzer.sans`:**

Extend `lookup_hover_at()`:

1. Check `lsp_last_expr_types` for an entry matching (line, col) — if found, return `type_to_string(type)`
2. Check `lsp_last_bindings` for a binding matching the identifier at (line, col) — if found, return its type + kind (local/param/global)
3. Fall back to existing hover logic (function/struct/enum signatures from `lsp_last_symbols`)

**Hover format examples:**
- Local variable: `(local) x: Int`
- Parameter: `(param) name: String`
- Function: `fn add(a:I b:I) I` (existing)
- Struct field on hover: `(field) x: Int` (when hovering a field access)

---

## 7. Feature E: Scope-Aware Rename

**In `references.sans`:**

Replace `compute_rename()` with scope-aware version:

1. Find the binding at cursor via `find_binding_at(line, col)` — looks up `tc_bindings` for a binding declared at this position, OR the closest binding visible at this position with matching name
2. Determine the binding's scope from `tc_scopes` (get scope_id → start_line, end_line)
3. **For local/param bindings (kind 0, 1):**
   - Scan token stream only within the scope's line range
   - For each occurrence, verify it refers to THIS binding (not a deeper shadow): check that no BindingEntry with the same name exists in a child scope that contains the occurrence
   - Collect as rename edits
4. **For module-level symbols (function, struct, enum):**
   - Rename in current file (all occurrences outside shadowing scopes)
   - Use cross-file reference index to rename in importing files
   - Return WorkspaceEdit with changes across multiple files

**Shadow detection:**

For each candidate occurrence at position P:
- Find the innermost scope containing P
- Walk bindings in that scope and ancestors up to (but not including) the original binding's scope
- If any intermediate scope has a binding with the same name → this occurrence is shadowed, skip it

---

## 8. LSP Module Changes Summary

### analyzer.sans
- New globals: `lsp_last_scopes`, `lsp_last_bindings`, `lsp_last_expr_types`
- New functions: `find_scope_at(line)`, `find_binding_at(line, col)`, `find_expr_type_at(line, col)`, `visible_bindings_at(line)`
- Extend `analyze_file()` to read `tc_scopes`, `tc_bindings`, `tc_expr_types` after type checking
- Extend `lookup_hover_at()` for expression type hover

### completion.sans
- Extend `get_completions()` to include local variable completions
- New dot-completion path: detect `.` trigger, resolve receiver type, return fields/methods

### references.sans
- Extend `find_references()` with cross-file search via module cache + lazy file discovery
- Replace `compute_rename()` with scope-aware version
- New: `lsp_token_cache` for caching parsed token streams across files

### main.sans
- Update `handle_hover` to use new expression type lookup
- Update `handle_completion` to pass trigger character
- Update `handle_references` for cross-file mode
- Update `handle_rename` for scope-aware logic

---

## 9. Testing

### Test Fixtures

`tests/fixtures/test_scope_collect.sans` — exercises scoping (nested blocks, shadowing, params). Verifies compiler doesn't crash with collect_mode.

### LSP Tests (in `tests/lsp/`)

1. **test_local_completion.sh** — completions inside function body include local variables with types
2. **test_dot_completion.sh** — `.` after struct instance shows fields
3. **test_cross_file_refs.sh** — find references spans importing files
4. **test_expr_hover.sh** — hover on variable shows inferred type
5. **test_scope_rename.sh** — rename inner shadowed variable doesn't affect outer

Each test: spawn LSP, send JSON-RPC, verify response with grep. Follows existing test pattern.

---

## 10. Files Modified

### Compiler
- `compiler/typeck.sans` — add scope/binding/expr_type collection hooks (gated on collect_mode)

### LSP
- `lsp/analyzer.sans` — new scope/binding lookup functions, extend analyze_file and hover
- `lsp/completion.sans` — local variable completion, dot completion with type resolution
- `lsp/references.sans` — cross-file search, scope-aware rename
- `lsp/main.sans` — update handlers to use new features

### Tests
- `tests/fixtures/test_scope_collect.sans` — scope collection test
- `tests/lsp/test_local_completion.sh`
- `tests/lsp/test_dot_completion.sh`
- `tests/lsp/test_cross_file_refs.sh`
- `tests/lsp/test_expr_hover.sh`
- `tests/lsp/test_scope_rename.sh`

### Docs
- `docs/reference.md` — update LSP features section
- `docs/ai-reference.md` — update LSP section
- `editors/vscode-sans/src/extension.ts` — no changes needed (client already supports these LSP features)
