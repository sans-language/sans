# Plan 6a: Arrays & String Operations Design Spec

## Goal

Add dynamically-sized `Array<T>` with push/get/set/len, `for..in` iteration, string methods (len, concatenation, substring), and int/string conversion built-ins — all via dedicated IR instructions lowered to heap operations and C stdlib calls.

## Scope

- `Array<T>` — heap-allocated, growable, generic
- Array methods: `.push(val)`, `.get(idx)`, `.set(idx, val)`, `.len()`
- `for x in arr { }` loop syntax
- String methods: `.len()`, `.substring(start, end)`
- String `+` concatenation
- Built-in functions: `int_to_string(n)`, `string_to_int(s)`

**Out of scope (deferred):** Array literal syntax (`[1, 2, 3]`), bounds checking, `.remove()`, `.pop()`, `.contains()`, string `.split()`, `.trim()`, `.starts_with()`, `.contains()`, GC/memory cleanup, explicit `Array<T>` type annotations (type is always inferred from `array<T>()`), `.substring()` bounds validation.

## Decisions

- **Array memory model:** Heap-allocated 24-byte struct (data pointer, len, capacity) behind an i64 pointer. Matches channel/mutex pattern.
- **Array construction:** `array<T>()` syntax, same pattern as `channel<T>()`. No literal syntax in this plan.
- **Array growth:** Double capacity when full, malloc+copy+free (same pattern as unbounded channel buffer growth).
- **No bounds checking:** `.get()` and `.set()` do not validate index. Out-of-bounds is undefined behavior. Bounds checking deferred to error handling plan.
- **String representation:** Unchanged — null-terminated C strings (`char*` stored as i64). Concatenation and substring produce new heap-allocated strings.
- **For-in implementation:** No dedicated IR instruction. `Stmt::ForIn` lowers to a counted loop using `ArrayLen` + `ArrayGet` + existing branch/compare instructions.
- **Implementation approach:** Dedicated IR instructions that codegen lowers to heap operations and C stdlib calls, same pattern as Plans 5a/5b.
- **`+` overloading:** The existing `+` binary operator is extended to handle `String + String`. Two changes are needed: (1) the type checker's `BinOp::Add` handling must allow `String + String → String` in addition to `Int + Int → Int`; (2) the IR lowering's `BinOp::Add` handling must check `reg_types` for the left operand — if `IrType::Str`, emit `StringConcat` instead of `Add`.
- **Reserved keywords:** `array` and `in` become reserved keywords. They cannot be used as variable or function names. This is a breaking change for any existing code using these identifiers (acceptable since the language is pre-1.0).
- **`string_to_int` on invalid input:** `strtol` returns 0 for non-numeric strings or empty strings. No error is raised. Error detection deferred to a future error-handling plan.
- **`substring` with invalid indices:** `.substring(start, end)` where start > end or indices are out of bounds is undefined behavior, same as array `.get()`/`.set()` bounds checking deferral.

## Syntax

```cyflym
// Array creation
let a = array<Int>()

// Array operations
a.push(42)
a.push(99)
let x = a.get(0)     // returns 42
a.set(0, 100)        // a[0] is now 100
let n = a.len()       // returns 2

// For-in loop
for x in a {
    print(x)
}

// String operations
let s = "hello"
let n = s.len()               // returns 5
let s2 = s + " world"         // new string "hello world"
let sub = s.substring(1, 3)   // returns "el"

// Conversion built-ins
let s = int_to_string(42)     // "42"
let n = string_to_int("42")   // 42
```

### Keywords

- `array` — new keyword, used as `array<T>()`
- `in` — new keyword, used in `for x in expr`

### Method Calls

`.push(val)`, `.get(idx)`, `.set(idx, val)`, `.len()`, `.substring(start, end)` use existing `Expr::MethodCall` syntax. The type checker and IR lowering distinguish them from user-defined methods based on receiver type (same pattern as `.send()` / `.recv()` / `.lock()` / `.unlock()`).

### Built-in Functions

`int_to_string(n)` and `string_to_int(s)` are function calls, special-cased in the type checker like `print()`.

## Type System

### New Type

```rust
pub enum Type {
    // ... existing ...
    Array { inner: Box<Type> },
}
```

### Type Checking Rules

| Expression | Type |
|---|---|
| `array<T>()` | `Array<T>` |
| `a.push(val)` | `val` must match `T` of `Array<T>`. Returns `Int` |
| `a.get(idx)` | `idx` must be `Int`. Returns `T` |
| `a.set(idx, val)` | `idx` must be `Int`, `val` must match `T`. Returns `Int` |
| `a.len()` | Returns `Int` |
| `s.len()` on String | Returns `Int` |
| `s.substring(start, end)` | Both must be `Int`. Returns `String` |
| `String + String` | Returns `String` |
| `int_to_string(n)` | `n` must be `Int`. Returns `String` |
| `string_to_int(s)` | `s` must be `String`. Returns `Int` |
| `for x in arr` | `arr` must be `Array<T>`. `x` bound as `T` (immutable) in body |

### Exhaustive Match Updates

Adding `Stmt::ForIn` requires updating all exhaustive `match stmt` arms in the compiler:

1. **`check_stmt()` in typeck** — New arm: type-check `iterable` as `Array<T>`, create a new locals scope with `var` bound as type `T`, then `check_stmts(body)`.
2. **"Last statement" check in typeck** — `Stmt::ForIn` must be added alongside `While` as a statement that doesn't yield a return value (function cannot end with `for..in`).
3. **`lower_stmt()` in IR** — New arm implementing the ForIn lowering pseudocode (ArrayLen + counter loop + ArrayGet).
4. **`lower_function()` "last statement" check in IR** — `Stmt::ForIn` added alongside `While`.

Similarly, `Expr::ArrayCreate` must be added to `expr_span()` in the parser.

### Type Errors

- `.push(val)` where val type doesn't match array element type
- `.get()` / `.set()` with non-Int index
- `.push()` / `.get()` / `.set()` / `.len()` on non-Array
- `.substring()` on non-String or with non-Int arguments
- `String + Int` or `Int + String` (no implicit conversion)
- `for x in expr` where expr is not `Array<T>`
- `int_to_string()` with non-Int argument
- `string_to_int()` with non-String argument

## AST Changes

### New Expression Variant

```rust
pub enum Expr {
    // ... existing ...
    ArrayCreate {
        element_type: TypeName,
        span: Span,
    },
}
```

### New Statement Variant

```rust
pub enum Stmt {
    // ... existing ...
    ForIn {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
}
```

### New Tokens

- `Token::Array` — keyword `array`
- `Token::In` — keyword `in`

### Parser Rules

- `array<T>()`: `array` keyword, `<`, type name, `>`, `(`, `)` → `Expr::ArrayCreate`
- `for x in expr { body }`: `for` keyword, identifier, `in`, expression, `{`, block, `}` → `Stmt::ForIn`
- Disambiguation: `for` in `impl Trait for Type` is parsed inside `parse_impl_block()` at top level, never inside `parse_stmt()`. At statement position, `TokenKind::For` always means for-in loop. `parse_stmt()` needs a new branch: if `self.peek().kind == TokenKind::For`, parse a for-in loop.
- `.push(val)`, `.get(idx)`, `.set(idx, val)`, `.len()`, `.substring(start, end)`: existing `Expr::MethodCall` — no new AST nodes needed
- `int_to_string(n)`, `string_to_int(s)`: existing `Expr::Call` — no new AST nodes needed
- `String + String`: existing `Expr::BinaryOp` — no new AST nodes needed

## IR Instructions

### New Instructions

```rust
pub enum Instruction {
    // ... existing ...

    // Array operations
    ArrayCreate { dest: Reg, element_type: IrType },
    ArrayPush { array: Reg, value: Reg },
    ArrayGet { dest: Reg, array: Reg, index: Reg },
    ArraySet { array: Reg, index: Reg, value: Reg },
    ArrayLen { dest: Reg, array: Reg },

    // String operations
    StringLen { dest: Reg, string: Reg },
    StringConcat { dest: Reg, left: Reg, right: Reg },
    StringSubstring { dest: Reg, string: Reg, start: Reg, end: Reg },
    IntToString { dest: Reg, value: Reg },
    StringToInt { dest: Reg, string: Reg },
}
```

### IrType Changes

```rust
enum IrType {
    // ... existing: Int, Bool, Str, Struct(String), Enum(String), Sender, Receiver, JoinHandle, Mutex ...
    Array(Box<IrType>),
}
```

The inner type is needed so that `ArrayGet` can assign the correct `IrType` to its result register. For example, `ArrayGet` on an `Array(Box::new(IrType::Str))` assigns `IrType::Str` to the dest register, which ensures `print(x)` dispatches correctly.

### IR Lowering

| AST | IR |
|---|---|
| `Expr::ArrayCreate` | `ArrayCreate { dest, element_type }` — element_type derived from resolved TypeName |
| MethodCall `.push(val)` on Array | `ArrayPush { array, value }` |
| MethodCall `.get(idx)` on Array | `ArrayGet { dest, array, index }` |
| MethodCall `.set(idx, val)` on Array | `ArraySet { array, index, value }` |
| MethodCall `.len()` on Array | `ArrayLen { dest, array }` |
| MethodCall `.len()` on String | `StringLen { dest, string }` |
| MethodCall `.substring(s, e)` on String | `StringSubstring { dest, string, start, end }` |
| BinaryOp `+` on String operands | `StringConcat { dest, left, right }` |
| Call `int_to_string(n)` | `IntToString { dest, value }` |
| Call `string_to_int(s)` | `StringToInt { dest, string }` |
| `Stmt::ForIn { var, iterable, body }` | `ArrayLen` + counter loop with `ArrayGet` (see below) |

### ForIn IR Lowering

`for x in arr { body }` lowers to:

```
len = ArrayLen(arr)
idx_ptr = alloca
store 0 → idx_ptr
loop:
  idx = load idx_ptr
  cmp = idx < len
  branch cmp → body_bb, done_bb
body_bb:
  x = ArrayGet(arr, idx)
  <lower body with x bound>
  next = idx + 1
  store next → idx_ptr
  branch → loop
done_bb:
  ...
```

This uses existing IR instructions (Const, Add, Compare, Branch) plus the new ArrayLen and ArrayGet.

## Codegen (LLVM)

### Array Data Structure

Heap-allocated struct (3 i64s = 24 bytes):

```
{
    i64 data,       // offset 0 — pointer to heap buffer (as i64)
    i64 len,        // offset 1 — current element count
    i64 capacity,   // offset 2 — buffer capacity
}
```

- **ArrayCreate:** `malloc(24)`, set data=`malloc(64)` (initial capacity 8, buffer = 8 × 8 bytes), len=0, capacity=8. Return pointer as i64.
- **ArrayPush:** Load len (offset 1) and capacity (offset 2). If len == capacity: malloc 2× buffer, `memcpy(new_buf, old_buf, len * 8)` to copy elements, free old buffer, update data pointer and capacity. Store value at `data[len]`, increment len.
- **ArrayGet:** Load data pointer (offset 0), convert to pointer, load `data[index]`. Return value.
- **ArraySet:** Load data pointer (offset 0), convert to pointer, store value at `data[index]`.
- **ArrayLen:** Load len from offset 1. Return it.

### String Operations

Strings are null-terminated C strings (`char*` stored as i64).

- **StringLen:** Convert i64 to pointer, call `strlen(ptr)`. Return result as i64.
- **StringConcat:** `strlen` both, `malloc(len1 + len2 + 1)`, `memcpy(dest, left, len1)`, `memcpy(dest + len1, right, len2)`, null-terminate `dest[len1 + len2] = 0`. Return new pointer as i64.
- **StringSubstring:** Calculate length = end - start, `malloc(length + 1)`, pointer arithmetic `src = ptr + start`, `memcpy(dest, src, length)`, null-terminate. Return new pointer as i64.
- **IntToString:** `malloc(21)` (max i64 decimal digits + sign + null), `snprintf(buf, 21, "%ld", val)`. Return pointer as i64.
- **StringToInt:** Convert i64 to pointer, `strtol(ptr, null, 10)`. Return result as i64.

### New C Function Declarations

Add to codegen preamble (alongside existing malloc, printf, pthread functions):
- `strlen(ptr) → i64`
- `memcpy(dest, src, len) → ptr` (return value unused; dest pointer already in register)
- `snprintf(buf, size, fmt, ...) → i32` (format string `"%ld"` correct for i64 on Darwin/macOS)
- `strtol(str, endptr, base) → i64` (returns 0 on invalid input; no error detection)
- `free(ptr)` (if not already declared)

### Memory Management

Same as channels/mutex — all heap allocations (array structs, buffers, concatenated strings) are leaked until process exit. Cleanup deferred to GC phase.

## Testing

### Unit Tests (~25 new)

**Lexer (2):**
- Tokenize `array` keyword
- Tokenize `in` keyword

**Parser (~5):**
- Parse `array<Int>()` expression
- Parse `for x in arr { }` statement
- Parse `.push()`, `.get()`, `.set()` as method calls on array
- Parse `int_to_string(42)` as function call
- Parse string `+` concatenation

**Type Checker (~8):**
- `array<Int>()` produces `Array<Int>` type
- `.push(val)` with matching type passes
- `.push(val)` with wrong type errors
- `.get(idx)` returns element type, idx must be Int
- `.set(idx, val)` checks index and value types
- `.len()` on Array returns Int
- `.len()` on String returns Int
- `for x in arr` binds x as element type
- `for x in non_array` errors
- `int_to_string` / `string_to_int` type checking
- `String + String` returns String
- `String + Int` errors

**IR (~4):**
- ArrayCreate lowers correctly
- push/get/set/len lower to correct instructions
- ForIn lowers to counted loop (ArrayLen + ArrayGet)
- StringConcat lowers from `+` on string operands

**Codegen (~3):**
- ArrayCreate/Push/Get/Len emit valid LLVM IR
- StringConcat emits valid LLVM IR
- IntToString/StringToInt emit valid LLVM IR

### E2E Fixtures (4)

**`array_basic.cy`** — Create array, push values, get and len. Exit with computed value.

**`array_for_in.cy`** — Create array, push values, iterate with for-in summing elements. Exit with sum.

**`string_ops.cy`** — String len, concatenation, substring. Exit with computed value.

**`string_conversion.cy`** — int_to_string and string_to_int round-trip. Exit with computed value.

### Estimated Total: ~220 tests (195 existing + ~25 new)

## Deferred

- Array literal syntax (`[1, 2, 3]`)
- Bounds checking for `.get()` / `.set()`
- `.remove()`, `.pop()`, `.contains()`, `.map()`, `.filter()`
- String `.split()`, `.trim()`, `.starts_with()`, `.contains()`
- GC / memory cleanup
- Module/import system (Plan 6b)
