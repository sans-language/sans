# Plan 6c: File I/O Design Spec

## Goal

Add minimal file I/O via built-in functions: `file_read`, `file_write`, `file_append`, `file_exists`. No new types — files are opened, used, and closed within a single call. Error handling uses sentinel values (empty string, 0, false) matching the language's current pattern.

## Scope

- `file_read(path)` — read entire file to string
- `file_write(path, content)` — write string to file (create/overwrite)
- `file_append(path, content)` — append string to file (create if missing)
- `file_exists(path)` — check if file exists

**Out of scope (deferred):** File handles as first-class values, streaming reads, binary I/O, directory operations (`mkdir`, `readdir`), file metadata (`stat`, permissions), line-by-line reading, proper error types/Result.

## Decisions

- **No new types.** File handles are internal to each C stdlib call sequence. The user never sees a file pointer.
- **Sentinel values on error.** `file_read` returns `""` on failure. `file_write`/`file_append` return `0` on failure, `1` on success. `file_exists` returns `false` on any error.
- **Null-terminated strings.** Cyflym strings are null-terminated C strings (char pointers). File content is read as bytes and null-terminated. Files containing null bytes will be truncated at the first null.
- **No size limit.** `file_read` reads the entire file into a malloc'd buffer. Large files will use large amounts of memory. No streaming.
- **Implementation pattern.** Same as `int_to_string`/`string_to_int`: type checker recognizes function name → IR instruction → codegen emits C stdlib calls.

## Syntax

```cyflym
fn main() Int {
    // Write a file
    let ok = file_write("output.txt", "hello world")

    // Append to a file
    file_append("output.txt", "\nmore content")

    // Read a file
    let content = file_read("output.txt")
    print(content)

    // Check existence
    if file_exists("output.txt") {
        print("file exists")
    }

    ok
}
```

## Function Specifications

### `file_read(path String) String`

Reads the entire contents of the file at `path` into a heap-allocated string.

**C implementation:** `fopen(path, "r")` → `fseek(END)` → `ftell` (get size) → `fseek(SET)` → `malloc(size + 1)` → `fread` → null-terminate → `fclose` → return pointer.

**Returns:** File contents as a String. On any error (file not found, permission denied, read failure), returns `""` (pointer to a malloc'd empty string `"\0"`).

### `file_write(path String, content String) Int`

Writes `content` to the file at `path`, creating the file if it doesn't exist, overwriting if it does.

**C implementation:** `fopen(path, "w")` → `fwrite(content, 1, strlen(content), file)` → `fclose` → return 1.

**Returns:** `1` on success, `0` on any error (permission denied, disk full, etc.).

### `file_append(path String, content String) Int`

Appends `content` to the file at `path`, creating the file if it doesn't exist.

**C implementation:** `fopen(path, "a")` → `fwrite(content, 1, strlen(content), file)` → `fclose` → return 1.

**Returns:** `1` on success, `0` on any error.

### `file_exists(path String) Bool`

Checks whether a file exists at `path`.

**C implementation:** `access(path, F_OK)` where `F_OK` is `0`. Returns `access() == 0`.

**Returns:** `true` if file exists, `false` otherwise (including permission errors).

## Type System

### Type Checking Rules

| Expression | Rule |
|---|---|
| `file_read(path)` | `path` must be `String`. Returns `String`. Exactly 1 argument. |
| `file_write(path, content)` | Both must be `String`. Returns `Int`. Exactly 2 arguments. |
| `file_append(path, content)` | Both must be `String`. Returns `Int`. Exactly 2 arguments. |
| `file_exists(path)` | `path` must be `String`. Returns `Bool`. Exactly 1 argument. |

These are added to the existing built-in function checks in `check_expr`'s `Expr::Call` arm, alongside `print`, `int_to_string`, `string_to_int`.

### Type Errors

- Wrong argument count → existing "expected N arguments" pattern
- Wrong argument type → existing "expected String" pattern

## IR Changes

### New Instructions

```rust
FileRead { dest: Reg, path: Reg },
FileWrite { dest: Reg, path: Reg, content: Reg },
FileAppend { dest: Reg, path: Reg, content: Reg },
FileExists { dest: Reg, path: Reg },
```

Each instruction takes register operands and writes the result to `dest`.

### IR Lowering

In `lower_expr` for `Expr::Call`, add cases for `"file_read"`, `"file_write"`, `"file_append"`, `"file_exists"` matching the existing `"int_to_string"`/`"string_to_int"` pattern.

`dest` register types: `FileRead` → `IrType::Str`, `FileWrite`/`FileAppend` → `IrType::Int`, `FileExists` → `IrType::Bool`.

## Codegen Changes

### External Function Declarations

Add these C stdlib function declarations:

```
declare i8* @fopen(i8*, i8*)
declare i32 @fclose(i8*)
declare i64 @fread(i8*, i64, i64, i8*)
declare i64 @fwrite(i8*, i64, i64, i8*)
declare i32 @fseek(i8*, i64, i32)
declare i64 @ftell(i8*)
declare i32 @access(i8*, i32)
```

Note: `strlen` and `malloc` are already declared from string operations.

### Instruction Compilation

**FileRead:**
1. Call `fopen(path, "r")`. If null, malloc 1 byte, set to `\0`, return.
2. `fseek(file, 0, SEEK_END)` (SEEK_END = 2)
3. `size = ftell(file)`
4. `fseek(file, 0, SEEK_SET)` (SEEK_SET = 0)
5. `buf = malloc(size + 1)`
6. `fread(buf, 1, size, file)`
7. `buf[size] = 0` (null-terminate)
8. `fclose(file)`
9. Store `buf` in both `regs` (as i64) and `ptrs` (as pointer)

**FileWrite:**
1. Get content length via `strlen(content)`
2. Call `fopen(path, "w")`. If null, store 0 in dest, skip to end.
3. `fwrite(content, 1, len, file)`
4. `fclose(file)`
5. Store 1 in dest

**FileAppend:**
Same as FileWrite but with `fopen(path, "a")`.

**FileExists:**
1. Call `access(path, 0)` (F_OK = 0)
2. Compare result == 0
3. Store boolean result in dest

## Testing

### Unit Tests (~8 new)

**Type Checker (~4):**
- `file_read` accepts String, returns String
- `file_write` accepts (String, String), returns Int
- `file_exists` accepts String, returns Bool
- Error: wrong argument type to `file_read`

**IR (~2):**
- `file_read` lowers to `FileRead` instruction
- `file_write` lowers to `FileWrite` instruction

**Codegen (~2):**
- (Codegen tests are E2E by nature — covered by E2E tests below)

### E2E Tests (~2 new)

**`file_write_read.cy`** — Write a string to a temp file, read it back, compare length. Exit code = string length (verifies round-trip).

**`file_exists_check.cy`** — Write a file, check it exists (true=1), check a nonexistent file (false=0). Exit code = sum of checks.

### Estimated Total: ~249 tests (241 existing + ~8 new)

## Deferred

- File handles as first-class values (`let f = file_open("path", "r")`)
- Streaming reads (`f.read_line()`, `f.read_bytes(n)`)
- Binary I/O (read/write raw bytes)
- Directory operations (`mkdir`, `readdir`, `rmdir`)
- File metadata (`file_size`, `file_modified`, permissions)
- Line-by-line iteration (`for line in file_lines("path")`)
- Proper error handling (Result type)
- File deletion (`file_delete`)
