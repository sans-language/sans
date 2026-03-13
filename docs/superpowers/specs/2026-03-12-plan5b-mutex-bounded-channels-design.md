# Plan 5b: Mutex & Bounded Channels Design Spec

## Goal

Add mutex primitives and bounded channels to Cyflym's concurrency toolkit — explicit lock/unlock mutex and blocking-send bounded channels, all via direct pthread syscalls.

## Scope

- Mutex with explicit lock/unlock
- Bounded channels with blocking send when full

**Out of scope (deferred):** Select expressions, RwLock, Send trait, closures for spawn.

## Decisions

- **Mutex model:** Explicit lock/unlock. `lock()` returns the stored value, `unlock(new_value)` writes and releases. No guard pattern (requires destructors/RAII not yet in the language).
- **Bounded channel behavior:** `send()` blocks when buffer is full. `recv()` signals blocked senders after consuming. Matches Go's buffered channel semantics.
- **Channel struct unification:** Unbounded and bounded channels share the same struct layout (224 bytes) with an `is_bounded` flag. This keeps send/recv codegen uniform.
- **Implementation approach:** Dedicated IR instructions that codegen lowers to pthread syscalls and heap-allocated structs, same pattern as Plan 5a.

## Syntax

```cyflym
// Create a mutex wrapping an initial value
let m = mutex(0)

// Lock — blocks until acquired, returns the current value
let val = m.lock()

// Unlock with a new value
m.unlock(val + 1)

// Create a bounded channel with capacity 10
let (tx, rx) = channel<Int>(10)

// Send blocks if buffer is full
tx.send(42)

// Recv blocks if buffer is empty (unchanged from unbounded)
let val = rx.recv()

// Unbounded channels unchanged
let (tx, rx) = channel<Int>()
```

### Keywords

- `mutex` — new keyword, used as `mutex(initial_value)`

### Method Calls

`.lock()`, `.unlock(val)` use existing `Expr::MethodCall` syntax. The type checker and IR lowering distinguish them from user-defined methods based on receiver type (same pattern as `.send()` / `.recv()` / `.join()`).

## Type System

### New Type

```rust
pub enum Type {
    // ... existing ...
    Mutex { inner: Box<Type> },
}
```

### Type Checking Rules

| Expression | Type |
|---|---|
| `mutex(val)` | `Mutex<T>` where T is the type of `val` |
| `m.lock()` | Returns `T` of the `Mutex<T>` |
| `m.unlock(val)` | `val` must match `T` — statement only, no return value used |
| `channel<T>(capacity)` | Same as `channel<T>()` — produces `(Sender<T>, Receiver<T>)`. `capacity` must be `Int`. |

### Type Errors

- `m.lock()` on non-Mutex
- `m.unlock(val)` on non-Mutex
- `m.unlock(val)` where val type doesn't match Mutex element type
- `m.lock()` with arguments
- `m.unlock()` with wrong argument count
- `channel<T>(cap)` where cap is not Int

## AST Changes

### New Expression Variant

```rust
pub enum Expr {
    // ... existing ...
    MutexCreate {
        value: Box<Expr>,
        span: Span,
    },
}
```

### Modified Expression Variant

```rust
Expr::ChannelCreate {
    element_type: TypeName,
    capacity: Option<Box<Expr>>,  // None = unbounded, Some = bounded
    span: Span,
}
```

### New Token

- `Token::Mutex` — keyword `mutex`

### Parser Rules

- `mutex(expr)`: `mutex` keyword, `(`, expression, `)` → `Expr::MutexCreate`
- `channel<T>(expr)`: existing `channel` keyword path, but after `>`, `(`, check if there's an argument → `Expr::ChannelCreate { capacity: Some(expr) }` or `Expr::ChannelCreate { capacity: None }`
- `.lock()`, `.unlock(val)`: existing `Expr::MethodCall` — no new AST nodes needed

## IR Instructions

### New Instructions

```rust
pub enum Instruction {
    // ... existing ...

    // Mutex operations
    MutexCreate {
        dest: Reg,          // Mutex (opaque pointer as i64)
        value: Reg,         // initial value to store
    },
    MutexLock {
        dest: Reg,          // destination for the stored value
        mutex: Reg,         // Mutex register
    },
    MutexUnlock {
        mutex: Reg,         // Mutex register
        value: Reg,         // new value to store
    },

    // Bounded channel creation
    ChannelCreateBounded {
        tx_dest: Reg,       // Sender (opaque pointer as i64)
        rx_dest: Reg,       // Receiver (opaque pointer as i64)
        capacity: Reg,      // buffer capacity
    },
}
```

### IrType Changes

```rust
enum IrType {
    // ... existing: Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle ...
    Mutex,
}
```

### IR Lowering

| AST | IR |
|---|---|
| `Expr::MutexCreate { value }` | `MutexCreate { dest, value }` |
| `MethodCall { method: "lock" }` on Mutex | `MutexLock { dest, mutex }` |
| `MethodCall { method: "unlock" }` on Mutex | `MutexUnlock { mutex, value }` |
| `ChannelCreate { capacity: Some(cap) }` in LetDestructure | `ChannelCreateBounded { tx_dest, rx_dest, capacity }` |
| `ChannelCreate { capacity: None }` in LetDestructure | `ChannelCreate { tx_dest, rx_dest }` (unchanged) |

Existing `ChannelSend` and `ChannelRecv` IR instructions are unchanged — the bounded vs unbounded distinction is handled purely in codegen based on the channel struct's `is_bounded` flag.

## Codegen (LLVM)

### Mutex Data Structure

Heap-allocated struct (9 i64s = 72 bytes):

```
{
    i64 value,              // offset 0 — stored value
    pthread_mutex_t,        // offsets 1-8 (64 bytes)
}
```

- **MutexCreate:** `malloc(72)`, store initial value at offset 0, `pthread_mutex_init` at offset 1. Return pointer as i64.
- **MutexLock:** `pthread_mutex_lock` at offset 1, load value from offset 0, return it.
- **MutexUnlock:** Store new value at offset 0, `pthread_mutex_unlock` at offset 1.

### Channel Data Structure (Updated)

Both unbounded and bounded channels use the same struct layout (28 i64s = 224 bytes):

```
{
    i64* buffer,            // offset 0
    i64 capacity,           // offset 1
    i64 count,              // offset 2
    i64 head,               // offset 3
    i64 tail,               // offset 4
    pthread_mutex_t,        // offsets 5-12 (64 bytes)
    pthread_cond_t,         // offsets 13-19 (56 bytes) — recv condvar ("not empty")
    pthread_cond_t,         // offsets 20-26 (56 bytes) — send condvar ("not full")
    i64 is_bounded,         // offset 27 (0 = unbounded, 1 = bounded)
}
```

This is an expansion of the Plan 5a layout (was 152 bytes / 19 i64s). The existing `ChannelCreate` codegen must be updated to use the new 224-byte layout.

### ChannelCreate (Updated — Unbounded)

Same as before but:
- Allocates 224 bytes instead of 152
- Initializes second condvar at offset 20
- Sets `is_bounded` to 0 at offset 27

### ChannelCreateBounded

Same as unbounded but:
- Sets capacity to user-provided value (instead of hardcoded 16)
- Allocates `capacity * 8` bytes for buffer
- Sets `is_bounded` to 1 at offset 27

### ChannelSend (Updated)

- Lock mutex at offset 5
- If `is_bounded` (load offset 27): while `count == capacity`, wait on send condvar at offset 20
- Write value at `buffer[tail % capacity]`
- Increment tail and count
- If unbounded and `count == capacity`: realloc buffer at 2x capacity, copy elements unwrapping circular layout, reset head=0, tail=count, update capacity
- Signal recv condvar at offset 13
- Unlock mutex

### ChannelRecv (Updated)

- Lock mutex at offset 5
- While `count == 0`, wait on recv condvar at offset 13
- Read value from `buffer[head % capacity]`
- Increment head, decrement count
- If `is_bounded`, signal send condvar at offset 20
- Unlock mutex

### Memory Management

Same as Plan 5a — mutex and channel structs are leaked until process exit. Cleanup deferred to GC phase.

## Testing

### Unit Tests (~14 new)

**Lexer (1):**
- Tokenize `mutex` keyword

**Parser (~3):**
- Parse `mutex(expr)` expression
- Parse `channel<Int>(10)` bounded channel with capacity
- Parse `.lock()` and `.unlock(val)` as method calls

**Type Checker (~5):**
- `mutex(0)` produces `Mutex<Int>` type
- `.lock()` on Mutex returns inner type
- `.unlock(val)` with matching type passes
- `.unlock(val)` with wrong type produces error
- `.lock()` on non-Mutex produces error
- `channel<Int>(10)` with non-Int capacity produces error

**IR (~3):**
- MutexCreate lowers to correct instruction
- Lock/unlock lower to MutexLock/MutexUnlock
- Bounded channel lowers to ChannelCreateBounded

**Codegen (~2):**
- MutexCreate/Lock/Unlock emit correct LLVM IR
- ChannelCreateBounded emits correct LLVM IR

### E2E Fixtures (3)

**`mutex_basic.cy`** — Create mutex with initial value, lock, read, unlock with new value, lock again, exit with final value.

**`mutex_threaded.cy`** — Two threads incrementing a shared mutex counter, exit with final count.

**`channel_bounded.cy`** — Bounded channel with small capacity, sender and receiver in separate threads, exit with sum of received values.

### Estimated Total: ~194 tests (177 existing + ~17 new)

## Deferred

- Select expressions (waiting on multiple channels)
- RwLock
- Send trait for thread-safety enforcement
- Bounded channel try_send (non-blocking send returning success/failure)
- Closures / anonymous functions for spawn
