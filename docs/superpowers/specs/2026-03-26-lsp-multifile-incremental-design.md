# Sans LSP Multi-file Support & Incremental Parsing Design Spec

**Date:** 2026-03-26
**Version target:** 0.10.0
**Branch:** `lsp-multifile`
**Status:** Approved

---

## Overview

Add multi-file import resolution and module caching to the Sans LSP server. Currently the LSP analyzes each file in isolation (`check_module(program, M())` with empty exports), which means imported symbols are invisible — hover, completion, diagnostics, and go-to-def all fail for cross-file references. This spec adds real import resolution and a module cache so that analysis is both correct across files and fast for the common case (editing one file while imports are stable).

## Architecture

### New Component: Module Cache (`lsp/module_cache.sans`)

A global in-memory cache mapping file paths to their parsed AST and type-checked module exports. Each entry stores:

- File path (key)
- Parsed Program AST
- Module exports (the `ModuleExports` struct returned by `check_module`)
- Disk mtime at time of caching (for staleness checks on non-open files)

Cache operations:
- `mcache_get(path) I` — returns cached entry or 0
- `mcache_put(path, program, exports, mtime) I` — store/update entry
- `mcache_invalidate(path) I` — remove entry for a path
- `mcache_is_stale(path) B` — check if disk mtime is newer than cached mtime

### Modified Component: Analyzer (`lsp/analyzer.sans`)

`analyze_file(uri, content)` is rewritten to:

1. Parse the current file's content (always fresh)
2. Extract `import` statements from the parsed AST
3. For each import, resolve the file path:
   - Try relative to the current file's directory first
   - Fall back to workspace root (`lsp_project_root`)
4. For each resolved import, check the module cache:
   - Cache hit + not stale → use cached exports
   - Cache miss or stale → read from disk, parse, type-check with `check_module(prog, upstream_exports)`, cache result
5. Handle recursive imports (imported files may have their own imports) with circular detection
6. Build `mod_exports` map from all resolved module exports
7. Type-check current file: `check_module(program, mod_exports)` instead of `check_module(program, M())`
8. Collect diagnostics and symbols as before

### Modified Component: Main Loop (`lsp/main.sans`)

- `handle_did_change` / `handle_did_save`: invalidate cache for the changed file's path
- `handle_did_close`: invalidate cache for the closed file

### Import Resolution Strategy

**Path resolution order:**
1. Relative to the current file's directory: `dir_of(current_file) + "/" + import_path + ".sans"`
2. Relative to workspace root: `lsp_project_root + "/" + import_path + ".sans"`

This matches the compiler's behavior (option 1) with workspace root as fallback (option 2).

**Circular import detection:** Track an `in_progress` set during recursive resolution. If a path is encountered while already in progress, skip it (the compiler exits on circular imports, but the LSP should be lenient and just not re-process).

**Recursive resolution:** Imported modules may themselves have imports. Process transitively in dependency order, building up `mod_exports` as each module is resolved.

### Cache Invalidation Strategy

**Lazy invalidation:**
- When a file changes (didChange/didSave), only its cache entry is cleared
- Only the currently focused/edited file is re-analyzed
- Other open files that depend on the changed module get stale diagnostics until the user switches to them
- This avoids cascade re-analysis storms and is how most LSPs work in practice

**Open files vs disk files:**
- Files open in the editor: invalidated by LSP events (didChange, didSave, didClose)
- Imported files not open in the editor: staleness checked by comparing disk mtime against cached mtime before each use

### What This Enables

- Hover shows type info for imported functions, structs, enums
- Completion includes symbols from imported modules
- Diagnostics correctly report "undefined" only for truly undefined symbols (not just missing imports)
- Go-to-definition works for imported symbols (within the same project)
- Signature help works for imported functions

### What This Does NOT Change

- No modifications to the compiler itself (typeck, parser, lexer, codegen, constants)
- No modifications to existing LSP features (they use `lsp_last_symbols` which will now include richer data)
- No changes to the VSCode extension
- The compiler's own `resolve_imports()` in `main.sans` is not imported directly — the LSP implements its own lighter version that works with the cache

## File Map

| File | Status | Responsibility |
|------|--------|---------------|
| `lsp/module_cache.sans` | New | Module cache: store/retrieve/invalidate parsed ASTs and exports |
| `lsp/analyzer.sans` | Modified | Rewrite `analyze_file()` with import resolution and cache integration |
| `lsp/main.sans` | Modified | Add cache invalidation on didChange/didSave/didClose |
| `tests/lsp/test_multifile.sh` | New | Integration test: multi-file diagnostics and hover |

## Success Criteria

- Open a file that imports another module → no spurious "undefined" diagnostics for imported symbols
- Hover on an imported function shows its signature
- Completion includes imported symbols
- Editing a file with stable imports is fast (cache hit path)
- Modifying an imported file and switching back to the importer picks up the change
- Circular imports don't crash the LSP
- All existing LSP tests still pass
