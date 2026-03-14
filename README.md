# Sans

A fast, compiled programming language designed for backend and API development. Sans compiles to native code via LLVM, producing standalone binaries with no runtime dependencies.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- LLVM 17: `brew install llvm@17`
- libcurl (included with macOS)

### Build

```sh
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo build --release
```

### Install

```sh
ln -sf $(pwd)/target/release/sans ~/.local/bin/sans
```

### Usage

```sh
sans build myfile.cy && ./myfile
```

## Hello World

```sans
fn add(a Int, b Int) Int {
    a + b
}

fn main() Int {
    let name = "world"
    print("Hello, " + name + "!")

    let sum = add(3, 4)
    print(int_to_string(sum))

    0
}
```

## Features

| Feature | Syntax |
|---|---|
| **Types** | `Int`, `Float`, `Bool`, `String` |
| **Variables** | `let x = 10` / `let mut y = 0` (type inference) |
| **Functions** | `fn add(a Int, b Int) Int { a + b }` |
| **If/Else** | `if x > 0 { ... } else { ... }` |
| **While loops** | `while i < 10 { ... }` |
| **For-in loops** | `for item in items { ... }` |
| **Match** | `match value { Ok(v) => ..., Err(e) => ... }` |
| **Structs** | `struct Point { x Int, y Int }` |
| **Enums** | `enum Color { Red, Green, Blue }` |
| **Traits** | `trait Display { fn show(self) String }` |
| **Generics** | `fn identity<T>(val T) T { val }` |
| **Arrays** | `let nums = [1, 2, 3]` with `map`, `filter` |
| **String methods** | `len`, `trim`, `split`, `starts_with`, `contains`, `substring` |
| **Modules/imports** | `import models/user` |
| **Concurrency** | `spawn`, channels (`chan_new`, `send`, `recv`), `mutex` |
| **File I/O** | `file_write`, `file_read`, `file_exists` |
| **JSON** | `json_parse`, `json_stringify`, `json_object`, `set`, `get` |
| **HTTP client** | `http_get`, `http_post` |
| **HTTP server** | `http_listen`, `accept`, `respond` |
| **Logging** | `log_debug`, `log_info`, `log_warn`, `log_error` |
| **Error handling** | `Result<T>` with `ok`, `err`, `unwrap`, `is_ok`, `is_err` |
| **Float math** | `float_sqrt`, `float_to_int`, `int_to_float` |

## HTTP Server Example

```sans
fn main() Int {
    log_set_level(0)
    print("Starting server on http://localhost:8080")
    let server = http_listen(8080)

    let mut count = 0
    while count < 20 {
        log_debug("waiting for request...")
        let req = server.accept()
        let path = req.path()
        log_info("request: " + path)

        if path == "/" {
            req.respond(200, "Hello from Sans!")
        } else {
            req.respond(404, "Not Found")
        }
        count = count + 1
    }
    0
}
```

## Running Tests

```sh
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test
```

## Architecture

The compiler is split into 6 Rust crates forming a pipeline:

1. **sans-lexer** -- tokenization
2. **sans-parser** -- AST construction
3. **sans-typeck** -- type checking and inference
4. **sans-ir** -- intermediate representation and lowering
5. **sans-codegen** -- LLVM IR generation
6. **sans-driver** -- CLI, linking, and orchestration

The runtime layer consists of 8 C files (`runtime/*.c`) providing built-in capabilities: strings, arrays, functional combinators, JSON, HTTP client/server, file I/O, logging, and error handling.

## Known Limitations

- No garbage collector -- all heap memory is leaked until process exit.
- No bounds checking on array access.
- Multiple opaque type method calls in complex expressions may crash.
- No lambda/closure syntax yet (use named function references).
