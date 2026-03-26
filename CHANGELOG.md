# Changelog

All notable changes to Sans are documented here.

## [0.8.5] - 2026-03-26

### Added
- Website playground page with editor, run, share, and snippet loading
- Mini-playground in homepage hero section
- "Try it" buttons on documentation examples
- Examples page on website
- Playground editor with syntax highlighting, line numbers, and resizable editors
- Windows x86_64 build in release workflow (via MSYS2/MinGW)

### Changed
- Website redesign: replace Catppuccin palette with professional design system
- Homepage messaging updated to "The language AI gets right on the first try"
- Benchmarks page reframed as "Comparisons"
- CI auto-updates `ai-reference.md` and `llms.txt` versions on release

### Fixed
- Playground examples updated to use valid Sans syntax with return types
- Various line number alignment and layout fixes across the website
- Mobile responsive code blocks
- Download page: removed line numbers from terminal command blocks, fixed build-from-source instructions

### Security
- Content-Security-Policy headers on all website pages
- Snippet ID format validation before API fetch
- Shell injection fix in `fuzz.yml` workflow_dispatch inputs

### Docs
- Security policy and best practices guide
- Community (Discussions) section in README
- Documentation synced across README, website, and reference files

## [0.8.3] - 2026-03-25

### Added
- Native 64-bit bitwise builtins: `bxor`, `band`, `bor`, `bshl`, `bshr` (single LLVM instructions)
- Runtime fuzz testing for JSON parser, string ops, and map operations with CI integration

### Security
- Replace djb2 hash with keyed FNV-1a in map and JSON runtime to prevent hash-flooding DoS

## [0.8.2] - 2026-03-25

### Added
- `pmutex_init()`, `pmutex_lock()`, `pmutex_unlock()` builtins for raw pthread mutex access
- Server configuration: `set_max_workers`, `set_read_timeout`, `set_keepalive_timeout`, `set_drain_timeout`, `set_max_body`, `set_max_headers`, `set_max_header_count`, `set_max_url`
- Bounded thread pool for HTTP/HTTPS servers — 503 rejection at capacity, graceful drain on shutdown
- Multi-read request assembly with timeouts and input validation

### Fixed
- Release bootstrap updated from v0.6.1 to latest release (v0.6.1 did not know Result unwrap `!` syntax)
- LLVM IR dominance error in multi-read request assembly
- `pkg.sans` updated to unwrap `json_parse` Result for v0.8.1 compatibility

## [0.8.1] - 2026-03-24

### Added
- JSON recursion depth limit (512) — returns error Result on overflow instead of stack overflow
- Memory safety audit report (`docs/memory-audit-v0.8.1.md`)

### Fixed
- `scope_should_keep` now walks JSON types (string/array/object) — prevents use-after-free when returning nested JSON from functions
- Result unwrap preserves inner IRTY type — `R<J>.unwrap()` now correctly dispatches JSON methods
- `map_delete` backward-shift to preserve linear probe chains — previously broke probing chain causing silent data loss

### Changed
- **Breaking:** `json_parse()` returns `Result<JsonValue>` instead of `JsonValue` — callers must unwrap with `!` or handle the error

## [0.8.0] - 2026-03-24

### Added
- Runtime bounds checking for array GET/SET — prints descriptive error and exits instead of silent corruption
- String `char_at` emits runtime error on out-of-bounds instead of silent empty string
- Panic recovery with `setjmp`/`longjmp` for error boundaries — HTTP server handlers catch unwrap failures instead of crashing
- New builtins: `setjmp`, `longjmp`, `panic_enable`, `panic_disable`, `panic_get_buf`, `panic_is_active`, `panic_fire`
- SIGPIPE handler in HTTP/HTTPS server accept loops — prevents server crash on client disconnect
- Fuzz testing harness for lexer/parser (Python generator + Bash runner, daily CI with 100K iterations)

## [0.7.4] - 2026-03-23

### Added
- Module re-exports via `pub import "mod"` — only `pub`-marked symbols re-exported
- Struct and tuple destructuring in match arms: `Point { x, y } => x + y`
- Per-iteration scope cleanup for `while`, `for-in`, and `for-in-destr` loops
- Depth guard for generic monomorphization (max 32)

### Fixed
- Loop scope frames popped on early return from inside loops (prevents double-free)
- Added `-no-pie` flag to Linux linker command

## [0.7.3] - 2026-03-23

### Added
- Compiler warnings: unused variables (skips `_`-prefixed), unreachable code after return
- `assert(cond)` and `assert_eq(a, b)` compiler builtins with source line reporting
- `assert_ne`, `assert_ok`, `assert_err`, `assert_some`, `assert_none` builtins
- Compatibility test suite: 10 frozen tests in `tests/compat/`

### Fixed
- Scope GC recursive keep: preserves nested containers at depth 2+; added `rc_kept_head` to track kept nodes

## [0.7.2] - 2026-03-22

### Added
- Trait objects (`dyn Trait`) with vtable dispatch: `expr as dyn Trait` coercion, fat pointers, indirect method calls
- `dyn TraitName` usable in type positions (function parameters, variables)
- Polymorphic arrays: `array<dyn Trait>()` with typed element indexing
- `defer` statement — LIFO execution on function return, works with early returns
- Channel `select` with timeout: `select { v = ch.recv() => body, timeout(ms) => body }`
- `pub` visibility modifier for module exports
- Import aliases: `import "http" as h`
- Dead code elimination via call graph analysis
- Constant folding for integer arithmetic on literals
- `sans test [dir]` command — discovers and runs `*_test.sans` files
- Negative test infrastructure (expected compilation failures)
- Test suite expanded to 250+ fixtures

### Fixed
- **Breaking:** `Array.find()` returns `Option<T>` instead of raw element
- Global variable init values read wrong AST offset — caused wrong values for curl constants (segfaults)
- Result combinator scope tracking: no double-tracking propagated results (was causing double-free)

## [0.7.1] - 2026-03-21

### Added
- `Option<T>` / `O<T>` type with `some(value)` and `none()` constructors
- Option methods: `is_some`, `is_none`, `unwrap`, `unwrap_or`, `!` unwrap, `?` try operator
- Generalized `Map<K,V>` / `M<K,V>` with integer key support: `M<I,I>`, `M<I,S>`, `M<S,S>`
- Result combinators: `map`, `and_then`, `map_err`, `or_else`

### Changed
- **Breaking:** `Map.get()` returns `Option<V>` instead of raw Int
- Opaque handles (`HttpServer`, `HttpResponse`, etc.) are now type-distinct from Int

## [0.7.0] - 2026-03-21

### Added
- Source location tracking (line:col) on all expression and statement AST node types
- Structured compiler diagnostics: `file:line:col: error: message` with source context and caret
- Multi-error diagnostic collection — compiler shows all diagnostics before exiting
- CI test workflow running on both macOS and Linux via `.ll` cross-compilation

### Fixed
- `random()` was a stub returning `max/2` — replaced with libc `rand()` seeded from `time()`
- JSON parser handles float values — `3.14` was previously truncated to `3`
- Closures with 3+ captures silently miscompiled — expanded to support up to 8 captures
- Platform-specific linker flags for Linux stack size

## [0.6.1] - 2026-03-20

### Fixed
- Strip `sh`/`time`/`random` from stage 2 bootstrap — v0.4.0 does not have them

### Security
- Command injection: quote all paths in shell commands (`llc`, `cc`, `rm`)
- Random temp paths for `sans run` output (prevents symlink attacks)
- Package manager validation switched from denylist to allowlist
- Scope GC globals made thread-local to prevent data corruption under concurrent load
- Fix CRLF header injection in HTTP responses
- Increase SSL read buffer and header capacity

## [0.6.0] - 2026-03-20

### Added
- Package manager CLI (`sans pkg`) with `init`, `add`, `install`, `remove`, `list`, `update`, `search`
- `sans.json` manifest format with dependency resolution and community index
- `sh()` builtin — execute shell command and capture stdout
- `listdir()`, `mkdir()`, `is_dir()`, `rmdir()`, `remove()`, `getenv()` builtins
- `J` type alias for `JsonValue` — preserves JSON type info across function boundaries
- `.keys()`, `.has()`, `.delete()` methods on `JsonValue`
- `scope_disable()` / `scope_enable()` builtins for IR data protection during compilation
- VSCode extension with complete hover docs and syntax highlighting
- Claude Code skills and workflow guides (architecture-review, PR review, skeptic-review, testing, planning)
- CI version-guard workflow

### Security
- Package manager input validation: allowlist-only characters, block flag injection and path traversal

## [0.5.4] - 2026-03-19

### Added
- **Short aliases for builtins**: `ab`/`aa`/`ae` (arena), `gz` (gzip), `ca` (cors_all), `ud` (url_decode), `ps` (path_segment), `sigh`/`sigc` (signals)
- **Short aliases for methods**: `idx` (index_of), `pl`/`pr` (pad_left/right), `ti` (to_int), `fm` (flat_map), `gidx`/`gs`/`geti`/`gb` (JSON getters), `typeof` (type_of), `sh`/`cl`/`rj` (HTTP request)
- **Documented 15 existing undocumented aliases**: `fread`, `fwrite`, `fappend`, `fexists`, `itos`, `jparse`, `jobj`, `jarr`, `jstr`, `jstringify`, `hget`, `hpost`, `hl`, `HS`, `HR`

## [0.5.3] - 2026-03-19

### Fixed
- Recursive promotion of nested container contents in scope GC — array of arrays no longer leaks
- Global pointer escape — heap values stored in globals no longer freed prematurely by scope_exit
- IR type tracking for JoinHandle parameters across function boundaries
- Cross-module capturing lambdas — heap-allocated closure objects preserve capture context
- Generic methods on generic structs: `impl Box<T> { get(self) ... }`
- Nested generics: `Box<Pair<I I>>` with recursive monomorphization

### Improved
- Default parameters now support negative literals: `f(x:I=-1)`
- For-loop destructuring supports N-element tuples: `for (a b c) in arr`

## [0.5.2] - 2026-03-19

### Added
- **Default function parameters** — trailing params can have `=literal` defaults: `f(x:I y:I=0) = x+y`
- **Error codes on Result** — `err(404 "not found")` with `.code()` method to retrieve the code
- **Pattern match guards** — `match x { n if n > 0 => "pos", _ => "other" }` with binding patterns
- **User-defined generic structs** — `struct Pair<A B> { first A, second B }` with `Pair<I S>{...}` instantiation
- **For-loop destructuring** — `for (k v) in m.entries() { ... }` to destructure tuples in iteration

## [0.5.1] - 2026-03-19

### Added
- Array: `sum`, `min`, `max`, `flat`
- String: `pad_left(width, fill)`, `pad_right(width, fill)`, `bytes`
- Map: `entries` — returns array of (key, value) tuples

### Fixed
- Cross-module struct field access — field order preserved from AST, struct names propagated through fn_ret_struct_names

## [0.5.0] - 2026-03-19

### Added
- **`match` expressions** on integers and strings: `match x { 1 => "one", _ => "other" }`
- **Tuple destructuring**: `let (a, b) = tuple_expr`
- **`stof(s)` / `string_to_float(s)`** — parse string to float via C strtod

### Fixed
- Lambda segfault: three bugs in capture detection and context inheritance

## [0.4.6] - 2026-03-19

### Added
- `abs(n)`, `min(a,b)`, `max(a,b)` — math built-in functions
- `s.char_at(i)` / `s.get(i)` — get single character as string
- `s.repeat(n)` — repeat string n times
- `a.slice(start, end)` — array slicing
- `a.reduce(init, fn)` — fold/reduce array to single value
- `a.each(fn)` / `a.for_each(fn)` — iterate with side effects
- `a.flat_map(fn)` — map + flatten
- `m.delete(key)` — remove key from map
- `n.to_str()` / `n.to_string()` — method syntax for int-to-string
- `s.to_int()` — method syntax for string-to-int
- `sleep(ms)` — pause execution
- `time()` / `now()` — current Unix timestamp
- `random(max)` / `rand(max)` — random integer in `[0, max)`
- Website custom domain sans.dev

### Fixed
- Removed hardcoded macOS ARM64 target triple — compiler now works cross-platform
- Linux CI builds via cross-compiled LLVM IR from macOS

## [0.4.5] - 2026-03-18

### Added
- `abs(n)`, `min(a,b)`, `max(a,b)` — math built-in functions
- `s.char_at(i)` — get single character as string
- `a.slice(start, end)` — array slicing
- `m.delete(key)` — map key removal

## [0.4.4] - 2026-03-18

### Added
- `s.upper()` / `s.lower()` — string case conversion
- `s.index_of(sub)` — find substring position (-1 if not found)
- `a.join(sep)` — join array elements into string with separator
- `a.reverse()` — reverse array in place
- CHANGELOG.md

## [0.4.3] - 2026-03-18

### Added
- `range(n)` and `range(a, b)` built-in functions for iteration
- `a.sort()` in-place insertion sort for integer arrays

## [0.4.2] - 2026-03-18

### Fixed
- Cross-module method calls (`utils.add()`) — pointer comparison replaced with map lookup
- Nested module imports (`import "models/user"`) — short name extraction for prefix matching
- Float type compatibility — `float_to_int()` and `float_to_string()` no longer reject Float arguments

## [0.4.1] - 2026-03-18

### Added
- **Automatic memory management** — type-tagged scope GC tracks all heap allocations (arrays, maps, JSON, Result, strings, structs, enums) with type-aware destructors
- Return value promotion — heap values returned from functions are automatically re-tracked in the caller's scope
- `fn_ret_types` propagation — return values from user functions with heap return types are scope-tracked

## [0.4.0] - 2026-03-17

### Added
- **Self-hosted compiler** — Sans compiles itself, Rust bootstrap compiler removed
- Reference counting runtime primitives (`rc_alloc`, `rc_retain`, `rc_release`, `rc_count`)
- Scope-based memory management functions (`scope_enter`, `scope_exit`, `scope_track`)
- Compiler auto-emits scope instrumentation for user code `alloc()` calls

## [0.3.0] - 2026-03-16

### Added
- HTTPS/TLS server support with certificate configuration (Rust bootstrap compiler)
- WebSocket protocol (upgrade, send, recv, close)
- Gzip compression for HTTP responses
- HTTP request headers, cookies, query params, form data
- CORS middleware helpers
- Signal handling and graceful shutdown
- Static file serving

## [0.2.0] - 2026-03-14

### Added
- Website and documentation at sans.dev
- Benchmark suite

## [0.1.0] - 2026-03-12

### Added
- Initial release — Rust bootstrap compiler targeting LLVM IR
- Core language: functions, variables, control flow, closures
- Types: Int, Float, Bool, String, Array, Map, Tuple, Struct, Enum, Result
- Concurrency: spawn, channels, mutexes
- HTTP client and server
- JSON parsing and building
- File I/O, logging, string operations
- Arena allocator for phase-based deallocation
- VSCode extension with syntax highlighting
