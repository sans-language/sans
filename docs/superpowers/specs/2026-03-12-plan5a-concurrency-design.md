# Plan 5a: Concurrency (Spawn + Channels) Design Spec

## Goal

Add OS-thread-based concurrency to Cyflym with `spawn`, `channel`, and `join` — no runtime, no external dependencies beyond libc.

## Decisions

- **Threading model:** 1:1 OS threads via pthreads. No green threads, no runtime scheduler.
- **Channels:** Unbounded, mutex-guarded growable queue. Sends never block; receives block until a value is available.
- **Spawn model:** Fire-and-forget. Communication happens via channels. `spawn` returns a `JoinHandle` for explicit joining.
- **Join behavior:** Explicit via `handle.join()`. Unjoined threads are terminated when main exits.
- **Implementation approach:** Dedicated IR instructions that codegen lowers to pthread syscalls and heap-allocated channel structs.

## Syntax

```cyflym
// Spawn a thread — returns a JoinHandle
let handle = spawn some_function(arg1, arg2)

// Join a thread — blocks until it finishes
handle.join()

// Create a channel — returns a (sender, receiver) pair
let (tx, rx) = channel<Int>()

// Send a value (non-blocking, unbounded)
tx.send(42)

// Receive a value (blocks until available)
let val = rx.recv()
```

### Keywords

- `spawn` — new keyword, precedes a function call expression
- `channel` — new keyword, used as `channel<Type>()`

### Destructuring Let

`let (a, b) = expr` is new syntax, initially only supported for `channel<T>()` expressions. No general-purpose tuple type is introduced. The **type checker** enforces this restriction — the parser accepts any expression in `LetDestructure`, but the type checker rejects it unless the expression produces a channel pair.

### Generic Delimiter Parsing

`channel<T>()` reuses the existing `Lt` (`<`) and `Gt` (`>`) tokens as generic delimiters. Since `channel` is a keyword, there is no ambiguity — `channel<` always begins a generic parameter, never a comparison. The parser handles this specifically in the `channel` keyword branch.

### Method Calls

`.send(val)`, `.recv()`, `.join()` use existing method call syntax (`Expr::MethodCall`). No new AST nodes needed for these — the type checker and IR lowering distinguish them from user-defined methods based on the receiver type.

## Type System

### New Types

```rust
pub enum Type {
    // ... existing ...
    JoinHandle,
    Sender { inner: Box<Type> },
    Receiver { inner: Box<Type> },
}
```

### Type Checking Rules

| Expression | Type |
|---|---|
| `spawn f(args)` | `JoinHandle` — `f` must be a known function, args type-checked normally |
| `channel<T>()` | Produces `(Sender<T>, Receiver<T>)` via destructuring let |
| `tx.send(val)` | `val` must match the `T` of the `Sender<T>` — statement only, no return value used |
| `rx.recv()` | Returns `T` of the `Receiver<T>` |
| `handle.join()` | Returns `Int` (0) — only valid on `JoinHandle`. Spawned function's return value is discarded; use channels to communicate results. |

### Type Errors

- `spawn` on a non-function or with wrong arg types
- `.send(val)` where val type doesn't match channel element type
- `.recv()` on a non-Receiver
- `.join()` on a non-JoinHandle
- `.send()` / `.recv()` / `.join()` with wrong argument counts

## AST Changes

### New Expression Variants

```rust
pub enum Expr {
    // ... existing ...
    Spawn {
        function: String,
        args: Vec<Expr>,
        span: Span,
    },
    ChannelCreate {
        element_type: TypeName,
        span: Span,
    },
}
```

### New Statement Variant

```rust
pub enum Stmt {
    // ... existing ...
    LetDestructure {
        names: Vec<String>,
        value: Expr,
        span: Span,
    },
}
```

### New Tokens

- `Token::Spawn` — keyword `spawn`
- `Token::Channel` — keyword `channel`

### Parser Rules

- `spawn f(args)`: `spawn` keyword followed by a call expression → `Expr::Spawn`
- `channel<T>()`: `channel` keyword, `<`, type name, `>`, `(`, `)` → `Expr::ChannelCreate`
- `let (a, b) = expr`: `let`, `(`, ident, `,`, ident, `)`, `=`, expr → `Stmt::LetDestructure`

## IR Instructions

```rust
pub enum Instruction {
    // ... existing ...

    // Thread operations
    ThreadSpawn {
        dest: Reg,           // JoinHandle (opaque pointer as i64)
        function: String,    // name of function to run
        args: Vec<Reg>,      // arguments to pass
    },
    ThreadJoin {
        handle: Reg,         // JoinHandle register
    },

    // Channel operations
    ChannelCreate {
        tx_dest: Reg,        // Sender (opaque pointer as i64)
        rx_dest: Reg,        // Receiver (opaque pointer as i64)
    },
    ChannelSend {
        tx: Reg,             // Sender register
        value: Reg,          // value to send (i64)
        // No dest register — send is statement-only, not used in value position
    },
    ChannelRecv {
        dest: Reg,           // destination for received value
        rx: Reg,             // Receiver register
    },
}
```

### IR Lowering

| AST | IR |
|---|---|
| `Expr::Spawn { function, args }` | `ThreadSpawn { dest, function, args }` |
| `Expr::ChannelCreate { element_type }` | `ChannelCreate { tx_dest, rx_dest }` |
| `MethodCall { method: "send" }` on Sender | `ChannelSend { tx, value }` |
| `MethodCall { method: "recv" }` on Receiver | `ChannelRecv { dest, rx }` |
| `MethodCall { method: "join" }` on JoinHandle | `ThreadJoin { handle }` |

The IR lowering needs type information to distinguish built-in method calls from user-defined ones. The lowering pass will track register types — `IrType` gains new variants `Sender`, `Receiver`, and `JoinHandle` so the IR lowering can match on receiver type when encountering `MethodCall` and emit the correct IR instruction instead of a regular `Call`.

### IR Type Changes

```rust
pub enum IrType {
    // ... existing: Int, Bool, Str, Struct(String), Enum(String) ...
    Sender,
    Receiver,
    JoinHandle,
}
```

## Codegen (LLVM)

### Extern Declarations

The codegen declares these libc/pthread functions as extern in the LLVM module:

- `pthread_create(thread*, attr*, fn_ptr, arg_ptr) -> i32`
- `pthread_join(thread, retval**) -> i32`
- `pthread_mutex_init(mutex*, attr*) -> i32`
- `pthread_mutex_lock(mutex*) -> i32`
- `pthread_mutex_unlock(mutex*) -> i32`
- `pthread_cond_init(cond*, attr*) -> i32`
- `pthread_cond_wait(cond*, mutex*) -> i32`
- `pthread_cond_signal(cond*) -> i32`
- `malloc(size) -> ptr`
- `realloc(ptr, size) -> ptr`
- `free(ptr)`

### Channel Data Structure

A channel is a heap-allocated struct:

```
{
    i64* buffer,        // growable array of i64 values
    i64 capacity,       // current buffer capacity
    i64 count,          // number of items in queue
    i64 head,           // read index
    i64 tail,           // write index
    pthread_mutex_t,    // mutex for synchronization
    pthread_cond_t      // condvar for recv blocking
}
```

- `ChannelCreate`: malloc the struct, malloc initial buffer (capacity 16), init mutex and condvar. Both tx_dest and rx_dest point to the same struct (as i64).
- `ChannelSend`: lock mutex → if count == capacity, malloc new buffer at 2x capacity, copy elements in order from head to head+count (unwrapping the circular layout into a contiguous block), free old buffer, reset head=0 and tail=count → write value at buffer[tail % capacity] → increment tail and count → signal condvar → unlock mutex.
- `ChannelRecv`: lock mutex → while count == 0, wait on condvar → read value at buffer[head % capacity] → increment head, decrement count → unlock mutex → return value.

### Argument Passing for Spawned Threads

All values in the IR are i64. Struct and enum arguments are pointers cast to i64 — they are passed by pointer (shared memory) to the spawned thread. No deep copy is performed. This is intentionally simple; ownership/safety enforcement is deferred to Plan 5b's Send trait.

### Memory Management

Channel structs, buffers, and arg structs are heap-allocated and **not freed** — they are leaked until process exit. Cleanup and resource management are deferred to the GC phase of the project. This is acceptable for the current stage.

### Thread Spawn

`ThreadSpawn` requires wrapping the target function for pthread compatibility:

1. **Arg struct:** Malloc a struct holding all i64 args for the target function.
2. **Trampoline function:** Generate an LLVM function `__trampoline_<name>_<id>(void* arg) -> void*` that:
   - Casts arg pointer back to the arg struct type
   - Loads each arg from the struct
   - Calls the real target function
   - Frees the arg struct
   - Returns null
3. **pthread_create:** Allocate space for `pthread_t`, call `pthread_create` with the trampoline and arg struct pointer.
4. **Result:** The `pthread_t` value (cast to i64) is stored in the dest register as the JoinHandle.

### Thread Join

`ThreadJoin`: call `pthread_join` with the handle (cast from i64 back to pthread_t), pass null for retval.

## Testing

### Unit Tests (~23 new)

**Lexer (2):**
- Tokenize `spawn` keyword
- Tokenize `channel` keyword

**Parser (~6):**
- Parse `spawn f(x)` expression
- Parse `channel<Int>()` expression
- Parse `let (tx, rx) = channel<Int>()` destructuring
- Parse `.send(val)` as method call
- Parse `.recv()` as method call
- Parse `.join()` as method call

**Type Checker (~8):**
- `spawn f(x)` produces JoinHandle type
- `channel<Int>()` in destructuring let gives Sender<Int> and Receiver<Int>
- `.send(val)` with matching type passes
- `.send(val)` with wrong type produces error
- `.recv()` returns correct element type
- `.join()` on JoinHandle passes
- `.join()` on non-JoinHandle produces error
- `.send()` on non-Sender produces error

**IR (~4):**
- Spawn lowers to ThreadSpawn instruction
- Channel create lowers to ChannelCreate instruction
- Send/recv lower to ChannelSend/ChannelRecv instructions
- Join lowers to ThreadJoin instruction

**Codegen (~3):**
- ThreadSpawn emits pthread_create call
- ChannelCreate/Send/Recv emit correct LLVM IR
- ThreadJoin emits pthread_join call

### E2E Fixtures (3)

**`spawn_basic.cy`** — Spawn a thread that sends a value through a channel, main receives it and exits with that value.

**`spawn_join.cy`** — Spawn a thread, join it, verify completion by exiting with a known value.

**`channel_pingpong.cy`** — Two threads passing values through channels, exit with final computed value.

### Estimated Total: ~174 tests (151 existing + ~23 new)

## Deferred to Plan 5b

- Select expressions (waiting on multiple channels)
- Mutex / RwLock primitives
- Send trait for thread-safety enforcement
- Bounded channels
- Closures / anonymous functions for spawn
