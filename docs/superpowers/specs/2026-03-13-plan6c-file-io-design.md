# Plan 6c: File I/O Design Spec

## Goal

Add minimal file I/O via built-in functions: `file_read`, `file_write`, `file_append`, `file_exists`. No new types — files are opened, used, and closed within a single call. Error handling uses sentinel values (empty string, 0, false) matching the language's current pattern.

## Scope

- `file_read(path)` — read entire file to string
- `file_write(path, content)` — write string to file (create/overwrite)
- `file_append(path, content)` — append string to file (create if missing)
- `file_exists(path)` — check if file exists

**Out of scope (deferred):** File handles as first-class values, streaming reads, binary I/O, directory operations (`mkdir`, `readdir`), file metadata (`stat`, permissions), line-by-line reading, proper error types/Result, file deletion.

## Decisions

- **No new types.** File handles are internal to each C stdlib call sequence. The user never sees a file pointer.
- **Sentinel values on error.** `file_read` returns `""` on failure. `file_write`/`file_append` return `0` on failure, `1` on success. `file_exists` returns `false` on any error.
- **Null-terminated strings.** Sans strings are null-terminated C strings (char pointers). File content is read as bytes and null-terminated. Files containing null bytes will be truncated at the first null — this applies to both reading and writing (writes use `strlen` to determine content length).
- **No size limit.** `file_read` reads the entire file into a malloc'd buffer. Large files will use large amounts of memory. No streaming.
- **Implementation pattern.** Same as `int_to_string`/`string_to_int`: type checker recognizes function name → IR instruction → codegen emits C stdlib calls.
- **POSIX only.** `file_exists` uses `access()` from `<unistd.h>`, which is POSIX. Windows would need `_access`. Acceptable since the compiler targets `cc` on Unix-like systems.
- **Boolean representation.** All booleans in the codebase are stored as `i64` (0 or 1), never LLVM `i1`. `file_exists` follows this pattern.
- **Codegen branching for error paths.** `FileRead`, `FileWrite`, and `FileAppend` need conditional branches in codegen to handle `fopen` returning NULL. This uses inkwell basic block splits directly within the instruction handler — create a `then_bb` (success path) and `merge_bb` (join point), branch on the null check, and use `build_phi` at the merge point to select the result. This pattern is new to the codebase but straightforward with inkwell's builder API.

## Syntax

```sans
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

**Error path codegen detail:** After `fopen`, check if result is null. If null: `malloc(1)`, store `\0` at byte 0, store the pointer in both `regs` (via `ptr_to_int`) and `ptrs`, then branch to the merge block. The merge block uses a phi node to select between the error-path pointer and the success-path pointer.

### `file_write(path String, content String) Int`

Writes `content` to the file at `path`, creating the file if it doesn't exist, overwriting if it does.

**C implementation:** `fopen(path, "w")` → `fwrite(content, 1, strlen(content), file)` → `fclose` → return 1.

**Returns:** `1` on success, `0` on any error (permission denied, disk full, etc.).

**Error path:** After `fopen`, if null, the phi node at the merge block selects `0` (failure). On success path, phi selects `1`.

### `file_append(path String, content String) Int`

Appends `content` to the file at `path`, creating the file if it doesn't exist.

**C implementation:** `fopen(path, "a")` → `fwrite(content, 1, strlen(content), file)` → `fclose` → return 1.

**Returns:** `1` on success, `0` on any error.

### `file_exists(path String) Bool`

Checks whether a file exists at `path`.

**C implementation:** `access(path, F_OK)` where `F_OK` is `0`. Returns `access() == 0`.

**Returns:** `true` (`1` as i64) if file exists, `false` (`0` as i64) otherwise (including permission errors).

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

Add these C function declarations (LLVM IR types shown for reference):

```
declare i8* @fopen(i8*, i8*)       ; FILE* fopen(const char*, const char*)
declare i32 @fclose(i8*)           ; int fclose(FILE*)
declare i64 @fread(i8*, i64, i64, i8*)   ; size_t fread(void*, size_t, size_t, FILE*)
declare i64 @fwrite(i8*, i64, i64, i8*)  ; size_t fwrite(const void*, size_t, size_t, FILE*)
declare i32 @fseek(i8*, i64, i32)  ; int fseek(FILE*, long, int)
declare i64 @ftell(i8*)            ; long ftell(FILE*)
declare i32 @access(i8*, i32)      ; int access(const char*, int) — POSIX
```

Note: `strlen` and `malloc` are already declared from string operations. `fread`/`fwrite` return `size_t` (unsigned) but mapped to `i64` — acceptable since return values are not used for signed arithmetic.

### Instruction Compilation

**FileRead:**
1. Call `fopen(path, "r")`. Check if result is null pointer.
2. If null: `malloc(1)`, store `\0` at byte 0, store pointer in `regs`/`ptrs`, branch to merge.
3. If non-null (success path):
   - `fseek(file, 0, 2)` (SEEK_END = 2)
   - `size = ftell(file)`
   - `fseek(file, 0, 0)` (SEEK_SET = 0)
   - `buf = malloc(size + 1)`
   - `fread(buf, 1, size, file)`
   - GEP to `buf[size]`, store `0` (null-terminate)
   - `fclose(file)`
   - Store `buf` in `regs`/`ptrs`, branch to merge.
4. Merge block: phi node selects the correct pointer from error or success path. Store final value in `regs` (as i64) and `ptrs` (as pointer).

**FileWrite:**
1. Get content pointer from `ptrs` (or `regs` via `int_to_ptr`). Get length via `strlen(content)`.
2. Call `fopen(path, "w")`. Check if null.
3. If null: branch to merge with value `0`.
4. If non-null: `fwrite(content, 1, len, file)` → `fclose(file)` → branch to merge with value `1`.
5. Merge block: phi selects `0` or `1`. Store in `regs`.

**FileAppend:**
Same as FileWrite but with `fopen(path, "a")`.

**FileExists:**
1. Call `access(path, 0)` (F_OK = 0)
2. `icmp eq` result with `0`
3. `zext i1` to `i64` (0 or 1)
4. Store in `regs`

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

### E2E Tests (~2 new)

**`file_write_read.sans`** — Write a string to a temp file, read it back, compare length. Exit code = string length (verifies round-trip).

**`file_exists_check.sans`** — Write a file, check it exists (true=1), check a nonexistent file (false=0). Exit code = sum of checks.

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
- Windows compatibility (`_access` instead of `access`)
