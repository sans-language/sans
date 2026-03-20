# Package Manager Builtins Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 7 new builtins (`getenv`, `mkdir`, `rmdir`, `remove`, `listdir`, `is_dir`, `sh`) to the Sans compiler as prerequisites for the package manager.

**Architecture:** Each builtin goes through the 4-stage compiler pipeline: typeck -> constants -> ir -> codegen. Simple libc wrappers (`getenv`, `rmdir`, `remove`) emit inline LLVM IR. Complex builtins (`mkdir`, `is_dir`, `listdir`, `sh`) call Sans runtime functions in new `runtime/fs.sans` and `runtime/process.sans` modules.

**Tech Stack:** Sans self-hosted compiler, LLVM IR, libc (`getenv`, `rmdir`, `remove`, `popen`/`pclose`), shell commands (`mkdir -p`, `test -d`, `ls -1`)

**Spec:** `docs/superpowers/specs/2026-03-19-package-manager-design.md` (Part 1)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `compiler/constants.sans` | Modify | 7 new IR opcode constants (246-252) |
| `compiler/typeck.sans` | Modify | Type checking for all 7 builtins + aliases |
| `compiler/ir.sans` | Modify | IR lowering for all 7 builtins + aliases |
| `compiler/codegen.sans` | Modify | LLVM codegen + extern declarations |
| `runtime/fs.sans` | Create | Runtime for `mkdir`, `is_dir`, `listdir` |
| `runtime/process.sans` | Create | Runtime for `sh` |
| `compiler/main.sans` | Modify | Add `fs` and `process` to runtime module list |
| `tests/fixtures/getenv_basic.sans` | Create | Test getenv + alias |
| `tests/fixtures/mkdir_rmdir_basic.sans` | Create | Test mkdir + rmdir + is_dir |
| `tests/fixtures/remove_basic.sans` | Create | Test remove + alias |
| `tests/fixtures/listdir_basic.sans` | Create | Test listdir + alias |
| `tests/fixtures/sh_basic.sans` | Create | Test sh + alias |

### Implementation patterns reference

All patterns are derived from existing builtins. Key reference implementations:

- **Simple libc wrapper:** `file_exists` (typeck:875, ir:527, codegen:2268) — 1-arg, calls `@access`, returns bool
- **System call wrapper:** `system` (typeck:1617, ir:734, codegen:3563) — 1-arg, calls `@system`, returns int
- **Runtime function call:** `json_parse` (codegen:2285) — uses `compile_rt1(cg, inst, "sans_json_parse")`
- **Returns Array:** `args` (typeck:1607, ir:732, codegen:3534) — calls `@__sans_args()`, tracks with `emit_scope_track(cg, r, 1)`
- **Alias pattern:** Each alias is a separate `else if` in ir.sans mapping to the same IR opcode
- **Highest IR constant:** `IR_RESULT_CODE = 245` (constants.sans:383)

---

## Chunk 1: Constants + Simple Libc Builtins

### Task 1: Add all IR opcode constants

**Files:**
- Modify: `compiler/constants.sans:383` (after `IR_RESULT_CODE = 245`)

- [ ] **Step 1: Add 7 constants**

Add after the `g IR_RESULT_CODE = 245` line:

```sans
g IR_GETENV = 246
g IR_MKDIR = 247
g IR_RMDIR = 248
g IR_REMOVE = 249
g IR_LISTDIR = 250
g IR_IS_DIR = 251
g IR_SH = 252
```

- [ ] **Step 2: Build to verify no regressions**

Run: `sans build compiler/main.sans`
Expected: Success

- [ ] **Step 3: Commit**

```bash
git add compiler/constants.sans
git commit -m "feat: add IR opcode constants for package manager builtins (246-252)"
```

---

### Task 2: Add `getenv` builtin

`getenv(name: String) -> String`. Alias: `genv`. Wraps libc `getenv()`.

**Files:**
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`
- Create: `tests/fixtures/getenv_basic.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/getenv_basic.sans`:

```sans
main() I {
  home = getenv("HOME")
  home2 = genv("HOME")
  missing = getenv("SANS_NONEXISTENT_VAR_XYZ")
  if home.len() == 0 { return 1 }
  if home2.len() == 0 { return 2 }
  if missing.len() != 0 { return 3 }
  0
}
```

- [ ] **Step 2: Add type checking in typeck.sans**

Add in the builtin name matching chain (near `file_exists` around line 875):

```sans
if name == "getenv" || name == "genv" {
  if nargs != 1 { tc_error("getenv() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("getenv() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_STRING)
}
```

- [ ] **Step 3: Add IR lowering in ir.sans**

Add in the builtin lowering chain (near `file_exists` around line 527):

```sans
else if name == "getenv" { lower_call_1(ctx, args, IR_GETENV, IRTY_STR) }
else if name == "genv" { lower_call_1(ctx, args, IR_GETENV, IRTY_STR) }
```

- [ ] **Step 4: Add extern declaration in codegen.sans**

In `emit_externals_core()` (after `declare i32 @access` at line 311):

```sans
emit(cg, "declare ptr @getenv(ptr)")
```

In `emit_externals()` (after the duplicate `declare i32 @access` at line 396):

```sans
emit(cg, "declare ptr @getenv(ptr)")
```

- [ ] **Step 5: Add codegen in codegen.sans**

Add in the opcode dispatch. `getenv()` returns a pointer to internal storage (or NULL), so we must copy it to owned memory:

```sans
if op == IR_GETENV {
  nv = cg_get_val(cg, ir_field(inst, 16))
  np = cg_fresh_reg(cg)
  emit(cg, "  " + np + " = inttoptr i64 " + nv + " to ptr")
  rp = cg_fresh_reg(cg)
  emit(cg, "  " + rp + " = call ptr @getenv(ptr " + np + ")")
  is_null = cg_fresh_reg(cg)
  emit(cg, "  " + is_null + " = icmp eq ptr " + rp + ", null")
  safe_ptr = cg_fresh_reg(cg)
  emit(cg, "  " + safe_ptr + " = select i1 " + is_null + ", ptr @.empty_str, ptr " + rp)
  slen = cg_fresh_reg(cg)
  emit(cg, "  " + slen + " = call i64 @strlen(ptr " + safe_ptr + ")")
  sz = cg_fresh_reg(cg)
  emit(cg, "  " + sz + " = add i64 " + slen + ", 1")
  buf = cg_fresh_reg(cg)
  emit(cg, "  " + buf + " = call ptr @malloc(i64 " + sz + ")")
  emit(cg, "  call ptr @memcpy(ptr " + buf + ", ptr " + safe_ptr + ", i64 " + sz + ")")
  r = cg_fresh_reg(cg)
  emit(cg, "  " + r + " = ptrtoint ptr " + buf + " to i64")
  cg_set_val(cg, dest, r)
  emit_scope_track(cg, r, 0)
  return 0
}
```

Also add the `@.empty_str` global constant. Search `codegen.sans` for `@.empty_str` — if it already exists, skip this. If not, add it in the globals emission section (near other `@.` string constants, typically around line 280-290 in the preamble):

```sans
emit(cg, "@.empty_str = private unnamed_addr constant [1 x i8] zeroinitializer")
```

Add this to both `emit_externals_core()` and `emit_externals()` to ensure it's available in both runtime and user compilation modes.

- [ ] **Step 6: Build and test**

```bash
sans build compiler/main.sans
sans build tests/fixtures/getenv_basic.sans && ./a.out; echo $?
```

Expected: Exit code `0`

- [ ] **Step 7: Run full test suite**

```bash
bash tests/run_tests.sh
```

Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add compiler/typeck.sans compiler/ir.sans compiler/codegen.sans tests/fixtures/getenv_basic.sans
git commit -m "feat: add getenv() builtin — read environment variables"
```

---

### Task 3: Add `rmdir` builtin

`rmdir(path: String) -> Int`. Wraps libc `rmdir()`. Returns 1 on success, 0 on error.

**Files:**
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`

- [ ] **Step 1: Add type checking**

```sans
if name == "rmdir" {
  if nargs != 1 { tc_error("rmdir() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("rmdir() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_INT)
}
```

- [ ] **Step 2: Add IR lowering**

```sans
else if name == "rmdir" { lower_call_1(ctx, args, IR_RMDIR, IRTY_INT) }
```

- [ ] **Step 3: Add extern declaration**

In both `emit_externals_core()` and `emit_externals()`:

```sans
emit(cg, "declare i32 @rmdir(ptr)")
```

- [ ] **Step 4: Add codegen**

Follow the `system()` pattern exactly — 1-arg libc call returning int, convert 0=success to 1, non-zero to 0:

```sans
if op == IR_RMDIR {
  pv = cg_get_val(cg, ir_field(inst, 16))
  pp = cg_fresh_reg(cg)
  emit(cg, "  " + pp + " = inttoptr i64 " + pv + " to ptr")
  rv = cg_fresh_reg(cg)
  emit(cg, "  " + rv + " = call i32 @rmdir(ptr " + pp + ")")
  is_ok = cg_fresh_reg(cg)
  emit(cg, "  " + is_ok + " = icmp eq i32 " + rv + ", 0")
  r = cg_fresh_reg(cg)
  emit(cg, "  " + r + " = select i1 " + is_ok + ", i64 1, i64 0")
  cg_set_val(cg, dest, r)
  return 0
}
```

---

### Task 4: Add `remove` builtin

`remove(path: String) -> Int`. Alias: `rm`. Wraps libc `remove()`. Returns 1 on success, 0 on error.

**Files:**
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`
- Create: `tests/fixtures/remove_basic.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/remove_basic.sans`:

```sans
main() I {
  fw("/tmp/sans_test_remove_file.txt" "delete me")
  if fe("/tmp/sans_test_remove_file.txt") != true { return 1 }
  r = remove("/tmp/sans_test_remove_file.txt")
  if r != 1 { return 2 }
  if fe("/tmp/sans_test_remove_file.txt") == true { return 3 }
  // Removing nonexistent file returns 0
  r2 = rm("/tmp/sans_test_remove_nonexistent.txt")
  if r2 != 0 { return 4 }
  0
}
```

- [ ] **Step 2: Add type checking**

```sans
if name == "remove" || name == "rm" {
  if nargs != 1 { tc_error("remove() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("remove() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_INT)
}
```

- [ ] **Step 3: Add IR lowering**

```sans
else if name == "remove" { lower_call_1(ctx, args, IR_REMOVE, IRTY_INT) }
else if name == "rm" { lower_call_1(ctx, args, IR_REMOVE, IRTY_INT) }
```

- [ ] **Step 4: Add extern declaration**

In both `emit_externals_core()` and `emit_externals()`:

```sans
emit(cg, "declare i32 @remove(ptr)")
```

- [ ] **Step 5: Add codegen**

Same pattern as rmdir:

```sans
if op == IR_REMOVE {
  pv = cg_get_val(cg, ir_field(inst, 16))
  pp = cg_fresh_reg(cg)
  emit(cg, "  " + pp + " = inttoptr i64 " + pv + " to ptr")
  rv = cg_fresh_reg(cg)
  emit(cg, "  " + rv + " = call i32 @remove(ptr " + pp + ")")
  is_ok = cg_fresh_reg(cg)
  emit(cg, "  " + is_ok + " = icmp eq i32 " + rv + ", 0")
  r = cg_fresh_reg(cg)
  emit(cg, "  " + r + " = select i1 " + is_ok + ", i64 1, i64 0")
  cg_set_val(cg, dest, r)
  return 0
}
```

- [ ] **Step 6: Build, test, commit**

```bash
sans build compiler/main.sans
sans build tests/fixtures/remove_basic.sans && ./a.out; echo $?
bash tests/run_tests.sh
git add compiler/typeck.sans compiler/ir.sans compiler/codegen.sans tests/fixtures/remove_basic.sans
git commit -m "feat: add rmdir() and remove() builtins — delete files and directories"
```

---

## Chunk 2: Runtime-Backed Builtins (mkdir, is_dir, listdir, sh)

### Task 5: Create `runtime/fs.sans` and add `mkdir` + `is_dir` builtins

`mkdir(path: String) -> Int` — recursive directory creation via `system("mkdir -p ...")`
`is_dir(path: String) -> Bool` — check if path is directory via `system("test -d ...")`

**Files:**
- Create: `runtime/fs.sans`
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`
- Modify: `compiler/main.sans` (add "fs" to runtime_modules)
- Create: `tests/fixtures/mkdir_rmdir_basic.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/mkdir_rmdir_basic.sans`:

```sans
main() I {
  // Create nested directory
  r = mkdir("/tmp/sans_test_mkdir_2/sub/deep")
  if r != 1 { return 1 }
  // Verify with is_dir
  if is_dir("/tmp/sans_test_mkdir_2/sub/deep") != true { return 2 }
  // is_dir on a file should return false
  fw("/tmp/sans_test_mkdir_2/file.txt" "hi")
  if is_dir("/tmp/sans_test_mkdir_2/file.txt") != false { return 3 }
  // is_dir on nonexistent returns false
  if is_dir("/tmp/sans_test_mkdir_2/nope") != false { return 4 }
  // Idempotent mkdir
  r2 = mkdir("/tmp/sans_test_mkdir_2/sub/deep")
  if r2 != 1 { return 5 }
  // Cleanup
  remove("/tmp/sans_test_mkdir_2/file.txt")
  rmdir("/tmp/sans_test_mkdir_2/sub/deep")
  rmdir("/tmp/sans_test_mkdir_2/sub")
  rmdir("/tmp/sans_test_mkdir_2")
  0
}
```

- [ ] **Step 2: Create `runtime/fs.sans`**

```sans
// Filesystem runtime — mkdir, is_dir, listdir

sans_mkdir(path_i: I) I {
  plen = slen(path_i)
  if plen == 0 { return 0 }
  // "mkdir -p " = 9 chars
  cmd = alloc(9 + plen + 1)
  mcpy(cmd, ptr("mkdir -p "), 9)
  mcpy(cmd + 9, path_i, plen)
  store8(cmd + 9 + plen, 0)
  r = system(cmd)
  dealloc(cmd)
  r == 0 ? 1 : 0
}

sans_is_dir(path_i: I) I {
  plen = slen(path_i)
  // "test -d " = 8 chars
  cmd = alloc(8 + plen + 1)
  mcpy(cmd, ptr("test -d "), 8)
  mcpy(cmd + 8, path_i, plen)
  store8(cmd + 8 + plen, 0)
  r = system(cmd)
  dealloc(cmd)
  r == 0 ? 1 : 0
}
```

- [ ] **Step 3: Add type checking for `mkdir` and `is_dir`**

In `compiler/typeck.sans`:

```sans
if name == "mkdir" {
  if nargs != 1 { tc_error("mkdir() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("mkdir() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_INT)
}

if name == "is_dir" {
  if nargs != 1 { tc_error("is_dir() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("is_dir() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_BOOL)
}
```

- [ ] **Step 4: Add IR lowering**

In `compiler/ir.sans`:

```sans
else if name == "mkdir" { lower_call_1(ctx, args, IR_MKDIR, IRTY_INT) }
else if name == "is_dir" { lower_call_1(ctx, args, IR_IS_DIR, IRTY_BOOL) }
```

- [ ] **Step 5: Add codegen**

In `compiler/codegen.sans`, add to opcode dispatch:

```sans
if op == IR_MKDIR {
  compile_rt1(cg, inst, "sans_mkdir")
  return 0
}

if op == IR_IS_DIR {
  compile_rt1(cg, inst, "sans_is_dir")
  return 0
}
```

Add extern declarations in `emit_externals()` (NOT `emit_externals_core` — runtime functions are only available in user programs):

```sans
emit(cg, "declare i64 @sans_mkdir(i64)")
emit(cg, "declare i64 @sans_is_dir(i64)")
```

- [ ] **Step 6: Add "fs" to runtime modules in main.sans**

In `compiler/main.sans`, in the `runtime_modules` list (around line 410):

```sans
runtime_modules.push("fs")
```

- [ ] **Step 7: Build and test**

```bash
sans build compiler/main.sans
sans build tests/fixtures/mkdir_rmdir_basic.sans && ./a.out; echo $?
bash tests/run_tests.sh
```

- [ ] **Step 8: Commit**

```bash
git add runtime/fs.sans compiler/typeck.sans compiler/ir.sans compiler/codegen.sans compiler/main.sans tests/fixtures/mkdir_rmdir_basic.sans
git commit -m "feat: add mkdir() and is_dir() builtins with runtime/fs.sans"
```

---

### Task 6: Add `listdir` builtin

`listdir(path: String) -> Array<String>`. Alias: `ls`. Uses `popen("ls -1 <path>")` to capture output, splits by newline.

**Files:**
- Modify: `runtime/fs.sans` (add `sans_listdir`)
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`
- Create: `tests/fixtures/listdir_basic.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/listdir_basic.sans`:

```sans
main() I {
  // Setup: create a directory with known files
  mkdir("/tmp/sans_test_listdir_1")
  fw("/tmp/sans_test_listdir_1/a.txt" "a")
  fw("/tmp/sans_test_listdir_1/b.txt" "b")
  fw("/tmp/sans_test_listdir_1/c.txt" "c")

  files = listdir("/tmp/sans_test_listdir_1")
  if files.len() != 3 { return 1 }

  // Test alias
  files2 = ls("/tmp/sans_test_listdir_1")
  if files2.len() != 3 { return 2 }

  // Nonexistent directory returns empty array
  empty = listdir("/tmp/sans_test_listdir_nonexistent")
  if empty.len() != 0 { return 3 }

  // Cleanup
  remove("/tmp/sans_test_listdir_1/a.txt")
  remove("/tmp/sans_test_listdir_1/b.txt")
  remove("/tmp/sans_test_listdir_1/c.txt")
  rmdir("/tmp/sans_test_listdir_1")
  0
}
```

- [ ] **Step 2: Add `sans_listdir` to runtime/fs.sans**

This function uses `popen()` to run `ls -1 <path>` and capture output, then splits by newline into an array. The `popen`/`pclose` externs must be declared in codegen.

```sans
sans_listdir(path_i: I) I {
  plen = slen(path_i)

  // Build "ls -1 <path> 2>/dev/null" command
  // "ls -1 " = 6 chars, " 2>/dev/null" = 12 chars
  cmd = alloc(6 + plen + 12 + 1)
  mcpy(cmd, ptr("ls -1 "), 6)
  mcpy(cmd + 6, path_i, plen)
  mcpy(cmd + 6 + plen, ptr(" 2>/dev/null"), 12)
  store8(cmd + 6 + plen + 12, 0)

  // Write to unique temp file and read back
  // Use random() for unique filename to avoid parallel test races
  rnd = random(999999)
  rnd_s = str(rnd)
  rnd_len = slen(ptr(rnd_s))
  // "/tmp/sans_ld_<rnd>.txt" = 13 + rnd_len + 4
  tmp_len = 13 + rnd_len + 4 + 1
  tmp = alloc(tmp_len)
  mcpy(tmp, ptr("/tmp/sans_ld_"), 13)
  mcpy(tmp + 13, ptr(rnd_s), rnd_len)
  mcpy(tmp + 13 + rnd_len, ptr(".txt"), 4)
  store8(tmp + 13 + rnd_len + 4, 0)

  // "ls -1 <path> > <tmp> 2>/dev/null"
  cmd2_len = 6 + plen + 3 + tmp_len + 12
  cmd2 = alloc(cmd2_len)
  pos := 0
  mcpy(cmd2 + pos, ptr("ls -1 "), 6)
  pos = pos + 6
  mcpy(cmd2 + pos, path_i, plen)
  pos = pos + plen
  mcpy(cmd2 + pos, ptr(" > "), 3)
  pos = pos + 3
  mcpy(cmd2 + pos, tmp, 13 + rnd_len + 4)
  pos = pos + 13 + rnd_len + 4
  mcpy(cmd2 + pos, ptr(" 2>/dev/null"), 12)
  pos = pos + 12
  store8(cmd2 + pos, 0)

  r = system(cmd2)
  dealloc(cmd)
  dealloc(cmd2)

  if r != 0 {
    dealloc(tmp)
    return ptr(array<S>())
  }

  content = fr(tmp)
  // Clean up temp file
  rm_cmd = alloc(4 + tmp_len)
  mcpy(rm_cmd, ptr("rm -f "), 6)
  mcpy(rm_cmd + 6, tmp, 13 + rnd_len + 4)
  store8(rm_cmd + 6 + 13 + rnd_len + 4, 0)
  system(rm_cmd)
  dealloc(rm_cmd)
  dealloc(tmp)

  if content.len() == 0 {
    return ptr(array<S>())
  }

  result = content.split("\n")

  // Remove empty trailing entry from trailing newline
  if result.len() > 0 {
    last = result.get(result.len() - 1)
    if last.len() == 0 {
      result.pop()
    }
  }

  ptr(result)
}
```

**Implementation note:** This approach shells out to `ls -1` and reads the output from a unique temp file. This deviates from the spec (which mentions `opendir`/`readdir`/`closedir`) for pragmatic reasons — libc directory iteration requires struct manipulation that's complex in the Sans runtime. The `ls -1` approach is portable across macOS and Linux.

**Runtime builtin availability:** The runtime module uses `fr()` (file_read), `array<S>()`, `.split()`, `.get()`, `.pop()`, `ptr()`, `random()`, and `str()`. Existing runtime modules like `runtime/server.sans` use high-level builtins (string operations, arrays), so these should work. If any don't compile, the implementer should check how `runtime/server.sans` handles similar patterns and adapt. The key constraint: runtime modules compile with `emit_externals_core()` for C stdlib, but they DO get full Sans language features (builtins, methods, types) since they go through the same compiler pipeline.

- [ ] **Step 3: Add type checking**

```sans
if name == "listdir" || name == "ls" {
  if nargs != 1 { tc_error("listdir() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("listdir() requires String argument, got " + type_to_string(at)) }
  return make_array_type(make_type(TY_STRING))
}
```

- [ ] **Step 4: Add IR lowering**

```sans
else if name == "listdir" { lower_call_1(ctx, args, IR_LISTDIR, IRTY_ARRAY) }
else if name == "ls" { lower_call_1(ctx, args, IR_LISTDIR, IRTY_ARRAY) }
```

- [ ] **Step 5: Add codegen**

```sans
if op == IR_LISTDIR {
  compile_rt1(cg, inst, "sans_listdir")
  emit_scope_track(cg, cg_get_val(cg, dest), 1)
  return 0
}
```

Note: `emit_scope_track(cg, r, 1)` is needed because we're returning an array (type tag 1).

Add extern in `emit_externals()`:

```sans
emit(cg, "declare i64 @sans_listdir(i64)")
```

- [ ] **Step 6: Build and test**

```bash
sans build compiler/main.sans
sans build tests/fixtures/listdir_basic.sans && ./a.out; echo $?
bash tests/run_tests.sh
```

- [ ] **Step 7: Commit**

```bash
git add runtime/fs.sans compiler/typeck.sans compiler/ir.sans compiler/codegen.sans tests/fixtures/listdir_basic.sans
git commit -m "feat: add listdir() builtin — list directory contents"
```

---

### Task 7: Create `runtime/process.sans` and add `sh` builtin

`sh(cmd: String) -> String`. Alias: `shell`. Executes command and captures stdout.

**Files:**
- Create: `runtime/process.sans`
- Modify: `compiler/typeck.sans`, `compiler/ir.sans`, `compiler/codegen.sans`
- Modify: `compiler/main.sans` (add "process" to runtime_modules)
- Create: `tests/fixtures/sh_basic.sans`

- [ ] **Step 1: Write test fixture**

Create `tests/fixtures/sh_basic.sans`:

```sans
main() I {
  // Capture echo output
  r = sh("echo hello")
  if r.trim() != "hello" { return 1 }

  // Alias works
  r2 = shell("echo world")
  if r2.trim() != "world" { return 2 }

  // Failed command returns ""
  r3 = sh("false 2>/dev/null")
  // false produces no output, returns ""
  if r3.len() != 0 { return 3 }

  // Multi-line output
  r4 = sh("echo 'line1'; echo 'line2'")
  lines = r4.trim().split("\n")
  if lines.len() != 2 { return 4 }

  0
}
```

- [ ] **Step 2: Create `runtime/process.sans`**

Uses the temp-file approach (same as listdir) since wiring `popen` into Sans runtime is complex:

```sans
// Process execution runtime

sans_sh(cmd_i: I) I {
  cmd_len = slen(cmd_i)

  // Generate unique temp filename using random()
  rnd = random(999999)
  rnd_s = str(rnd)
  rnd_len = slen(ptr(rnd_s))
  // "/tmp/sans_sh_<rnd>.txt"
  tmp_len = 13 + rnd_len + 4
  tmp = alloc(tmp_len + 1)
  mcpy(tmp, ptr("/tmp/sans_sh_"), 13)
  mcpy(tmp + 13, ptr(rnd_s), rnd_len)
  mcpy(tmp + 13 + rnd_len, ptr(".txt"), 4)
  store8(tmp + tmp_len, 0)

  // Build "<cmd> > <tmp> 2>/dev/null"
  // Captures stdout only, discards stderr (per spec)
  sfx_pre = " > "
  sfx_post = " 2>/dev/null"
  full_len = cmd_len + 3 + tmp_len + 12 + 1
  full = alloc(full_len)
  pos := 0
  mcpy(full + pos, cmd_i, cmd_len)
  pos = pos + cmd_len
  mcpy(full + pos, ptr(sfx_pre), 3)
  pos = pos + 3
  mcpy(full + pos, tmp, tmp_len)
  pos = pos + tmp_len
  mcpy(full + pos, ptr(sfx_post), 12)
  pos = pos + 12
  store8(full + pos, 0)

  system(full)
  dealloc(full)

  content = fr(tmp)

  // Clean up temp file
  rm_len = 6 + tmp_len + 1
  rm_cmd = alloc(rm_len)
  mcpy(rm_cmd, ptr("rm -f "), 6)
  mcpy(rm_cmd + 6, tmp, tmp_len)
  store8(rm_cmd + 6 + tmp_len, 0)
  system(rm_cmd)
  dealloc(rm_cmd)
  dealloc(tmp)

  ptr(content)
}
```

**Design decision:** `sh()` captures stdout only and discards stderr (`2>/dev/null`), matching the spec. The package manager validates `sh()` results by checking for empty strings and verifying side effects (e.g., `is_dir()` after `git clone`). If the package manager needs stderr for diagnostics in the future, a separate `sh_err()` builtin can be added.

- [ ] **Step 3: Add type checking**

```sans
if name == "sh" || name == "shell" {
  if nargs != 1 { tc_error("sh() takes exactly 1 argument") }
  at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if is_i64_compat(at) != 1 { tc_error("sh() requires String argument, got " + type_to_string(at)) }
  return make_type(TY_STRING)
}
```

- [ ] **Step 4: Add IR lowering**

```sans
else if name == "sh" { lower_call_1(ctx, args, IR_SH, IRTY_STR) }
else if name == "shell" { lower_call_1(ctx, args, IR_SH, IRTY_STR) }
```

- [ ] **Step 5: Add codegen**

```sans
if op == IR_SH {
  compile_rt1(cg, inst, "sans_sh")
  emit_scope_track(cg, cg_get_val(cg, dest), 0)
  return 0
}
```

Scope track type 0 = string.

Add extern in `emit_externals()`:

```sans
emit(cg, "declare i64 @sans_sh(i64)")
```

- [ ] **Step 6: Add "process" to runtime modules in main.sans**

In `compiler/main.sans`, runtime_modules list:

```sans
runtime_modules.push("process")
```

- [ ] **Step 7: Build and test**

```bash
sans build compiler/main.sans
sans build tests/fixtures/sh_basic.sans && ./a.out; echo $?
bash tests/run_tests.sh
```

- [ ] **Step 8: Commit**

```bash
git add runtime/process.sans compiler/typeck.sans compiler/ir.sans compiler/codegen.sans compiler/main.sans tests/fixtures/sh_basic.sans
git commit -m "feat: add sh() builtin — execute command and capture stdout"
```

---

## Chunk 3: Documentation + Final Verification

### Task 8: Update all documentation

Per the Documentation Update Checklist in CLAUDE.md, every new builtin must update:

**Files:**
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `website/docs/index.html`
- Modify: `editors/vscode-sans/src/extension.ts` (HOVER_DATA)
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json`

- [ ] **Step 1: Update `docs/reference.md`**

Add a new "Filesystem & Process" section documenting all 7 builtins with examples, parameter types, return values, and aliases. Follow the style of existing sections.

- [ ] **Step 2: Update `docs/ai-reference.md`**

Add compact entries in the Functions section:

```
getenv(name)/genv(name)                 S -> S (read env var, "" if unset)
mkdir(path)                             S -> I (mkdir -p, 1=ok 0=err)
rmdir(path)                             S -> I (remove empty dir, 1=ok 0=err)
remove(path)/rm(path)                   S -> I (delete file, 1=ok 0=err)
listdir(path)/ls(path)                  S -> [S] (directory listing)
is_dir(path)                            S -> B (true if directory)
sh(cmd)/shell(cmd)                      S -> S (execute, capture stdout)
```

- [ ] **Step 3: Update `website/docs/index.html`**

Add the same documentation to the website docs page, following the existing HTML structure.

- [ ] **Step 4: Update VSCode extension HOVER_DATA**

Add entries to `editors/vscode-sans/src/extension.ts`:

```typescript
'getenv': '**getenv**(name: String) -> String\n\nRead environment variable. Returns "" if not set.\n\nUsage: `home = getenv("HOME")`',
'genv': '**getenv**(name: String) -> String\n\nAlias for `getenv()`. Read environment variable.\n\nUsage: `genv("PATH")`',
'mkdir': '**mkdir**(path: String) -> Int\n\nCreate directory and parents (mkdir -p). Returns 1 on success.\n\nUsage: `mkdir("src/lib")`',
'rmdir': '**rmdir**(path: String) -> Int\n\nRemove empty directory. Returns 1 on success.\n\nUsage: `rmdir("build/tmp")`',
'remove': '**remove**(path: String) -> Int\n\nDelete a file. Returns 1 on success.\n\nUsage: `remove("old.txt")`',
'rm': '**remove**(path: String) -> Int\n\nAlias for `remove()`. Delete a file.\n\nUsage: `rm("old.txt")`',
'listdir': '**listdir**(path: String) -> Array\\<String\\>\n\nList directory contents. Returns empty array on error.\n\nUsage: `files = listdir("src/")`',
'ls': '**listdir**(path: String) -> Array\\<String\\>\n\nAlias for `listdir()`.\n\nUsage: `ls("src/")`',
'is_dir': '**is_dir**(path: String) -> Bool\n\nCheck if path is a directory.\n\nUsage: `is_dir("/tmp")`',
'sh': '**sh**(cmd: String) -> String\n\nExecute command and capture stdout. Returns "" on failure.\n\nUsage: `output = sh("git status")`',
'shell': '**sh**(cmd: String) -> String\n\nAlias for `sh()`. Execute command and capture stdout.\n\nUsage: `shell("ls -la")`',
```

- [ ] **Step 5: Update syntax highlighting**

Add to `editors/vscode-sans/syntaxes/sans.tmLanguage.json`:

In `support.function.system.sans` pattern, add: `getenv|genv|mkdir|rmdir|remove|rm|listdir|ls|is_dir|sh|shell`

Or create a new group `support.function.fs.sans`:

```json
{
  "name": "support.function.fs.sans",
  "match": "\\b(getenv|genv|mkdir|rmdir|remove|rm|listdir|ls|is_dir|sh|shell)\\b"
}
```

- [ ] **Step 6: Update `README.md`**

Add filesystem and process builtins to the feature list in `README.md` (e.g., under a "Filesystem & Process" bullet or in the existing builtins section).

- [ ] **Step 7: Add example to `examples/`**

Create `examples/filesystem_demo.sans` (or add to an existing example) showcasing the new builtins:

```sans
main() I {
  // Read environment
  home = getenv("HOME")
  p("Home: " + home)

  // Create and inspect directories
  mkdir(home + "/.sans-demo")
  p("Created dir: " + str(is_dir(home + "/.sans-demo")))

  // List contents
  files = listdir(home)
  p("Files in home: " + str(files.len()))

  // Execute commands
  output = sh("uname -s")
  p("OS: " + output.trim())

  // Cleanup
  rmdir(home + "/.sans-demo")
  0
}
```

- [ ] **Step 8: Commit docs**

```bash
git add docs/reference.md docs/ai-reference.md website/docs/index.html editors/vscode-sans/src/extension.ts editors/vscode-sans/syntaxes/sans.tmLanguage.json README.md examples/filesystem_demo.sans
git commit -m "docs: add documentation for 7 new filesystem/process builtins"
```

---

### Task 9: Final verification

- [ ] **Step 1: Full test suite**

```bash
bash tests/run_tests.sh
```

Expected: All tests pass including the 5 new fixtures.

- [ ] **Step 2: Verify compiler self-build**

```bash
sans build compiler/main.sans
```

Expected: Compiler builds successfully with the new builtins.

- [ ] **Step 3: Manual smoke test**

Create a quick throwaway test:

```sans
main() I {
  home = getenv("HOME")
  p("HOME=" + home)
  mkdir(home + "/.sans/test_smoke")
  p("is_dir: " + str(is_dir(home + "/.sans/test_smoke")))
  files = listdir(home)
  p("home files: " + str(files.len()))
  output = sh("echo 'sans package manager'")
  p("sh: " + output.trim())
  rmdir(home + "/.sans/test_smoke")
  rmdir(home + "/.sans")
  0
}
```

- [ ] **Step 4: Create PR**

```bash
git push -u origin feat/package-manager-builtins
gh pr create --title "feat: add 7 package manager builtins" --body "..."
```
