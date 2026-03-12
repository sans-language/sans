# Cyflym Language Design Specification

## Overview

**Cyflym** is a speed-first, memory-safe backend programming language with automatic garbage collection, designed for building APIs, web servers, and general-purpose programs. It compiles to native machine code via LLVM, features green thread concurrency, and ships with a batteries-included standard library and all-in-one toolchain.

The name "Cyflym" comes from Welsh, meaning "fast."

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
| Self-hosting | Long-term goal: rewrite compiler in Cyflym |

---

## Section 1: Syntax

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

Non-null by default. `?` suffix marks nullable types. Compiler enforces safe access.

```
let name String = "cyflym"       // never null
let email String? = None         // nullable

let len = email?.len() ?? 0     // nil coalescing
let upper = email?.to_upper()   // returns String?

match email {
    Some(e) -> send_to(e),
    None -> log("no email")
}
```

### Error Handling

Result types with `?` propagation. Errors are values, not exceptions.

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

// Update syntax
let updated = user { age: 31 }
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

### Built-in Collections

```
// Arrays — fixed size
let nums [3]Int = [1, 2, 3]

// Slices — dynamic
let names []String = ["Alice", "Bob"]

// Maps
let ages Map<String, Int> = {
    "Alice": 30,
    "Bob": 25,
}

// Sets
let tags Set<String> = {"api", "v2", "public"}
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
        let mut val = counter.lock()
        *val += 1
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

Compiler-enforced:
- Mutable data cannot be shared across threads without `Mutex` or `RwLock`
- Channels enforce ownership transfer — sending a value moves it
- No data races by construction

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
    let body = req.json<CreateUserInput>()?
    let user = insert_user(body)?
    http.json(201, user)
}
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
                    Err(_) -> http.json(401, { "error": "invalid token" })
                }
            },
            None -> http.json(401, { "error": "missing token" })
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
        let claims = verify_token(req.header("Authorization")?)?
        next(req.with_context("user_id", claims.sub))
    }
}

fn get_profile(req http.Request) http.Response {
    let user_id = req.context<String>("user_id")?
    let profile = load_profile(user_id)?
    http.json(200, profile)
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

### Imports

```
import "models/user"
import "handlers/users"
import "middleware/auth"
import "http"

// External packages
import "postgres"
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
cyflym build              # compile to native binary
cyflym run                # build and run
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

### Database Trait

Defined in stdlib. Drivers are external packages.

```
// stdlib: "database"
pub trait Database {
    fn connect(url String) Result<Self, Error>
    fn close(self) Result<(), Error>
}

pub trait Connection {
    fn query<T : Deserializable>(self, sql String, params ...Any) Result<List<T>, Error>
    fn exec(self, sql String, params ...Any) Result<ExecResult, Error>
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

Enforced at compile time. No manual memory management, no unsafe pointer arithmetic by default.

```
let name = "Alice"        // immutable
let mut count = 0         // explicitly mutable

// Ownership transfer through channels
let data = build_payload()
ch.send(data)             // data moves into channel
// print(data)            // compile error: data was moved

// Borrowing for function calls
fn process(data []Byte) Result<(), Error> {
    // data is borrowed, caller retains ownership
    ...
}
```

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
  Cyflym IR --> (clean abstraction layer for self-hosting)
    |
    v
  LLVM IR --> (Phase 1: LLVM backend)
    |           (Phase 3: swap for custom native codegen)
    v
  Native Binary (x86-64, ARM64)
```

### Cyflym IR

The intermediate representation is the critical boundary between frontend and backend. It preserves Cyflym semantics (green threads, GC roots, pattern matching) while mapping cleanly to LLVM or a future custom backend.

When Cyflym becomes self-hosting, the compiler rewrite only needs to emit this same IR. The backend stays the same or gets swapped independently.

### Self-Hosting Roadmap

| Phase | Compiler | Backend | Runtime | GC |
|-------|----------|---------|---------|-----|
| 1 — Bootstrap | Rust | LLVM | Rust | Generational semi-space |
| 2 — Stdlib | Rust | LLVM | Cyflym + Rust | Generational semi-space |
| 3 — Self-hosting | Cyflym | LLVM or custom | Cyflym | Concurrent tri-color |

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

fn main() {
    let db = postgres.connect("postgres://localhost/myapp")?

    let router = http.router()
    router.use(logging)
    router.get("/users/:id", fn(req) { get_user(req, db) })
    router.post("/users", fn(req) { create_user(req, db) })

    log.info("Starting server on :8080")
    http.serve(":8080", router)
}

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

fn get_user(req http.Request, db postgres.Connection) http.Response {
    let id = req.param("id")
    match db.query<User>("SELECT * FROM users WHERE id = ?", id) {
        Ok(users) -> {
            match users.first() {
                Some(user) -> http.json(200, user),
                None -> http.json(404, { "error": "not found" })
            }
        },
        Err(e) -> {
            log.error("db error: {}", e)
            http.json(500, { "error": "internal error" })
        }
    }
}

fn create_user(req http.Request, db postgres.Connection) http.Response {
    let input = req.json<CreateUserInput>()?
    let id = generate_id()
    db.exec("INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
        id, input.name, input.email)?
    let user = User { id: id, name: input.name, email: input.email }
    http.json(201, user)
}
```
