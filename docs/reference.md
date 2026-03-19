# Sans Language Reference

Sans is a compiled programming language optimized for AI code generation. It compiles to native code via LLVM with performance comparable to C, Rust, and Go.

## File Extension

`.sans`

## Quick Example

```sans
divide(a:I b:I) R<I> = b == 0 ? err("div/0") : ok(a / b)

main() {
    nums = [1 2 3 4 5]
    total := 0
    for n in nums { total += n }
    result = divide(total 3)!
    p(str(result))
    result
}
```

---

## Types

| Type | Short | Description |
|------|-------|-------------|
| `Int` | `I` | 64-bit signed integer |
| `Float` | `F` | 64-bit floating point |
| `Bool` | `B` | Boolean (`true` / `false`) |
| `String` | `S` | UTF-8 string |
| `Array<T>` | — | Dynamic growable array |
| `Map` | `M` | Hash map with string keys |
| `Result<T>` | `R<T>` | Success or error value |
| `JsonValue` | — | Opaque JSON value |
| `HttpResponse` | — | HTTP client response |
| `HttpServer` | — | HTTP server socket |
| `HttpRequest` | — | HTTP server request |
| `Sender<T>` | — | Channel sender |
| `Receiver<T>` | — | Channel receiver |
| `Mutex<T>` | — | Mutual exclusion lock |
| `JoinHandle` | — | Thread handle |

User-defined types: `struct`, `enum`, `trait`.

---

## Variable Declaration

```sans
x = 42              // immutable (type inferred)
x := 0              // mutable
let x Int = 42      // explicit type (verbose, optional)
let mut x = 0       // verbose mutable (optional)
```

## Global Variables

```sans
g counter = 0       // global mutable variable

inc() I {
  counter = counter + 1
  counter
}
```

Globals are mutable and accessible from any function. Declared at the top level with `g`.

## Function Definition

```sans
// Full form
fn add(a Int, b Int) Int { a + b }

// Compact form (preferred)
add(a:I b:I) I = a + b

// Implicit return type (defaults to Int)
main() { 0 }
```

### Default Parameters

Trailing parameters can have default values using `=literal`:

```sans
greet(name:S greeting:S="Hello") S = "{greeting} {name}!"

main() {
    p(greet("Alice"))            // "Hello Alice!"
    p(greet("Bob" "Hi"))         // "Hi Bob!"
    0
}
```

Rules:
- Only trailing parameters can have defaults
- Defaults must be literals: integers, strings, `true`, `false`
- Callers can omit defaulted arguments from the end

```sans
// Multiple defaults
connect(host:S port:I=8080 tls:B=false) I { ... }

connect("localhost")              // port=8080 tls=false
connect("localhost" 443)          // tls=false
connect("localhost" 443 true)     // all explicit
```

## Control Flow

```sans
// If-else expression
result = x > 0 ? x * 2 : x * -1

// If-else block
if condition {
    body
} else {
    body
}

// While loop
while condition {
    body
}

// For-in loop
for item in array {
    body
}

// Loop control
while cond {
    if done { break }       // exit loop immediately
    if skip { continue }    // skip to next iteration
}
for x in arr {
    if x == 0 { continue }  // works in for-in too
    if x < 0 { break }
}

// Match expression
match value {
    EnumName::Variant1 => expr1,
    EnumName::Variant2(x) => x + 1,
}

// Match with guards
match x {
    n if n > 0 => "positive",
    n if n < 0 => "negative",
    _ => "zero",
}

// For-loop destructuring (tuples)
for (k v) in m.entries() {
    p("{k}: {str(v)}")
}
```

## Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+`, `-`, `*`, `/`, `%` |
| Comparison | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| Boolean | `&&`, `\|\|`, `!` |
| Unary | `-` (negation) |
| Assignment | `=`, `:=`, `+=`, `-=`, `*=`, `/=`, `%=` |
| Special | `?:` (ternary), `?` (try/propagate), `!` (postfix unwrap), `[]` (index) |

String comparison (`==`, `!=`) is supported.

---

## Built-in Functions

### I/O

| Function | Alias | Signature |
|----------|-------|-----------|
| `print(value)` | `p` | `(String\|Int\|Float\|Bool) -> Int` |
| `file_read(path)` | `fr` | `(String) -> String` |
| `file_write(path, content)` | `fw` | `(String, String) -> Int` |
| `file_append(path, content)` | `fa` | `(String, String) -> Int` |
| `file_exists(path)` | `fe` | `(String) -> Bool` |

### Type Conversion

| Function | Alias | Signature |
|----------|-------|-----------|
| `int_to_string(n)` | `str` | `(Int) -> String` |
| `string_to_int(s)` | `stoi` | `(String) -> Int` |
| `int_to_float(n)` | `itof` | `(Int) -> Float` |
| `float_to_int(f)` | `ftoi` | `(Float) -> Int` |
| `float_to_string(f)` | `ftos` | `(Float) -> String` |
| `string_to_float(s)` | `stof` | `(String) -> Float` |

### Math

| Function | Signature |
|----------|-----------|
| `abs(n)` | `(Int) -> Int` |
| `min(a, b)` | `(Int, Int) -> Int` |
| `max(a, b)` | `(Int, Int) -> Int` |

### Collections

| Function | Signature | Description |
|----------|-----------|-------------|
| `range(n)` | `(Int) -> Array<Int>` | Returns `[0, 1, ..., n-1]` |
| `range(a, b)` | `(Int, Int) -> Array<Int>` | Returns `[a, a+1, ..., b-1]` |

### System

| Function | Signature | Description |
|----------|-----------|-------------|
| `sleep(ms)` | `(Int) -> Int` | Pause execution for milliseconds |
| `time()` / `now()` | `() -> Int` | Current Unix timestamp (seconds) |
| `random(max)` / `rand(max)` | `(Int) -> Int` | Random integer in `[0, max)` |

### JSON

| Function | Alias | Signature |
|----------|-------|-----------|
| `json_object()` | `jo` | `() -> JsonValue` |
| `json_array()` | `ja` | `() -> JsonValue` |
| `json_string(s)` | `js` | `(String) -> JsonValue` |
| `json_int(n)` | `ji` | `(Int) -> JsonValue` |
| `json_bool(b)` | `jb` | `(Bool) -> JsonValue` |
| `json_null()` | `jn` | `() -> JsonValue` |
| `json_parse(s)` | `jp` | `(String) -> JsonValue` |
| `json_stringify(v)` | `jfy` | `(JsonValue) -> String` |

### HTTP Client

| Function | Alias | Signature |
|----------|-------|-----------|
| `http_get(url)` | `hg` | `(String) -> HttpResponse` |
| `http_post(url, body, content_type)` | `hp` | `(String, String, String) -> HttpResponse` |

### HTTP Server

| Function | Alias | Signature |
|----------|-------|-----------|
| `http_listen(port)` | `listen` | `(Int) -> HttpServer` |
| `https_listen(port, cert, key)` | `hl_s` | `(Int, String, String) -> HttpServer` |
| `serve(port, handler)` | — | `(Int, Fn) -> Int` |
| `serve_tls(port, cert, key, handler)` | — | `(Int, String, String, Fn) -> Int` |
| `stream_write(writer, data)` | — | `(Int, String) -> Int` |
| `stream_end(writer)` | — | `(Int) -> Int` |
| `signal_handler(signum)` | — | `(Int) -> Int` |
| `signal_check()` | — | `() -> Int` |
| `spoll(fd, timeout_ms)` | — | `(Int, Int) -> Int` |
| `ws_send(ws, msg)` | — | `(Int, String) -> Int` |
| `ws_recv(ws)` | — | `(Int) -> String` |
| `ws_close(ws)` | — | `(Int) -> Int` |
| `serve_file(req, dir)` | — | `(HttpRequest, String) -> Int` |
| `url_decode(s)` | — | `(String) -> String` |
| `path_segment(path, idx)` | — | `(String, Int) -> String` |

`serve_file(req, dir)` serves a static file from `dir` matching the request path. Handles content-type detection, 404 for missing files, and directory traversal protection.

`url_decode(s)` decodes a URL-encoded string (e.g. `%20` to space, `+` to space).

`path_segment(path, idx)` extracts the segment at index `idx` from a URL path. `path_segment("/api/users/42" 2)` returns `"42"`.

`serve(port, handler)` starts a production HTTP server with auto-threading and HTTP/1.1 keep-alive. Each connection is handled in a new thread. The handler receives an `HttpRequest` and should call `respond` or `respond_stream`. The server automatically handles SIGINT and SIGTERM for graceful shutdown — in-flight requests complete before the server exits.

`serve_tls(port, cert, key, handler)` is the HTTPS variant with the same graceful shutdown behavior.

#### Automatic Gzip Compression

`respond()` automatically gzip-compresses response bodies when all conditions are met:

1. Client sent `Accept-Encoding` containing `gzip`
2. Response body >= 1024 bytes
3. No `X-No-Compress: 1` response header set by user
4. Content-Type is compressible (`text/*`, `application/json`, `application/javascript`, `application/xml`, `image/svg+xml`)

No code changes needed — compression is transparent:

```sans
req.respond(200 large_body)  // auto-gzipped if client supports it
```

Opt out for a specific response:

```sans
req.set_header("X-No-Compress" "1")
req.respond(200 large_body)  // not compressed
```

#### Signal Handling

`signal_handler(signum)` registers a signal handler that sets a global flag. `signal_check()` returns 1 if the signal was received, 0 otherwise. `spoll(fd, timeout_ms)` polls a file descriptor for readability with a timeout in milliseconds, returning 1 if ready, 0 otherwise. These are used internally by `serve()` and `serve_tls()` but are available for custom server loops.

`req.respond_stream(status)` sends HTTP headers with `Transfer-Encoding: chunked` and returns a writer handle. Use `stream_write(w, data)` to send chunks and `stream_end(w)` to finalize.

```sans
handle(req:HR) I {
  path = req.path()
  path == "/" ? req.respond(200 "Hello!") : req.respond(404 "Not Found")
}

main() I {
  serve(8080 fptr("handle"))
}
```

#### WebSocket

`req.is_ws_upgrade()` returns 1 if the request is a WebSocket upgrade request (has `Upgrade: websocket` header), 0 otherwise.

`req.upgrade_ws()` performs the WebSocket handshake (SHA-1 + Base64 of the `Sec-WebSocket-Key` header concatenated with the magic GUID) and sends the 101 Switching Protocols response. Returns a WebSocket handle.

`ws_send(ws, msg)` sends a text frame. `ws_recv(ws)` receives the next text frame (handles ping/pong automatically, returns `""` on close). `ws_close(ws)` sends a close frame and closes the socket.

```sans
handle(req:I) I {
  req.is_ws_upgrade() ? {
    ws = req.upgrade_ws()
    msg := ws_recv(ws)
    while slen(msg) > 0 {
      ws_send(ws "echo: " + msg)
      msg = ws_recv(ws)
    }
    ws_close(ws)
  } : {
    req.respond(200 "WebSocket server")
  }
}

main() I {
  serve(8080 fptr("handle"))
}
```

### CORS

| Function | Alias | Signature |
|----------|-------|-----------|
| `cors(req, origin)` | — | `(HttpRequest, String) -> Int` |
| `cors_all(req)` | — | `(HttpRequest) -> Int` |

`cors(req, origin)` sets `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`, and `Access-Control-Allow-Headers` on the response. Call before `respond`.

`cors_all(req)` is shorthand for `cors(req, "*")`.

```sans
srv = listen(8080)
while true {
    req = srv.accept
    cors_all(req)
    req.respond(200 "ok")
}
```

### Logging

| Function | Alias | Signature |
|----------|-------|-----------|
| `log_debug(msg)` | `ld` | `(String) -> Int` |
| `log_info(msg)` | `li` | `(String) -> Int` |
| `log_warn(msg)` | `lw` | `(String) -> Int` |
| `log_error(msg)` | `le` | `(String) -> Int` |
| `log_set_level(n)` | `ll` | `(Int) -> Int` |

Log levels: 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR

### Low-Level Primitives

These enable Sans to replace its own C runtime. Pointers are stored as Int (i64).

#### Memory

| Function | Signature | Description |
|----------|-----------|-------------|
| `alloc(size)` | `(Int) -> Int` | malloc, returns pointer |
| `dealloc(ptr)` | `(Int) -> Int` | free |
| `ralloc(ptr, size)` | `(Int, Int) -> Int` | realloc |
| `mcpy(dst, src, n)` | `(Int, Int, Int) -> Int` | memcpy |
| `mcmp(a, b, n)` | `(Int, Int, Int) -> Int` | memcmp |
| `slen(ptr)` | `(Int) -> Int` | strlen on raw pointer |
| `load8(ptr)` | `(Int) -> Int` | load byte (0-255) |
| `store8(ptr, val)` | `(Int, Int) -> Int` | store byte |
| `load16(ptr)` | `(Int) -> Int` | load 16-bit value |
| `store16(ptr, val)` | `(Int, Int) -> Int` | store 16-bit value |
| `load32(ptr)` | `(Int) -> Int` | load 32-bit value |
| `store32(ptr, val)` | `(Int, Int) -> Int` | store 32-bit value |
| `load64(ptr)` | `(Int) -> Int` | load 64-bit value |
| `store64(ptr, val)` | `(Int, Int) -> Int` | store 64-bit value |
| `strstr(haystack, needle)` | `(Int, Int) -> Int` | find substring, 0 if not found |
| `bswap16(val)` | `(Int) -> Int` | byte-swap 16-bit (htons) |
| `exit(code)` | `(Int) -> Int` | exit process |
| `system(cmd)` / `sys(cmd)` | `(String) -> Int` | run shell command, return exit code |
| `gzip_compress(data, len)` | `(Int, Int) -> Int` | gzip-compress data; returns ptr to `[compressed_ptr, compressed_len]` |

#### Arena Allocator

Phase-based bump allocator. All allocations between `arena_begin()` and `arena_end()` are freed at once. Arenas can be nested up to 8 deep.

| Function | Signature | Description |
|----------|-----------|-------------|
| `arena_begin()` | `() -> Int` | Push a new arena onto the stack |
| `arena_alloc(size)` | `(Int) -> Int` | Bump-allocate from the current arena (8-byte aligned) |
| `arena_end()` | `() -> Int` | Pop and free all memory in the current arena |

```sans
arena_begin()
a = arena_alloc(16)
store64(a, 42)
arena_end()  // frees everything at once
```

#### I/O

| Function | Signature | Description |
|----------|-----------|-------------|
| `wfd(fd, msg)` | `(Int, String) -> Int` | write string to file descriptor |

#### SSL (Advanced)

Low-level TLS/SSL bindings. For most use cases, prefer `https_listen`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `ssl_ctx(cert, key)` | `(String, String) -> Int` | Create SSL context from cert/key file paths |
| `ssl_accept(ctx, fd)` | `(Int, Int) -> Int` | Perform TLS handshake on accepted socket fd |
| `ssl_read(ssl, buf, len)` | `(Int, Int, Int) -> Int` | Read bytes from TLS connection |
| `ssl_write(ssl, buf, len)` | `(Int, Int, Int) -> Int` | Write bytes to TLS connection |
| `ssl_close(ssl)` | `(Int) -> Int` | Shut down TLS connection and free SSL object |

#### Sockets

| Function | Signature | Description |
|----------|-----------|-------------|
| `sock(domain, type, proto)` | `(Int, Int, Int) -> Int` | socket() |
| `sbind(fd, port)` | `(Int, Int) -> Int` | bind to port |
| `slisten(fd, backlog)` | `(Int, Int) -> Int` | listen() |
| `saccept(fd)` | `(Int) -> Int` | accept() |
| `srecv(fd, buf, len)` | `(Int, Int, Int) -> Int` | recv() |
| `ssend(fd, buf, len)` | `(Int, Int, Int) -> Int` | send() |
| `sclose(fd)` | `(Int) -> Int` | close() |
| `rbind(fd, addr, len)` | `(Int, Int, Int) -> Int` | raw bind() |
| `rsetsockopt(fd, level, opt, val, len)` | `(Int, Int, Int, Int, Int) -> Int` | raw setsockopt() |

#### Curl

| Function | Signature | Description |
|----------|-----------|-------------|
| `cinit()` | `() -> Int` | curl_easy_init |
| `csets(h, opt, val)` | `(Int, Int, String) -> Int` | setopt with string |
| `cseti(h, opt, val)` | `(Int, Int, Int) -> Int` | setopt with long |
| `cperf(h)` | `(Int) -> Int` | curl_easy_perform |
| `cclean(h)` | `(Int) -> Int` | curl_easy_cleanup |
| `cinfo(h, info, buf)` | `(Int, Int, Int) -> Int` | curl_easy_getinfo |
| `curl_slist_append(slist, str)` | `(Int, Int) -> Int` | append to curl header list |
| `curl_slist_free(slist)` | `(Int) -> Int` | free curl header list |

#### Function Pointers

| Function | Signature | Description |
|----------|-----------|-------------|
| `fptr("name")` | `(String) -> Int` | get pointer to named function |
| `fcall(ptr, arg)` | `(Int, Int) -> Int` | call function pointer with 1 arg |
| `fcall2(ptr, a, b)` | `(Int, Int, Int) -> Int` | call function pointer with 2 args |
| `fcall3(ptr, a, b, c)` | `(Int, Int, Int, Int) -> Int` | call function pointer with 3 args |

#### Pointer Access

| Function | Signature | Description |
|----------|-----------|-------------|
| `ptr(s)` | `(String\|Map\|Array) -> Int` | get raw i64 pointer of string, map, or array |
| `char_at(s, i)` | `(String, Int) -> Int` | read byte at index i (shorthand for `load8(ptr(s) + i)`) |

#### Map Operations

Explicit Map built-ins. Use these when a Map is stored as Int (e.g. from `load64`) and method dispatch cannot determine the correct type.

| Function | Signature | Description |
|----------|-----------|-------------|
| `mget(map, key)` | `(Int, String) -> Int` | get value from Map by key (0 if not found) |
| `mset(map, key, val)` | `(Int, String, Int) -> Int` | set key-value pair in Map |
| `mhas(map, key)` | `(Int, String) -> Int` | check if Map contains key (1=yes, 0=no) |

#### File I/O

| Function | Signature | Description |
|----------|-----------|-------------|
| `read_file(path)` | `(String) -> String` | read entire file to string |
| `write_file(path, content)` | `(String, String) -> Int` | write string to file |
| `args()` | `() -> Array<String>` | get command-line arguments |

### Error Handling

| Function | Signature |
|----------|-----------|
| `ok(value)` | `(T) -> Result<T>` |
| `err(message)` | `(String) -> Result<_>` |
| `err(code, message)` | `(Int, String) -> Result<_>` |

---

## Methods by Type

### Array\<T\>

| Method | Signature | Notes |
|--------|-----------|-------|
| `push(value)` | `(T) -> Int` | Append element |
| `pop` | `() -> T` | Remove and return last |
| `get(index)` or `[index]` | `(Int) -> T` | Read element |
| `set(index, value)` or `[index] = value` | `(Int, T) -> Int` | Write element |
| `len` | `() -> Int` | Length |
| `remove(index)` | `(Int) -> T` | Remove at index |
| `contains(value)` | `(T) -> Bool` | Check membership |
| `map(fn)` | `((T) -> U) -> Array<U>` | Transform elements |
| `filter(fn)` | `((T) -> Bool) -> Array<T>` | Filter elements |
| `any(fn)` | `((T) -> Bool) -> Bool` | True if any element matches |
| `find(fn)` | `((T) -> Bool) -> T` | First match, or 0 |
| `enumerate` | `() -> Array<(Int, T)>` | Index-value tuples |
| `zip(other)` | `(Array<U>) -> Array<(T, U)>` | Paired tuples |
| `sort` | `() -> Array<T>` | In-place sort (integers) |
| `reverse` | `() -> Array<T>` | In-place reverse |
| `join(sep)` | `(String) -> String` | Join elements with separator |
| `slice(start, end)` | `(Int, Int) -> Array<T>` | Sub-array `[start..end)` |
| `reduce(init, fn)` | `(T, (T, T) -> T) -> T` | Fold to single value |
| `each(fn)` / `for_each(fn)` | `((T) -> Int) -> Int` | Iterate with side effects |
| `flat_map(fn)` | `((T) -> Array<U>) -> Array<U>` | Map + flatten |
| `sum` | `() -> Int` | Sum of elements |
| `min` | `() -> Int` | Minimum element |
| `max` | `() -> Int` | Maximum element |
| `flat` | `() -> Array<T>` | Flatten nested arrays |

### String

| Method | Signature |
|--------|-----------|
| `len` | `() -> Int` |
| `substring(start, end)` or `[start:end]` | `(Int, Int) -> String` |
| `trim` | `() -> String` |
| `starts_with(prefix)` | `(String) -> Bool` |
| `ends_with(suffix)` | `(String) -> Bool` |
| `contains(needle)` | `(String) -> Bool` |
| `split(delimiter)` | `(String) -> Array<String>` |
| `replace(old, new)` | `(String, String) -> String` |
| `upper` | `() -> String` |
| `lower` | `() -> String` |
| `index_of(sub)` | `(String) -> Int` |
| `char_at(index)` or `get(index)` | `(Int) -> String` |
| `repeat(n)` | `(Int) -> String` |
| `to_int` | `() -> Int` |
| `get(index)` | `(Int) -> String` |
| `pad_left(width, fill)` | `(Int, String) -> String` |
| `pad_right(width, fill)` | `(Int, String) -> String` |
| `bytes` | `() -> Array<Int>` |

### Int

| Method | Signature |
|--------|-----------|
| `to_str` / `to_string` | `() -> String` |

### Map

| Method | Signature |
|--------|-----------|
| `get(key)` | `(String) -> Int` |
| `set(key, value)` | `(String, Int) -> Int` |
| `has(key)` | `(String) -> Bool` |
| `len` | `() -> Int` |
| `keys` | `() -> Array<String>` |
| `vals` | `() -> Array<Int>` |
| `delete(key)` | `(String) -> Int` |
| `entries` | `() -> Array<(String, Int)>` |

### JsonValue

| Method | Signature |
|--------|-----------|
| `get(key)` | `(String) -> JsonValue` |
| `get_index(index)` | `(Int) -> JsonValue` |
| `get_string` | `() -> String` |
| `get_int` | `() -> Int` |
| `get_bool` | `() -> Bool` |
| `len` | `() -> Int` |
| `type_of` | `() -> String` |
| `set(key, value)` | `(String, JsonValue) -> Int` |
| `push(value)` | `(JsonValue) -> Int` |

### HttpResponse

| Method | Signature |
|--------|-----------|
| `status` | `() -> Int` |
| `body` | `() -> String` |
| `header(name)` | `(String) -> String` |
| `ok` | `() -> Bool` |

### HttpServer

| Method | Signature |
|--------|-----------|
| `accept` | `() -> HttpRequest` |

### HttpRequest

| Method | Signature | Notes |
|--------|-----------|-------|
| `path` | `() -> String` | |
| `method` | `() -> String` | |
| `body` | `() -> String` | |
| `header(name)` | `(String) -> String` | Get request header value (case-insensitive) |
| `set_header(name, value)` | `(String, String) -> Int` | Add custom response header (call before respond) |
| `query(name)` | `(String) -> String` | Get query parameter value by name |
| `path_only` | `() -> String` | Path without query string |
| `content_length` | `() -> Int` | Get Content-Length as int |
| `cookie(name)` | `(String) -> String` | Get cookie value from Cookie header |
| `form(name)` | `(String) -> String` | Parse form field from POST body (URL-encoded or multipart) |
| `respond(status, body)` | `(Int, String) -> Int` | Defaults to `text/html` content-type |
| `respond(status, body, content_type)` | `(Int, String, String) -> Int` | Explicit content-type |
| `respond_json(status, body)` | `(Int, String) -> Int` | JSON response (sets Content-Type: application/json) |
| `respond_stream(status)` | `(Int) -> Int` | Chunked streaming response, returns writer handle |
| `is_ws_upgrade` | `() -> Int` | Returns 1 if WebSocket upgrade request |
| `upgrade_ws` | `() -> Int` | Perform WS handshake, return WebSocket handle |

### Result\<T\>

| Method | Signature | Notes |
|--------|-----------|-------|
| `is_ok` | `() -> Bool` | |
| `is_err` | `() -> Bool` | |
| `unwrap` or `!` | `() -> T` | Exits on error |
| `unwrap_or(default)` | `(T) -> T` | |
| `error` | `() -> String` | Error message |
| `code` | `() -> Int` | Error code (0 if not set) |

### Concurrency Types

| Type | Method | Signature |
|------|--------|-----------|
| `Sender<T>` | `send(value)` | `(T) -> Int` |
| `Receiver<T>` | `recv` | `() -> T` |
| `Mutex<T>` | `lock` | `() -> T` |
| `Mutex<T>` | `unlock(value)` | `(T) -> Int` |
| `JoinHandle` | `join` | `() -> Int` |

---

## Tuples

Tuples are fixed-size, ordered collections of values that can have different types.

### Syntax
- Literal: `(1 "hello" true)` — no commas, space-separated
- Access: `t.0`, `t.1`, `t.2` — zero-indexed positional access
- Type: `(I S B)` — parenthesized type list
- Return type: `f(x:I) (I S) = (x str(x))`

### Examples

```sans
t = (10 20 30)
p(t.0 + t.1 + t.2)  // 60

// Multi-return
pair(a:I b:I) I {
  t = (a b)
  t.0 + t.1
}
```

Single expressions in parens are grouping, not tuples: `(1 + 2)` evaluates to `3`, not a 1-tuple.

---

## Lambdas & Closures

Lambda expressions are anonymous functions that can capture variables from their enclosing scope.

### Syntax
`|params| ReturnType { body }`

### Examples

```sans
// Non-capturing lambda
f = |x:I| I { x + 10 }
f(5)  // 15

// Multiple parameters
add = |a:I b:I| I { a + b }

// Used with map/filter
nums = [1 2 3 4 5]
doubled = nums.map(|x:I| I { x * 2 })

// Implicit capture — variables from enclosing scope are captured automatically
multiplier = 3
scaled = nums.map(|x:I| I { x * multiplier })
```

---

## Iterator Chains

Array methods return arrays, so they can be chained without `.collect()`:

### Chaining
```sans
a.map(|x:I| I { x * 2 }).filter(|x:I| B { x > 3 })
```

### New Methods
- `.any(f)` — returns `B`, true if any element satisfies predicate
- `.find(f)` — returns first element matching predicate, or 0
- `.enumerate()` — returns array of `(index value)` tuples
- `.zip(other)` — returns array of `(a_elem b_elem)` tuples

### Examples

```sans
nums = [1 2 3 4 5]
nums.any(|x:I| B { x > 3 })     // true
nums.find(|x:I| B { x > 10 })   // 0 (not found)

pairs = nums.enumerate()
t = pairs.get(2)                  // (2 3)
t.0                               // 2 (index)
t.1                               // 3 (value)

a = [1 2 3]
b = [10 20 30]
a.zip(b).map(|t:I| I { t })      // [(1 10) (2 20) (3 30)]
```

---

## Map

Hash map with string keys and integer values. Constructor: `M()` or `map()`.

### Methods
| Method | Signature | Description |
|--------|-----------|-------------|
| `set(key, val)` | `(S, I) -> I` | Set key-value pair |
| `get(key)` | `(S) -> I` | Get value, 0 if missing |
| `has(key)` | `(S) -> B` | Check if key exists |
| `len()` | `() -> I` | Number of entries |
| `keys()` | `() -> [S]` | Array of all keys |
| `vals()` | `() -> [I]` | Array of all values |

### Examples

```sans
m = M()
m.set("x" 10)
m.set("y" 20)
m.get("x")       // 10
m.has("z")        // false
m.len()           // 2
m.keys()          // ["x" "y"]
```

---

## Structs

```sans
struct Point { x I, y I }

make_point(x:I y:I) Point = Point { x: x, y: y }

main() {
    pt = Point { x: 10, y: 20 }
    p(str(pt.x + pt.y))
    0
}
```

### Generic Structs

Structs can have type parameters:

```sans
struct Pair<A B> { first A, second B }

main() {
    p = Pair<I S>{ first: 1, second: "hello" }
    p(str(p.first))     // 1
    p(p.second)          // "hello"
    0
}
```

Multiple type parameters are space-separated in angle brackets. The type arguments are specified at construction time:

```sans
struct Triple<A B C> { a A, b B, c C }
t = Triple<I S B>{ a: 42, b: "hi", c: true }
```

## Enums

```sans
enum Shape {
    Circle(I),
    Rect(I, I),
}

area(s Shape) I = match s {
    Shape::Circle(r) => r * r * 3,
    Shape::Rect(w h) => w * h,
}
```

## Traits and Generics

```sans
trait Describable {
    fn describe(self) I
}

impl Describable for Point {
    fn describe(self) I { self.x + self.y }
}

identity<T>(x T) T = x
```

## Modules

```sans
// math.sans
add(a:I b:I) = a + b

// main.sans
import "math"

main() {
    result = math.add(3 4)
    result
}
```

## Concurrency

```sans
worker(tx Sender<Int>) I {
    tx.send(42)
    0
}

main() {
    let (tx rx) = channel<I>()
    spawn worker(tx)
    val = rx.recv
    val
}
```

## String Interpolation

```sans
name = "Sans"
msg = "Hello {name}!"    // "Hello Sans!"
```

### Expression Interpolation

Full expressions are supported inside `{}`:

```sans
x = 10
"result is {x + 1}"     // "result is 11"
"len is {a.len()}"       // method calls
"sum is {x * 2 + 3}"    // arithmetic
```

Identifiers and arbitrary expressions both work. For non-string results, the value is automatically converted.

## String Slicing

Slice strings with `[start:end]` syntax (desugars to `.substring()`):

```sans
s = "hello world"
s[0:5]    // "hello"
s[6:]     // "world" (to end)
s[:5]     // "hello" (from start)
```

## Error Handling

```sans
divide(a:I b:I) R<I> = b == 0 ? err("div/0") : ok(a / b)

main() {
    r = divide(10 3)
    r.is_ok ? r! : 0
}
```

### Error Codes

`err()` accepts an optional integer error code as the first argument:

```sans
fetch(url:S) R<S> {
    resp = hg(url)
    resp.ok() ? ok(resp.body()) : err(resp.status() "request failed")
}

main() {
    r = fetch("https://example.com/missing")
    r.is_err ? p("code: {str(r.code())} msg: {r.error()}") : 0
}
```

- `err("message")` -- error with no code (code defaults to 0)
- `err(404 "not found")` -- error with code 404
- `r.code()` -- get the error code (returns 0 if none set)
- Backwards compatible: existing `err("msg")` calls work unchanged

### Error Propagation (`?` operator)

The `?` operator unwraps a `Result<T>` or early-returns the error:

```sans
safe_div(a:I b:I) R<I> {
  b == 0 ? err("div by zero") : ok(a / b)
}

compute(x:I) R<I> {
  r = safe_div(x 2)?    // unwraps ok(5), or returns err early
  ok(r + 1)
}
```

`x?` desugars to: `if x.is_err() { return x }` followed by `x!` (unwrap).

---

## Pattern Match Guards

Match arms can include `if` guards that add conditions to a pattern:

```sans
classify(n:I) S = match n {
    x if x > 0 => "positive",
    x if x < 0 => "negative",
    _ => "zero",
}
```

The binding variable (`x`) is bound to the matched value and available in the guard expression. Guards work with both integer and string match values:

```sans
describe(s:S) S = match s {
    v if v.len() > 10 => "long",
    v if v.len() > 0 => "short",
    _ => "empty",
}
```

Guards are checked after the pattern matches. If the guard is false, the next arm is tried.

---

## For-Loop Destructuring

For-loops can destructure tuples from the iterable:

```sans
m = M()
m.set("x" 10)
m.set("y" 20)
for (k v) in m.entries() {
    p("{k} = {str(v)}")
}
```

Works with any iterable that produces tuples, including `.enumerate()`:

```sans
names = ["Alice" "Bob" "Charlie"]
for (i name) in names.enumerate() {
    p("{str(i)}: {name}")
}
```

The tuple elements are bound as local variables in the loop body.

---

## Self-Hosting

Sans is self-hosted: the compiler and runtime are both written in Sans.

### Runtime (100% Sans, zero C)

All built-in capabilities are implemented in Sans using its own low-level primitives (`alloc`, `load8`/`store8`, `mcpy`, sockets, etc.):

- `runtime/server.sans` — HTTP server, keep-alive, WebSocket, streaming
- `runtime/json.sans` — JSON parser and serializer
- `runtime/string_ext.sans` — String methods
- `runtime/array_ext.sans` — Array methods
- `runtime/map.sans` — Hash map
- `runtime/ssl.sans` — TLS/SSL
- `runtime/http.sans` — HTTP client
- `runtime/curl.sans` — Curl bindings
- `runtime/arena.sans` — Arena allocator
- `runtime/result.sans` — Result type
- `runtime/functional.sans` — Higher-order functions

### Compiler (written in Sans)

`compiler/` contains a full Sans compiler (~11,600 LOC across 7 modules): lexer, parser, typeck, IR, codegen, main. It compiles to LLVM IR via `llc`, then links with `clang`.

Bootstrap stages: **stage 0** (Rust-compiled) → **stage 1** (self-compiled once) → **stage 2** (self-compiled twice) → **stage 3** (fixed point, output identical to stage 2).

### Reserved Builtin Names

User-defined functions take precedence over builtins of the same name. However, to avoid confusion, avoid redefining builtins unless intentional. The following names have builtin implementations: `p`, `print`, `str`, `stoi`, `itos`, `itof`, `ftoi`, `ftos`, `fr`, `fw`, `fa`, `fe`, `file_read`, `file_write`, `file_append`, `file_exists`, `listen`, `serve`, `serve_file`, `serve_tls`, `alloc`, `dealloc`, `load8`, `load16`, `load32`, `load64`, `store8`, `store16`, `store32`, `store64`, `mcpy`, `mcmp`, `slen`, `wfd`, `exit`, `system`, `sys`, `ok`, `err`, `map`, `M`, `jp`, `jparse`, `jfy`, `jo`, `ja`, `js`, `ji`, `jb`, `jn`, `hg`, `hp`, `sock`, `saccept`, `srecv`, `ssend`, `sclose`, `args`, `spawn`, `signal_handler`, `signal_check`, and all other documented built-in names.

---

## Known Limitations

- **Scope GC gaps**: Automatic scope-based memory management frees heap allocations on function return, but nested heap values in containers (array of arrays) are not recursively freed. Global pointer escape — heap pointers stored in globals outlive their creating scope. Arenas available for hot paths.
- Type checker is relaxed for bootstrap compatibility — some type mismatches (e.g. if/else branch type mismatch, wrong arg types to certain builtins) are not caught at compile time and may produce incorrect behavior at runtime.
- Capturing lambdas (closures with captured variables) passed across module boundaries return incorrect results — the capture context is not preserved. Non-capturing lambdas work correctly across modules.
- Generics: no generic methods on generic structs, no nested generics (`Box<Pair<I S>>`). Default params limited to literals only.
- For-loop destructuring limited to 2-element tuples (map entries).
