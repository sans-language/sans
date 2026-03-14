# Sans Language Design Specification

## Overview

**Sans** is a speed-first, memory-safe backend programming language with automatic garbage collection, designed for building APIs, web servers, and general-purpose programs. It compiles to native machine code via LLVM, features green thread concurrency, and ships with a batteries-included standard library and all-in-one toolchain.

The name "Sans" comes from Welsh, meaning "fast."

### Design Principles

- **Speed first**: Native compilation, zero-cost abstractions where possible, optimized runtime
- **Memory safety by default**: GC + compiler-enforced safety, no manual memory management
- **Batteries included**: HTTP server, JSON, logging, crypto in stdlib; unified toolchain
- **Simple but powerful**: C-family syntax with ML-inspired expression semantics; no unnecessary complexity
- **Backend-native**: Designed from the ground up for APIs, web servers, and backend services

### Key Decisions Summary

| Decision | Choice |
|----------|--------|
| Compilation target | Native machine code via LLVM |
| Syntax | C-family structure + ML expression semantics |
| Concurrency | Green threads (M:N) + channels |
| Error handling | Result types + `?` propagation |
| Type system | Strong static with inference |
| Structs/Interfaces | Structs + explicit traits (no inheritance) |
| Null handling | Nullable types (`T?`), non-null by default |
| Generics | Simple with trait constraints |
| Stdlib | HTTP, JSON, logging, crypto, TLS, networking |
| Toolchain | All-in-one `cyflym` binary |
| Module system | File-based + `cyflym.toml` manifest |
| GC | Phased: generational semi-space -> concurrent tri-color |
| Self-hosting | Long-term goal: rewrite compiler in Sans |

---

## Section 1: Syntax

### Primitive Types

| Type | Description | Size |
|------|-------------|------|
| `Bool` | Boolean | 1 byte |
| `Int8`, `Int16`, `Int32`, `Int64` | Signed integers | 1, 2, 4, 8 bytes |
| `UInt8`, `UInt16`, `UInt32`, `UInt64` | Unsigned integers | 1, 2, 4, 8 bytes |
| `Int` | Platform-native signed integer (64-bit on 64-bit platforms) | 8 bytes |
| `UInt` | Platform-native unsigned integer | 8 bytes |
| `Float32`, `Float64` | IEEE 754 floating point | 4, 8 bytes |
| `Float` | Alias for `Float64` | 8 bytes |
| `Byte` | Alias for `UInt8` | 1 byte |
| `Rune` | Unicode code point (alias for `Int32`) | 4 bytes |
| `String` | UTF-8 encoded, immutable, reference type | pointer + length |
| `()` | Unit type (void equivalent) | 0 bytes |

All primitive types are value types (copied on assignment) except `String`, which is an immutable reference type (cheap to copy — only copies the pointer and length).

### Variables

Immutable by default. `mut` keyword for mutable bindings.

```
let name = "cyflym"
let mut counter = 0
```

### Functions

Explicit parameter and return types. Local variables are type-inferred. Last expression is the implicit return value.

```
fn add(a Int, b Int) Int {
    a + b
}

fn greet(name String) String {
    let prefix = "Hello"
    format("{}, {}!", prefix, name)
}

// Explicit return for early exit
fn validate(age Int) Result<Int, Error> {
    if age < 0 {
        return Err(error("invalid age"))
    }
    Ok(age)
}
```

### Pattern Matching

First-class, exhaustive. Compiler ensures all cases are handled.

```
fn describe(value Int) String {
    match value {
        0 -> "zero",
        1..=9 -> "single digit",
        n if n < 0 -> "negative",
        _ -> "large"
    }
}

fn handle_result(r Result<User, Error>) String {
    match r {
        Ok(user) -> format("Found: {}", user.name),
        Err(e) -> format("Error: {}", e.message)
    }
}
```

### Nullable Types

Non-null by default. `T?` is syntactic sugar for `Option<T>` — they are the same type. `Option<T>` is a built-in enum with variants `Some(T)` and `None`.

```
// These are equivalent:
let email String? = None
let email Option<String> = None

let name String = "cyflym"       // never null
let email String? = None         // nullable

// Optional chaining and nil coalescing
let len = email?.len() ?? 0     // returns Int
let upper = email?.to_upper()   // returns String?

// Pattern matching — works because T? is Option<T>
match email {
    Some(e) -> send_to(e),
    None -> log("no email")
}

// if-let shorthand
if let Some(e) = email {
    send_to(e)
}
```

### Error Handling

Result types with `?` propagation. Errors are values, not exceptions.

The `?` operator works in two contexts:
1. **In functions returning `Result<T, E>`**: propagates the `Err`, unwraps the `Ok`
2. **In functions returning `T?` / `Option<T>`**: propagates `None`, unwraps the `Some`

For HTTP handlers that return `http.Response`, the `?` operator is **not** used directly. Instead, use explicit `match` or helper methods:

```
fn fetch_user(id String) Result<User, Error> {
    let conn = db.connect()?
    let row = conn.query("SELECT * FROM users WHERE id = ?", id)?
    let user = User.from_row(row)?
    Ok(user)
}

match fetch_user("123") {
    Ok(user) -> respond(json(user)),
    Err(e) -> respond(status(500), e.message)
}

// main() returns Result<(), Error> to allow ? usage
fn main() Result<(), Error> {
    let db = postgres.connect("postgres://localhost/myapp")?
    http.serve(":8080", router)?
    Ok(())
}
```

---

## Section 2: Type System & Data Structures

### Structs

Plain data containers. No inheritance.

```
struct User {
    name String,
    email String?,
    age Int,
}

let user = User {
    name: "Alice",
    email: Some("alice@example.com"),
    age: 30,
}

// Struct update syntax — uses `..` spread to distinguish from blocks
let updated = User { ..user, age: 31 }
```

### Enums

Algebraic data types with variant data.

```
enum HttpMethod {
    Get,
    Post(Body),
    Put(Body),
    Delete,
}

enum Status {
    Active,
    Inactive { reason String },
    Banned { reason String, until Timestamp? },
}

fn describe(method HttpMethod) String {
    match method {
        Get -> "GET",
        Post(body) -> format("POST: {} bytes", body.len()),
        Put(body) -> format("PUT: {} bytes", body.len()),
        Delete -> "DELETE"
    }
}
```

### Traits & Implementations

Explicit trait implementation. No implicit satisfaction.

```
trait Serializable {
    fn serialize(self) String
}

trait Deserializable {
    fn deserialize(data String) Result<Self, Error>
}

impl User : Serializable {
    fn serialize(self) String {
        json.encode(self)
    }
}

impl User : Deserializable {
    fn deserialize(data String) Result<User, Error> {
        json.decode<User>(data)
    }
}

fn send<T : Serializable>(value T, conn Connection) Result<(), Error> {
    conn.write(value.serialize())
}
```

### Generics

Simple type parameters with trait constraints. No higher-kinded types, no lifetime annotations.

```
struct List<T> {
    items []T,
}

fn map<T, U>(list List<T>, f fn(T) U) List<U> {
    ...
}

fn log_and_send<T : Serializable + Display>(value T) {
    print(value.display())
    send(value.serialize())
}
```

### Closures

Closures are anonymous functions that capture variables from their enclosing scope. Capture is **by reference** for GC-managed data (the GC keeps captured values alive). Closures that cross thread boundaries (via `spawn` or channel send) capture by **cloning** — the compiler automatically clones captured values into the closure.

```
// Basic closure
let double = fn(x Int) Int { x * 2 }

// Closure capturing from environment
let multiplier = 3
let times = fn(x Int) Int { x * multiplier }  // captures multiplier by ref

// Type alias for function/closure types
type Handler = fn(http.Request) http.Response
type Transform<T> = fn(T) T
type Predicate<T> = fn(T) Bool

// Closures as arguments
fn filter<T>(list List<T>, pred fn(T) Bool) List<T> { ... }
let adults = filter(users, fn(u) { u.age >= 18 })

// Closures crossing thread boundaries — auto-clone
let config = load_config()
spawn {
    // config is cloned into this green thread
    serve(config)
}
```

### Error Types

`Error` is a built-in trait, not a concrete type. Any type implementing `Error` can be used with `Result`.

```
// The Error trait
trait Error {
    fn message(self) String
}

// Custom error types
enum AppError {
    NotFound { resource String },
    Unauthorized,
    ValidationFailed { field String, reason String },
    Internal { cause String },
}

impl AppError : Error {
    fn message(self) String {
        match self {
            NotFound(r) -> format("not found: {}", r.resource),
            Unauthorized -> "unauthorized",
            ValidationFailed(v) -> format("{}: {}", v.field, v.reason),
            Internal(i) -> i.cause
        }
    }
}

// Pattern matching on error variants
fn handle(req http.Request) http.Response {
    match fetch_user(req.param("id")) {
        Ok(user) -> http.json(200, user),
        Err(AppError.NotFound(_)) -> http.json(404, { "error": "not found" }),
        Err(AppError.Unauthorized) -> http.json(401, { "error": "unauthorized" }),
        Err(e) -> http.json(500, { "error": e.message() })
    }
}

// The From trait — built-in, enables automatic ? conversion between error types
trait From<T> {
    fn from(t T) Self
}

// Implement From to allow ? to auto-convert between error types
impl AppError : From<postgres.Error> {
    fn from(e postgres.Error) AppError {
        AppError.Internal { cause: e.message() }
    }
}

// Now ? auto-converts postgres.Error -> AppError in functions returning Result<T, AppError>
```

### String Formatting & Interpolation

Sans supports both a `format()` function and string interpolation with `${}`.

```
let name = "world"

// Format function — type-checked at compile time
let greeting = format("Hello, {}!", name)

// String interpolation — syntactic sugar for format()
let greeting = "Hello, ${name}!"
let info = "User ${user.name} is ${user.age} years old"
let calc = "Result: ${a + b}"

// The Display trait controls how types appear in format/interpolation
trait Display {
    fn display(self) String
}
```

### Attributes

Attributes annotate declarations with metadata. Built-in attributes are provided; user-defined attributes are not supported in Phase 1.

```
// Derive — compiler generates trait implementations
#[derive(Serialize, Deserialize, Display)]
struct User { ... }

// JSON field mapping
#[json(name = "created_at")]
created Timestamp,

// Conditional compilation
#[cfg(target = "linux")]
fn linux_specific() { ... }

// Deprecation
#[deprecated(message = "use new_api() instead")]
fn old_api() { ... }

// Test attributes
#[test]
fn test_something() { ... }
```

### Control Flow

```
// if/else — expression-based
let label = if count > 0 { "some" } else { "none" }

// loop — infinite loop
loop {
    let msg = ch.recv()
    if msg == "quit" { break }
}

// for — iteration via Iterable trait
for item in collection { ... }
for i in 0..10 { ... }         // range: 0 to 9
for i in 0..=10 { ... }        // inclusive: 0 to 10
for (key, value) in map { ... } // maps iterate as (K, V) tuples

// while
while condition {
    ...
}
```

### Iterable Trait

Types implement `Iterable` to support `for` loops.

```
trait Iterable<T> {
    fn iter(self) Iterator<T>
}

trait Iterator<T> {
    fn next(mut self) Option<T>
}
```

### Operator Overloading & Equality

Operators are defined via traits. Types opt in by implementing the relevant trait.

```
trait Eq {
    fn eq(self, other Self) Bool
}

trait Ord : Eq {
    fn cmp(self, other Self) Ordering
}

trait Add<Rhs = Self> {
    type Output
    fn add(self, rhs Rhs) Output
}

// Compiler derives Eq for structs with all-Eq fields
#[derive(Eq)]
struct Point {
    x Float,
    y Float,
}

// Manual implementation
impl Point : Add {
    type Output = Point
    fn add(self, rhs Point) Point {
        Point { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}
```

### Built-in Collections

```
// Arrays — fixed size
let nums [3]Int = [1, 2, 3]

// Slices — dynamic
let names []String = ["Alice", "Bob"]

// Maps — use map() constructor to avoid ambiguity with blocks
let ages = map(("Alice", 30), ("Bob", 25))
let ages Map<String, Int> = map(
    ("Alice", 30),
    ("Bob", 25),
)

// Sets
let tags = set("api", "v2", "public")
```

---

## Section 3: Concurrency

### Green Threads

Lightweight, M:N scheduled onto OS threads. Cheap to spawn — supports millions of concurrent tasks.

```
spawn process_request(req)

let handle = spawn fetch_data(url)
let result = handle.await()
```

### Channels

Typed, bounded or unbounded.

```
let ch = channel<String>()          // unbounded
let ch = channel<String>(100)       // buffered

ch.send("hello")
let msg = ch.recv()

match ch.try_recv() {
    Ok(msg) -> handle(msg),
    Err(Closed) -> break,
    Err(Empty) -> continue
}
```

### Select

Wait on multiple channels simultaneously.

```
fn broker(requests channel<Request>, shutdown channel<()>) {
    loop {
        select {
            req from requests -> {
                spawn handle(req)
            },
            _ from shutdown -> {
                log("shutting down")
                break
            }
        }
    }
}
```

### Worker Pool Pattern

```
fn start_workers(n Int, jobs channel<Job>, results channel<Result<Output, Error>>) {
    for i in 0..n {
        spawn {
            for job in jobs {
                let result = process(job)
                results.send(result)
            }
        }
    }
}
```

### Mutex & Shared State

Safe shared state when channels aren't the right fit.

```
let counter = Mutex(0)

for i in 0..100 {
    spawn {
        let mut val = counter.lock()   // val is a MutexGuard<Int>
        *val += 1                      // * dereferences the guard via Deref trait (not a raw pointer)
    }
}

let cache = RwLock(Map<String, User>())

spawn {
    let read = cache.read()
    let user = read.get("alice")
}

spawn {
    let mut write = cache.write()
    write.set("bob", new_user)
}
```

### Concurrency Safety

Compiler-enforced via the `Send` trait:
- Only types implementing `Send` can cross thread boundaries (via `spawn` or channel send)
- All primitive types, immutable structs, and `Mutex<T>`/`RwLock<T>` are `Send`
- Mutable references are **not** `Send` — mutable data cannot be shared without synchronization
- `spawn` blocks auto-clone captured variables (captured values must implement `Send + Clone`)
- Channels transfer ownership — sending a value moves it; the sender can no longer access it
- No data races by construction

```
let mut count = 0
// spawn { count += 1 }       // compile error: mut variable not Send

let count = Mutex(0)
spawn {
    let mut val = count.lock()  // OK: Mutex<Int> is Send
    *val += 1
}
```

---

## Section 4: Web & API Development

### HTTP Server

Built into standard library. Zero dependencies to start serving.

```
import "http"

fn main() {
    let router = http.router()

    router.get("/health", health)
    router.get("/users/:id", get_user)
    router.post("/users", create_user)
    router.group("/api/v2", api_v2_routes)

    http.serve(":8080", router)
}

fn health(req http.Request) http.Response {
    http.ok("healthy")
}

fn get_user(req http.Request) http.Response {
    let id = req.param("id")
    match find_user(id) {
        Ok(user) -> http.json(200, user),
        Err(NotFound) -> http.json(404, { "error": "not found" }),
        Err(e) -> http.json(500, { "error": e.message })
    }
}

fn create_user(req http.Request) http.Response {
    match req.json<CreateUserInput>() {
        Ok(body) -> {
            match insert_user(body) {
                Ok(user) -> http.json(201, user),
                Err(e) -> http.json(500, json.obj(("error", e.message())))
            }
        },
        Err(e) -> http.json(400, json.obj(("error", "invalid body")))
    }
}

// Alternatively, handlers can return Result<http.Response, http.Error>
// and the framework converts errors to 500 responses automatically:
fn create_user_v2(req http.Request) Result<http.Response, http.Error> {
    let body = req.json<CreateUserInput>()?
    let user = insert_user(body)?
    Ok(http.json(201, user))
}
```

**Note on inline JSON objects**: Rather than using `{ "key": "value" }` syntax (which is ambiguous with blocks), use `json.obj()` for inline construction:

```
json.obj(("error", "not found"))                        // {"error": "not found"}
json.obj(("name", user.name), ("age", user.age))        // {"name": "...", "age": ...}
```

### Middleware

Functions that wrap handlers.

```
type Middleware = fn(http.Handler) http.Handler

fn logging(next http.Handler) http.Handler {
    fn(req http.Request) http.Response {
        let start = time.now()
        let resp = next(req)
        log.info("{} {} {} {}ms",
            req.method, req.path, resp.status,
            time.since(start).ms())
        resp
    }
}

fn auth(next http.Handler) http.Handler {
    fn(req http.Request) http.Response {
        match req.header("Authorization") {
            Some(token) -> {
                match verify_token(token) {
                    Ok(claims) -> next(req.with_context("claims", claims)),
                    Err(_) -> http.json(401, json.obj(("error", "invalid token")))
                }
            },
            None -> http.json(401, json.obj(("error", "missing token")))
        }
    }
}

router.use(logging)
router.use(auth)
router.get("/admin", admin_handler).with(require_role("admin"))
```

### JSON

First-class support. Compiler can derive serialization.

```
import "json"

#[derive(Serialize, Deserialize)]
struct User {
    name String,
    email String?,
    #[json(name = "created_at")]
    created Timestamp,
}

let data = json.encode(user)
let user = json.decode<User>(data)?

let obj = json.parse(raw_string)?
let name = obj.get("name")?.as_string()?
```

### Request Context

Pass data through the request pipeline without global state.

```
fn auth_middleware(next http.Handler) http.Handler {
    fn(req http.Request) http.Response {
        match req.header("Authorization") {
            Some(token) -> {
                match verify_token(token) {
                    Ok(claims) -> next(req.with_context("user_id", claims.sub)),
                    Err(_) -> http.json(401, json.obj(("error", "invalid token")))
                }
            },
            None -> http.json(401, json.obj(("error", "missing token")))
        }
    }
}

fn get_profile(req http.Request) http.Response {
    // context() returns Option<T> — None if key not set
    let user_id = req.context<String>("user_id")
    match user_id {
        None -> http.json(401, json.obj(("error", "not authenticated"))),
        Some(id) -> match load_profile(id) {
            Ok(profile) -> http.json(200, profile),
            Err(e) -> http.json(500, json.obj(("error", e.message())))
        }
    }
}
```

---

## Section 5: Module System, Tooling & Standard Library

### Module System

File-based. Directory structure is the module structure.

```
myapp/
  cyflym.toml
  src/
    main.cy
    models/
      user.cy          // module: models/user
      post.cy          // module: models/post
    handlers/
      users.cy         // module: handlers/users
    middleware/
      auth.cy          // module: middleware/auth
```

### Project Manifest

```toml
# cyflym.toml
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
postgres = "0.5.0"
redis = "1.2.0"
```

### Imports & Resolution Order

The compiler resolves imports in this order:
1. **Local modules** — paths matching `src/` directory structure
2. **Standard library** — built-in packages (`http`, `json`, `log`, etc.)
3. **External dependencies** — packages listed in `cyflym.toml` `[dependencies]`

If a name collision occurs between stdlib and an external package, the compiler raises an error and requires an explicit alias.

```
import "models/user"              // local: src/models/user.cy
import "handlers/users"           // local: src/handlers/users.cy
import "http"                     // stdlib
import "postgres"                 // external (from cyflym.toml)

// Aliased imports
import "models/user" as u
import "postgres" as pg

// Selective imports
import { Router, Request, Response } from "http"
```

### Visibility

`pub` for public, private by default.

```
pub struct User {
    pub name String,
    pub email String?,
    password_hash String,     // private
}

pub fn new(name String, email String?) User {
    User { name: name, email: email, password_hash: "" }
}

fn hash_password(raw String) String {   // private
    crypto.hash(raw)
}
```

### Toolchain

Single `cyflym` binary.

```
cyflym new myapp          # scaffold new project
sans build              # compile to native binary
sans run                # build and run
cyflym test               # run all tests
cyflym fmt                # format code
cyflym lint               # static analysis
cyflym doc                # generate documentation
cyflym dep add postgres   # add dependency
cyflym dep update         # update dependencies
cyflym lsp                # start language server
```

### Testing

Built-in test framework. Tests in `_test.cy` files.

```
import "testing"
import "http/testutil"

test "GET /users/:id returns user" {
    let app = setup_test_app()
    let resp = testutil.get(app, "/users/123")

    assert.eq(resp.status, 200)

    let user = resp.json<User>()?
    assert.eq(user.name, "Alice")
}

test "GET /users/:id returns 404 for missing user" {
    let app = setup_test_app()
    let resp = testutil.get(app, "/users/unknown")

    assert.eq(resp.status, 404)
}

// Table-driven tests
test "validate age" with [
    (25, true),
    (-1, false),
    (0, true),
    (200, false),
] as (age, expected) {
    assert.eq(validate_age(age).is_ok(), expected)
}
```

### Standard Library

| Package | Purpose |
|---------|---------|
| `http` | HTTP server, client, router, middleware |
| `json` | JSON encode/decode, dynamic access |
| `xml` | XML encode/decode |
| `log` | Structured logging |
| `crypto` | Hashing, encryption, TLS |
| `net` | TCP, UDP, DNS |
| `io` | Readers, writers, streams, files |
| `os` | Environment, process, signals |
| `fmt` | String formatting |
| `time` | Timestamps, durations, timers |
| `sync` | Mutex, RwLock, WaitGroup |
| `testing` | Test framework, assertions, benchmarks |
| `math` | Numeric operations |
| `strings` | String manipulation |
| `bytes` | Byte buffer operations |
| `regex` | Regular expressions |
| `encoding` | Base64, hex, URL encoding |
| `database` | Database trait (drivers are external) |

### Variadic Functions

Functions can accept a variable number of arguments of the same type using `...T` syntax. Variadic args are received as `[]T` (a slice).

```
fn sum(nums ...Int) Int {
    let mut total = 0
    for n in nums {
        total += n
    }
    total
}

let result = sum(1, 2, 3, 4)   // nums is []Int

// Spread a slice into variadic position
let numbers = [1, 2, 3]
let result = sum(numbers...)
```

**Note on `Any`**: Sans does **not** have a top type `Any`. The database trait uses `trait SqlParam` instead of `...Any` to maintain type safety. Types that can be passed as SQL parameters implement `SqlParam`.

### Database Trait

Defined in stdlib. Drivers are external packages.

```
// stdlib: "database"
pub trait Database {
    fn connect(url String) Result<Self, Error>
    fn close(self) Result<(), Error>
}

pub trait Connection {
    fn query<T : Deserializable>(self, sql String, params ...SqlParam) Result<List<T>, Error>
    fn exec(self, sql String, params ...SqlParam) Result<ExecResult, Error>
    fn transaction(self, f fn(Tx) Result<(), Error>) Result<(), Error>
}

// External driver usage
import "postgres"

fn main() {
    let db = postgres.connect("postgres://localhost/myapp")?
    let users = db.query<User>("SELECT * FROM users WHERE age > ?", 21)?
}
```

---

## Section 6: Memory Model, GC & Compilation Architecture

### Memory Safety

Sans is a **GC-managed language**. The garbage collector handles all memory reclamation. There is no borrow checker or lifetime system like Rust.

Move semantics apply in exactly **two** cases:
1. **Channel sends** — sending a value through a channel transfers ownership to prevent data races
2. **Explicit `move` keyword** — for rare cases where you want to transfer ownership explicitly

All other value passing is by reference (GC keeps values alive as long as they're reachable). Function parameters are passed by reference; the GC tracks all references.

```
let name = "Alice"        // immutable, GC-managed
let mut count = 0         // explicitly mutable

// Normal function calls — pass by reference, GC-managed
fn process(data []Byte) Result<(), Error> {
    // data is a reference; caller can still use it after this call
    ...
}

// Channel sends — move semantics to prevent data races
let data = build_payload()
ch.send(data)             // data moves into channel
// print(data)            // compile error: data was moved

// Explicit move (rare)
let data2 = move data     // transfers ownership

// move is not needed for closures — spawn auto-clones captured values.
// move is primarily for transferring ownership to channels or between variables.
```

### Program Entry Point

`main()` accepts two signatures:

```
fn main() { ... }                        // no error handling
fn main() Result<(), Error> { ... }      // allows ? operator, prints error on failure
```

### Unsafe & FFI

For interoperability with C libraries (critical for database drivers, system calls, etc.), Sans provides an `unsafe` block and FFI declarations.

```
// FFI: declare external C functions
extern "C" {
    fn malloc(size UInt) *Byte
    fn free(ptr *Byte)
    fn strlen(s *Byte) UInt
}

// Unsafe block — required for raw pointer operations
unsafe {
    let ptr = malloc(1024)
    // ... use raw pointer
    free(ptr)
}

// Safe wrapper pattern — expose safe API over unsafe internals
pub fn allocate_buffer(size UInt) []Byte {
    unsafe {
        let ptr = malloc(size)
        slice_from_raw(ptr, size)   // converts to GC-managed slice
    }
}
```

Raw pointers (`*T`) and `unsafe` blocks are only available within `unsafe` — the compiler rejects raw pointer operations outside of `unsafe` blocks. This keeps the unsafe surface area explicit and auditable.

### Bounds Checking

All array/slice access is bounds-checked. Compiler elides checks where provably safe.

```
let items = [1, 2, 3]
let x = items[5]          // runtime panic: index out of bounds

let x = items.get(5)      // returns Int? (None if out of bounds)

for i in 0..items.len() {
    items[i]               // no bounds check — compiler proves safety
}
```

### Garbage Collector

**Phase 1 (MVP)**: Generational semi-space collector
- Two generations: young (frequent, fast) and old (infrequent)
- Young gen uses semi-space copying — fast allocation, good cache locality
- Simple to implement correctly

**Phase 2 (Maturity)**: Concurrent tri-color mark-and-sweep
- Tri-color marking with write barriers
- Most GC work concurrent with application threads
- Sub-millisecond pause times for low-latency APIs
- Generational — most objects die young

```
// GC tuning via environment
// CYFLYM_GC_THREADS=4
// CYFLYM_GC_HEAP_MAX=2G

// Or in code
import "runtime"

fn main() {
    runtime.gc_set(heap_max: 2.gb(), threads: 4)
    ...
}
```

### Compilation Pipeline

```
Source (.cy)
    |
    v
  Lexer/Parser --> AST
    |
    v
  Type Checker --> Typed AST (inference, null safety, trait resolution)
    |
    v
  Sans IR --> (clean abstraction layer for self-hosting)
    |
    v
  LLVM IR --> (Phase 1: LLVM backend)
    |           (Phase 3: swap for custom native codegen)
    v
  Native Binary (x86-64, ARM64)
```

### Sans IR

The intermediate representation is the critical boundary between frontend and backend. It preserves Sans semantics (green threads, GC roots, pattern matching) while mapping cleanly to LLVM or a future custom backend.

When Sans becomes self-hosting, the compiler rewrite only needs to emit this same IR. The backend stays the same or gets swapped independently.

### Self-Hosting Roadmap

| Phase | Compiler | Backend | Runtime | GC |
|-------|----------|---------|---------|-----|
| 1 — Bootstrap | Rust | LLVM | Rust | Generational semi-space |
| 2 — Stdlib | Rust | LLVM | Sans + Rust | Generational semi-space |
| 3 — Self-hosting | Sans | LLVM or custom | Sans | Concurrent tri-color |

---

## File Extension

`.cy`

## Example: Complete API Server

```
import "http"
import "json"
import "log"
import "postgres"

#[derive(Serialize, Deserialize)]
struct User {
    id String,
    name String,
    email String?,
}

#[derive(Deserialize)]
struct CreateUserInput {
    name String,
    email String?,
}

fn main() Result<(), Error> {
    let db = postgres.connect("postgres://localhost/myapp")?

    let router = http.router()
    router.use(logging)
    router.get("/users/:id", fn(req) { get_user(req, db) })
    router.post("/users", fn(req) { create_user(req, db) })

    log.info("Starting server on :8080")
    http.serve(":8080", router)?
    Ok(())
}

fn logging(next http.Handler) http.Handler {
    fn(req http.Request) http.Response {
        let start = time.now()
        let resp = next(req)
        log.info("${req.method} ${req.path} ${resp.status} ${time.since(start).ms()}ms")
        resp
    }
}

fn get_user(req http.Request, db postgres.Connection) http.Response {
    let id = req.param("id")
    match db.query<User>("SELECT * FROM users WHERE id = ?", id) {
        Ok(users) -> {
            match users.first() {
                Some(user) -> http.json(200, user),
                None -> http.json(404, json.obj(("error", "not found")))
            }
        },
        Err(e) -> {
            log.error("db error: ${e}")
            http.json(500, json.obj(("error", "internal error")))
        }
    }
}

fn create_user(req http.Request, db postgres.Connection) http.Response {
    match req.json<CreateUserInput>() {
        Ok(input) -> {
            let id = generate_id()
            match db.exec("INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
                          id, input.name, input.email) {
                Ok(_) -> {
                    let user = User { id: id, name: input.name, email: input.email }
                    http.json(201, user)
                },
                Err(e) -> http.json(500, json.obj(("error", e.message())))
            }
        },
        Err(_) -> http.json(400, json.obj(("error", "invalid request body")))
    }
}
```
