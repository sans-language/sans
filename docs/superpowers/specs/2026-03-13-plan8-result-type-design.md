# Phase 8: Result Type Design Spec

## Goal

Add a `Result<T>` type for error handling. Functions can return `ok(value)` or `err("message")` instead of using sentinel values. Callers check results with `.is_ok()`, `.is_err()`, `.unwrap()`, `.unwrap_or(default)`, and `.error()`.

## Scope

**Constructors:**
- `ok(value)` — wrap a value in a successful Result. Returns `Result<T>` where T is inferred from the value type.
- `err(message)` — create an error Result. Message must be a String. Returns `ResultErr` which is compatible with any `Result<T>`.

**Methods on Result:**
- `.is_ok()` — returns Bool (true if ok)
- `.is_err()` — returns Bool (true if err)
- `.unwrap()` — returns the inner value T, or prints error and exits if err
- `.unwrap_or(default)` — returns inner value if ok, or default if err
- `.error()` — returns the error message String ("" if ok)

**Type syntax:**
- `Result<Int>`, `Result<String>`, `Result<Bool>` in function return types and parameter types
- Parser extended to handle `Ident<Ident>` in type positions

**Out of scope:** `?` operator, pattern matching on Result, `Result<T, E>` with custom error types, retrofitting existing stdlib functions.

## Decisions

- **Opaque built-in type backed by C runtime.** `Result` is a pointer to a heap-allocated `CyResult` struct, following the JsonValue/HttpResponse pattern.
- **`ResultErr` sentinel type.** `err()` returns a special `ResultErr` type that is compatible with any `Result<T>` in if/else branches and function return type checks. This avoids needing type annotations on `err()`.
- **`unwrap()` exits on error.** No exceptions or panics — `cy_result_unwrap` prints the error message to stderr and calls `exit(1)`.
- **Phi merging heuristic.** When if/else merges `err` (Result<Int> default) with `ok(value)` (Result<T>), the phi node prefers the non-default inner type.
- **Local function return type tracking.** The IR lowerer builds a map of local function return types so that `let r = divide(10, 2)` correctly assigns `IrType::Result(Int)` to `r`, enabling method calls.

## Runtime (`runtime/result.c`)

```c
typedef struct CyResult {
    int tag;         // 0 = ok, 1 = err
    long value;      // ok value as i64
    char* error;     // error message (NULL if ok)
} CyResult;
```

## Testing

- 8 new typeck tests (ok, err, methods, return type, if/else compatibility)
- 3 new IR tests (ResultOk, ResultErr, ResultUnwrap instructions)
- 2 new E2E tests (ok+unwrap, error handling with unwrap_or)
- Total: 306 tests passing
