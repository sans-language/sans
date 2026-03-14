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

// Match expression
match value {
    EnumName::Variant1 => expr1,
    EnumName::Variant2(x) => x + 1,
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
| Special | `?:` (ternary), `!` (postfix unwrap), `[]` (index) |

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

#### I/O

| Function | Signature | Description |
|----------|-----------|-------------|
| `wfd(fd, msg)` | `(Int, String) -> Int` | write string to file descriptor |

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

#### Curl

| Function | Signature | Description |
|----------|-----------|-------------|
| `cinit()` | `() -> Int` | curl_easy_init |
| `csets(h, opt, val)` | `(Int, Int, String) -> Int` | setopt with string |
| `cseti(h, opt, val)` | `(Int, Int, Int) -> Int` | setopt with long |
| `cperf(h)` | `(Int) -> Int` | curl_easy_perform |
| `cclean(h)` | `(Int) -> Int` | curl_easy_cleanup |
| `cinfo(h, info, buf)` | `(Int, Int, Int) -> Int` | curl_easy_getinfo |

#### Function Pointers

| Function | Signature | Description |
|----------|-----------|-------------|
| `fptr("name")` | `(String) -> Int` | get pointer to named function |
| `fcall(ptr, arg)` | `(Int, Int) -> Int` | call function pointer with 1 arg |

### Error Handling

| Function | Signature |
|----------|-----------|
| `ok(value)` | `(T) -> Result<T>` |
| `err(message)` | `(String) -> Result<_>` |

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

### String

| Method | Signature |
|--------|-----------|
| `len` | `() -> Int` |
| `substring(start, end)` | `(Int, Int) -> String` |
| `trim` | `() -> String` |
| `starts_with(prefix)` | `(String) -> Bool` |
| `ends_with(suffix)` | `(String) -> Bool` |
| `contains(needle)` | `(String) -> Bool` |
| `split(delimiter)` | `(String) -> Array<String>` |
| `replace(old, new)` | `(String, String) -> String` |

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

| Method | Signature |
|--------|-----------|
| `path` | `() -> String` |
| `method` | `() -> String` |
| `body` | `() -> String` |
| `respond(status, body)` | `(Int, String) -> Int` | Defaults to `text/html` content-type |
| `respond(status, body, content_type)` | `(Int, String, String) -> Int` | Explicit content-type |

### Result\<T\>

| Method | Signature | Notes |
|--------|-----------|-------|
| `is_ok` | `() -> Bool` | |
| `is_err` | `() -> Bool` | |
| `unwrap` or `!` | `() -> T` | Exits on error |
| `unwrap_or(default)` | `(T) -> T` | |
| `error` | `() -> String` | |

### Concurrency Types

| Type | Method | Signature |
|------|--------|-----------|
| `Sender<T>` | `send(value)` | `(T) -> Int` |
| `Receiver<T>` | `recv` | `() -> T` |
| `Mutex<T>` | `lock` | `() -> T` |
| `Mutex<T>` | `unlock(value)` | `(T) -> Int` |
| `JoinHandle` | `join` | `() -> Int` |

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

Only identifiers (not expressions) inside `{}`. For non-string types, convert first: `"count: {str(n)}"` won't work — use `"count: " + str(n)`.

## Error Handling

```sans
divide(a:I b:I) R<I> = b == 0 ? err("div/0") : ok(a / b)

main() {
    r = divide(10 3)
    r.is_ok ? r! : 0
}
```

---

## Known Limitations

- No garbage collector — all heap memory leaked until process exit
- No array bounds checking — out-of-bounds is undefined behavior
- Multiple opaque type method calls in complex expressions may crash
- String interpolation only supports identifiers, not expressions
- No lambda syntax with capture — use named function references
