# LSP Multi-file Support & Incremental Parsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add import resolution and module caching to the Sans LSP so that cross-file symbols are visible for diagnostics, hover, completion, and go-to-definition.

**Architecture:** A new module cache (`lsp/module_cache.sans`) stores parsed ASTs and type-checked module exports by file path. The analyzer (`lsp/analyzer.sans`) is rewritten to resolve imports recursively, check cached modules, and pass real `mod_exports` to `check_module()` instead of an empty map. Cache entries are invalidated on didChange/didSave/didClose events.

**Tech Stack:** Sans (self-hosted), existing compiler pipeline (lexer/parser/typeck)

---

## File Map

### New Files

| File | Responsibility |
|------|---------------|
| `lsp/module_cache.sans` | Module cache: store/retrieve/invalidate parsed ASTs and exports per file path |
| `tests/lsp/test_multifile.sh` | Integration test: multi-file import resolution, cross-file diagnostics |

### Modified Files

| File | What Changes |
|------|-------------|
| `lsp/analyzer.sans` | Rewrite `analyze_file()` to resolve imports, use cache, pass real `mod_exports` |
| `lsp/main.sans` | Add cache invalidation on didChange/didSave/didClose |

---

## Task 1: Module Cache

**Files:**
- Create: `lsp/module_cache.sans`

A simple global map-based cache storing parsed programs and module exports per file path.

- [ ] **Step 1: Create `lsp/module_cache.sans`**

```sans
// ------ Module cache for LSP incremental analysis ------
// Caches parsed ASTs and type-checked module exports per file path.
// Each cache entry is 24 bytes:
//   offset 0: parsed Program AST pointer
//   offset 8: module exports pointer (from check_module)
//   offset 16: timestamp (from time()) when cached

g mcache_map = 0

mcache_init() I {
  if mcache_map == 0 { mcache_map = M() }
  0
}

// Get cached entry for a file path. Returns entry pointer or 0.
mcache_get(path:S) I {
  mcache_init()
  if mhas(mcache_map, path) { mget(mcache_map, path) } else { 0 }
}

// Store a cache entry for a file path.
mcache_put(path:S program:I exports:I) I {
  mcache_init()
  entry = alloc(24)
  store64(entry, program)
  store64(entry + 8, exports)
  store64(entry + 16, time())
  mset(mcache_map, path, entry)
  0
}

// Invalidate cache entry for a file path.
mcache_invalidate(path:S) I {
  mcache_init()
  if mhas(mcache_map, path) { mcache_map.delete(path) }
  0
}

// Get cached program AST from entry.
mcache_program(entry:I) I = load64(entry)

// Get cached module exports from entry.
mcache_exports(entry:I) I = load64(entry + 8)
```

- [ ] **Step 2: Build and verify compiles**

```bash
cd /home/scott/Development/sans-language/sans
sans build compiler/main.sans && ./compiler/main build lsp/main.sans -o sans-lsp
```

Note: The module_cache.sans won't be imported yet, but verify the LSP still builds.

- [ ] **Step 3: Commit**

```bash
git add lsp/module_cache.sans
git commit -m "feat(lsp): add module cache for incremental analysis"
```

---

## Task 2: Rewrite Analyzer with Import Resolution

**Files:**
- Modify: `lsp/analyzer.sans`

This is the core change. Rewrite `analyze_file()` to:
1. Parse the current file
2. Extract its imports from the AST
3. Recursively resolve imports using the module cache
4. Build `mod_exports` map from resolved modules
5. Type-check with real `mod_exports`

- [ ] **Step 1: Add import and helper functions to `lsp/analyzer.sans`**

Add `import "module_cache"` at the top of the file (after existing imports).

Add these helper functions after the `analyzer_set_root` function:

```sans
// Resolve a single import path to a file path.
// Tries: 1) relative to base_dir, 2) relative to project root.
// Returns the file path or "" if not found.
lsp_resolve_path(import_path:S base_dir:S) S {
  // Try relative to base directory
  candidate = base_dir + "/" + import_path + ".sans"
  if fe(candidate) == 1 { return candidate }
  // Try relative to project root
  if slen(lsp_project_root) > 0 {
    candidate2 = lsp_project_root + "/" + import_path + ".sans"
    if fe(candidate2) == 1 { return candidate2 }
  }
  ""
}

// Recursively resolve imports for a file.
// Populates mod_exports with all transitive dependencies.
// visited: Map<path:S, 1> to prevent re-processing
// in_progress: Map<path:S, 1> for circular import detection
lsp_resolve_imports(file_path:S mod_exports:I visited:I in_progress:I) I {
  if mhas(visited, file_path) { return 0 }
  if mhas(in_progress, file_path) { return 0 }
  mset(in_progress, file_path, 1)

  // Check cache first
  cached = mcache_get(file_path)
  program := 0
  if cached != 0 {
    program = mcache_program(cached)
  } else {
    // Read and parse the file
    src = fr(file_path)
    if slen(src) == 0 { return 0 }
    program = parse(src)
    if program == 0 { return 0 }
  }

  // Resolve sub-imports first (dependency order)
  base_dir = dir_of(file_path)
  imports_arr = load64(program)
  num_imports = imports_arr.len()
  ii := 0
  while ii < num_imports {
    imp_node = imports_arr.get(ii)
    imp_path = imp_get_path(imp_node)
    resolved_path = lsp_resolve_path(imp_path, base_dir)
    if slen(resolved_path) > 0 {
      lsp_resolve_imports(resolved_path, mod_exports, visited, in_progress)
    }
    ii += 1
  }

  // Now type-check this module (all its deps are in mod_exports)
  if cached != 0 && mcache_exports(cached) != 0 {
    // Use cached exports
    exports = mcache_exports(cached)
  } else {
    // Type-check the module
    tc_collect_mode = 1
    p_collect_mode = 1
    tc_init_diags()
    tc_has_error = 0
    tc_current_file = file_path
    tc_current_source = fr(file_path)
    tc_has_source = 1
    exports = check_module(program, mod_exports)
    tc_collect_mode = 0
    p_collect_mode = 0
    // Cache the result
    mcache_put(file_path, program, exports)
  }

  // Register exports under the module name
  // Extract module name from path (last component without .sans)
  mod_name = mod_name_of(file_path)
  // Strip .sans extension if present
  if mod_name.ends_with(".sans") == 1 {
    mod_name = mod_name.substring(0, slen(mod_name) - 5)
  }
  mset(mod_exports, mod_name, exports)

  // Handle aliases from import nodes (caller's responsibility, not here)
  mset(visited, file_path, 1)
  0
}
```

Note: `imp_get_path` and `imp_get_alias` are defined in `compiler/parser.sans`. They're accessible because `analyzer.sans` imports `../compiler/parser`. `dir_of` and `mod_name_of` are in `compiler/main.sans` — you need to either import `../compiler/main` or reimplement them. Since importing main.sans would pull in the entire compiler driver (including its own `main()` function which would conflict), **reimplement `dir_of` and `mod_name_of` locally** in analyzer.sans:

```sans
// Extract directory from file path
lsp_dir_of(path:S) S {
  i := slen(path) - 1
  found := 0
  while i >= 0 && found == 0 {
    if char_at(path, i) == 47 { found = 1 } else { i -= 1 }
  }
  if found == 1 { path.substring(0, i) } else { "." }
}

// Extract module name from path (filename without directory and extension)
lsp_mod_name_of(path:S) S {
  // Find last /
  i := slen(path) - 1
  slash := 0 - 1
  found := 0
  while i >= 0 && found == 0 {
    if char_at(path, i) == 47 { slash = i; found = 1 } else { i -= 1 }
  }
  start = if slash >= 0 { slash + 1 } else { 0 }
  name = path.substring(start, slen(path))
  // Strip .sans extension
  if name.ends_with(".sans") == 1 {
    name = name.substring(0, slen(name) - 5)
  }
  name
}
```

Then use `lsp_dir_of` instead of `dir_of` and `lsp_mod_name_of` instead of `mod_name_of` in the code above.

- [ ] **Step 2: Rewrite `analyze_file()` to use import resolution**

Replace the existing `analyze_file` function in `lsp/analyzer.sans`:

```sans
analyze_file(uri:S content:S) I {
  path = uri_to_path(uri)
  base_dir = lsp_dir_of(path)

  // Enable collect mode
  tc_collect_mode = 1
  p_collect_mode = 1
  tc_init_diags()
  tc_has_error = 0
  tc_current_file = path
  tc_current_source = content
  tc_has_source = 1

  // Parse the current file
  program = parse(content)

  if program != 0 {
    // Resolve imports and build mod_exports
    mod_exports = M()
    visited = M()
    in_progress = M()
    mset(in_progress, path, 1)

    // Process each import in the current file
    imports_arr = load64(program)
    num_imports = imports_arr.len()
    ii := 0
    while ii < num_imports {
      imp_node = imports_arr.get(ii)
      imp_path = imp_get_path(imp_node)
      resolved_path = lsp_resolve_path(imp_path, base_dir)
      if slen(resolved_path) > 0 {
        lsp_resolve_imports(resolved_path, mod_exports, visited, in_progress)
      }
      ii += 1
    }

    // Re-enable collect mode (may have been toggled during import resolution)
    tc_collect_mode = 1
    p_collect_mode = 1
    tc_init_diags()
    tc_has_error = 0
    tc_current_file = path
    tc_current_source = content
    tc_has_source = 1

    // Type-check the current file with real imports
    check_module(program, mod_exports)
  }

  // Disable collect mode
  tc_collect_mode = 0
  p_collect_mode = 0

  // Store results
  lsp_last_diags = tc_diags
  lsp_last_symbols = tc_collected_symbols

  if lsp_last_diags != 0 { lsp_last_diags.len() } else { 0 }
}
```

- [ ] **Step 3: Build and test**

```bash
sans build compiler/main.sans && ./compiler/main build lsp/main.sans -o sans-lsp
bash tests/lsp/test_lifecycle.sh ./sans-lsp
bash tests/lsp/test_diagnostics.sh ./sans-lsp
bash tests/lsp/test_completion.sh ./sans-lsp
```

All existing tests must pass.

- [ ] **Step 4: Commit**

```bash
git add lsp/analyzer.sans
git commit -m "feat(lsp): add import resolution and module caching to analyzer"
```

---

## Task 3: Cache Invalidation on File Changes

**Files:**
- Modify: `lsp/main.sans`

When a file is modified or closed, invalidate its module cache entry so dependent files pick up the changes.

- [ ] **Step 1: Add import and invalidation calls**

Add `import "module_cache"` at the top of `lsp/main.sans` (with the other imports).

In `handle_did_change`, add cache invalidation after `doc_store`:

```sans
handle_did_change(params:J) I {
  uri = params_uri(params)
  content = params_text(params)
  doc_store(uri, content)
  mcache_invalidate(uri_to_path(uri))
  run_diagnostics(uri, content)
  0
}
```

In `handle_did_save`, add cache invalidation:

```sans
handle_did_save(params:J) I {
  uri = params_uri(params)
  mcache_invalidate(uri_to_path(uri))
  content = doc_get(uri)
  if slen(content) > 0 { run_diagnostics(uri, content) }
  0
}
```

In `handle_did_close`, add cache invalidation:

```sans
handle_did_close(params:J) I {
  uri = params_uri(params)
  doc_remove(uri)
  mcache_invalidate(uri_to_path(uri))
  params_out = make_publish_diagnostics(uri, ja())
  rpc_notify("textDocument/publishDiagnostics", params_out)
  0
}
```

- [ ] **Step 2: Build and test**

```bash
sans build compiler/main.sans && ./compiler/main build lsp/main.sans -o sans-lsp
bash tests/lsp/test_lifecycle.sh ./sans-lsp
bash tests/lsp/test_diagnostics.sh ./sans-lsp
```

- [ ] **Step 3: Commit**

```bash
git add lsp/main.sans
git commit -m "feat(lsp): invalidate module cache on file changes"
```

---

## Task 4: Multi-file Integration Test

**Files:**
- Create: `tests/lsp/test_multifile.sh`

Test that the LSP correctly resolves imports and provides diagnostics, hover, and completion for cross-file symbols.

- [ ] **Step 1: Create `tests/lsp/test_multifile.sh`**

```bash
#!/bin/bash
set -euo pipefail

SANS_LSP="${1:-./sans-lsp}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
PASS=0
FAIL=0

send_msg() {
  local json="$1"
  local len=${#json}
  printf "Content-Length: %d\r\n\r\n%s" "$len" "$json"
}

check() {
  local desc="$1" pattern="$2" response="$3"
  printf "  "
  if echo "$response" | grep -q "$pattern"; then
    echo -e "${GREEN}\xE2\x9C\x93${NC}  $desc"
    ((PASS++)) || true
  else
    echo -e "${RED}\xE2\x9C\x97${NC}  $desc"
    ((FAIL++)) || true
  fi
}

echo "LSP Multi-file Tests ($SANS_LSP)"
echo "================================="

# Create test project with two files
TEST_DIR="/tmp/sans_lsp_multifile_$$"
mkdir -p "$TEST_DIR"

# utils.sans — a module with exported functions
cat > "$TEST_DIR/utils.sans" << 'SANS'
add(a:I b:I) I = a + b
greet(name:S) S = "hello " + name
SANS

# main.sans — imports utils
cat > "$TEST_DIR/main.sans" << 'SANS'
import "utils"

main() I {
  x = add(1, 2)
  p(x)
  0
}
SANS

MAIN_URI="file://${TEST_DIR}/main.sans"
MAIN_CONTENT=$(python3 -c 'import sys,json; print(json.dumps(open(sys.argv[1]).read())[1:-1])' "$TEST_DIR/main.sans")

INIT="{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"rootUri\":\"file://${TEST_DIR}\",\"capabilities\":{}}}"
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DID_OPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${MAIN_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"${MAIN_CONTENT}\"}}}"
# Hover on "add" at line 3, col 6 (0-based)
HOVER="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{\"textDocument\":{\"uri\":\"${MAIN_URI}\"},\"position\":{\"line\":3,\"character\":6}}}"
SHUTDOWN='{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN"; sleep 1; send_msg "$HOVER"; sleep 0.3; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 15 "$SANS_LSP" 2>/dev/null || true)

# Test 1: No "undefined" error for imported function
check "no undefined error for imported add()" '!' "$RESPONSE" || true
# Actually check that diagnostics don't contain "undefined.*add"
if echo "$RESPONSE" | grep -q "undefined.*add"; then
  echo -e "  ${RED}\xE2\x9C\x97${NC}  imported function add() reported as undefined"
  ((FAIL++)) || true
else
  echo -e "  ${GREEN}\xE2\x9C\x93${NC}  imported function add() not reported as undefined"
  ((PASS++)) || true
fi

# Test 2: Hover shows add function signature
check "hover shows function signature for add" '"contents"' "$RESPONSE"

# Test 3: publishDiagnostics was sent
check "diagnostics published for main.sans" 'publishDiagnostics' "$RESPONSE"

# --- Test file with bad import ---
cat > "$TEST_DIR/bad.sans" << 'SANS'
import "nonexistent"

main() I { 0 }
SANS

BAD_URI="file://${TEST_DIR}/bad.sans"
BAD_CONTENT=$(python3 -c 'import sys,json; print(json.dumps(open(sys.argv[1]).read())[1:-1])' "$TEST_DIR/bad.sans")

DID_OPEN_BAD="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${BAD_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"${BAD_CONTENT}\"}}}"

RESPONSE2=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN_BAD"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 15 "$SANS_LSP" 2>/dev/null || true)

# Test 4: Bad import doesn't crash the LSP
check "LSP handles missing import without crash" 'publishDiagnostics' "$RESPONSE2"

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
```

```bash
chmod +x tests/lsp/test_multifile.sh
```

- [ ] **Step 2: Run the test**

```bash
bash tests/lsp/test_multifile.sh ./sans-lsp
```

- [ ] **Step 3: Run all existing tests to verify no regressions**

```bash
bash tests/lsp/test_lifecycle.sh ./sans-lsp
bash tests/lsp/test_diagnostics.sh ./sans-lsp
bash tests/lsp/test_completion.sh ./sans-lsp
bash tests/lsp/test_semantic_tokens.sh ./sans-lsp
bash tests/lsp/test_references.sh ./sans-lsp
bash tests/run_tests.sh
```

- [ ] **Step 4: Commit**

```bash
git add tests/lsp/test_multifile.sh
git commit -m "test(lsp): add multi-file import resolution integration test"
```

---

## Implementation Notes

**Import of `../compiler/parser`:** The analyzer already imports `../compiler/parser`, which gives access to `imp_get_path()` and `imp_get_alias()` for reading import AST nodes. It also imports `../compiler/typeck` which provides `check_module()`, `tc_collect_mode`, and all diagnostic infrastructure.

**Why not import `compiler/main.sans`:** The compiler's `main.sans` contains a `main()` function and the full build pipeline. Importing it into the LSP would create a conflict (two `main()` functions). Instead, the LSP reimplements the small utility functions it needs (`lsp_dir_of`, `lsp_mod_name_of`, `lsp_resolve_path`).

**Collect mode toggling:** During import resolution, `lsp_resolve_imports` may call `check_module` on imported files with collect mode on. After resolving all imports, `analyze_file` resets collect mode and diagnostics before type-checking the current file. This ensures only the current file's diagnostics are reported to the editor.

**Cache lifetime:** Cache entries persist for the LSP process lifetime. They're invalidated by didChange/didSave/didClose events. For imported files not open in the editor, the cache is populated on first analysis and persists until the LSP restarts. This is acceptable for Phase 1 incremental support.

**`fe()` for file existence:** The `fe()` builtin (alias for `file_exists()`) returns 1 if a file exists, 0 otherwise. Used in `lsp_resolve_path` to check candidate import paths.

**Bootstrap compiler bug:** The bootstrap (v0.8.3) has a codegen bug where `S + I` does string concatenation instead of pointer arithmetic. Use `char_at(str, idx)` to read bytes from strings, not `load8(str + idx)`. Both branches of if/else must assign to mutable variables.
