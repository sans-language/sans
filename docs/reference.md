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
| `Map<K,V>` | `M` | Generic hash map (default `M<S,I>`) |
| `Option<T>` | `O<T>` | Optional value: Some or None |
| `Result<T>` | `R<T>` | Success or error value |
| `JsonValue` | `J` | Opaque JSON value |
| `HttpResponse` | — | HTTP client response (opaque handle) |
| `HttpServer` | — | HTTP server socket (opaque handle) |
| `HttpRequest` | — | HTTP server request |
| `Sender<T>` | — | Channel sender (opaque handle) |
| `Receiver<T>` | — | Channel receiver (opaque handle) |
| `Mutex<T>` | — | Mutual exclusion lock (opaque handle) |
| `JoinHandle` | — | Thread handle (opaque handle) |

User-defined types: `struct`, `enum`, `trait`. Trait objects: `dyn TraitName`.

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

// Struct destructuring in match
match pt {
    Point { x, y } => x + y,
}

// Tuple destructuring in match
match pair {
    (a, b) => a + b,
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
| `read_lines(path)` | `rl` | `(String) -> Array<String>` |
| `write_lines(path, lines)` | `wl` | `(String, Array<String>) -> Int` |
| `append_line(path, line)` | `al` | `(String, String) -> Int` |
| `read_line(prompt)` | -- | `(String) -> String` |

#### Buffered I/O

Line-oriented file I/O for common read/write patterns:

```sans
// Write lines to a file (each line terminated by \n)
wl("/tmp/data.txt", ["hello" "world" "sans"])

// Read all lines from a file
lines = rl("/tmp/data.txt")
p(lines[0])  // "hello"

// Append a single line (with trailing \n)
al("/tmp/data.txt", "extra")

// Interactive prompt (prints prompt, reads line, trims)
name = read_line("Enter name: ")
```

`read_lines` strips the trailing newline before splitting, so a file with content `"a\nb\n"` returns `["a", "b"]` (not `["a", "b", ""]`). `write_lines` adds a trailing newline after each line. `append_line` appends the line followed by a newline.

### Stdin I/O

| Function | Alias | Description |
|----------|-------|-------------|
| `stdin_read_line()` | `srl()` | Read one line from stdin (blocking). Returns the line without trailing newline. |
| `stdin_read_bytes(n)` | `srb(n)` | Read exactly `n` bytes from stdin (blocking). |

These functions use an internal 64KB buffer for efficient reading. Used for building interactive programs and protocol servers (e.g., the LSP server).

```sans
// Read a line from stdin
line = srl()
p(line)

// Read exactly 10 bytes
data = srb(10)
p(data)
```

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

### Float Math

| Function | Signature | Description |
|----------|-----------|-------------|
| `floor(x)` | `(Float) -> Float` | Round down to nearest integer |
| `ceil(x)` | `(Float) -> Float` | Round up to nearest integer |
| `round(x)` | `(Float) -> Float` | Round to nearest integer |
| `sqrt(x)` | `(Float) -> Float` | Square root |
| `sin(x)` | `(Float) -> Float` | Sine (radians) |
| `cos(x)` | `(Float) -> Float` | Cosine (radians) |
| `tan(x)` | `(Float) -> Float` | Tangent (radians) |
| `asin(x)` | `(Float) -> Float` | Inverse sine |
| `acos(x)` | `(Float) -> Float` | Inverse cosine |
| `atan(x)` | `(Float) -> Float` | Inverse tangent |
| `atan2(y, x)` | `(Float, Float) -> Float` | Two-argument arctangent |
| `log(x)` | `(Float) -> Float` | Natural logarithm |
| `log10(x)` | `(Float) -> Float` | Base-10 logarithm |
| `exp(x)` | `(Float) -> Float` | Exponential (e^x) |
| `pow(base, exp)` | `(Float, Float) -> Float` | Raise base to power |
| `fabs(x)` | `(Float) -> Float` | Absolute value (float) |
| `fmin(a, b)` | `(Float, Float) -> Float` | Minimum of two floats |
| `fmax(a, b)` | `(Float, Float) -> Float` | Maximum of two floats |
| `PI()` | `() -> Float` | Pi constant (3.141592653589793) |
| `E_CONST()` | `() -> Float` | Euler's number (2.718281828459045) |

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
| `random(max)` / `rand(max)` | `(Int) -> Int` | Cryptographically seeded random integer in `[0, max)` |

### Date/Time

All functions operate on Unix timestamps (i64 seconds since epoch). Timestamps are `I`, formatted strings are `S`. Uses local time via `localtime_r`.

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `time_now()` | `tnow` | `() -> Int` | Current Unix timestamp (same as `time()`) |
| `time_format(t, fmt)` | `tfmt` | `(Int, String) -> String` | Format timestamp with strftime pattern (e.g. `"%Y-%m-%d"`) |
| `time_year(t)` | `tyear` | `(Int) -> Int` | Extract year (e.g. 2026) |
| `time_month(t)` | `tmon` | `(Int) -> Int` | Extract month (1-12) |
| `time_day(t)` | `tday` | `(Int) -> Int` | Extract day of month (1-31) |
| `time_hour(t)` | `thour` | `(Int) -> Int` | Extract hour (0-23) |
| `time_minute(t)` | `tmin` | `(Int) -> Int` | Extract minute (0-59) |
| `time_second(t)` | `tsec` | `(Int) -> Int` | Extract second (0-59) |
| `time_weekday(t)` | `twday` | `(Int) -> Int` | Day of week (0=Sunday, 6=Saturday) |
| `time_add(t, secs)` | `tadd` | `(Int, Int) -> Int` | Add seconds to timestamp |
| `time_diff(a, b)` | `tdiff` | `(Int, Int) -> Int` | Difference in seconds (a - b) |

```sans
t = tnow()
p(tfmt(t "%Y-%m-%d %H:%M:%S"))
p("year: " + str(tyear(t)))
tomorrow = tadd(t 86400)
p("hours until tomorrow: " + str(tdiff(tomorrow t) / 3600))
```

### Assertions

Built-in assertion functions for testing. Each prints a diagnostic with the source line number on failure and exits with code 1.

| Function | Signature | Description |
|----------|-----------|-------------|
| `assert(cond)` | `(Bool) -> Int` | Fail if `cond` is false (zero) |
| `assert_eq(a, b)` | `(Int, Int) -> Int` | Fail if `a != b`, prints expected vs got |
| `assert_ne(a, b)` | `(Int, Int) -> Int` | Fail if `a == b`, prints the equal value |
| `assert_ok(r)` | `(Result<T>) -> Int` | Fail if `r` is an err |
| `assert_err(r)` | `(Result<T>) -> Int` | Fail if `r` is ok |
| `assert_some(o)` | `(Option<T>) -> Int` | Fail if `o` is none |
| `assert_none(o)` | `(Option<T>) -> Int` | Fail if `o` is some |

```sans
assert(1 == 1)
assert_eq(42, 42)
assert_ne(1, 2)
assert_ok(ok(42))
assert_err(err("bad"))
assert_some(some(1))
assert_none(none())
```

Failure output example:
```
assert_eq failed: expected 42, got 99 at line 5
```

### Path Manipulation

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `path_join(a, b)` | `pjoin` | `(String, String) -> String` | Join two path segments with `/`. If `b` is absolute, returns `b`. |
| `path_dir(p)` | `pdir` | `(String) -> String` | Directory component (everything before last `/`). Returns `"."` if no `/`. |
| `path_base(p)` | `pbase` | `(String) -> String` | Filename component (everything after last `/`). Returns whole string if no `/`. |
| `path_ext(p)` | `pext` | `(String) -> String` | File extension including `.`. Returns `""` if no extension. |
| `path_stem(p)` | `pstem` | `(String) -> String` | Filename without extension. |
| `path_is_abs(p)` | `pabs` | `(String) -> Int` | Returns `1` if path starts with `/`, else `0`. |

```sans
path_join("foo" "bar")            // "foo/bar"
path_dir("/home/user/file.sans")  // "/home/user"
path_base("/home/user/file.sans") // "file.sans"
path_ext("file.sans")             // ".sans"
path_stem("file.sans")            // "file"
path_is_abs("/home")              // 1
```

### Encoding

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `base64_encode(s)` | `b64e` | `(String) -> String` | Base64 encode a string. |
| `base64_decode(s)` | `b64d` | `(String) -> String` | Base64 decode a string. |
| `url_encode(s)` | `urle` | `(String) -> String` | Percent-encode for URLs (unreserved chars pass through). |
| `url_decode(s)` | `urld` / `ud` | `(String) -> String` | Decode percent-encoded string (`+` becomes space). |
| `hex_encode(s)` | `hexe` | `(String) -> String` | Hex encode (each byte to 2 lowercase hex chars). |
| `hex_decode(s)` | `hexd` | `(String) -> String` | Hex decode (2 hex chars to byte). |

```sans
base64_encode("Hello, World!")  // "SGVsbG8sIFdvcmxkIQ=="
base64_decode("SGVsbG8sIFdvcmxkIQ==")  // "Hello, World!"
url_encode("hello world&foo=bar")  // "hello%20world%26foo%3Dbar"
url_decode("hello%20world")  // "hello world"
hex_encode("ABC")  // "414243"
hex_decode("414243")  // "ABC"
```

### Crypto

Hash functions, HMAC, and cryptographic random bytes via OpenSSL. All return lowercase hex-encoded strings.

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `sha256(s)` | — | `(String) -> String` | SHA-256 hash, returns 64-char hex string. |
| `sha512(s)` | — | `(String) -> String` | SHA-512 hash, returns 128-char hex string. |
| `md5(s)` | — | `(String) -> String` | MD5 hash, returns 32-char hex string. |
| `hmac_sha256(key, msg)` | `hmac256` | `(String, String) -> String` | HMAC-SHA256, returns 64-char hex string. |
| `random_bytes(n)` | `randb` | `(Int) -> String` | n cryptographically random bytes as hex (2*n chars). |

```sans
sha256("hello")              // "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
sha512("hello")              // "9b71d224bd62f378..."
md5("hello")                 // "5d41402abc4b2a76b9719d911017c592"
hmac_sha256("secret" "msg")  // HMAC-SHA256 hex digest
hmac256("key" "data")        // alias
r = random_bytes(16)         // 32-char random hex string
r = randb(8)                 // 16-char random hex string
```

### Regex (POSIX ERE)

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `regex_match(pattern, text)` | `rmatch` | `(String, String) -> Int` | Returns 1 if text matches pattern, 0 otherwise. |
| `regex_find(pattern, text)` | `rfind` | `(String, String) -> String` | Returns first match substring, or empty string `""` if none. |
| `regex_replace(pattern, text, replacement)` | `rrepl` | `(String, String, String) -> String` | Replace first match with replacement. Returns original text if no match. |

Uses POSIX Extended Regular Expressions (ERE) via `regcomp`/`regexec`. **Note:** Available on Linux and macOS only. Not available on Windows (a pure Sans regex engine is planned).

```sans
regex_match("[0-9]+" "hello123")    // 1
rmatch("[0-9]+" "hello")            // 0
regex_find("[0-9]+" "hello123world") // "123"
rfind("[a-z]+" "123abc456")         // "abc"
regex_replace("[0-9]+" "hello123world" "XXX")  // "helloXXXworld"
rrepl("[0-9]+" "no digits" "XXX")              // "no digits"
```

### Filesystem & Process

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `getenv(name)` | `genv` | `(String) -> String` | Read environment variable. Returns `""` if not set. |
| `mkdir(path)` | — | `(String) -> Int` | Create directory and parents (like `mkdir -p`). Returns 1 on success, 0 on error. |
| `rmdir(path)` | — | `(String) -> Int` | Remove an empty directory. Returns 1 on success, 0 on error. |
| `remove(path)` | `rm` | `(String) -> Int` | Delete a file. Returns 1 on success, 0 on error. |
| `listdir(path)` | `ls` | `(String) -> Array<String>` | List directory contents. Returns empty array on error. |
| `is_dir(path)` | — | `(String) -> Bool` | Check if path is a directory. |
| `sh(cmd)` | `shell` | `(String) -> String` | Execute shell command and capture stdout. Returns `""` on failure. |

```sans
// Environment
home = getenv("HOME")

// Filesystem
mkdir("build/output")       // creates parents
is_dir("build/output")      // true
files = listdir("src/")     // ["main.sans" "lib.sans" ...]
remove("old.txt")           // delete file
rmdir("build/output")       // remove empty dir

// Process
output = sh("uname -s")    // "Darwin\n" or "Linux\n"
```

### JSON

| Function | Alias | Signature |
|----------|-------|-----------|
| `json_object()` | `jo` | `() -> JsonValue` |
| `json_array()` | `ja` | `() -> JsonValue` |
| `json_string(s)` | `js` | `(String) -> JsonValue` |
| `json_int(n)` | `ji` | `(Int) -> JsonValue` |
| `json_bool(b)` | `jb` | `(Bool) -> JsonValue` |
| `json_null()` | `jn` | `() -> JsonValue` |
| `json_parse(s)` | `jp` | `(String) -> Result<JsonValue>` — parses objects, arrays, strings, ints, floats, bools, null. Returns error on invalid JSON or depth > 512. **Breaking change (v0.8.1):** previously returned `JsonValue` (null on error). Migration: add `!` after `json_parse()` calls. |
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

#### Server Configuration

Configure the HTTP server before calling `serve()` or `serve_tls()`. All settings have sensible defaults.

| Function | Default | Description |
|----------|---------|-------------|
| `set_max_workers(n)` | 256 | Max concurrent worker threads. Connections beyond this limit receive HTTP 503. |
| `set_read_timeout(s)` | 30 | Seconds to wait for client data before closing the connection. |
| `set_keepalive_timeout(s)` | 60 | Seconds to wait for next request on keep-alive connection. |
| `set_drain_timeout(s)` | 5 | Seconds to wait for in-flight requests during shutdown. |
| `set_max_body(n)` | 1048576 (1MB) | Max request body size in bytes. Oversized requests receive HTTP 413. |
| `set_max_headers(n)` | 8192 (8KB) | Max total header size in bytes. Oversized headers receive HTTP 431. |
| `set_max_header_count(n)` | 100 | Max number of request headers. Excess headers receive HTTP 431. |
| `set_max_url(n)` | 8192 (8KB) | Max URL length in bytes. Oversized URLs receive HTTP 414. |

```sans
main() I {
  set_max_workers(128)
  set_read_timeout(10)
  set_max_body(4096)
  serve(8080 fptr("handle"))
}
```

**Server behaviors:** `serve()` and `serve_tls()` use a bounded thread pool (default 256 workers). Requests are read incrementally until headers are complete, then body is read based on Content-Length. On SIGTERM, the server stops accepting connections and drains in-flight workers (default 5s timeout). Oversized input is automatically rejected with 413/414/431 responses.

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

#### Bitwise Operations

Native LLVM i64 bitwise operations.

| Function | Signature | Description |
|----------|-----------|-------------|
| `bxor(a, b)` | `(Int, Int) -> Int` | bitwise XOR |
| `band(a, b)` | `(Int, Int) -> Int` | bitwise AND |
| `bor(a, b)` | `(Int, Int) -> Int` | bitwise OR |
| `bshl(a, b)` | `(Int, Int) -> Int` | bitwise shift left |
| `bshr(a, b)` | `(Int, Int) -> Int` | bitwise shift right |

```sans
x = bxor(0xFF 0x0F)   // 0xF0
mask = band(val 0xFF)  // low byte
flags = bor(a b)       // combine flags
shifted = bshl(1 8)    // 256
high = bshr(val 32)    // upper 32 bits
```

#### Low-Level Threading

Raw pthread mutex operations for when you need manual synchronization.

| Function | Signature | Description |
|----------|-----------|-------------|
| `pmutex_init(ptr)` | `(Int) -> Int` | Initialize a raw pthread mutex at the given address |
| `pmutex_lock(ptr)` | `(Int) -> Int` | Lock a raw pthread mutex |
| `pmutex_unlock(ptr)` | `(Int) -> Int` | Unlock a raw pthread mutex |

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

### Option

| Function | Signature |
|----------|-----------|
| `some(value)` | `(T) -> Option<T>` |
| `none()` | `() -> Option<T>` |

---

## Option\<T\>

`Option<T>` (short: `O<T>`) represents an optional value — either `Some(v)` or `None`. Used as the return type of operations that may produce no result (e.g. `Map.get`, `Array.find`).

Runtime layout: 16 bytes — tag at offset 0 (0=None, 1=Some), value at offset 8.

### Creating Options

```sans
x = some(42)       // Some(42)
y = none()         // None
```

### Methods

| Method | Signature | Notes |
|--------|-----------|-------|
| `is_some` | `() -> Bool` | True if Some |
| `is_none` | `() -> Bool` | True if None |
| `unwrap` or `!` | `() -> T` | Extract value; exits on None |
| `unwrap_or(default)` | `(T) -> T` | Extract or return default |

### Operators

- `opt!` — unwrap (exits if None)
- `opt?` — try operator: unwraps Some, or early-returns `none()` from the enclosing function

### Examples

```sans
find_user(id:I) O<S> {
    id == 1 ? some("alice") : none()
}

main() {
    u = find_user(1)
    u.is_some          // true
    u!                 // "alice"
    u.unwrap_or("unknown")  // "alice"

    m = M()
    m.set("x" 10)
    v = m.get("x")     // Option<I>
    v!                 // 10
    m.get("z").unwrap_or(0)  // 0

    0
}
```

### Option with ? propagation

```sans
lookup(m:M<S I> k:S) O<I> {
    v = m.get(k)?  // returns none() early if missing
    some(v * 2)
}
```

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
| `find(fn)` | `((T) -> Bool) -> Option<T>` | First match, or None |
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

### Map\<K,V\>

| Method | Signature |
|--------|-----------|
| `get(key)` | `(K) -> Option<V>` |
| `set(key, value)` | `(K, V) -> Int` |
| `has(key)` | `(K) -> Bool` |
| `len` | `() -> Int` |
| `keys` | `() -> Array<K>` |
| `vals` | `() -> Array<V>` |
| `delete(key)` | `(K) -> Int` |
| `entries` | `() -> Array<(K, V)>` |

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
| `map(fn)` | `((T) -> U) -> Result<U>` | Transform ok value |
| `and_then(fn)` | `((T) -> Result<U>) -> Result<U>` | Chain fallible operations |
| `map_err(fn)` | `((String) -> String) -> Result<T>` | Transform error message |
| `or_else(fn)` | `((String) -> Result<T>) -> Result<T>` | Recover from error |

#### Result Combinators

`map(fn)` applies `fn` to the ok value and wraps the result in a new `ok`. On error, returns the error unchanged.

`and_then(fn)` applies `fn` to the ok value where `fn` itself returns a `Result`. Useful for chaining fallible steps. On error, returns the error unchanged.

`map_err(fn)` transforms the error message string. On ok, returns the ok unchanged.

`or_else(fn)` applies `fn` to the error message and returns its `Result`. On ok, returns the ok unchanged.

```sans
parse(s:S) R<I> = s == "" ? err("empty") : ok(stoi(s))

// map: transform ok value
parse("42").map(|n:I| I { n * 2 })   // ok(84)

// and_then: chain fallible operations
parse("10").and_then(|n:I| R<I> { n > 0 ? ok(n) : err("negative") })

// map_err: rewrite error message
parse("").map_err(|e:S| S { "parse failed: {e}" })

// or_else: recover from error
parse("").or_else(|e:S| R<I> { ok(0) })  // ok(0) as fallback
```

### Option\<T\>

| Method | Signature | Notes |
|--------|-----------|-------|
| `is_some` | `() -> Bool` | |
| `is_none` | `() -> Bool` | |
| `unwrap` or `!` | `() -> T` | Exits on None |
| `unwrap_or(default)` | `(T) -> T` | |

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

Up to 8 variables can be captured from the enclosing scope per closure.

---

## Iterator Chains

Array methods return arrays, so they can be chained without `.collect()`:

### Chaining
```sans
a.map(|x:I| I { x * 2 }).filter(|x:I| B { x > 3 })
```

### New Methods
- `.any(f)` — returns `B`, true if any element satisfies predicate
- `.find(f)` — returns `Option<T>`, first match or None (breaking change in v0.7.2: was `T`)
- `.enumerate()` — returns array of `(index value)` tuples
- `.zip(other)` — returns array of `(a_elem b_elem)` tuples

### Examples

```sans
nums = [1 2 3 4 5]
nums.any(|x:I| B { x > 3 })              // true
nums.find(|x:I| B { x > 10 })            // None
nums.find(|x:I| B { x > 3 })!            // 4 (unwrap)
nums.find(|x:I| B { x > 3 }).unwrap_or(0) // 4

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

Generic hash map. The type parameter specifies key and value types: `M<K V>`. Bare `M()` defaults to `M<S I>` (string keys, integer values).

Supported key types: `S` (String), `I` (Int). Float keys are not allowed.

### Constructors

```sans
m = M()            // M<S I> — string keys, int values
m = M<S I>()       // explicit string→int
m = M<I I>()       // int→int
m = M<I S>()       // int→string
m = M<S S>()       // string→string
```

### Methods
| Method | Signature | Description |
|--------|-----------|-------------|
| `set(key, val)` | `(K, V) -> I` | Set key-value pair |
| `get(key)` | `(K) -> Option<V>` | Get value — returns `Some(v)` or `None` |
| `has(key)` | `(K) -> B` | Check if key exists |
| `len()` | `() -> I` | Number of entries |
| `keys()` | `() -> [K]` | Array of all keys |
| `vals()` | `() -> [V]` | Array of all values |
| `delete(key)` | `(K) -> I` | Remove key |
| `entries` | `() -> [(K V)]` | Array of key-value tuples |

**Breaking change (v0.7.1):** `m.get(key)` now returns `Option<V>` instead of a raw value. Use `!`, `.unwrap()`, or `.unwrap_or(default)` to extract.

### Examples

```sans
m = M()
m.set("x" 10)
m.set("y" 20)
m.get("x")!          // 10 (unwrap)
m.get("z").unwrap_or(0)  // 0 (missing key)
m.has("z")           // false
m.len()              // 2
m.keys()             // ["x" "y"]

// Int keys
counts = M<I I>()
counts.set(42 1)
counts.get(42)!      // 1
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

## Trait Objects (`dyn Trait`)

Trait objects enable dynamic dispatch through a vtable. Use `dyn TraitName` as a type and `expr as dyn TraitName` to coerce a concrete struct into a trait object.

```sans
trait Valued {
    fn value(self) I
}

struct Num { n I }
impl Valued for Num {
    fn value(self) I { self.n }
}

// Coerce concrete struct to trait object
x = Num{ n: 42 }
v = x as dyn Valued    // fat pointer: (data ptr, vtable ptr)
v.value()              // 42 — dispatched via vtable

// Use dyn Trait as parameter type
show(v dyn Valued) I { v.value() }
show(x as dyn Valued) // 42

// Polymorphic collections
items = [x1 as dyn Valued  x2 as dyn Valued]
for item in items { p(str(item.value())) }
```

**Runtime layout:** 16-byte heap-allocated fat pointer — data pointer at offset 0, vtable pointer at offset 8.

**Limitations:** No trait inheritance, no default implementations, no associated types, no generic bounds on trait objects.

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

### Module Re-exports

Use `pub import` to re-export all public symbols from another module. Downstream consumers see a single clean API without needing to know about internal module structure.

```sans
// impl.sans
pub add(a:I b:I) I = a + b
pub sub(a:I b:I) I = a - b
helper() I = 42        // not pub — not re-exported

// facade.sans
pub import "impl"      // re-exports add and sub

// main.sans
import "facade"
main() I {
  p(facade.add(1 2))   // 3
  p(facade.sub(5 3))   // 2
  0
}
```

Only symbols marked `pub` in the source module are re-exported. Non-pub symbols remain private.

## Package Manager

Sans includes a built-in package manager accessed via `sans pkg`. Packages are git repositories fetched by version tag.

### sans.json

Every project has a `sans.json` manifest:

```json
{
  "name": "my-project",
  "version": "0.1.0",
  "deps": {
    "github.com/user/repo": "v1.0.0"
  }
}
```

### Commands

| Command | Description |
|---------|-------------|
| `sans pkg init` | Create `sans.json` in current directory |
| `sans pkg init --name mylib --version 2.0.0` | Create with custom name/version |
| `sans pkg add <url> [tag]` | Add dependency (auto-resolves latest tag if omitted) |
| `sans pkg install` | Install all dependencies from `sans.json` |
| `sans pkg remove <url>` | Remove dependency |
| `sans pkg list` | List direct and transitive dependencies |
| `sans pkg update <url> [tag]` | Update dependency to new version |
| `sans pkg search <query>` | Search community package index |

Short aliases: `sans pkg i` (install), `sans pkg ls` (list), `sans pkg rm` (remove).

### Global Cache

Packages are cached at `~/.sans/packages/<url>/<version>/`. Each version is a shallow git clone. Repeated installs reuse cached packages.

### Dependency Resolution

Dependencies are resolved transitively via BFS. Each package's `sans.json` is checked for its own dependencies. Version conflicts (same package, different versions) are rejected with an error.

## Linter

`sans lint` runs static analysis (parse + type check) without building. It reports diagnostics for common issues.

### Usage

```
sans lint foo.sans                        # lint single file
sans lint compiler/                       # lint all .sans files recursively
sans lint .                               # lint current directory
sans lint --error=unused-imports foo.sans  # promote rule to error
sans lint --quiet foo.sans                # suppress warnings
```

### Rules

| Rule | Default | Description |
|------|---------|-------------|
| `unused-imports` | warn | Imported module never referenced |
| `unreachable-code` | warn | Code after return statement |
| `empty-catch` | warn | Result value silently discarded |
| `shadowed-vars` | warn | Inner scope redeclares outer variable |
| `unnecessary-mut` | warn | Variable declared `:=` but never reassigned |

### Configuration

Rules can be configured in `sans.json`:

```json
{
  "lint": {
    "unused-imports": "error",
    "shadowed-vars": "off"
  }
}
```

Valid severities: `"error"`, `"warn"`, `"off"`.

CLI `--error=<rule>` overrides the config file severity for that rule. `sans build --quiet` suppresses warnings during build.

### Exit Codes

- **0** — no error-severity diagnostics
- **1** — one or more error-severity diagnostics

## Editor Support

All editors connect to the shared `sans-lsp` language server for hover, go-to-definition, completion, diagnostics, semantic tokens, references, rename, folding, and more.

| Editor | Setup |
|---|---|
| **VSCode** | Install the Sans extension from the marketplace |
| **Neovim** | Copy `editors/neovim-sans/` to `~/.config/nvim/`, add `require("sans").setup()` |
| **Emacs** | Add `editors/emacs-sans/` to load-path, add `(require 'sans-mode)` — eglot auto-connects |
| **JetBrains** | Import TextMate bundle from `editors/jetbrains-sans/`, install LSP4IJ plugin, configure `sans-lsp` |

Prerequisite: `sans-lsp` must be on PATH. See each editor's README for detailed instructions.

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

---

## Compiler Diagnostics

The Sans compiler reports errors with source location and context:

```
file.sans:12:5: error: undefined variable 'foo'
    foo + 1
    ^
```

Format: `file:line:col: severity: message`, followed by the source line and a caret (`^`) pointing to the offending token.

The compiler collects multiple errors before exiting, so all errors in a file are reported in a single pass.

Error severities: `error` (build fails), `warning` (build continues).

### Warnings

The compiler emits warnings for:
- **Unused variables** — declared but never referenced
- **Unreachable code** — statements after a `return`

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

## Struct Destructuring in Match

Match arms can destructure struct values by field name:

```sans
struct Point { x:I y:I }

describe(p:Point) I = match p {
    Point { x, y } => x + y,
}
```

Each field name in the pattern becomes a local binding with the field's value and type. The field names must match actual fields of the struct; unknown fields produce a compile error.

---

## Tuple Destructuring in Match

Match arms can destructure tuple values by element position:

```sans
add(pair:(I I)) I = match pair {
    (a, b) => a + b,
}
```

The number of bindings in the pattern must match the tuple arity. Each binding receives the corresponding tuple element's value and type. Arity mismatches produce a compile error.

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

## Runtime Safety

### Array Bounds Checking

Array GET and SET are bounds-checked at runtime. Out-of-bounds access prints an error and exits:

```sans
a = [1 2 3]
x = a[10]  // error: index out of bounds: index 10 but length is 3
a[10] = 5  // error: index out of bounds: index 10 but length is 3
```

This replaces the prior behavior of returning `0` on GET and silently corrupting memory on SET.

### String Bounds Checking

`char_at()` is bounds-checked at runtime:

```sans
s = "hi"
c = s.char_at(99)  // error: string index out of bounds: index 99 but length is 2
```

### SIGPIPE Handling

HTTP and HTTPS servers automatically ignore SIGPIPE. Client disconnects during a write no longer crash the server process.

### JSON Parse Returns Result (v0.8.1 Breaking Change)

`json_parse(s)` / `jp(s)` now returns `Result<JsonValue>` instead of `JsonValue`. On invalid input, it returns an error Result with a descriptive message instead of a null JsonValue.

```sans
// Before (v0.8.0 and earlier):
j = jp("{\"name\":\"Alice\"}")    // JsonValue (null on error)

// After (v0.8.1+):
j = jp("{\"name\":\"Alice\"}")!   // unwrap Result to get JsonValue
// or handle the error:
r = jp(input)
if r.is_err { p("bad json"); 0 } else { r! }
```

**Migration:** Add `!` after every `json_parse()` / `jp()` call, or use `?` to propagate the error.

### JSON Recursion Depth Limit

JSON parsing enforces a maximum nesting depth of 512 levels. Inputs with deeper nesting return an error Result:

```sans
r = jp(deeply_nested_string)
// r.is_err == true, r.error() == "JSON parse error: maximum nesting depth exceeded"
```

### Scope GC Walks JSON Types

Returning nested JSON values from functions no longer causes use-after-free. The scope-based garbage collector now walks JSON object and array trees, promoting all referenced memory to the caller's scope on return.

### Panic Recovery

Panic recovery allows a program to catch unwrap failures (`!` on `Err`/`None`) instead of exiting. It is implemented with `setjmp`/`longjmp` and is designed for use in server request handlers.

```sans
buf := panic_get_buf()
rv := setjmp(buf)
if rv != 0 {
    // longjmp was called — unwrap failed somewhere
    req.respond(500 "internal error")
    panic_disable()
    0
} else {
    panic_enable()
    result = risky_op()!  // calls longjmp instead of exit on Err/None
    req.respond(200 str(result))
    panic_disable()
    0
}
```

When panic recovery is enabled, `!` on `Err` or `None` calls `longjmp` back to the `setjmp` point instead of calling `exit(1)`.

#### Panic Recovery Builtins

| Function | Signature | Description |
|----------|-----------|-------------|
| `setjmp(buf)` | `(Int) -> Int` | Set jump point. Returns 0 initially, non-zero when jumped to |
| `longjmp(buf, val)` | `(Int, Int) -> Int` | Jump back to the `setjmp` point with value `val` |
| `panic_enable()` | `() -> Int` | Enable panic recovery (unwrap uses longjmp instead of exit) |
| `panic_disable()` | `() -> Int` | Disable panic recovery |
| `panic_is_active()` | `() -> Int` | Returns 1 if recovery is active, 0 otherwise |
| `panic_get_buf()` | `() -> Int` | Get the jmp_buf pointer |
| `panic_fire()` | `() -> Int` | Fire longjmp to panic buf (call longjmp manually) |

---

## Internals

### Runtime Modules

The standard library is implemented across 13+ modules in `runtime/`:

`server.sans` (HTTP server, WebSocket, streaming), `json.sans` (JSON parser/serializer), `string_ext.sans` (string methods), `array_ext.sans` (array methods), `map.sans` (hash map), `ssl.sans` (TLS/SSL), `http.sans` (HTTP client), `curl.sans` (curl bindings), `arena.sans` (arena allocator), `result.sans` (Result type), `functional.sans` (higher-order functions), `rc.sans` (scope GC), `log.sans` (logging), `sock.sans` (raw sockets).

### Compiler Modules

The compiler is 7 modules in `compiler/` (~11,600 LOC): lexer, parser, typeck, constants, IR, codegen, main. Compiles to LLVM IR via `llc`, links with `clang`.

### Reserved Builtin Names

User-defined functions take precedence over builtins of the same name. However, to avoid confusion, avoid redefining builtins unless intentional. The following names have builtin implementations: `p`, `print`, `str`, `stoi`, `itos`, `itof`, `ftoi`, `ftos`, `fr`, `fw`, `fa`, `fe`, `rl`, `wl`, `al`, `read_lines`, `write_lines`, `append_line`, `read_line`, `file_read`, `file_write`, `file_append`, `file_exists`, `listen`, `serve`, `serve_file`, `serve_tls`, `alloc`, `dealloc`, `load8`, `load16`, `load32`, `load64`, `store8`, `store16`, `store32`, `store64`, `mcpy`, `mcmp`, `slen`, `wfd`, `exit`, `system`, `sys`, `ok`, `err`, `map`, `M`, `jp`, `jparse`, `jfy`, `jo`, `ja`, `js`, `ji`, `jb`, `jn`, `hg`, `hp`, `sock`, `saccept`, `srecv`, `ssend`, `sclose`, `args`, `spawn`, `signal_handler`, `signal_check`, `assert`, `assert_eq`, `assert_ne`, `assert_ok`, `assert_err`, `assert_some`, `assert_none`, and all other documented built-in names.

---

## Known Limitations

- **Scope GC**: Automatic scope-based memory management frees heap allocations on function return, including nested container contents (depth-2+) and global-escaped values. The compiler itself must be built from the bootstrap binary. Thread safety of scope globals (`rc_alloc_head`/`rc_scope_head`) is not guaranteed.
