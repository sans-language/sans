# Plan 6d: JSON Design Spec

## Goal

Add JSON support via an opaque `JsonValue` built-in type backed by a C runtime library. Users can parse, construct, access, mutate, and serialize JSON values through built-in functions and methods. No new syntax — everything uses existing function call and method call syntax.

## Scope

**Constructors (free functions):**
- `json_parse(s)` — parse JSON string into a JsonValue
- `json_object()` — create empty object
- `json_array()` — create empty array
- `json_string(s)` — wrap a String as JsonValue
- `json_int(n)` — wrap an Int as JsonValue
- `json_bool(b)` — wrap a Bool as JsonValue
- `json_null()` — create null JsonValue

**Accessors (methods on JsonValue):**
- `.get(key)` — object field access, returns JsonValue
- `.get_index(n)` — array element access, returns JsonValue
- `.get_string()` — extract String (returns "" if not a string)
- `.get_int()` — extract Int (returns 0 if not an int)
- `.get_bool()` — extract Bool (returns false if not a bool)
- `.len()` — array/object element count
- `.type_of()` — returns type name as String

**Mutators (methods on JsonValue):**
- `.set(key, value)` — set object field
- `.push(value)` — append to array

**Serialization:**
- `json_stringify(value)` — convert JsonValue to JSON string

**Out of scope (deferred):** Iteration over keys/values, `.remove()`, `.contains()`, pretty-printing, streaming parse, float/double type, JSON schema validation, JSONL, comments, `\uXXXX` unicode escape sequences in parser.

## Decisions

- **Opaque built-in type.** `JsonValue` is a new type like `Sender<T>` or `Mutex<T>`. Users never see the internal representation. All access goes through built-in functions and methods.
- **C runtime library.** JSON logic (parsing, stringifying, tree manipulation) lives in `runtime/json.c`, compiled separately and linked alongside the user's object file. This is a new pattern — file I/O used inline C stdlib calls, but JSON is too complex for inline LLVM IR.
- **Sentinel values on error.** `json_parse` of invalid JSON returns a null JsonValue. Accessor type mismatches return defaults (0, "", false). `.get()` on missing key returns null JsonValue. `.get_index()` on out-of-bounds returns null JsonValue. `.len()` on non-array/non-object types returns 0. No panics, no crashes.
- **Integer-only numbers.** JSON numbers are parsed as `long` (i64). Floats are truncated to integer. This matches the language's current Int-only numeric type.
- **No new keywords or syntax.** All constructors are built-in function names. All accessors/mutators use existing method call syntax.
- **`print` does not accept JsonValue.** Users must call `json_stringify()` first to get a String, then pass that to `print`.
- **Linking change.** The `cc` invocation in `main.rs` and both e2e test helpers (`compile_and_run` and `compile_and_run_dir`) must compile `runtime/json.c` and link the resulting object file alongside the user's code.

## Runtime Representation

A `JsonValue` in Sans is a pointer (stored as i64) to a heap-allocated C struct:

```c
typedef struct CyJsonValue {
    int tag;  // 0=null, 1=bool, 2=int, 3=string, 4=array, 5=object
    union {
        long bool_val;                  // tag 1 (0 or 1)
        long int_val;                   // tag 2
        char* string_val;              // tag 3 (malloc'd, null-terminated)
        struct {                       // tag 4 (array)
            struct CyJsonValue** items;
            long len;
            long cap;
        } array_val;
        struct {                       // tag 5 (object)
            char** keys;               // malloc'd array of malloc'd strings
            struct CyJsonValue** values;
            long len;
            long cap;
        } object_val;
    };
} CyJsonValue;
```

The struct is entirely internal. Sans code only interacts with it via the C functions declared in codegen.

## Syntax

```sans
fn main() Int {
    // Build JSON
    let obj = json_object()
    obj.set("name", json_string("sans"))
    obj.set("version", json_int(1))
    obj.set("fast", json_bool(true))

    let tags = json_array()
    tags.push(json_string("compiler"))
    tags.push(json_string("fast"))
    obj.set("tags", tags)

    // Serialize
    let s = json_stringify(obj)
    print(s)

    // Parse
    let parsed = json_parse(s)
    let name = parsed.get("name").get_string()
    let version = parsed.get("version").get_int()
    print(name)

    // Array access
    let first_tag = parsed.get("tags").get_index(0).get_string()
    print(first_tag)

    // Type checking
    let t = parsed.get("fast").type_of()
    print(t)  // "bool"

    version
}
```

## Type System

### New Type Variant

Add `JsonValue` to the `Type` enum in `crates/sans-typeck/src/types.rs`. The `Display` impl should format it as `"JsonValue"`.

### Type Checking Rules

| Expression | Args | Returns |
|---|---|---|
| `json_parse(s)` | `s: String` | `JsonValue` |
| `json_object()` | none | `JsonValue` |
| `json_array()` | none | `JsonValue` |
| `json_string(s)` | `s: String` | `JsonValue` |
| `json_int(n)` | `n: Int` | `JsonValue` |
| `json_bool(b)` | `b: Bool` | `JsonValue` |
| `json_null()` | none | `JsonValue` |
| `json_stringify(v)` | `v: JsonValue` | `String` |

### Method Checking Rules (on `Type::JsonValue`)

| Method | Args | Returns |
|---|---|---|
| `.get(key)` | `key: String` | `JsonValue` |
| `.get_index(n)` | `n: Int` | `JsonValue` |
| `.get_string()` | none | `String` |
| `.get_int()` | none | `Int` |
| `.get_bool()` | none | `Bool` |
| `.len()` | none | `Int` |
| `.type_of()` | none | `String` |
| `.set(key, val)` | `key: String, val: JsonValue` | `Int` |
| `.push(val)` | `val: JsonValue` | `Int` |

### Type Errors

- Wrong argument count → existing "expected N arguments" pattern
- Wrong argument type → existing "expected String" / "expected JsonValue" pattern
- Method call on non-JsonValue → existing "no method" pattern

## IR Changes

### New IrType Variant

```rust
IrType::JsonValue
```

The `ir_type_for_return` helper in `crates/sans-ir/src/lib.rs` must map `Type::JsonValue` to `IrType::JsonValue`.

### New Instructions

**Constructors:**
```rust
JsonParse { dest: Reg, source: Reg },        // json_parse(string)
JsonObject { dest: Reg },                      // json_object()
JsonArray { dest: Reg },                       // json_array()
JsonString { dest: Reg, value: Reg },          // json_string(s)
JsonInt { dest: Reg, value: Reg },             // json_int(n)
JsonBool { dest: Reg, value: Reg },            // json_bool(b)
JsonNull { dest: Reg },                        // json_null()
```

**Accessors:**
```rust
JsonGet { dest: Reg, object: Reg, key: Reg },          // .get(key)
JsonGetIndex { dest: Reg, array: Reg, index: Reg },     // .get_index(n)
JsonGetString { dest: Reg, value: Reg },                 // .get_string()
JsonGetInt { dest: Reg, value: Reg },                    // .get_int()
JsonGetBool { dest: Reg, value: Reg },                   // .get_bool()
JsonLen { dest: Reg, value: Reg },                       // .len()
JsonTypeOf { dest: Reg, value: Reg },                    // .type_of()
```

**Mutators:**
```rust
JsonSet { object: Reg, key: Reg, value: Reg },   // .set(key, val) — no dest
JsonPush { array: Reg, value: Reg },              // .push(val) — no dest
```

`JsonSet` and `JsonPush` have no `dest` register, matching the `ArrayPush`/`ArraySet` pattern. The IR lowering emits a separate `Instruction::Const { dest, value: 0 }` after the mutator instruction to produce the expression result.

**Serialization:**
```rust
JsonStringify { dest: Reg, value: Reg },         // json_stringify(v)
```

### IR Lowering

In `lower_expr` for `Expr::Call`, add cases for `"json_parse"`, `"json_object"`, `"json_array"`, `"json_string"`, `"json_int"`, `"json_bool"`, `"json_null"`, `"json_stringify"` matching the existing built-in function pattern.

Method calls on `IrType::JsonValue` lower to the corresponding accessor/mutator instructions.

`dest` register types: constructors/accessors returning JsonValue → `IrType::JsonValue`. `get_string`/`type_of`/`json_stringify` → `IrType::Str`. `get_int`/`len` → `IrType::Int`. `get_bool` → `IrType::Bool`. `set`/`push` → `IrType::Int`.

## Codegen Changes

### External Function Declarations

All functions from `runtime/json.c`:

```
declare i8* @cy_json_parse(i8*)           ; CyJsonValue* cy_json_parse(const char*)
declare i8* @cy_json_object()             ; CyJsonValue* cy_json_object()
declare i8* @cy_json_array()              ; CyJsonValue* cy_json_array()
declare i8* @cy_json_string(i8*)          ; CyJsonValue* cy_json_string(const char*)
declare i8* @cy_json_int(i64)             ; CyJsonValue* cy_json_int(long)
declare i8* @cy_json_bool(i64)            ; CyJsonValue* cy_json_bool(long)
declare i8* @cy_json_null()               ; CyJsonValue* cy_json_null()
declare i8* @cy_json_stringify(i8*)       ; char* cy_json_stringify(CyJsonValue*)
declare i8* @cy_json_get(i8*, i8*)        ; CyJsonValue* cy_json_get(CyJsonValue*, const char*)
declare i8* @cy_json_get_index(i8*, i64)  ; CyJsonValue* cy_json_get_index(CyJsonValue*, long)
declare i8* @cy_json_get_string(i8*)      ; char* cy_json_get_string(CyJsonValue*)
declare i64 @cy_json_get_int(i8*)         ; long cy_json_get_int(CyJsonValue*)
declare i64 @cy_json_get_bool(i8*)        ; long cy_json_get_bool(CyJsonValue*)
declare i64 @cy_json_len(i8*)             ; long cy_json_len(CyJsonValue*)
declare i8* @cy_json_type_of(i8*)         ; char* cy_json_type_of(CyJsonValue*)
declare void @cy_json_set(i8*, i8*, i8*)  ; void cy_json_set(CyJsonValue*, const char*, CyJsonValue*)
declare void @cy_json_push(i8*, i8*)      ; void cy_json_push(CyJsonValue*, CyJsonValue*)
```

### Instruction Compilation

Each JSON IR instruction compiles to a single C function call. No branching, no phi nodes — the C functions handle all error paths internally.

**Constructors:** Call `cy_json_*`, get back pointer, store as i64 in `regs` and as pointer in `ptrs`.

**Accessors returning JsonValue:** Call `cy_json_get` / `cy_json_get_index`, store pointer in both `regs` (ptr_to_int) and `ptrs`.

**Accessors returning String:** Call `cy_json_get_string` / `cy_json_type_of` / `cy_json_stringify`, store pointer in both `regs` and `ptrs`.

**Accessors returning Int/Bool:** Call `cy_json_get_int` / `cy_json_get_bool` / `cy_json_len`, store i64 in `regs`.

**Mutators:** Call the void C functions `cy_json_set` / `cy_json_push`. The Int expression result is synthesized in IR (a `Const { value: 0 }` instruction), not read from the C function return. Codegen for `JsonSet`/`JsonPush` only needs to emit the C call — the `Const` instruction handles the result register separately.

### Linking Change

**`main.rs`:** Before invoking `cc` to link, compile `runtime/json.c` to a temporary `json.o`:
```
cc -c runtime/json.c -o /tmp/json.o
cc user.o /tmp/json.o -o binary
```

The `runtime/` directory path is resolved relative to the `sans` binary's location (or the workspace root during development).

**E2E test helpers:** Both `compile_and_run` and `compile_and_run_dir` in `crates/sans-driver/tests/e2e.rs` need updating. The `json.c` path resolves via `CARGO_MANIFEST_DIR/../../runtime/json.c`. Compile it to a temp `json.o` and include in the `cc` link step.

## C Runtime Library (`runtime/json.c`)

### JSON Parser

Recursive descent parser handling:
- Objects: `{` (key `:` value (`,` key `:` value)*)? `}` (empty `{}` is valid)
- Arrays: `[` (value (`,` value)*)? `]` (empty `[]` is valid)
- Strings: `"` chars `"` with escape sequences (`\"`, `\\`, `\/`, `\n`, `\t`, `\r`, `\b`, `\f`)
- Numbers: optional `-`, digits (parsed as `long` via `strtol`; decimals truncated)
- Booleans: `true`, `false`
- Null: `null`
- Whitespace: spaces, tabs, newlines, carriage returns skipped between tokens

On parse error, returns a null JsonValue (tag=0).

### JSON Stringifier

Recursive stringifier producing compact JSON (no whitespace):
- Objects: `{"key":value,...}`
- Arrays: `[value,...]`
- Strings: `"..."` with escape sequences for `"`, `\`, `\n`, `\t`, `\r`
- Numbers: `%ld` format
- Booleans: `true`/`false`
- Null: `null`

Returns a malloc'd null-terminated string.

### Memory

All CyJsonValue nodes and strings are malloc'd. No free — consistent with the rest of Sans (leaked until process exit). The stringify function returns a malloc'd string that becomes a regular Sans String.

## Testing

### Unit Tests (~19 new)

**Type Checker (~10):**
- `json_parse` accepts String, returns JsonValue
- `json_object`/`json_array`/`json_null` accept no args, return JsonValue
- `json_string` accepts String, `json_int` accepts Int, `json_bool` accepts Bool — all return JsonValue
- `json_stringify` accepts JsonValue, returns String
- Methods: `.get(String)` → JsonValue, `.get_index(Int)` → JsonValue, `.get_string()` → String, `.get_int()` → Int, `.get_bool()` → Bool
- `.len()` → Int, `.type_of()` → String
- `.set(String, JsonValue)` → Int, `.push(JsonValue)` → Int
- Error: wrong argument type to `json_parse` (Int instead of String)

**IR (~4):**
- `json_parse` lowers to `JsonParse` instruction with `IrType::JsonValue`
- `json_object` lowers to `JsonObject` instruction
- `json_stringify` lowers to `JsonStringify` instruction with `IrType::Str`
- `.get()` method lowers to `JsonGet` instruction

**E2E (~5):**
- `json_build.sans` — Build object with set/push, stringify, exit with string length
- `json_parse_access.sans` — Parse JSON string, access nested fields, extract typed values, exit with sum
- `json_roundtrip.sans` — Build structure, stringify, re-parse, verify values match
- `json_object_stringify.sans` — json_object + json_stringify produces `{}` (verifies linking works)
- `json_int_roundtrip.sans` — json_int + get_int round-trip

Note: Codegen unit tests are not practical for JSON instructions because they require the C runtime (`json.o`) to be linked. All JSON codegen testing goes through E2E tests.

### Estimated Total: ~249 existing + ~19 new = ~268 tests

## Deferred

- Iteration over object keys/values (`for key in obj.keys()`)
- `.remove(key)` for objects
- `.contains(key)` for objects
- Pretty-printing (`json_stringify_pretty`)
- Float/double numeric type
- Streaming/incremental parsing
- JSON schema validation
- JSONL (newline-delimited JSON)
- `json_merge(a, b)` for combining objects
- Memory cleanup / reference counting
- `\uXXXX` unicode escape sequences in JSON string parser
