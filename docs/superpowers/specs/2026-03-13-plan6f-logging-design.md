# Plan 6f: Logging Design Spec

## Goal

Add leveled logging to stderr via built-in functions. Users can log messages at different severity levels (debug, info, warn, error) and control which levels are emitted. No new types — everything uses existing function call syntax.

## Scope

**Log functions (free functions):**
- `log_debug(msg)` — log at DEBUG level
- `log_info(msg)` — log at INFO level
- `log_warn(msg)` — log at WARN level
- `log_error(msg)` — log at ERROR level
- `log_set_level(level)` — set minimum log level (0=DEBUG, 1=INFO, 2=WARN, 3=ERROR)

**Out of scope (deferred):** Structured logging, log to file, timestamps, log format customization, per-module log levels, log context/fields, async logging, log rotation.

## Decisions

- **No new types.** All log functions take a String message and return Int (0). `log_set_level` takes an Int and returns Int (0).
- **Stderr output.** Log messages go to stderr via `fprintf(stderr, ...)`, keeping them separate from `print()` which goes to stdout. This is important for programs that pipe stdout.
- **Format:** `[LEVEL] message\n` — simple and readable. No timestamps (would require platform-specific time calls and add complexity).
- **Global log level.** A single global `int` in the C runtime controls filtering. Default level is 0 (DEBUG — show everything). Messages below the current level are silently dropped.
- **Level constants:** DEBUG=0, INFO=1, WARN=2, ERROR=3. Users pass integers to `log_set_level`.
- **Implementation pattern.** Inline C calls in codegen (like `print`), not a separate C runtime file. The log functions are simple enough to implement as `fprintf(stderr, "[LEVEL] %s\n", msg)` with a global level check. However, since we need a global variable for the log level, we'll use a small C runtime file `runtime/log.c`.
- **Return value.** All log functions return 0 (Int), matching the `print()` pattern where the return value is unused but the type system requires one.

## Syntax

```sans
fn main() Int {
    log_set_level(1)  // INFO and above

    log_debug("this won't print")
    log_info("server starting")
    log_warn("disk space low")
    log_error("connection failed")

    0
}
```

Output (to stderr):
```
[INFO] server starting
[WARN] disk space low
[ERROR] connection failed
```

## Type System

### Type Checking Rules

| Expression | Args | Returns |
|---|---|---|
| `log_debug(msg)` | `msg: String` | `Int` |
| `log_info(msg)` | `msg: String` | `Int` |
| `log_warn(msg)` | `msg: String` | `Int` |
| `log_error(msg)` | `msg: String` | `Int` |
| `log_set_level(level)` | `level: Int` | `Int` |

### Type Errors

- Wrong argument count -> existing "expected N arguments" pattern
- Wrong argument type -> existing "expected String" / "expected Int" pattern

## IR Changes

### New Instructions

```rust
LogDebug { dest: Reg, message: Reg },
LogInfo { dest: Reg, message: Reg },
LogWarn { dest: Reg, message: Reg },
LogError { dest: Reg, message: Reg },
LogSetLevel { dest: Reg, level: Reg },
```

### IR Lowering

In `lower_expr` for `Expr::Call`, add cases for `"log_debug"`, `"log_info"`, `"log_warn"`, `"log_error"`, `"log_set_level"` matching the existing built-in function pattern.

`dest` register types: all -> `IrType::Int`.

## Codegen Changes

### External Function Declarations

```
declare i64 @cy_log_debug(i8*)     ; long cy_log_debug(const char* msg)
declare i64 @cy_log_info(i8*)      ; long cy_log_info(const char* msg)
declare i64 @cy_log_warn(i8*)      ; long cy_log_warn(const char* msg)
declare i64 @cy_log_error(i8*)     ; long cy_log_error(const char* msg)
declare i64 @cy_log_set_level(i64) ; long cy_log_set_level(long level)
```

### Instruction Compilation

Each log IR instruction compiles to a single C function call returning i64 (always 0). No pointer results — just store the i64 return in `regs`.

For log message functions: get the message pointer from `ptrs` (or convert via `int_to_ptr`), call the C function, store i64 result in `regs`.

For `LogSetLevel`: pass the i64 level value directly, store i64 result in `regs`.

### Linking Change

Compile `runtime/log.c` to a temporary `log.o` and include in the `cc` link step alongside `json.o` and `http.o`.

## C Runtime Library (`runtime/log.c`)

```c
#include <stdio.h>

static int cy_log_level = 0;  // 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR

long cy_log_debug(const char* msg) {
    if (cy_log_level <= 0) fprintf(stderr, "[DEBUG] %s\n", msg);
    return 0;
}

long cy_log_info(const char* msg) {
    if (cy_log_level <= 1) fprintf(stderr, "[INFO] %s\n", msg);
    return 0;
}

long cy_log_warn(const char* msg) {
    if (cy_log_level <= 2) fprintf(stderr, "[WARN] %s\n", msg);
    return 0;
}

long cy_log_error(const char* msg) {
    if (cy_log_level <= 3) fprintf(stderr, "[ERROR] %s\n", msg);
    return 0;
}

long cy_log_set_level(long level) {
    cy_log_level = (int)level;
    return 0;
}
```

## Testing

### Unit Tests (~5 new)

**Type Checker (~3):**
- `log_info` accepts String, returns Int
- `log_set_level` accepts Int, returns Int
- Error: wrong argument type to `log_info` (Int instead of String)

**IR (~2):**
- `log_info` lowers to `LogInfo` instruction
- `log_set_level` lowers to `LogSetLevel` instruction

### E2E Tests (~1 new)

**`log_levels.sans`** — Set level to WARN, call all four log functions. The test captures stderr to verify only WARN and ERROR messages appear. Exit code encodes success.

Note: E2E tests for logging are tricky because the test helper captures exit code, not stderr. The E2E test will verify that the program compiles and runs without crashing, and that log_set_level + log functions work without error. The actual stderr output verification would require test infrastructure changes.

Simpler approach: test that `log_set_level` returns 0 and `log_error` returns 0. Exit code = sum of return values (should be 0).

### Estimated Total: ~286 existing + ~6 new = ~292 tests

## Deferred

- Structured logging (key=value pairs)
- Log to file
- Timestamps
- Log format customization
- Per-module log levels
- Log context/fields
- Async logging
- Log rotation
- Named log level constants (LOG_DEBUG, LOG_INFO, etc.)
