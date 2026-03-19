# Changelog

All notable changes to Sans are documented here.

## [0.5.0] - 2026-03-19

### Added
- **`match` expressions** on integers and strings: `match x { 1 => "one", _ => "other" }`
- **Tuple destructuring**: `let (a, b) = tuple_expr`
- **`stof(s)` / `string_to_float(s)`** — parse string to float via C strtod
- **Default parameters** — (partial: typeck allows, parser TBD)

### Fixed
- Lambda segfault: three bugs in capture detection and context inheritance
  - `find_captures_stmt` for ST_LET read wrong offset for value expression
  - `find_captures_expr` for EX_CALL treated function name as expression pointer
  - Lambda context didn't inherit local_fn_set/module_name/imported_fn_names
- Nested lambdas, inline lambdas with function calls, reduce/each/flat_map all work

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
- Reference counting runtime primitives (`rc_alloc`, `rc_retain`, `rc_release`, `rc_count`)
- Scope-based memory management functions (`scope_enter`, `scope_exit`, `scope_track`)
- Compiler auto-emits scope instrumentation for user code `alloc()` calls

## [0.3.0] - 2026-03-16

### Added
- HTTPS/TLS server support with certificate configuration
- WebSocket protocol (upgrade, send, recv, close)
- Gzip compression for HTTP responses
- HTTP request headers, cookies, query params, form data
- CORS middleware helpers
- Signal handling and graceful shutdown
- Static file serving

## [0.2.0] - 2026-03-14

### Added
- Self-hosted compiler — Sans compiles itself, Rust compiler removed
- Website and documentation at sans.dev

## [0.1.0] - 2026-03-12

### Added
- Initial release — self-hosted compiler targeting LLVM IR
- Core language: functions, variables, control flow, closures
- Types: Int, Float, Bool, String, Array, Map, Tuple, Struct, Enum, Result
- Concurrency: spawn, channels, mutexes
- HTTP client and server
- JSON parsing and building
- File I/O, logging, string operations
- Arena allocator for phase-based deallocation
- VSCode extension with syntax highlighting
