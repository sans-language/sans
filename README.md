# Sans

[![Release](https://img.shields.io/github/v/release/sans-language/sans?style=flat-square)](https://github.com/sans-language/sans/releases)
[![License](https://img.shields.io/github/license/sans-language/sans?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-blue?style=flat-square)]()

The language AI gets right on the first try. A fast, compiled language with 12 types, 40 keywords, and everything built in. Compiles to native code via LLVM — standalone binaries, no runtime dependencies.

## Quick Start

### Install (macOS)

```sh
curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-macos-arm64.tar.gz | tar xz && sudo mv sans /usr/local/bin/
```

Works on Apple Silicon and Intel (via Rosetta). Requires Xcode CLT and LLVM 17:

```sh
xcode-select --install
brew install llvm@17
```

### Install (Linux x86_64)

```sh
curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-linux-x86_64.tar.gz | tar xz && sudo mv sans /usr/local/bin/
```

Requires LLVM 17, gcc/clang, libcurl, libssl:

```sh
sudo apt install llvm-17 build-essential libcurl4-openssl-dev libssl-dev
```

See the [download page](https://sans-language.github.io/sans/download/) for more options.

### Usage

```sh
sans build myfile.sans   # compile to ./myfile
sans run myfile.sans     # compile + run, no output file
sans --version
```

### Build from source

Requires: LLVM 17 and a previous `sans` binary (for bootstrapping).

```sh
brew install llvm@17
git clone https://github.com/sans-language/sans && cd sans
sans build compiler/main.sans
sudo cp sans /usr/local/bin/
```

## Hello World

```sans
add(a:I b:I) I = a + b

main() {
    name = "world"
    p("Hello, {name}!")
    p(str(add(3 4)))
    0
}
```

## Features

| Feature | Syntax |
|---|---|
| **Types** | `I` (Int), `F` (Float), `B` (Bool), `S` (String), `J` (Json), `M<K,V>` (Map), `O<T>` (Option), `R<T>` (Result) |
| **Variables** | `x = 42` (immutable) / `x := 0` (mutable) / `g x = 0` (global) |
| **Functions** | `add(a:I b:I) I = a + b` (compact) or `fn add(a Int, b Int) Int { a + b }` — default params: `f(x:I y:I=0)` |
| **Lambdas** | `\|x:I\| I { x * 2 }` with implicit capture from enclosing scope (up to 8 variables) |
| **If/Else** | `if x > 0 { ... } else { ... }` or ternary `cond ? a : b` |
| **Loops** | `while cond { }`, `for item in arr { }`, `for (k v) in m.entries()`, `break`, `continue` |
| **Match** | `match value { Enum::A => ..., Enum::B(x) => x }` — guards: `n if n > 0 => ...` |
| **Structs** | `struct Point { x I, y I }` — generic: `struct Pair<A B> { first A, second B }` |
| **Enums** | `enum Color { Red, Green, Blue(I) }` |
| **Traits** | `trait Display { fn show(self) I }` |
| **Trait objects** | `x as dyn Trait` — dynamic dispatch via vtable; `dyn Trait` as parameter/variable type |
| **Generics** | `identity<T>(x T) T = x` — generic structs: `Pair<I S>{first: 1, second: "hi"}` |
| **Tuples** | `(1 "hello" true)` with `.0`, `.1` access |
| **Arrays** | `[1 2 3]` with `map`, `filter`, `any`, `find` (returns `Option<T>`), `enumerate`, `zip` |
| **Option** | `Option<T>` / `O<T>` — `some(v)`, `none()`, `.is_some`, `.unwrap_or(d)`, `opt!`, `opt?` |
| **Maps** | `M<K,V>()` (default `M<S,I>`) with `set`, `get` (returns `Option<V>`), `has`, `keys`, `vals` |
| **String methods** | `len`, `trim`, `split`, `starts_with`, `contains`, `replace`, `[0:5]` slicing |
| **String interpolation** | `"Hello {name}!"` with expression support `"{x + 1}"` |
| **Modules** | `import "math"`, `pub import "mod"` (re-exports) |
| **Package Manager** | `sans pkg init`, `sans pkg add <url>`, `sans pkg install`, `sans pkg remove <url>` |
| **Concurrency** | `spawn`, channels (`channel<I>()`, `send`, `recv`), `mutex` |
| **File I/O** | `file_read`/`fr`, `file_write`/`fw`, `file_exists`/`fe` |
| **Filesystem** | `mkdir`, `rmdir`, `remove`/`rm`, `listdir`/`ls`, `is_dir`, `getenv`/`genv` |
| **Process** | `sh`/`shell` (capture stdout), `system`/`sys` (exit code) |
| **JSON** | `json_parse`/`jp` returns `Result<JsonValue>` (handles floats, objects, arrays, strings, ints, bools, null; depth limit 512), `json_stringify`/`jfy`, `json_object`/`jo` |
| **HTTP client** | `http_get`/`hg`, `http_post`/`hp` |
| **HTTP server** | `serve(port handler)` with bounded thread pool, request timeouts, graceful shutdown, input validation (body/header/URL limits) |
| **HTTPS/TLS** | `serve_tls(port cert key handler)`, `https_listen` |
| **WebSocket** | `upgrade_ws`, `ws_send`, `ws_recv`, `ws_close` |
| **CORS** | `cors(req origin)`, `cors_all(req)` |
| **Streaming** | `respond_stream(status)`, `stream_write`, `stream_end` |
| **Static files** | `serve_file(req dir)` with content-type detection |
| **Logging** | `log_debug`/`ld`, `log_info`/`li`, `log_warn`/`lw`, `log_error`/`le` |
| **Error handling** | `Result<T>` with `ok`, `err(msg)`/`err(code msg)`, `?` propagation, `!` unwrap, `.code()`, `.map()`, `.and_then()`, `.map_err()`, `.or_else()` |
| **Low-level** | `alloc`, `load8`/`store8`, `mcpy`, sockets, curl, SSL, arena allocator, `pmutex_init`/`pmutex_lock`/`pmutex_unlock` |
| **Assertions** | `assert`, `assert_eq`, `assert_ne`, `assert_ok`, `assert_err`, `assert_some`, `assert_none` — line numbers in failure messages |
| **Memory Safety** | Scope-based GC walks nested JSON types on return (no use-after-free); `json_parse` returns `Result<JsonValue>` with descriptive errors; JSON depth limit (512) prevents stack overflow |
| **Runtime Safety** | Array/string bounds checking (exits with error on out-of-bounds); SIGPIPE ignored in HTTP servers; panic recovery via `setjmp`/`longjmp` (`panic_enable`, `panic_disable`, `panic_get_buf`, `panic_fire`) |
| **Diagnostics** | `file:line:col: error: message` with source context, caret, multi-error reporting, and warnings |

## HTTP Server Example

```sans
handle(req:HR) I {
  path = req.path()
  path == "/" ? req.respond(200 "Hello from Sans!") : req.respond(404 "Not Found")
}

main() I {
  p("Starting server on http://localhost:8080")
  serve(8080 fptr("handle"))
}
```

Production-ready: bounded thread pool (default 256 workers), HTTP/1.1 keep-alive, gzip compression, request timeouts, input validation, graceful shutdown (SIGINT/SIGTERM).

## Running Tests

```sh
bash tests/run_tests.sh
```

E2E tests live in `tests/fixtures/`. Each fixture is a `.sans` file with an expected output comment at the top.

## Architecture

The compiler pipeline: **lexer → parser → typeck → IR → codegen → LLVM**. Seven modules in `compiler/`, 13+ runtime modules in `runtime/`. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full architecture overview.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to set up the development environment, add features, and submit pull requests. AI agents: read [CLAUDE.md](CLAUDE.md) for the complete rule set.

## Known Limitations

- **Scope GC**: Automatic scope-based memory management frees heap allocations on function return (including nested container contents). The compiler itself must be built from the bootstrap binary. Thread safety of scope globals is not guaranteed.
