# Sans

A fast, compiled programming language designed for backend and API development. Sans compiles to native code via LLVM, producing standalone binaries with no runtime dependencies. **Sans is self-hosted** -- the compiler's runtime is written entirely in Sans itself.

## Quick Start

### Install (macOS)

```sh
curl -fsSL https://github.com/sans-language/sans/releases/latest/download/sans-macos-$(uname -m).tar.gz | tar xz && sudo mv sans /usr/local/bin/
```

Requires Xcode Command Line Tools (`xcode-select --install`) for the system linker. If macOS blocks the binary, run `xattr -d com.apple.quarantine /usr/local/bin/sans`.

See the [download page](https://sans-language.org/download) for manual downloads and build-from-source instructions.

### Usage

```sh
sans build myfile.sans   # compile to ./myfile
sans run myfile.sans     # compile + run, no output file
sans --version
```

### Build from source

Requires: Rust (stable), LLVM 17.

```sh
brew install llvm@17
git clone https://github.com/sans-language/sans && cd sans
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build --release
sudo cp target/release/sans /usr/local/bin/
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
| **Types** | `I` (Int), `F` (Float), `B` (Bool), `S` (String), `M` (Map), `R<T>` (Result) |
| **Variables** | `x = 42` (immutable) / `x := 0` (mutable) / `g x = 0` (global) |
| **Functions** | `add(a:I b:I) I = a + b` (compact) or `fn add(a Int, b Int) Int { a + b }` |
| **Lambdas** | `\|x:I\| I { x * 2 }` with implicit capture from enclosing scope |
| **If/Else** | `if x > 0 { ... } else { ... }` or ternary `cond ? a : b` |
| **Loops** | `while cond { }`, `for item in arr { }`, `break`, `continue` |
| **Match** | `match value { Enum::A => ..., Enum::B(x) => x }` |
| **Structs** | `struct Point { x I, y I }` |
| **Enums** | `enum Color { Red, Green, Blue(I) }` |
| **Traits** | `trait Display { fn show(self) I }` |
| **Generics** | `identity<T>(x T) T = x` |
| **Tuples** | `(1 "hello" true)` with `.0`, `.1` access |
| **Arrays** | `[1 2 3]` with `map`, `filter`, `any`, `find`, `enumerate`, `zip` |
| **Maps** | `M()` with `set`, `get`, `has`, `keys`, `vals` |
| **String methods** | `len`, `trim`, `split`, `starts_with`, `contains`, `replace`, `[0:5]` slicing |
| **String interpolation** | `"Hello {name}!"` with expression support `"{x + 1}"` |
| **Modules** | `import "math"` |
| **Concurrency** | `spawn`, channels (`channel<I>()`, `send`, `recv`), `mutex` |
| **File I/O** | `file_read`/`fr`, `file_write`/`fw`, `file_exists`/`fe` |
| **JSON** | `json_parse`/`jp`, `json_stringify`/`jfy`, `json_object`/`jo` |
| **HTTP client** | `http_get`/`hg`, `http_post`/`hp` |
| **HTTP server** | `serve(port handler)` with auto-threading, keep-alive, auto-gzip, graceful shutdown |
| **HTTPS/TLS** | `serve_tls(port cert key handler)`, `https_listen` |
| **WebSocket** | `upgrade_ws`, `ws_send`, `ws_recv`, `ws_close` |
| **CORS** | `cors(req origin)`, `cors_all(req)` |
| **Streaming** | `respond_stream(status)`, `stream_write`, `stream_end` |
| **Static files** | `serve_file(req dir)` with content-type detection |
| **Logging** | `log_debug`/`ld`, `log_info`/`li`, `log_warn`/`lw`, `log_error`/`le` |
| **Error handling** | `Result<T>` with `ok`, `err`, `?` propagation, `!` unwrap |
| **Low-level** | `alloc`, `load8`/`store8`, `mcpy`, sockets, curl, SSL, arena allocator |

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

Production-ready: auto-threading, HTTP/1.1 keep-alive, gzip compression, graceful shutdown (SIGINT/SIGTERM).

## Running Tests

```sh
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test
```

E2E tests live in `crates/sans-driver/tests/e2e.rs` with fixtures in `tests/fixtures/`. Each fixture is a `.sans` file with an expected output comment at the top.

## Architecture

The compiler is split into 6 Rust crates forming a pipeline:

1. **sans-lexer** -- tokenization
2. **sans-parser** -- AST construction
3. **sans-typeck** -- type checking and inference
4. **sans-ir** -- intermediate representation and lowering
5. **sans-codegen** -- LLVM IR generation
6. **sans-driver** -- CLI, linking, and orchestration

### Self-Hosted Compiler

The `compiler/` directory contains a **self-hosted Sans compiler** (~11,600 LOC across 7 modules) that can compile Sans programs. This demonstrates Sans's expressiveness -- the compiler is written entirely in the language it compiles.

### Self-Hosted Runtime

The runtime is **100% self-hosted** -- written entirely in Sans, with zero C files remaining. Built-in capabilities (strings, arrays, maps, JSON, HTTP, file I/O, logging, error handling) are implemented using Sans's low-level primitives (`alloc`, `load8`/`store8`, `mcpy`, sockets, curl bindings, etc.).

## Known Limitations

- No garbage collector -- use `arena_begin()`/`arena_alloc(n)`/`arena_end()` for phase-based bulk deallocation.
- No bounds checking on array access.
- Multiple opaque type method calls in complex expressions may crash.
