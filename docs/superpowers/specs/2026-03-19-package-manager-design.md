# Sans Package Manager — Design Spec

## Overview

A built-in, scoped package manager for Sans using git-based packages, decentralized hosting, and a community index file. Packages are git repos identified by their URL (Go-style), pinned to git tags for versioning, and cached globally at `~/.sans/packages/`.

**Goals:**
- `sans build` auto-resolves and fetches missing dependencies
- Zero new infrastructure for v1 — git repos + a JSON index on GitHub
- Minimal new language primitives — thin libc wrappers that are generally useful

**Non-goals for v1:**
- Registry backend / web API (future — evolve from index file to JSON-file registry to SQLite-backed)
- Private packages / authentication
- Lock files (git tags are already immutable)
- Binary package distribution

---

## Architecture

### Three components:

1. **Language primitives** — 7 new builtins added to the compiler (`getenv`, `mkdir`, `rmdir`, `remove`, `listdir`, `is_dir`, `sh`/`shell`)
2. **Package manager CLI** — `sans pkg` subcommand, written in Sans, handles init/add/install/remove/list
3. **Compiler integration** — `sans build` resolves `import "github.com/user/pkg"` from `~/.sans/packages/`

### Existing builtins the package manager relies on:

The package manager uses these already-implemented builtins extensively:
- **File I/O:** `file_read`, `file_write`, `file_exists` — reading/writing `sans.json`, checking cache
- **JSON:** `json_parse`, `json_object`, `json_string`, `json_set`, `json_get`, `json_stringify` — parsing and writing `sans.json`
- **HTTP:** `http_get` — fetching the community index from GitHub
- **Strings:** `split`, `starts_with`, `contains`, `trim`, `replace` — parsing git output, URL handling
- **Arrays:** `push`, `len`, `get`, `sort` — dependency collection and version sorting
- **System:** `system()` — running git commands where exit code matters (clone verification)
- **I/O:** `print` — CLI output

### Package identification:

Packages are identified by git URL path:
```
github.com/user/http-router
github.com/org/json-schema
gitlab.com/user/crypto
```

Versions are git tags following semver: `v1.0.0`, `v0.3.2`, etc.

---

## Part 1: New Language Primitives

Seven new builtins, all thin libc/popen wrappers. These go through the standard pipeline: typeck -> constants -> IR -> codegen -> runtime.

### 1.1 `getenv(name: String) -> String`

Read environment variable. Returns `""` if not set.

```
home = getenv("HOME")         // "/Users/sgordon"
path = getenv("SANS_PATH")    // "" if unset
```

**Implementation:** Wraps libc `getenv()`. Returns empty string (not null) when variable is unset.

**Aliases:** `genv`

### 1.2 `mkdir(path: String) -> Int`

Create directory, including parent directories (like `mkdir -p`). Returns 1 on success, 0 on error.

```
mkdir("/Users/sgordon/.sans/packages")
mkdir("src/lib")
```

**Implementation:** Iteratively creates each path component using libc `mkdir(path, 0755)`. Ignores `EEXIST` errors (idempotent).

### 1.3 `rmdir(path: String) -> Int`

Remove an empty directory. Returns 1 on success, 0 on error.

```
rmdir("build/tmp")
```

**Implementation:** Wraps libc `rmdir()`.

### 1.4 `remove(path: String) -> Int`

Delete a file. Returns 1 on success, 0 on error.

```
remove("/tmp/old-cache.json")
```

**Implementation:** Wraps libc `remove()`.

**Aliases:** `rm`

### 1.5 `listdir(path: String) -> Array<String>`

List directory contents. Returns array of filenames (excluding `.` and `..`). Returns empty array on error.

```
files = listdir("src/")       // ["main.sans" "lib.sans" "util.sans"]
```

**Implementation:** Wraps `opendir()` / `readdir()` / `closedir()`. Filters `.` and `..`.

**Aliases:** `ls`

### 1.6 `is_dir(path: String) -> Bool`

Check if path is a directory. Returns `false` for files, missing paths, or errors.

```
is_dir("/Users/sgordon/.sans")   // true
is_dir("main.sans")              // false
```

**Implementation:** Wraps `stat()`, checks `S_ISDIR(st_mode)`.

### 1.7 `sh(cmd: String) -> String`

Execute shell command and capture stdout as a string. Returns `""` if `popen()` fails. Unlike `system()` which returns only the exit code, `sh()` captures the command's output.

```
output = sh("git tag --list 'v*'")
branch = sh("git rev-parse --abbrev-ref HEAD").trim()
ver = sh("git describe --tags")
```

**Implementation:** Wraps `popen(cmd, "r")`, reads all output into a dynamically grown string buffer, calls `pclose()`.

**Distinguishing from `system()`:** `system(cmd)` returns the exit code (Int) and discards output. `sh(cmd)` returns stdout (String) and discards the exit code. For cases where you need both, use `sh()` for output and check for empty string / validate the result.

**Failure detection for git operations:** The package manager validates `sh()` results by checking:
- Empty string means the command failed or produced no output
- After `git clone`, verify the target directory exists via `is_dir()`
- After `git ls-remote --tags`, parse the output for tag lines

**Aliases:** `shell`

---

## Part 1b: CLI Subcommand Architecture

### How `sans pkg` works:

The `sans` binary's `main.sans` already dispatches on `args()` for `build`, `run`, etc. The `pkg` subcommand is added as a new dispatch branch:

```
a = args()
if a.len > 1 {
  cmd = a.get(1)
  if cmd == "build" { ... }
  if cmd == "pkg" { pkg_main(a) }
}
```

`pkg_main()` lives in a new file `compiler/pkg.sans` that is compiled into the `sans` binary alongside the existing compiler modules. It handles its own sub-dispatch:

```
pkg_main(a: [S]) {
  if a.len < 3 { pkg_help(); return 0 }
  sub = a.get(2)
  if sub == "init" { pkg_init(a) }
  if sub == "add" { pkg_add(a) }
  if sub == "install" { pkg_install(a) }
  if sub == "remove" { pkg_remove(a) }
  if sub == "list" { pkg_list(a) }
  if sub == "update" { pkg_update(a) }
  if sub == "search" { pkg_search(a) }
}
```

This is a single file addition — no new binary, no separate build step. The package manager ships inside the compiler binary.

### Writing `sans.json`:

Sans already has `json_stringify()` for serializing JsonValue to string, and `file_write()` for writing to disk. The package manager reads, modifies, and writes `sans.json` like this:

```
// Read
content = file_read("sans.json")
pkg = json_parse(content)

// Modify
deps = pkg.get("deps")
deps.set("github.com/user/lib" json_string("v1.0.0"))

// Write back
file_write("sans.json" jfy(pkg))
```

`json_stringify` produces compact JSON. For v1 this is acceptable — the file is machine-managed. Pretty-printing is a future enhancement.

### Bootstrap consideration:

Phase 1 adds new builtins to the compiler source. Since the compiler is self-hosted, building the modified compiler requires the existing bootstrap binary. The workflow is:

1. Edit `compiler/typeck.sans`, `constants.sans`, `ir.sans`, `codegen.sans` to add builtins
2. Add runtime implementations in `runtime/` files
3. Build with the current bootstrap binary: `sans build compiler/main.sans`
4. The resulting binary now supports the new builtins
5. Phase 2 code (`compiler/pkg.sans`) uses the new builtins and is compiled by the Phase 1 binary

---

## Part 2: Package Manifest — `sans.json`

Every Sans project with dependencies has a `sans.json` in its root:

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "description": "A web application",
  "deps": {
    "github.com/sans-lang/http-router": "v1.0.0",
    "github.com/sans-lang/json-schema": "v0.2.1"
  }
}
```

### Fields:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Package name (used in index) |
| `version` | yes | Semver version of this package |
| `description` | no | One-line description |
| `deps` | no | Map of git URL -> git tag |
| `entry` | no | Entry point file, defaults to `lib.sans` for libraries |

### For library packages:

Library packages use `entry` to declare their public module. Defaults to `lib.sans` if omitted. If neither `entry` is set nor `lib.sans` exists, `sans pkg install` errors with a clear message:

```
ERROR: github.com/user/pkg has no entry point
  No "entry" field in sans.json and no lib.sans found
  Add "entry": "your_file.sans" to sans.json
```

Example with explicit entry:

```json
{
  "name": "http-router",
  "version": "1.0.0",
  "entry": "router.sans"
}
```

---

## Part 3: Global Package Cache

### Directory structure:

```
~/.sans/
  packages/
    github.com/
      sans-lang/
        http-router/
          v1.0.0/           # checked out at tag v1.0.0
            sans.json
            router.sans
            ...
          v1.1.0/           # different version, separate directory
            ...
      user/
        json-schema/
          v0.2.1/
            ...
  index-cache.json           # cached copy of the community index
```

### Key decisions:

- **Each version gets its own directory.** No git checkout switching — `v1.0.0/` and `v1.1.0/` coexist. This avoids conflicts when multiple projects depend on different versions.
- **Installed via:** `git clone --depth 1 --branch <tag> <url> <path>` — shallow clone at the specific tag, minimal disk usage.
- **Cache is shared** across all projects. If `v1.0.0` is already cached, it's reused.
- **No automatic cache cleanup in v1.** The cache grows as new versions are installed. Users can manually `rm -rf ~/.sans/packages/` to clear it. A `sans pkg clean` command is a future enhancement.

---

## Part 4: Dependency Resolution

### Algorithm:

Uses a `resolved` map (URL -> version) and a `queue` (array of `(url, version, requester)` tuples):

1. Read `sans.json` from project root
2. Add all direct deps to the queue with requester = "project"
3. While queue is not empty:
   a. Pop `(url, version, requester)` from front
   b. If `url` is already in `resolved`:
      - If `resolved[url] == version`, skip (already satisfied)
      - If `resolved[url] != version`, **error** with conflict details (see below)
   c. Add `resolved[url] = version`
   d. Check if `~/.sans/packages/<url>/<version>/` exists; if not, clone it
   e. Read the dep's `sans.json`; add its deps to the queue with requester = `url`
4. Return the `resolved` map

This runs during both `sans pkg install` and `sans build` (auto-install). The same resolution function is shared.

### Version conflict strategy (v1):

Fail fast with a clear error, including the full dependency chain:

```
ERROR: Version conflict for github.com/sans-lang/json-utils
  project -> github.com/user/pkg-a requires v1.0.0
  project -> github.com/user/pkg-b requires v1.2.0

Fix: Align both to the same version in sans.json
```

Future versions can add semver range resolution (`^1.0.0`, `~1.2`), but pinned tags are simpler and more predictable for v1.

---

## Part 5: Compiler Integration

### Project root discovery:

The compiler finds the project root by walking up from the directory containing the target file (the argument to `sans build`), looking for `sans.json`. If no `sans.json` is found, the project has no dependencies and import resolution works as it does today (relative paths only).

```
sans build src/main.sans
  -> looks for src/sans.json
  -> looks for ./sans.json       <-- found, this is the project root
```

The project root is determined once at the start of compilation and stored for use during import resolution.

### Import resolution:

When the compiler encounters `import "github.com/user/pkg"`:

1. **URL detection:** Check if the import path contains a dot before the first `/` (e.g., `github.com/...`, `gitlab.com/...`). This distinguishes URL imports from relative imports like `import "utils"`.
2. **Version lookup:** Read the project's `sans.json` to find the pinned version for this URL. Error if the URL is not in `deps`.
3. **Path resolution:** Map to `~/.sans/packages/github.com/user/pkg/<version>/<entry>`, where `<entry>` comes from the package's `sans.json` (defaults to `lib.sans`).
4. **Auto-install:** If the package directory doesn't exist, run the install (clone at tag). This is the same logic as `sans pkg install` for a single package.
5. **Compile and link:** Parse and compile the imported module. Its exported symbols enter the importing module's scope.

### Transitive dependency resolution during build:

When compiling a package that itself has imports, the compiler reads *that package's* `sans.json` to resolve its dependencies. Each package's `sans.json` is the authority for its own dependency versions:

```
Project sans.json:     deps: { "github.com/a/pkg": "v1.0.0" }
pkg v1.0.0/sans.json:  deps: { "github.com/b/util": "v0.3.0" }
```

The compiler resolves `github.com/b/util` using pkg's `sans.json`, not the project's. This chaining happens naturally during recursive compilation.

### Auto-install on build:

`sans build` runs the Part 4 dependency resolution algorithm **first**, producing a flat `resolved` map of all dependencies (direct + transitive) with their versions. Any missing packages are installed during this phase. The compiler's import resolution in step 2-3 above then simply looks up versions from this pre-resolved map — it does not re-resolve or re-read `sans.json` files during compilation.

No separate `sans pkg install` step needed (though the explicit command still exists for pre-fetching).

### Import syntax in Sans code:

```
import "github.com/sans-lang/http-router"

// Functions from http-router are now available
route = create_router()
```

### Symbol collision behavior (v1):

Imported packages dump their public symbols into the caller's namespace (same as current `import "mod"` behavior). If two packages define a function with the same name, the **last import wins** (later import overwrites earlier). The compiler emits a warning:

```
WARNING: symbol 'parse' from github.com/b/json shadows
         symbol 'parse' from github.com/a/utils
```

Namespaced imports (e.g., `import router "github.com/user/http-router"` then `router.handle()`) are a future enhancement that fully resolves this.

---

## Part 6: CLI Commands

All commands are subcommands of `sans pkg`:

### `sans pkg init`

Create a `sans.json` in the current directory with default values. Non-interactive — uses the current directory name as the package name and `0.1.0` as the version. Override with flags:

```
$ sans pkg init
Created sans.json (my-app v0.1.0)

$ sans pkg init --name my-lib --version 1.0.0
Created sans.json (my-lib v1.0.0)
```

### `sans pkg add <url> [tag]`

Add a dependency. If tag is omitted, fetches the latest tag from the remote.

```
$ sans pkg add github.com/sans-lang/http-router
Resolved latest: v1.2.0
Added github.com/sans-lang/http-router v1.2.0
Installing to ~/.sans/packages/github.com/sans-lang/http-router/v1.2.0/
Done.
```

Steps:
1. If no tag specified: `git ls-remote --tags <url>` to find latest semver tag
2. Add to `sans.json` deps
3. Clone to cache if not already present
4. Resolve transitive deps and install those too

### `sans pkg install`

Install all dependencies from `sans.json` (and their transitive deps).

```
$ sans pkg install
Installing github.com/sans-lang/http-router v1.2.0... cached
Installing github.com/sans-lang/json-schema v0.2.1... cloning
Done. 2 packages installed.
```

### `sans pkg remove <url>`

Remove a dependency from `sans.json`. Does NOT delete from global cache (other projects may use it).

```
$ sans pkg remove github.com/sans-lang/http-router
Removed github.com/sans-lang/http-router from sans.json
```

### `sans pkg list`

List all dependencies (direct and transitive) with their versions.

```
$ sans pkg list
github.com/sans-lang/http-router  v1.2.0  (direct)
github.com/sans-lang/json-schema  v0.2.1  (direct)
github.com/sans-lang/string-utils v0.1.0  (transitive, via http-router)
```

### `sans pkg update <url> [tag]`

Update a dependency to a new tag. If no tag given, update to latest.

```
$ sans pkg update github.com/sans-lang/http-router
Updated github.com/sans-lang/http-router v1.2.0 -> v1.3.0
```

---

## Part 7: Community Index

A JSON file hosted at `github.com/sans-lang/packages` (the repo itself):

### `index.json`:

```json
{
  "packages": {
    "http-router": {
      "url": "github.com/sans-lang/http-router",
      "description": "URL routing for Sans HTTP servers",
      "latest": "v1.2.0"
    },
    "json-schema": {
      "url": "github.com/sans-lang/json-schema",
      "description": "JSON schema validation",
      "latest": "v0.2.1"
    }
  }
}
```

### `sans pkg search <query>`

Search the community index by name or description:

```
$ sans pkg search router
github.com/sans-lang/http-router  v1.2.0  "URL routing for Sans HTTP servers"
github.com/user/ws-router         v0.1.0  "WebSocket routing middleware"
```

### Usage:

- `sans pkg search <query>` searches the index by name/description
- `sans pkg add http-router` (short name) resolves via index to full git URL
- Index is cached locally at `~/.sans/index-cache.json`, refreshed on `sans pkg search` or `sans pkg add` with a short name
- Index is fetched via HTTP from the raw GitHub URL

### Package submission:

For v1, submit a PR to the `sans-lang/packages` repo adding your package to `index.json`. Simple, transparent, community-reviewed.

---

## Part 8: Implementation Phases

### Phase 1: Language Primitives
Add the 7 new builtins: `getenv`, `mkdir`, `rmdir`, `remove`, `listdir`, `is_dir`, `sh`. Each goes through the full pipeline (typeck -> constants -> IR -> codegen -> runtime). Test fixtures for each.

### Phase 2: Package Manager CLI
Build `sans pkg` commands in Sans itself. This requires the builtins from Phase 1 plus existing HTTP/JSON/string capabilities. Write as a module that ships with the compiler.

### Phase 3: Compiler Integration
Modify the compiler's import resolution to handle URL-style imports, read `sans.json`, and auto-install missing packages. This touches `compiler/lexer.sans` (or `parser.sans`) for import path parsing and adds dependency resolution logic.

### Phase 4: Community Index
Set up the `sans-lang/packages` repo with the initial `index.json`, add `sans pkg search` command, and document the submission process.

### Phase 5: Documentation & Tooling
Update all docs (reference.md, ai-reference.md, website, VSCode extension) per the documentation checklist. Add examples showcasing package usage.

---

## Open Questions for Future Versions

- **Namespaced imports:** `import router "github.com/user/http-router"` then `router.handle()`
- **Semver ranges:** `^1.0.0` instead of exact tag pins
- **Lock files:** `sans.lock` for reproducible builds
- **Private packages:** Auth tokens for private git repos
- **Registry backend:** Replace index file with a proper Sans-powered registry API
- **Pre/post install scripts:** Run setup commands after cloning
- **Workspaces/monorepos:** Multiple packages in one repo
