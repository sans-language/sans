# Type-Tagged Scope GC Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete automatic memory management for all heap types via scope-based tracking with type-aware destructors.

**Architecture:** Every heap allocation in user code is tracked in a per-function scope with a type tag. On function return, `scope_exit` frees each allocation using a type-appropriate destructor. Return values are promoted to the caller's scope. Runtime modules are excluded from all instrumentation.

**Tech Stack:** Sans compiler (self-hosted), LLVM IR codegen, `/tmp/sans_stage2` bootstrap binary.

**Spec:** `docs/superpowers/specs/2026-03-18-arc-scope-gc-design.md`

---

## Chunk 1: Runtime Foundation

### Task 1: Add scope tag constants

**Files:**
- Modify: `compiler/constants.sans:362` (after IR_RC_COUNT)

- [ ] **Step 1: Add SCOPE_TAG constants**

After line 362 (`g IR_RC_COUNT = 215`), add:

```sans
// Scope GC type tags
g SCOPE_TAG_RAW = 0
g SCOPE_TAG_ARRAY = 1
g SCOPE_TAG_MAP = 2
g SCOPE_TAG_JSON = 3
g SCOPE_TAG_RESULT = 4
g SCOPE_TAG_STRING = 5
```

- [ ] **Step 2: Build with bootstrap to verify no syntax errors**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

- [ ] **Step 3: Commit**

```bash
git add compiler/constants.sans
git commit -m "feat(arc): add SCOPE_TAG_* constants for type-tagged scope GC"
```

### Task 2: Rewrite rc.sans with type-tagged tracking, destructors, and scope promotion

**Files:**
- Modify: `runtime/rc.sans` (complete rewrite of scope section, lines 29-68)

The scope tracking section needs these changes:
1. `sans_scope_track(ptr, tag)` — 2 args, 24-byte nodes (ptr + tag + next)
2. `scope_free(ptr, tag)` — destructor dispatch
3. `sans_scope_exit(keep)` — promote keep to parent scope

- [ ] **Step 1: Rewrite the scope section of rc.sans**

Replace lines 29-68 (from `// Scope-based memory management` through end of file) with:

```sans
// Scope-based memory management
// Tracking nodes: 24 bytes [ptr, type_tag, next]
g rc_alloc_head = 0
g rc_scope_head = 0

sans_scope_track(ptr:I tag:I) I {
  node = alloc(24)
  store64(node, ptr)
  store64(node + 8, tag)
  store64(node + 16, rc_alloc_head)
  rc_alloc_head = node
  ptr
}

sans_scope_enter() I {
  frame = alloc(16)
  store64(frame, rc_alloc_head)
  store64(frame + 8, rc_scope_head)
  rc_scope_head = frame
  0
}

scope_free(ptr:I tag:I) I {
  if tag == 0 {
    // SCOPE_TAG_RAW: raw alloc, struct, enum
    dealloc(ptr)
  } else if tag == 1 {
    // SCOPE_TAG_ARRAY: [data_buf, len, cap] — 24 bytes
    dealloc(load64(ptr))
    dealloc(ptr)
  } else if tag == 2 {
    // SCOPE_TAG_MAP: [hashes, keys, vals, count, cap] — 40 bytes
    dealloc(load64(ptr))
    dealloc(load64(ptr + 8))
    dealloc(load64(ptr + 16))
    dealloc(ptr)
  } else if tag == 3 {
    // SCOPE_TAG_JSON: 16-byte [json_tag, payload]
    // json_tag: 0=Null 1=Bool 2=Int 3=String 4=Array 5=Object
    jt = load64(ptr)
    if jt == 3 {
      // String: free string buffer
      dealloc(load64(ptr + 8))
    } else if jt == 4 {
      // Array: arr_ptr -> [items_buf, len, cap]
      arr_ptr = load64(ptr + 8)
      if arr_ptr != 0 {
        dealloc(load64(arr_ptr))
        dealloc(arr_ptr)
      }
    } else if jt == 5 {
      // Object: obj_ptr -> [hashes, keys, vals, count, cap]
      obj_ptr = load64(ptr + 8)
      if obj_ptr != 0 {
        dealloc(load64(obj_ptr))
        dealloc(load64(obj_ptr + 8))
        dealloc(load64(obj_ptr + 16))
        dealloc(obj_ptr)
      }
    }
    dealloc(ptr)
  } else if tag == 4 {
    // SCOPE_TAG_RESULT: [tag, value, err_str] — 24 bytes
    rt = load64(ptr)
    if rt == 1 {
      // Error: free error string
      err_s = load64(ptr + 16)
      if err_s != 0 { dealloc(err_s) }
    }
    dealloc(ptr)
  } else if tag == 5 {
    // SCOPE_TAG_STRING: dynamic string
    dealloc(ptr)
  }
  0
}

sans_scope_exit(keep:I) I {
  if rc_scope_head != 0 {
    watermark = load64(rc_scope_head)
    old_frame = rc_scope_head
    rc_scope_head = load64(rc_scope_head + 8)
    dealloc(old_frame)

    // Walk tracking list, free everything except keep
    kept_node := 0
    while rc_alloc_head != watermark {
      node = rc_alloc_head
      ptr = load64(node)
      tag = load64(node + 8)
      next = load64(node + 16)
      if ptr == keep {
        // Save this node for promotion to parent scope
        kept_node = node
      } else {
        if ptr != 0 { scope_free(ptr, tag) }
        dealloc(node)
      }
      rc_alloc_head = next
    }

    // Promote kept node to parent scope
    if kept_node != 0 {
      store64(kept_node + 16, rc_alloc_head)
      rc_alloc_head = kept_node
    }
  }
  0
}
```

- [ ] **Step 2: Build with bootstrap to verify rc.sans compiles**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

The bootstrap compiler doesn't emit scope calls, so the old 1-arg `scope_track` signature in the bootstrap's codegen won't conflict — it only affects how user code is compiled, and we're compiling the compiler itself here.

- [ ] **Step 3: Test with simple program**

Create `/tmp/test_rc_new.sans`:
```sans
main() I {
  buf = alloc(64)
  store64(buf, 42)
  p(load64(buf))
  0
}
```

Run: `./compiler/main build /tmp/test_rc_new.sans 2>&1 | tail -3 && /tmp/test_rc_new`
Expected: `42`

- [ ] **Step 4: Commit**

```bash
git add runtime/rc.sans
git commit -m "feat(arc): type-tagged scope tracking with destructors and promotion"
```

---

## Chunk 2: IR Module Extension

### Task 3: Expose fn_ret_types on IrModule

**Files:**
- Modify: `compiler/ir.sans:203-220` (IrModule struct + constructor + accessors)
- Modify: `compiler/ir.sans:3478-3482` (store fn_ret_types after module creation)

The IrModule is currently 32 bytes (offsets 0-24, despite comment saying "24 bytes"). We add `fn_ret_types` at offset 32, making it 40 bytes.

**Note:** The spec lists `compiler/main.sans` as a changed file for threading fn_ret_types. This plan uses a cleaner approach: embed fn_ret_types directly on the IrModule struct, so codegen reads it from the module. No main.sans changes needed.

- [ ] **Step 1: Expand IrModule struct**

In `compiler/ir.sans`, update the comment at line 203:
```sans
// ------ IrModule (40 bytes) ------------------------------------------------------------------------------------------------------------------------
// offset 0:  functions (I) --- Array of IrFunction pointers
// offset 8:  globals (I) --- Array of global name strings
// offset 16: struct_defs (I) --- Map: name -> field names array
// offset 24: global_init_values (I) --- Map: global name -> i64 init value
// offset 32: fn_ret_types (I) --- Map: function name -> IRTY return type tag
```

Update `make_ir_module` at line 208 to allocate 40 bytes:
```sans
make_ir_module(funcs:I globals:I struct_defs:I) I {
  m = alloc(40)
  store64(m, funcs)
  store64(m + 8, globals)
  store64(m + 16, struct_defs)
  store64(m + 24, M())  // global_init_values map (name -> i64)
  store64(m + 32, M())  // fn_ret_types map (name -> IRTY tag)
  m
}
```

Add accessor after line 220:
```sans
irm_fn_ret_types(m:I) I = load64(m + 32)
```

- [ ] **Step 2: Store fn_ret_types in lower_full**

In `compiler/ir.sans`, after line 3480 (`mod = make_ir_module(ir_functions, globals_arr, struct_defs)`), add:
```sans
  store64(mod + 32, fn_ret_types)
```

So lines 3479-3483 become:
```sans
  // 7. Build and return IrModule
  mod = make_ir_module(ir_functions, globals_arr, struct_defs)
  store64(mod + 24, global_init_vals)
  store64(mod + 32, fn_ret_types)
  mod
```

- [ ] **Step 3: Build with bootstrap**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

- [ ] **Step 4: Verify compiler still self-tests**

Run: `bash tests/run_tests.sh ./compiler/main 2>&1 | tail -3`
Expected: `72/83 tests passed` (same as before)

- [ ] **Step 5: Commit**

```bash
git add compiler/ir.sans
git commit -m "feat(arc): expose fn_ret_types on IrModule at offset 32"
```

---

## Chunk 3: Codegen — Update Declarations & Existing Instrumentation

### Task 4: Update scope_track declaration and existing call sites

**Files:**
- Modify: `compiler/codegen.sans:524-526` (extern declarations)
- Modify: `compiler/codegen.sans:~1298` (IR_ALLOC scope_track call — add tag arg)

- [ ] **Step 1: Update extern declaration for scope_track**

In `emit_externals`, find the line:
```sans
  emit(cg, "declare i64 @sans_scope_track(i64)")
```
Replace with:
```sans
  emit(cg, "declare i64 @sans_scope_track(i64, i64)")
```

- [ ] **Step 2: Update existing IR_ALLOC scope_track call**

In the IR_ALLOC handler (~line 1298), find:
```sans
      st = cg_fresh_reg(cg)
      emit(cg, "  " + st + " = call i64 @sans_scope_track(i64 " + r + ")")
```
Replace with:
```sans
      st = cg_fresh_reg(cg)
      emit(cg, "  " + st + " = call i64 @sans_scope_track(i64 " + r + ", i64 0)")
```

The `0` is `SCOPE_TAG_RAW`.

- [ ] **Step 3: Build and run basic test**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

Run: `./compiler/main build /tmp/test_rc_new.sans 2>&1 | tail -3 && /tmp/test_rc_new`
Expected: `42`

- [ ] **Step 4: Commit**

```bash
git add compiler/codegen.sans
git commit -m "feat(arc): update scope_track to 2-arg (ptr, tag) signature"
```

### Task 5: Add scope_track emission helper

**Files:**
- Modify: `compiler/codegen.sans` (add helper function after compile_rt4, ~line 829)

A helper to avoid duplicating the scope_track emission pattern everywhere:

- [ ] **Step 1: Add emit_scope_track helper**

After the `compile_rt4` function (~line 829), add:

```sans
// Emit scope_track call for heap allocations in user code
// val = the LLVM register holding the pointer, tag = SCOPE_TAG_* constant
emit_scope_track(cg:I val:S tag:I) I {
  if cg_is_runtime_flag == 0 {
    st = cg_fresh_reg(cg)
    emit(cg, "  " + st + " = call i64 @sans_scope_track(i64 " + val + ", i64 " + str(tag) + ")")
  }
  0
}
```

- [ ] **Step 2: Refactor existing IR_ALLOC to use helper**

In the IR_ALLOC handler, replace:
```sans
    // Track allocation in scope for user code
    if cg_is_runtime_flag == 0 {
      st = cg_fresh_reg(cg)
      emit(cg, "  " + st + " = call i64 @sans_scope_track(i64 " + r + ", i64 0)")
    }
```
With:
```sans
    emit_scope_track(cg, r, 0)
```

- [ ] **Step 3: Build and verify**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

- [ ] **Step 4: Commit**

```bash
git add compiler/codegen.sans
git commit -m "refactor(arc): add emit_scope_track helper for codegen"
```

---

## Chunk 4: Codegen — Inlined Allocation Tracking

### Task 6: Add scope_track to inlined heap-creating instructions

**Files:**
- Modify: `compiler/codegen.sans` — IR_ARRAY_CREATE, IR_STRUCT_ALLOC, IR_ENUM_ALLOC, IR_STRING_CONCAT, IR_STRING_SUBSTRING, IR_INT_TO_STRING, IR_FLOAT_TO_STRING

For each handler, add `emit_scope_track(cg, val, TAG)` after the value is set. The `val` is the final LLVM register (the one passed to `cg_set_val`). Add the call just BEFORE the `return 0`.

- [ ] **Step 1: IR_ARRAY_CREATE (~line 1368)**

Find the `return 0` in the IR_ARRAY_CREATE block. Add before it:
```sans
    emit_scope_track(cg, r, 1)
```
Where `r` is the register holding the final ptrtoint result (the one passed to `cg_set_val`). Note: `1` = SCOPE_TAG_ARRAY.

- [ ] **Step 2: IR_STRUCT_ALLOC (~line 1187)**

Find the `return 0` in the IR_STRUCT_ALLOC block. Add before it:
```sans
    emit_scope_track(cg, r, 0)
```
Where `r` is the register from `cg_set_val`. `0` = SCOPE_TAG_RAW.

- [ ] **Step 3: IR_ENUM_ALLOC (~line 1229)**

Find the `return 0` in the IR_ENUM_ALLOC block. Add before it:
```sans
    emit_scope_track(cg, r, 0)
```

- [ ] **Step 4: IR_STRING_CONCAT (~line 1650)**

Find the `return 0` in the IR_STRING_CONCAT block. The final register is the ptrtoint result. Add before `return 0`:
```sans
    emit_scope_track(cg, r, 5)
```
`5` = SCOPE_TAG_STRING.

- [ ] **Step 5: IR_STRING_SUBSTRING (~line 1688)**

Same pattern. Add before `return 0`:
```sans
    emit_scope_track(cg, r, 5)
```

- [ ] **Step 6: IR_INT_TO_STRING (~line 1715)**

Add before `return 0`:
```sans
    emit_scope_track(cg, r, 5)
```

- [ ] **Step 7: IR_FLOAT_TO_STRING (~line 1741)**

Add before `return 0`:
```sans
    emit_scope_track(cg, r, 5)
```

- [ ] **Step 8: Build and run E2E tests**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

Run: `bash tests/run_tests.sh ./compiler/main 2>&1 | tail -3`
Expected: `72/83 tests passed` (no regressions)

- [ ] **Step 9: Commit**

```bash
git add compiler/codegen.sans
git commit -m "feat(arc): scope_track inlined allocations (array, struct, enum, string ops)"
```

---

## Chunk 5: Codegen — Runtime Call Tracking

### Task 7: Add scope_track to runtime-call heap-creating instructions

**Files:**
- Modify: `compiler/codegen.sans` — all compile_rt* based handlers

Many handlers use `return compile_rt*(...)` as a one-liner. These must be restructured into multi-line blocks to insert `emit_scope_track` after the call. Pattern:

**Before:**
```sans
  if op == IR_FOO { return compile_rt0(cg, inst, "sans_foo") }
```
**After:**
```sans
  if op == IR_FOO {
    compile_rt0(cg, inst, "sans_foo")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), TAG)
    return 0
  }
```

For handlers that already have multi-line blocks with `cg_set_val`/`cg_set_ptr`, add `emit_scope_track(cg, r, TAG)` before `return 0` (where `r` is the register passed to `cg_set_val`).

- [ ] **Step 1: IR_MAP_CREATE (line 2997) — SCOPE_TAG_MAP (2)**

Replace:
```sans
  if op == IR_MAP_CREATE { return compile_rt0(cg, inst, "sans_map_create") }
```
With:
```sans
  if op == IR_MAP_CREATE {
    compile_rt0(cg, inst, "sans_map_create")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 2)
    return 0
  }
```

- [ ] **Step 2: JSON create/parse opcodes (lines 1986-1992) — SCOPE_TAG_JSON (3)**

Restructure each JSON one-liner. Replace lines 1986-1992 with:
```sans
  if op == IR_JSON_PARSE {
    compile_rt1(cg, inst, "sans_json_parse")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_OBJECT {
    compile_rt0(cg, inst, "sans_json_object")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_ARRAY {
    compile_rt0(cg, inst, "sans_json_array")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_STRING {
    compile_rt1(cg, inst, "sans_json_string")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_INT {
    compile_rt1(cg, inst, "sans_json_int")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_BOOL {
    compile_rt1(cg, inst, "sans_json_bool")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
  if op == IR_JSON_NULL {
    compile_rt0(cg, inst, "sans_json_null")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 3)
    return 0
  }
```

- [ ] **Step 3: IR_RESULT_OK, IR_RESULT_ERR (lines 2701-2702) — SCOPE_TAG_RESULT (4)**

Replace:
```sans
  if op == IR_RESULT_OK { return compile_rt1(cg, inst, "sans_result_ok") }
  if op == IR_RESULT_ERR { return compile_rt1(cg, inst, "sans_result_err") }
```
With:
```sans
  if op == IR_RESULT_OK {
    compile_rt1(cg, inst, "sans_result_ok")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 4)
    return 0
  }
  if op == IR_RESULT_ERR {
    compile_rt1(cg, inst, "sans_result_err")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 4)
    return 0
  }
```

- [ ] **Step 4: IR_STRING_TRIM (line 1758) and IR_STRING_REPLACE (line 1809) — SCOPE_TAG_STRING (5)**

These already have multi-line blocks. Add `emit_scope_track(cg, r, 5)` before `return 0` in each.

- [ ] **Step 5: IR_STRING_SPLIT (line 1798) — SCOPE_TAG_ARRAY (1)**

Already multi-line. Add `emit_scope_track(cg, r, 1)` before `return 0`.

- [ ] **Step 6: Array functional ops — SCOPE_TAG_ARRAY (1)**

IR_ARRAY_MAP (~1547), IR_ARRAY_FILTER (~1558), IR_ARRAY_ENUMERATE (~1589), IR_ARRAY_ZIP (~1599) already have multi-line blocks. Add `emit_scope_track(cg, r, 1)` before `return 0` in each.

- [ ] **Step 7: IR_MAP_KEYS, IR_MAP_VALS (lines 3002-3003) — SCOPE_TAG_ARRAY (1)**

Replace:
```sans
  if op == IR_MAP_KEYS { return compile_rt1(cg, inst, "sans_map_keys") }
  if op == IR_MAP_VALS { return compile_rt1(cg, inst, "sans_map_vals") }
```
With:
```sans
  if op == IR_MAP_KEYS {
    compile_rt1(cg, inst, "sans_map_keys")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 1)
    return 0
  }
  if op == IR_MAP_VALS {
    compile_rt1(cg, inst, "sans_map_vals")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 1)
    return 0
  }
```

- [ ] **Step 8: IR_JSON_STRINGIFY (line 2002) — SCOPE_TAG_STRING (5)**

Replace:
```sans
  if op == IR_JSON_STRINGIFY { return compile_rt1(cg, inst, "sans_json_stringify") }
```
With:
```sans
  if op == IR_JSON_STRINGIFY {
    compile_rt1(cg, inst, "sans_json_stringify")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 5)
    return 0
  }
```

- [ ] **Step 9: IR_JSON_GET_STRING (line 1995) — SCOPE_TAG_STRING (5)**

Replace:
```sans
  if op == IR_JSON_GET_STRING { return compile_rt1(cg, inst, "sans_json_get_string") }
```
With:
```sans
  if op == IR_JSON_GET_STRING {
    compile_rt1(cg, inst, "sans_json_get_string")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 5)
    return 0
  }
```

- [ ] **Step 10: IR_JSON_TYPE_OF (line 1999) — SCOPE_TAG_STRING (5)**

Replace:
```sans
  if op == IR_JSON_TYPE_OF { return compile_rt1(cg, inst, "sans_json_type_of") }
```
With:
```sans
  if op == IR_JSON_TYPE_OF {
    compile_rt1(cg, inst, "sans_json_type_of")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 5)
    return 0
  }
```

- [ ] **Step 11: IR_URL_DECODE (line 2020) — SCOPE_TAG_STRING (5)**

Replace:
```sans
  if op == IR_URL_DECODE { return compile_rt1(cg, inst, "sans_url_decode") }
```
With:
```sans
  if op == IR_URL_DECODE {
    compile_rt1(cg, inst, "sans_url_decode")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 5)
    return 0
  }
```

- [ ] **Step 12: IR_PATH_SEGMENT (line 2021) — SCOPE_TAG_STRING (5)**

Replace:
```sans
  if op == IR_PATH_SEGMENT { return compile_rt2(cg, inst, "sans_path_segment") }
```
With:
```sans
  if op == IR_PATH_SEGMENT {
    compile_rt2(cg, inst, "sans_path_segment")
    emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 5)
    return 0
  }
```

- [ ] **Step 13: IR_FILE_READ (~line 1821) — SCOPE_TAG_STRING (5)**

Large inlined handler. Add before the final `return 0`:
```sans
    emit_scope_track(cg, cg_get_val(cg, dest), 5)
```

- [ ] **Step 14: IR_GZIP_COMPRESS (~line 3108) — SCOPE_TAG_RAW (0)**

Add before the final `return 0`:
```sans
    emit_scope_track(cg, cg_get_val(cg, dest), 0)
```

- [ ] **Step 15: IR_ARGS (line 3011) — SCOPE_TAG_ARRAY (1)**

Already multi-line. Add `emit_scope_track(cg, r, 1)` before `return 0`.

- [ ] **Step 16: Build and run E2E tests**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

Run: `bash tests/run_tests.sh ./compiler/main 2>&1 | tail -3`
Expected: `72/83 tests passed` (no regressions)

- [ ] **Step 17: Commit**

```bash
git add compiler/codegen.sans
git commit -m "feat(arc): scope_track all runtime-call heap allocations"
```

---

## Chunk 6: Codegen — Function Call Return Tracking

### Task 8: Store fn_ret_types in codegen global and track user function call returns

**Files:**
- Modify: `compiler/codegen.sans:5-6` (add global for fn_ret_types)
- Modify: `compiler/codegen.sans:~3526` (compile_to_ll_impl — read fn_ret_types from module)
- Modify: `compiler/codegen.sans:~940-997` (IR_CALL handler — add scope_track for heap returns)

- [ ] **Step 1: Add global for fn_ret_types map**

Near the top of codegen.sans, after the `g cg_is_runtime_flag = 0` line, add:
```sans
g cg_fn_ret_types = 0
```

- [ ] **Step 2: Set global in compile_to_ll_impl**

In `compile_to_ll_impl`, after the line `cg_is_runtime_flag = is_runtime`, add:
```sans
  cg_fn_ret_types = irm_fn_ret_types(module)
```

- [ ] **Step 3: Add scope_track after IR_CALL for heap-returning functions**

In the IR_CALL handler (the section that handles user function calls, ending around line 997), find the lines:
```sans
    r = cg_fresh_reg(cg)
    emit(cg, "  " + r + " = call i64 @" + llvm_fname + "(" + arg_str + ")")
    cg_set_val(cg, dest, r)
    return 0
```

Replace `return 0` with:
```sans
    // Track return value if function returns a heap type
    if cg_is_runtime_flag == 0 {
      if cg_fn_ret_types != 0 {
        frt = mget(cg_fn_ret_types, fname)
        if frt != 0 {
          // Map IRTY to scope tag (use named constants from ir.sans)
          stag := 0
          if frt == IRTY_ARRAY { stag = 1 }
          else if frt == IRTY_MAP { stag = 2 }
          else if frt == IRTY_JSON { stag = 3 }
          else if frt == IRTY_RESULT { stag = 4 }
          else if frt == IRTY_STR { stag = 5 }
          // IRTY_STRUCT (4), IRTY_ENUM (5) -> SCOPE_TAG_RAW (0) -- stag already 0
          emit_scope_track(cg, r, stag)
        }
      }
    }
    return 0
```

**IRTY constants** (from `compiler/ir.sans` lines 5-22, imported via `import "ir"`):
`IRTY_INT=0, IRTY_FLOAT=1, IRTY_BOOL=2, IRTY_STR=3, IRTY_STRUCT=4, IRTY_ENUM=5, IRTY_SENDER=6, IRTY_RECEIVER=7, IRTY_JOIN_HANDLE=8, IRTY_MUTEX=9, IRTY_ARRAY=10, IRTY_JSON=11, IRTY_HTTP_RESP=12, IRTY_RESULT=13, IRTY_HTTP_SERVER=14, IRTY_HTTP_REQ=15, IRTY_TUPLE=16, IRTY_MAP=17`

Use the named constants (not raw integers) since `codegen.sans` imports `ir`.

- [ ] **Step 4: Build and run E2E tests**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

Run: `bash tests/run_tests.sh ./compiler/main 2>&1 | tail -3`
Expected: `72/83 tests passed`

- [ ] **Step 5: Commit**

```bash
git add compiler/codegen.sans
git commit -m "feat(arc): track heap-returning user function calls via fn_ret_types"
```

---

## Chunk 7: Tests and Validation

### Task 9: Write test fixtures

**Files:**
- Create: `tests/fixtures/scope_typed.sans`
- Create: `tests/fixtures/scope_nested_calls.sans`
- Create: `tests/fixtures/scope_string.sans`
- Modify: `tests/run_tests.sh` (add test entries)

- [ ] **Step 1: Create scope_typed.sans**

```sans
// Test: all heap types created and freed on scope exit
make_things() I {
  a = array<I>()
  a.push(1)
  a.push(2)
  m = M()
  mset(m, "x", 10)
  j = json_object()
  json_set(j, "k", json_int(42))
  r = ok(99)
  // All freed on scope exit, return value (r) promoted
  r!
}

main() I {
  v = make_things()
  v
}
```

Expected exit code: `99`

- [ ] **Step 2: Create scope_nested_calls.sans**

```sans
// Test: return values promoted to caller scope
make_arr() I {
  a = array<I>()
  a.push(10)
  a.push(20)
  a.push(30)
  a
}

sum_arr(a:I) I {
  a.get(0) + a.get(1) + a.get(2)
}

main() I {
  a = make_arr()
  sum_arr(a)
}
```

Expected exit code: `60`

- [ ] **Step 3: Create scope_string.sans**

```sans
// Test: dynamic strings freed on scope exit
greet(name:S) S {
  "hello " + name
}

main() I {
  s = greet("world")
  p(s)
  0
}
```

Expected output: `hello world`, exit code: `0`

- [ ] **Step 4: Add tests to run_tests.sh**

After the `scope_basic` entry, add:
```bash
run_test "scope_typed"               "$REPO_ROOT/tests/fixtures/scope_typed.sans"               99
run_test "scope_nested_calls"        "$REPO_ROOT/tests/fixtures/scope_nested_calls.sans"        60
run_test "scope_string"              "$REPO_ROOT/tests/fixtures/scope_string.sans"              0
```

- [ ] **Step 5: Build compiler and run new tests**

Run: `/tmp/sans_stage2 build compiler/main.sans 2>&1 | tail -3`
Expected: `Built: compiler/main`

Run: `bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5`
Expected: `75/86 tests passed` (3 new tests + 72 existing)

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/scope_typed.sans tests/fixtures/scope_nested_calls.sans tests/fixtures/scope_string.sans tests/run_tests.sh
git commit -m "test(arc): add scope GC test fixtures for typed tracking and promotion"
```

### Task 10: Full validation — self-hosting and stress test

- [ ] **Step 1: Verify self-hosting**

The compiler should be able to compile itself with scope GC active:
```bash
./compiler/main build compiler/main.sans 2>&1 | tail -3
```
Expected: `Built: compiler/main`

Then verify the self-compiled compiler also works:
```bash
./compiler/main build /tmp/test_rc_new.sans 2>&1 | tail -3 && /tmp/test_rc_new
```
Expected: `42`

- [ ] **Step 2: Run full E2E suite with self-compiled compiler**

```bash
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```
Expected: `75/86 tests passed` (11 skipped, 0 failed)

- [ ] **Step 3: Commit all remaining changes**

```bash
git add -A
git commit -m "feat(arc): complete type-tagged scope GC — all heap types tracked and freed"
```
