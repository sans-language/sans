# Type-Tagged Scope GC — Design Spec

**Date:** 2026-03-18
**Status:** Approved
**Goal:** Automatic memory management for all heap types via scope-based tracking with type-aware destructors

## Context

Sans currently leaks all heap memory. Phase 1 of ARC added raw `alloc()` tracking via `scope_enter`/`scope_exit`/`scope_track` in codegen. This spec completes the system to cover all heap types (arrays, maps, JSON, Result, strings, structs, enums) with proper destruction.

## Design

### Approach: Type-Tagged Scope Tracking

Every heap allocation in user code is tracked in the current function's scope with a type tag. On function return, `scope_exit` frees each tracked allocation using a type-appropriate destructor. The return value is promoted to the parent scope (re-tracked in the caller) rather than freed.

**Not true ARC.** No retain/release on pointer copies. No ownership tracking. Scope-based: allocations live for the duration of the creating function, then are freed or promoted. This covers 95%+ of AI-generated code patterns.

### Tracking Node Layout (24 bytes)

```
offset 0:  ptr (I)       — heap pointer
offset 8:  type_tag (I)  — destructor selector
offset 16: next (I)      — next node in linked list
```

### Type Tags

```
SCOPE_TAG_RAW    = 0   — raw alloc/struct/enum: dealloc(ptr)
SCOPE_TAG_ARRAY  = 1   — array: dealloc(data_buf), dealloc(ptr)
SCOPE_TAG_MAP    = 2   — map: dealloc(hashes), dealloc(keys), dealloc(vals), dealloc(ptr)
SCOPE_TAG_JSON   = 3   — JSON: recursive free based on JSON type tag
SCOPE_TAG_RESULT = 4   — Result: if err, dealloc(err_str), dealloc(ptr)
SCOPE_TAG_STRING = 5   — dynamic string: dealloc(ptr)
```

### Destructor Logic (`scope_free` in rc.sans)

Dispatches on type_tag:

- **RAW/STRING**: `dealloc(ptr)`
- **ARRAY**: `dealloc(load64(ptr))` (data buffer at offset 0), then `dealloc(ptr)`
- **MAP**: `dealloc(load64(ptr))` (hashes at +0), `dealloc(load64(ptr+8))` (keys at +8), `dealloc(load64(ptr+16))` (vals at +16), then `dealloc(ptr)` (40-byte struct). Note: map internal buffers are allocated inside `sans_map_create` (runtime code), so they are NOT separately scope-tracked — no double-free risk.
- **JSON**: Read type tag at `load64(ptr)` (json.sans tags: 0=Null, 1=Bool, 2=Int, 3=String, 4=Array, 5=Object):
  - Null (0), Bool (1), Int (2): just `dealloc(ptr)` (16-byte JsonValue struct, no sub-allocs)
  - String (3): `dealloc(load64(ptr+8))` (string copy buffer), then `dealloc(ptr)`
  - Array (4): `arr_ptr = load64(ptr+8)` → `dealloc(load64(arr_ptr))` (items buffer), `dealloc(arr_ptr)` (24-byte array struct), then `dealloc(ptr)`. Note: individual JSON elements within the array are NOT recursively freed (see Known Limitations).
  - Object (5): `obj_ptr = load64(ptr+8)` → `dealloc(load64(obj_ptr))` (hashes), `dealloc(load64(obj_ptr+8))` (keys), `dealloc(load64(obj_ptr+16))` (vals), `dealloc(obj_ptr)` (40-byte obj struct), then `dealloc(ptr)`
- **RESULT**: Read tag at `load64(ptr)`. If err (tag 1): `dealloc(load64(ptr+16))` (error string). Then `dealloc(ptr)` (24-byte struct).

### Scope Promotion

`scope_exit(keep)` behavior change:

The tracking list is a singly-linked list from `rc_alloc_head` to the watermark (parent scope's head). The walk frees nodes and advances `rc_alloc_head`. For promotion:

1. Walk tracking list from `rc_alloc_head` toward watermark
2. For each node where `ptr != keep`: call `scope_free(ptr, tag)`, dealloc the node, advance
3. For the node where `ptr == keep`: **do not free it or the node**. Instead, set the node's `next` pointer to `watermark` (the parent scope's alloc head). This effectively splices the kept node into the parent scope's tracking list. Since we're walking and freeing all other nodes between the kept node and watermark, we need to collect the kept node separately: save it aside during the walk, then after the walk completes, set `kept_node.next = watermark` and `rc_alloc_head = kept_node`.
4. If `keep` is not found (e.g., returning a stack value like an integer), `rc_alloc_head = watermark` (everything freed).

This ensures return values are eventually freed when the caller's scope exits.

### Codegen Emission Points

All emission guarded by `cg_is_runtime_flag == 0` (skip for runtime modules).

**After heap-creating IR instructions (direct allocation):**

| IR Opcode | Scope Tag | Notes |
|-----------|-----------|-------|
| `IR_ALLOC` | `SCOPE_TAG_RAW` | Update existing call, add tag arg |
| `IR_ARRAY_CREATE` | `SCOPE_TAG_ARRAY` | Inlined malloc in codegen |
| `IR_STRUCT_ALLOC` | `SCOPE_TAG_RAW` | Inlined malloc |
| `IR_ENUM_ALLOC` | `SCOPE_TAG_RAW` | Inlined malloc |
| `IR_STRING_CONCAT` | `SCOPE_TAG_STRING` | Inlined malloc |
| `IR_STRING_SUBSTRING` | `SCOPE_TAG_STRING` | Inlined malloc |
| `IR_INT_TO_STRING` | `SCOPE_TAG_STRING` | Inlined malloc(21) |
| `IR_FLOAT_TO_STRING` | `SCOPE_TAG_STRING` | Inlined malloc(32) |

**After heap-creating IR instructions (runtime calls):**

| IR Opcode | Scope Tag | Notes |
|-----------|-----------|-------|
| `IR_MAP_CREATE` | `SCOPE_TAG_MAP` | `sans_map_create()` |
| `IR_JSON_OBJECT`, `IR_JSON_ARRAY`, `IR_JSON_STRING`, `IR_JSON_INT`, `IR_JSON_BOOL`, `IR_JSON_NULL` | `SCOPE_TAG_JSON` | `sans_json_*()` |
| `IR_JSON_PARSE` | `SCOPE_TAG_JSON` | `sans_json_parse()` |
| `IR_RESULT_OK`, `IR_RESULT_ERR` | `SCOPE_TAG_RESULT` | `sans_result_ok/err()` |
| `IR_STRING_TRIM` | `SCOPE_TAG_STRING` | `sans_string_trim()` |
| `IR_STRING_REPLACE` | `SCOPE_TAG_STRING` | `sans_string_replace()` |
| `IR_STRING_SPLIT` | `SCOPE_TAG_ARRAY` | Returns new array |
| `IR_ARRAY_MAP` | `SCOPE_TAG_ARRAY` | `sans_array_map()` |
| `IR_ARRAY_FILTER` | `SCOPE_TAG_ARRAY` | `sans_array_filter()` |
| `IR_ARRAY_ENUMERATE` | `SCOPE_TAG_ARRAY` | `sans_array_enumerate()` |
| `IR_ARRAY_ZIP` | `SCOPE_TAG_ARRAY` | `sans_array_zip()` |
| `IR_MAP_KEYS` | `SCOPE_TAG_ARRAY` | `sans_map_keys()` |
| `IR_MAP_VALS` | `SCOPE_TAG_ARRAY` | `sans_map_vals()` |
| `IR_JSON_STRINGIFY` | `SCOPE_TAG_STRING` | `sans_json_stringify()` |
| `IR_URL_DECODE` | `SCOPE_TAG_STRING` | `sans_url_decode()` |
| `IR_PATH_SEGMENT` | `SCOPE_TAG_STRING` | `sans_path_segment()` |
| `IR_FILE_READ` | `SCOPE_TAG_STRING` | Inlined malloc in codegen |
| `IR_JSON_GET_STRING` | `SCOPE_TAG_STRING` | `sans_json_get_string()` |
| `IR_JSON_TYPE_OF` | `SCOPE_TAG_STRING` | `sans_json_type_of()` |
| `IR_GZIP_COMPRESS` | `SCOPE_TAG_RAW` | Inlined malloc for result struct |
| `IR_ARGS` | `SCOPE_TAG_ARRAY` | `__sans_args()` returns new array |

**After user function calls returning heap types:**

Look up callee name in `fn_ret_types` map (passed from IR to codegen). If return IRTY is a heap type, emit `scope_track(ret, irty_to_scope_tag(irty))`.

IRTY → scope tag mapping:
- `IRTY_ARRAY` → `SCOPE_TAG_ARRAY`
- `IRTY_MAP` → `SCOPE_TAG_MAP`
- `IRTY_JSON` → `SCOPE_TAG_JSON`
- `IRTY_RESULT` → `SCOPE_TAG_RESULT`
- `IRTY_STR` → `SCOPE_TAG_STRING`
- `IRTY_STRUCT`, `IRTY_ENUM` → `SCOPE_TAG_RAW`
- All others → no tracking

**String interpolation note:** String interpolation (`"hello {name}"`) compiles to a chain of `IR_STRING_CONCAT` operations. Each intermediate concat result is a fresh malloc that gets scope-tracked. All but the final result will be freed on scope exit. This is correct behavior — intermediate strings are temporary.

### IR Changes

**Expose `fn_ret_types` map.** The IR module already builds this during lowering. Store it on the IR module struct so codegen can access it via `irm_fn_ret_types(module)`.

No changes to IR instruction layouts. No new IR opcodes.

### Main.sans Changes

Pass `fn_ret_types` from IR lowering result through to `compile_to_ll_impl`. For runtime modules, pass an empty map (they don't get scope instrumentation anyway).

## Files Changed

1. **`compiler/constants.sans`** — Add `SCOPE_TAG_*` constants (~6 lines)
2. **`runtime/rc.sans`** — Modify `scope_track` to 2 args, add `scope_free` destructor, modify `scope_exit` for promotion (~70 lines)
3. **`compiler/ir.sans`** — Expose `fn_ret_types` on IR module (~10 lines)
4. **`compiler/codegen.sans`** — Type-tagged scope_track emission after all heap instructions + function calls (~100 lines)
5. **`compiler/main.sans`** — Thread `fn_ret_types` through to codegen (~5 lines)
6. **`tests/fixtures/`** — `scope_typed.sans`, `scope_nested_calls.sans`, `scope_string.sans`

## Known Limitations

- **Nested heap values in containers**: An array holding other arrays — inner arrays not recursively freed by outer array's destructor. JSON arrays/objects similarly don't recursively free their elements. Requires container-aware destruction (future work).
- **Global pointer escape**: Storing a heap pointer in a global variable, then exiting the creating scope → use-after-free. Rare in AI code.
- **Closure capture escape**: Closures capturing heap pointers may outlive the creating scope.
- **Single keep value**: `scope_exit(keep)` promotes one value. Tuples returning multiple heap values only promote the tuple itself (which is `SCOPE_TAG_RAW`), not its heap-typed elements.
- **Function pointer calls**: Indirect calls via `IR_FCALL`/`IR_FCALL2`/`IR_FCALL3` cannot be looked up in `fn_ret_types` — return values from function-pointer calls are not scope-tracked.
- **Mixed-return runtime functions**: Some runtime functions (`sans_http_request_header`, `sans_http_request_cookie`, `sans_http_request_query`, `sans_http_request_path_only`, `sans_http_request_form`, `sans_result_error`) may return either a fresh allocation or an interior pointer depending on runtime conditions. These are NOT scope-tracked to avoid double-free of interior pointers, which means their fresh allocations will leak.
- **Thread safety**: The globals `rc_alloc_head` and `rc_scope_head` are not thread-safe. Concurrent `spawn` calls will corrupt the tracking lists. Thread-local storage is future work.

## Bootstrap Safety

All changes compile under the current `/tmp/sans_stage2` bootstrap binary. No IR instruction layout changes. The `fn_ret_types` map is an additive parameter. `scope_track` signature change (1 arg → 2 args) only affects codegen emission and rc.sans — the bootstrap compiler doesn't emit scope calls.

## Test Plan

- `scope_typed.sans` — Create arrays, maps, JSON, Result in a function; verify no crash on exit (allocations freed)
- `scope_nested_calls.sans` — Function returns array, caller uses it, verify value survives (promotion works)
- `scope_string.sans` — String concatenation in loops, verify no unbounded growth
- Full E2E test suite — all existing 72 tests still pass
- Self-hosting — compiler builds itself with scope GC enabled
