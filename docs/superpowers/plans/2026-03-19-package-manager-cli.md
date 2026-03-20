# Package Manager CLI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `sans pkg` subcommand with init, add, install, remove, list, update, and search commands for managing Sans package dependencies.

**Architecture:** A single new file `compiler/pkg.sans` containing all package manager logic, dispatched from `main.sans` when `args()[1] == "pkg"`. Uses existing builtins (file I/O, JSON, HTTP, string ops) plus the new filesystem/process builtins (getenv, mkdir, is_dir, sh, etc). No changes to the compilation pipeline — this is pure CLI tooling.

**Tech Stack:** Sans builtins only — JSON for sans.json, sh/system for git operations, http_get for index fetching

**Spec:** `docs/superpowers/specs/2026-03-19-package-manager-design.md` (Parts 1b, 2, 3, 4, 6, 7)

---

## Prerequisites

**JsonValue needs `.keys()`, `.has()`, `.delete()` methods.** These exist on Map but not JsonValue. The package manager needs to enumerate JSON object keys, check for key existence, and remove keys. This is a prerequisite that adds 3 methods through the standard pipeline (typeck -> IR -> codegen -> runtime). Implemented as Task 0.

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `compiler/typeck.sans` | Modify | Add `.keys()`, `.has()`, `.delete()` methods on JsonValue |
| `compiler/constants.sans` | Modify | Add IR opcodes for JSON keys/has/delete |
| `compiler/ir.sans` | Modify | Add IR lowering for JSON keys/has/delete |
| `compiler/codegen.sans` | Modify | Add codegen for JSON keys/has/delete |
| `runtime/json.sans` | Modify | Add runtime functions for JSON keys/has/delete |
| `compiler/pkg.sans` | Create | All package manager logic: dispatch, init, add, install, remove, list, update, search, dependency resolution, git operations |
| `compiler/main.sans` | Modify | Add `import "pkg"` and dispatch `sans pkg` to `pkg_main()` |
| `tests/fixtures/json_keys.sans` | Create | Test JsonValue .keys()/.has()/.delete() |
| `tests/fixtures/pkg_init.sans` | Create | Test sans.json creation |
| `tests/fixtures/pkg_install.sans` | Create | Test dependency resolution and install |

### Key patterns from the codebase

- **CLI dispatch:** `compiler/main.sans` line ~497: `if cmd == "build" { ... }`. Add `if cmd == "pkg"` branch.
- **Module imports:** `compiler/main.sans` lines 4-9: `import "constants"` etc. Add `import "pkg"`.
- **JSON round-trip:** `json_parse(fr("sans.json"))` to read, `fw("sans.json" jfy(obj))` to write.
- **Shell commands:** `sh("git ls-remote --tags ...")` to capture output, `system("git clone ...")` for exit code.
- **Build process:** `sans build compiler/main.sans` compiles all imported modules into one binary. Adding `import "pkg"` automatically includes pkg.sans.

### Build notes

- Build with bootstrap: `export DYLD_LIBRARY_PATH="/Users/sgordon/homebrew/opt/openssl@3/lib" && export PATH="/Users/sgordon/homebrew/opt/llvm@17/bin:$PATH" && /tmp/sans build compiler/main.sans`
- Install: `cp compiler/main /Users/sgordon/.local/bin/sans && rm -f compiler/main`
- The v0.4.0 bootstrap is at `/tmp/sans` — re-download if missing: `curl -fsSL https://github.com/sans-language/sans/releases/download/v0.4.0/sans-macos-arm64.tar.gz | tar xz -C /tmp && chmod +x /tmp/sans`

---

## Chunk 0: Add `.keys()`, `.has()`, `.delete()` to JsonValue

### Task 0: Add JsonValue methods

These 3 methods follow the exact pattern of existing JsonValue methods. Reference: Map already has all three — use the same typeck/IR/codegen pattern but dispatch to `sans_json_*` runtime functions.

**Files:**
- Modify: `compiler/constants.sans` — add `IR_JSON_KEYS`, `IR_JSON_HAS`, `IR_JSON_DELETE`
- Modify: `compiler/typeck.sans` — add method dispatch for JsonValue `.keys()`, `.has()`, `.delete()`
- Modify: `compiler/ir.sans` — add IR lowering
- Modify: `compiler/codegen.sans` — add codegen calling runtime functions
- Modify: `runtime/json.sans` — add `sans_json_keys`, `sans_json_has`, `sans_json_delete`
- Create: `tests/fixtures/json_keys.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/json_keys.sans`:

```sans
main() I {
  obj = jo()
  obj.set("name" js("test"))
  obj.set("version" js("1.0"))
  obj.set("count" ji(42))

  // .keys() returns Array<String>
  k = obj.keys()
  if k.len() != 3 { return 1 }

  // .has() checks key existence
  if obj.has("name") != true { return 2 }
  if obj.has("missing") != false { return 3 }

  // .delete() removes a key
  obj.delete("count")
  if obj.keys().len() != 2 { return 4 }
  if obj.has("count") != false { return 5 }

  0
}
```

- [ ] **Step 2: Add IR constants**

In `compiler/constants.sans` after `IR_SH = 252`:

```sans
g IR_JSON_KEYS = 253
g IR_JSON_HAS = 254
g IR_JSON_DELETE = 255
```

- [ ] **Step 3: Add type checking**

In `compiler/typeck.sans`, find the JsonValue method dispatch section (search for `type_of` near line 2220). Add before the error fallthrough:

```sans
if mc_method == "keys" {
  if mc_nargs != 0 { tc_error("JsonValue.keys() takes 0 arguments") }
  return make_array_type(make_type(TY_STRING))
}
if mc_method == "has" {
  if mc_nargs != 1 { tc_error("JsonValue.has() takes 1 argument") }
  check_expr(mc_args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  return make_type(TY_BOOL)
}
if mc_method == "delete" {
  if mc_nargs != 1 { tc_error("JsonValue.delete() takes 1 argument") }
  check_expr(mc_args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  return make_type(TY_INT)
}
```

- [ ] **Step 4: Add IR lowering**

In `compiler/ir.sans`, find JsonValue method lowering. Add:

```sans
// For .keys() — 0-arg method on object
else if method == "keys" && obj_type == IRTY_JSON { lower_method_0(ctx, obj_reg, IR_JSON_KEYS, IRTY_ARRAY) }
else if method == "has" && obj_type == IRTY_JSON { lower_method_1(ctx, obj_reg, args, IR_JSON_HAS, IRTY_BOOL) }
else if method == "delete" && obj_type == IRTY_JSON { lower_method_1(ctx, obj_reg, args, IR_JSON_DELETE, IRTY_INT) }
```

The implementer must find the exact method dispatch pattern — search for existing JsonValue methods like `type_of` or `get_string` in ir.sans to see how methods are lowered.

- [ ] **Step 5: Add codegen**

```sans
if op == IR_JSON_KEYS {
  compile_rt1(cg, inst, "sans_json_keys")
  emit_scope_track(cg, cg_get_val(cg, dest), 1)
  return 0
}
if op == IR_JSON_HAS {
  compile_rt2(cg, inst, "sans_json_has")
  return 0
}
if op == IR_JSON_DELETE {
  compile_rt2(cg, inst, "sans_json_delete")
  return 0
}
```

Add extern declarations in `emit_externals()`:

```sans
emit(cg, "declare i64 @sans_json_keys(i64)")
emit(cg, "declare i64 @sans_json_has(i64, i64)")
emit(cg, "declare i64 @sans_json_delete(i64, i64)")
```

- [ ] **Step 6: Add runtime functions in `runtime/json.sans`**

The implementer must read `runtime/json.sans` to understand the internal JsonValue structure (it's a tagged union with type, data pointer, and length). The runtime functions iterate the object's key-value pairs.

The exact implementation depends on the internal JSON object layout. The implementer should study existing functions like `sans_json_set` and `sans_json_get` to understand how keys are stored and iterated.

- [ ] **Step 7: Build, test, commit**

```bash
# Build, install, test
sans build tests/fixtures/json_keys.sans
./tests/fixtures/json_keys; echo "exit: $?"
bash tests/run_tests.sh  # verify no regressions
git add compiler/constants.sans compiler/typeck.sans compiler/ir.sans compiler/codegen.sans runtime/json.sans tests/fixtures/json_keys.sans
git commit -m "feat: add .keys(), .has(), .delete() methods to JsonValue"
```

---

## Chunk 1: Core Infrastructure (dispatch, helpers, init)

### Task 1: Create `compiler/pkg.sans` with dispatch and `pkg init`

**Files:**
- Create: `compiler/pkg.sans`
- Modify: `compiler/main.sans`

- [ ] **Step 1: Add `import "pkg"` and dispatch in main.sans**

In `compiler/main.sans`, add `import "pkg"` after line 9 (`import "codegen"`):

```sans
import "pkg"
```

In the `main()` function, add a dispatch branch for `pkg` BEFORE the `argc < 3` check (around line 481). Search for `if cmd == "--version"` and add after that block:

```sans
if cmd == "pkg" {
  pkg_main(argv)
  exit(0)
}
```

- [ ] **Step 2: Create `compiler/pkg.sans` with dispatch and helpers**

Create `compiler/pkg.sans`:

```sans
// Sans Package Manager — sans pkg subcommand

// ---- Helpers ----

// Get ~/.sans base directory
pkg_sans_dir() S {
  home = getenv("HOME")
  if home.len() == 0 {
    p("error: HOME environment variable not set")
    exit(1)
  }
  home + "/.sans"
}

// Get ~/.sans/packages directory
pkg_cache_dir() S {
  pkg_sans_dir() + "/packages"
}

// Check if a string looks like a URL (has a dot before first slash, and has a slash)
pkg_is_url(s: S) B {
  has_dot := false
  i := 0
  while i < s.len() {
    ch = char_at(s i)
    if ch == 46 { has_dot = true }
    if ch == 47 { return has_dot }
    i = i + 1
  }
  false
}

// Get the cache path for a package: ~/.sans/packages/<url>/<version>/
pkg_cache_path(url: S version: S) S {
  pkg_cache_dir() + "/" + url + "/" + version
}

// Read and parse sans.json from current directory. Returns 0 if not found.
pkg_read_manifest() I {
  if fe("sans.json") != true { return 0 }
  content = fr("sans.json")
  if content.len() == 0 { return 0 }
  jp(content)
}

// Write a JsonValue to sans.json
pkg_write_manifest(obj: I) I {
  fw("sans.json" jfy(obj))
}

// Get current directory name (last path component of cwd)
pkg_cwd_name() S {
  cwd = sh("pwd").trim()
  parts = cwd.split("/")
  if parts.len() > 0 { parts.get(parts.len() - 1) } else { "my-project" }
}

// Clone a package at a specific tag into the cache
pkg_clone(url: S version: S) I {
  dest = pkg_cache_path(url version)
  if is_dir(dest) {
    p("  " + url + " " + version + "... cached")
    return 1
  }
  // Create parent directories
  mkdir(pkg_cache_dir() + "/" + url)
  p("  " + url + " " + version + "... cloning")
  r = system("git clone --depth 1 --branch " + version + " https://" + url + ".git '" + dest + "' 2>/dev/null")
  if r != 0 {
    p("error: failed to clone " + url + " at " + version)
    return 0
  }
  1
}

// Get latest tag from a git remote
pkg_latest_tag(url: S) S {
  output = sh("git ls-remote --tags --sort=-v:refname https://" + url + ".git 'v*' 2>/dev/null")
  if output.len() == 0 { return "" }
  // First line has the latest tag: "<hash>\trefs/tags/v1.2.3"
  lines = output.split("\n")
  if lines.len() == 0 { return "" }
  first = lines.get(0)
  // Extract tag name after "refs/tags/"
  idx = first.index_of("refs/tags/")
  if idx < 0 { return "" }
  first[idx + 10:]
}

// Parse --name and --version flags from args
pkg_parse_flag(a: [S] flag: S) S {
  i := 2
  while i < a.len() - 1 {
    if a.get(i) == flag { return a.get(i + 1) }
    i = i + 1
  }
  ""
}

// ---- Commands ----

// sans pkg init [--name NAME] [--version VERSION]
pkg_init(a: [S]) I {
  if fe("sans.json") {
    p("error: sans.json already exists")
    return 1
  }
  name = pkg_parse_flag(a "--name")
  if name.len() == 0 { name = pkg_cwd_name() }
  version = pkg_parse_flag(a "--version")
  if version.len() == 0 { version = "0.1.0" }

  obj = jo()
  obj.set("name" js(name))
  obj.set("version" js(version))
  obj.set("deps" jo())
  pkg_write_manifest(obj)
  p("Created sans.json (" + name + " v" + version + ")")
  0
}

// ---- Dispatch ----

pkg_help() I {
  p("usage: sans pkg <command>")
  p("")
  p("commands:")
  p("  init               Create sans.json")
  p("  add <url> [tag]    Add dependency")
  p("  install            Install all dependencies")
  p("  remove <url>       Remove dependency")
  p("  list               List dependencies")
  p("  update <url> [tag] Update dependency")
  p("  search <query>     Search package index")
  0
}

pkg_main(a: [S]) I {
  if a.len() < 3 { return pkg_help() }
  sub = a.get(2)
  if sub == "init" { return pkg_init(a) }
  if sub == "help" || sub == "--help" { return pkg_help() }
  p("unknown command: sans pkg " + sub)
  p("run 'sans pkg help' for usage")
  1
}
```

- [ ] **Step 3: Build and test dispatch**

```bash
# Build
export DYLD_LIBRARY_PATH="/Users/sgordon/homebrew/opt/openssl@3/lib"
export PATH="/Users/sgordon/homebrew/opt/llvm@17/bin:$PATH"
/tmp/sans build compiler/main.sans
cp compiler/main /Users/sgordon/.local/bin/sans
rm -f compiler/main

# Test dispatch
sans pkg help
sans pkg init
cat sans.json
rm sans.json
sans pkg init --name test-lib --version 1.0.0
cat sans.json
rm sans.json
```

- [ ] **Step 4: Commit**

```bash
git add compiler/pkg.sans compiler/main.sans
git commit -m "feat: add sans pkg subcommand with init command"
```

---

### Task 2: Add `pkg install` with dependency resolution

This is the core — reads `sans.json`, resolves transitive deps, clones missing packages.

**Files:**
- Modify: `compiler/pkg.sans`

- [ ] **Step 1: Add `pkg_resolve_deps` function**

Add to `compiler/pkg.sans` in the helpers section:

```sans
// Resolve all dependencies (direct + transitive) from a sans.json.
// Returns a map of url -> version. Clones missing packages.
// manifest_path: path to the sans.json to start from
// requester: name for error messages ("project" for root)
pkg_resolve_deps(manifest_path: S requester: S) I {
  resolved = M()
  // Parallel arrays for the BFS queue (avoids unsafe pointer packing)
  q_urls = array<S>()
  q_versions = array<S>()
  q_requesters = array<S>()

  // Read starting manifest
  content = fr(manifest_path)
  if content.len() == 0 {
    p("error: cannot read " + manifest_path)
    return ptr(resolved)
  }
  manifest = jp(content)
  deps = manifest.get("deps")
  if deps.type_of() != "object" { return ptr(resolved) }

  // Enqueue direct deps
  dep_keys = deps.keys()
  i := 0
  while i < dep_keys.len() {
    url = dep_keys.get(i)
    version = deps.get(url).get_string()
    q_urls.push(url)
    q_versions.push(version)
    q_requesters.push(requester)
    i = i + 1
  }

  // BFS resolution
  qi := 0
  while qi < q_urls.len() {
    url = q_urls.get(qi)
    version = q_versions.get(qi)
    req = q_requesters.get(qi)
    qi = qi + 1

    if resolved.has(url) {
      existing = resolved.get(url)
      if existing != version {
        p("ERROR: Version conflict for " + url)
        p("  " + req + " requires " + version)
        p("  already resolved to " + existing)
        p("")
        p("Fix: Align both to the same version in sans.json")
        exit(1)
      }
      continue
    }

    resolved.set(url version)

    // Clone if missing
    r = pkg_clone(url version)
    if r != 1 { exit(1) }

    // Check for transitive deps
    dep_manifest = pkg_cache_path(url version) + "/sans.json"
    if fe(dep_manifest) {
      sub_content = fr(dep_manifest)
      if sub_content.len() > 0 {
        sub_obj = jp(sub_content)
        sub_deps = sub_obj.get("deps")
        if sub_deps.type_of() == "object" {
          sub_keys = sub_deps.keys()
          j := 0
          while j < sub_keys.len() {
            sub_url = sub_keys.get(j)
            sub_ver = sub_deps.get(sub_url).get_string()
            q_urls.push(sub_url)
            q_versions.push(sub_ver)
            q_requesters.push(url)
            j = j + 1
          }
        }
      }
    }
  }

  ptr(resolved)
}
```

**Note:** Uses parallel arrays with an advancing index (`qi`) instead of `remove(0)` which would be O(n). The `resolved` Map is returned as a raw pointer via `ptr()` since the function returns `I`.

- [ ] **Step 2: Add `pkg_install` command**

```sans
// sans pkg install
pkg_install(a: [S]) I {
  manifest = pkg_read_manifest()
  if manifest == 0 {
    p("error: no sans.json found in current directory")
    return 1
  }

  p("Resolving dependencies...")
  resolved = pkg_resolve_deps("sans.json" "project")

  keys = resolved.keys()
  if keys.len() == 0 {
    p("No dependencies to install.")
  } else {
    p("Done. " + str(keys.len()) + " package(s) installed.")
  }
  0
}
```

- [ ] **Step 3: Add to dispatch**

In `pkg_main`, add before the "unknown command" line:

```sans
if sub == "install" { return pkg_install(a) }
```

- [ ] **Step 4: Build and test**

Create a test `sans.json` manually and test install. Since we need a real git repo to clone, test with error handling (no deps, missing repo, etc).

```bash
# Build and install
# Test with empty deps
echo '{"name":"test","version":"0.1.0","deps":{}}' > sans.json
sans pkg install
rm sans.json
```

- [ ] **Step 5: Commit**

```bash
git add compiler/pkg.sans
git commit -m "feat: add sans pkg install with dependency resolution"
```

---

## Chunk 2: Add, Remove, List, Update Commands

### Task 3: Add `pkg add` command

**Files:**
- Modify: `compiler/pkg.sans`

- [ ] **Step 1: Add `pkg_add` function**

```sans
// sans pkg add <url> [tag]
pkg_add(a: [S]) I {
  if a.len() < 4 {
    p("usage: sans pkg add <url> [tag]")
    return 1
  }

  url = a.get(3)

  // If not a URL, try resolving from index
  if pkg_is_url(url) != true {
    resolved_url = pkg_index_resolve(url)
    if resolved_url.len() == 0 {
      p("error: '" + url + "' is not a URL and was not found in the package index")
      return 1
    }
    url = resolved_url
  }

  // Get version tag
  version = if a.len() > 4 { a.get(4) } else { "" }
  if version.len() == 0 {
    p("Resolving latest tag for " + url + "...")
    version = pkg_latest_tag(url)
    if version.len() == 0 {
      p("error: could not find any tags for " + url)
      return 1
    }
    p("Resolved latest: " + version)
  }

  // Read or create manifest
  manifest = pkg_read_manifest()
  if manifest == 0 {
    p("error: no sans.json found. Run 'sans pkg init' first.")
    return 1
  }

  // Add to deps
  deps = manifest.get("deps")
  if deps.type_of() != "object" {
    deps = jo()
    manifest.set("deps" deps)
  }
  deps.set(url js(version))
  pkg_write_manifest(manifest)
  p("Added " + url + " " + version)

  // Clone if needed
  pkg_clone(url version)

  // Resolve transitive deps
  pkg_resolve_deps("sans.json" "project")
  p("Done.")
  0
}
```

- [ ] **Step 2: Add to dispatch**

```sans
if sub == "add" { return pkg_add(a) }
```

- [ ] **Step 3: Build and test**

- [ ] **Step 4: Commit**

```bash
git add compiler/pkg.sans
git commit -m "feat: add sans pkg add — add dependency by URL or name"
```

---

### Task 4: Add `pkg remove` and `pkg list`

**Files:**
- Modify: `compiler/pkg.sans`

- [ ] **Step 1: Add `pkg_remove` function**

```sans
// sans pkg remove <url>
pkg_remove(a: [S]) I {
  if a.len() < 4 {
    p("usage: sans pkg remove <url>")
    return 1
  }
  url = a.get(3)
  manifest = pkg_read_manifest()
  if manifest == 0 {
    p("error: no sans.json found")
    return 1
  }
  deps = manifest.get("deps")
  if deps.type_of() != "object" || deps.has(url) != true {
    p("error: " + url + " is not in dependencies")
    return 1
  }
  deps.delete(url)
  pkg_write_manifest(manifest)
  p("Removed " + url + " from sans.json")
  0
}
```

- [ ] **Step 2: Add `pkg_list` function**

```sans
// sans pkg list
pkg_list(a: [S]) I {
  manifest = pkg_read_manifest()
  if manifest == 0 {
    p("error: no sans.json found")
    return 1
  }
  deps = manifest.get("deps")
  if deps.type_of() != "object" {
    p("No dependencies.")
    return 0
  }
  keys = deps.keys()
  if keys.len() == 0 {
    p("No dependencies.")
    return 0
  }

  // Show direct deps
  i := 0
  while i < keys.len() {
    url = keys.get(i)
    version = deps.get(url).get_string()
    p("  " + url + "  " + version + "  (direct)")
    i = i + 1
  }

  // Resolve to find transitive deps
  resolved = pkg_resolve_deps("sans.json" "project")
  rkeys = resolved.keys()
  j := 0
  while j < rkeys.len() {
    rurl = rkeys.get(j)
    if deps.has(rurl) != true {
      rver = resolved.get(rurl)
      p("  " + rurl + "  " + rver + "  (transitive)")
    }
    j = j + 1
  }
  0
}
```

- [ ] **Step 3: Add to dispatch**

```sans
if sub == "remove" { return pkg_remove(a) }
if sub == "list" { return pkg_list(a) }
```

- [ ] **Step 4: Build and test**

- [ ] **Step 5: Commit**

```bash
git add compiler/pkg.sans
git commit -m "feat: add sans pkg remove and list commands"
```

---

### Task 5: Add `pkg update`

**Files:**
- Modify: `compiler/pkg.sans`

- [ ] **Step 1: Add `pkg_update` function**

```sans
// sans pkg update <url> [tag]
pkg_update(a: [S]) I {
  if a.len() < 4 {
    p("usage: sans pkg update <url> [tag]")
    return 1
  }
  url = a.get(3)
  manifest = pkg_read_manifest()
  if manifest == 0 {
    p("error: no sans.json found")
    return 1
  }
  deps = manifest.get("deps")
  if deps.type_of() != "object" || deps.has(url) != true {
    p("error: " + url + " is not in dependencies")
    return 1
  }

  old_version = deps.get(url).get_string()

  // Get new version
  new_version = if a.len() > 4 { a.get(4) } else { "" }
  if new_version.len() == 0 {
    p("Resolving latest tag for " + url + "...")
    new_version = pkg_latest_tag(url)
    if new_version.len() == 0 {
      p("error: could not find any tags for " + url)
      return 1
    }
  }

  if old_version == new_version {
    p(url + " is already at " + new_version)
    return 0
  }

  deps.set(url js(new_version))
  pkg_write_manifest(manifest)
  p("Updated " + url + " " + old_version + " -> " + new_version)

  // Clone new version if needed
  pkg_clone(url new_version)
  p("Done.")
  0
}
```

- [ ] **Step 2: Add to dispatch**

```sans
if sub == "update" { return pkg_update(a) }
```

- [ ] **Step 3: Build, test, commit**

```bash
git add compiler/pkg.sans
git commit -m "feat: add sans pkg update — update dependency version"
```

---

## Chunk 3: Community Index (search + short-name resolution)

### Task 6: Add `pkg search` and index integration

**Files:**
- Modify: `compiler/pkg.sans`

- [ ] **Step 1: Add index fetching and caching**

```sans
// Community index URL
g PKG_INDEX_URL = "https://raw.githubusercontent.com/sans-lang/packages/main/index.json"

// Fetch or read cached index. Returns JsonValue or 0.
pkg_get_index() I {
  cache_path = pkg_sans_dir() + "/index-cache.json"

  // Try fetching fresh index
  resp = hg(PKG_INDEX_URL)
  if resp.status() == 200 {
    body = resp.body()
    if body.len() > 0 {
      mkdir(pkg_sans_dir())
      fw(cache_path body)
      return jp(body)
    }
  }

  // Fall back to cache
  if fe(cache_path) {
    cached = fr(cache_path)
    if cached.len() > 0 { return jp(cached) }
  }

  0
}

// Resolve a short package name to a full URL via index
pkg_index_resolve(name: S) S {
  idx = pkg_get_index()
  if idx == 0 { return "" }
  packages = idx.get("packages")
  if packages.type_of() != "object" { return "" }
  entry = packages.get(name)
  if entry.type_of() != "object" { return "" }
  url_val = entry.get("url")
  if url_val.type_of() != "string" { return "" }
  url_val.get_string()
}
```

- [ ] **Step 2: Add `pkg_search` function**

```sans
// sans pkg search <query>
pkg_search(a: [S]) I {
  if a.len() < 4 {
    p("usage: sans pkg search <query>")
    return 1
  }
  query = a.get(3)

  p("Fetching package index...")
  idx = pkg_get_index()
  if idx == 0 {
    p("error: could not fetch package index")
    return 1
  }

  packages = idx.get("packages")
  if packages.type_of() != "object" {
    p("No packages in index.")
    return 0
  }

  keys = packages.keys()
  found := 0
  i := 0
  while i < keys.len() {
    name = keys.get(i)
    entry = packages.get(name)
    url = entry.get("url").get_string()
    desc = entry.get("description").get_string()
    latest = entry.get("latest").get_string()

    // Match on name or description (case-insensitive would be nice but .lower() works)
    if name.contains(query) || desc.lower().contains(query.lower()) {
      p("  " + url + "  " + latest + "  \"" + desc + "\"")
      found = found + 1
    }
    i = i + 1
  }

  if found == 0 {
    p("No packages matching '" + query + "'")
  }
  0
}
```

- [ ] **Step 3: Add to dispatch**

```sans
if sub == "search" { return pkg_search(a) }
```

- [ ] **Step 4: Build, test, commit**

```bash
git add compiler/pkg.sans
git commit -m "feat: add sans pkg search and index integration"
```

---

## Chunk 4: Documentation and Testing

### Task 7: Update all documentation

Per the CLAUDE.md Documentation Update Checklist, update ALL of:

**Files:**
- Modify: `docs/reference.md` — add Package Manager section with all commands
- Modify: `docs/ai-reference.md` — add compact pkg command reference
- Modify: `website/docs/index.html` — add package manager docs
- Modify: `editors/vscode-sans/src/extension.ts` — add HOVER_DATA for `sans.json` fields and package manager concepts
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json` — no syntax changes needed (pkg is a CLI tool, not language syntax)
- Modify: `README.md` — add package manager to features
- Create: `examples/package_demo.sans` — example showing import and usage of a package

- [ ] **Step 1: Update reference.md**

Add a "Package Manager" section documenting: `sans.json` format, all 7 commands with examples, the global cache at `~/.sans/packages/`, dependency resolution behavior, and the community index.

- [ ] **Step 2: Update ai-reference.md**

Add compact reference:
```
## Package Manager
sans pkg init [--name N --version V]   // create sans.json
sans pkg add <url> [tag]               // add dependency
sans pkg install                       // install all deps
sans pkg remove <url>                  // remove dependency
sans pkg list                          // list deps
sans pkg update <url> [tag]            // update dependency
sans pkg search <query>                // search index
```

- [ ] **Step 3: Update website, VSCode extension, README, examples**

- [ ] **Step 4: Commit**

```bash
git add docs/reference.md docs/ai-reference.md website/docs/index.html editors/vscode-sans/src/extension.ts README.md examples/package_demo.sans
git commit -m "docs: add package manager CLI documentation"
```

---

### Task 8: Final verification and PR

- [ ] **Step 1: Full test suite**

```bash
bash tests/run_tests.sh
```

Ensure no regressions.

- [ ] **Step 2: Manual smoke test of all commands**

```bash
mkdir /tmp/sans-pkg-test && cd /tmp/sans-pkg-test
sans pkg init
sans pkg init --name mylib --version 2.0.0  # should error (already exists)
cat sans.json
# Test with a real public repo if available, otherwise test error paths
sans pkg list
sans pkg search router  # may fail if index repo doesn't exist yet — that's ok
cd - && rm -rf /tmp/sans-pkg-test
```

- [ ] **Step 3: Push and create PR**

```bash
git push -u origin feat/package-manager-cli
```
