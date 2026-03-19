# Limitations Fixes — Design Spec (v0.5.3)

Fix 8 known limitations, remove 4 as "by design". All remaining documented limitations resolved.

## Removed from Limitations (by design, not bugs)

- **Float stored as i64 via bitcast** — deliberate design choice, all values are i64 in register map
- **Typeck relaxed for bootstrap** — deliberate trade-off for self-hosting
- **Compiler must use bootstrap binary** — scope GC architectural constraint, not user-facing
- **rc globals not thread-safe** — runtime architecture, deferred to v0.6+

## Fix 1: Default Params — Allow Negative Literals

**Current:** Only positive literals (0, 1.0, "hi", true).
**Fix:** Also allow unary negation on numeric literals: `f(x:I=-1)`, `g(y:F=-3.14)`.

**Implementation:**
- **Parser:** In `parse_params`, when parsing default after `=`, if next token is `-` followed by a numeric literal, parse it as a negated literal (same as `parse_pattern` handles negative ints).
- **Typeck:** No changes needed — negated literals already type-check correctly.

## Fix 2: For-Loop Destructuring — N-Element Tuples

**Current:** Hardcoded 2-element `(k v)`.
**Fix:** Support N-element destructuring: `for (a b c) in arr { ... }`.

**Implementation:**
- **Parser:** Change `make_st_for_in_destr` to store an array of variable names instead of two fixed slots. Layout: `[ST_FOR_IN_DESTR, vars_array_ptr, iterable_expr, body_stmts_ptr]`.
- **Typeck:** Validate N matches tuple arity (currently hardcoded String+Int — generalize to check tuple inner types).
- **IR:** In `lower_for_in_destr`, loop `0..N` extracting `ir_field_load(reg, elem, i, N)` for each binding.

## Fix 3: Recursive Freeing of Nested Heap Values

**Current:** `scope_free()` frees containers but not their elements.
**Fix:** When freeing arrays/maps, iterate elements and free any that are scope-tracked.

**Implementation:**
- **Runtime (`rc.sans`):** In `scope_free()`, for SCOPE_TAG_ARRAY: iterate array elements, for each element check if it exists in the scope tracking list. If found, free it and unlink from the list. Same for SCOPE_TAG_MAP values.
- Add `sans_scope_free_recursive(ptr, tag)` helper that handles the element iteration.
- O(n*m) worst case but only runs at function return on tracked allocations.
- Prevents double-free by removing entries from tracking list before freeing.

## Fix 4: Global Pointer Escape

**Current:** Heap pointers stored in globals are freed by scope_exit.
**Fix:** Untrack heap values when stored into globals.

**Implementation:**
- **Runtime (`rc.sans`):** Add `sans_scope_untrack(ptr)` — walks the scope linked list, finds matching pointer, unlinks it. No-op if not found.
- **Codegen:** After emitting `IR_GLOBAL_STORE`, if the value is a pointer type (check reg_types or always emit for safety), emit `call void @sans_scope_untrack(i64 %val)`.
- **IR:** No changes — codegen handles this at emission time.
- Declare `sans_scope_untrack` as external in the LLVM preamble.

## Fix 5: IR Type Tracking Across Functions

**Current:** Opaque types (JsonValue, HttpResponse, etc.) lose IRTY info when passed as function parameters.
**Fix:** Complete the type-name-to-IRTY mapping tables.

**Implementation:**
- **IR (`ir.sans`):** In `lower_function_body` parameter setup (lines 3411-3460), add missing opaque type mappings:
  - `"JsonValue"` → `IRTY_JSON`
  - `"HttpResponse"` → `IRTY_HTTP_RESP`
  - `"HttpRequest"` → `IRTY_HTTP_REQ`
  - `"HttpServer"` → `IRTY_HTTP_SERVER`
  - `"JoinHandle"` → `IRTY_JOIN_HANDLE`
  - `"Sender"` → `IRTY_SENDER`
  - `"Receiver"` → `IRTY_RECEIVER`
  - `"Mutex"` → `IRTY_MUTEX`
  - `"Result"` / `"R"` → `IRTY_RESULT`
  - `"Tuple"` → `IRTY_TUPLE`
- Also update `ir_type_for_return` if any of these are missing there.
- This is a targeted completion of existing mapping tables, not an architectural change.

## Fix 6: Cross-Module Capturing Lambdas

**Current:** Capture context stored as IR metadata, lost across module boundaries.
**Fix:** Heap-allocated closure objects that bundle function pointer + captures.

**Closure object layout:**
```
offset 0:  function pointer (i64, ptr to lifted function)
offset 8:  num_captures (i64)
offset 16+: capture values (i64 each)
```

**Implementation:**
- **IR — closure creation:** When lifting a lambda with captures, instead of storing capture info in `closure_info` map only, also emit:
  1. `ir_alloc(closure_reg, 16 + num_captures * 8)` — allocate closure object
  2. `ir_store64(closure_reg + 0, fn_ptr)` — store function pointer
  3. `ir_store64(closure_reg + 8, num_captures)` — store capture count
  4. For each capture: `ir_store64(closure_reg + 16 + i*8, capture_reg)` — store capture values
  5. The local register holds the closure pointer

- **IR — closure call:** When calling a value that is a closure (detected via `IRTY_FN` or closure_info):
  1. Load function pointer from `closure_reg + 0`
  2. Load num_captures from `closure_reg + 8`
  3. Load each capture from `closure_reg + 16 + i*8`
  4. Build arg list: captures first, then explicit args
  5. Emit indirect call via function pointer

- **Codegen:** Indirect calls use `call i64 %fptr(i64 %arg0, i64 %arg1, ...)` with the function pointer loaded from the closure object. Need to emit the correct LLVM function type for the indirect call based on total arg count.

- **Backwards compatibility:** Non-capturing lambdas can still use the direct `IR_FN_REF` path (no closure object needed). Only capturing lambdas get the heap closure.

- **Scope tracking:** Closure objects are heap-allocated, so track with `emit_scope_track` for automatic cleanup.

## Fix 7: Generic Methods on Generic Structs

**Current:** Generic structs support field access and construction only.
**Fix:** Allow `impl` blocks with type params that monomorphize on use.

```sans
struct Stack<T> { items:[T] }
impl Stack<T> {
  push(self:Stack<T> val:T) { self.items.push(val) }
  len(self:Stack<T>) I = self.items.len()
}
s = Stack<I>{items:[1 2 3]}
s.push(4)
```

**Implementation:**
- **Parser:** `parse_impl_block` already parses `impl Name { methods }`. Extend to parse `impl Name<T> { methods }` — store type params on the impl block AST node.
- **Typeck:** When registering methods from `impl Stack<T>`, store as generic method templates keyed by base name (`Stack`). When a method is called on `Stack$$I`, look up the template, substitute `T→I` in parameter and return types, cache the monomorphized method signature as `Stack$$I.method_name`.
- **IR:** Method dispatch already looks up by `struct_name.method_name`. With mangled names like `Stack$$I`, the lookup key becomes `Stack$$I.push`. The monomorphized method body is lowered like any other method.
- **Codegen:** No changes — monomorphized methods are regular functions.

## Fix 8: Nested Generics

**Current:** `Box<Pair<I S>>` not supported.
**Fix:** Recursive type argument parsing and monomorphization.

```sans
struct Box<T> { value:T }
struct Pair<A B> { first:A second:B }
b = Box<Pair<I S>>{value: Pair<I S>{first:1 second:"hi"}}
```

**Implementation:**
- **Parser:** Update `parse_type_name` to handle nested `<...>` by tracking angle bracket depth. When parsing type args, if a type arg itself contains `<`, recursively parse it as a generic type. The mangling for `Box<Pair<I S>>` produces `Box$$Pair$$I$$S` (all `$$` separated, flattened).
- **Parser (struct literals):** When parsing `Box<Pair<I S>>{...}`, the type arg parser must handle `Pair<I S>` as a single type arg (not stop at the first `>`). Use bracket depth counting.
- **Typeck:** In `instantiate_generic_struct`, when a field type resolves to a mangled generic name (e.g. `Pair$$I$$S`), check if it's already instantiated. If not, recursively instantiate it before continuing.
- **IR/Codegen:** No changes — nested generic structs are just structs with longer mangled names.

## Implementation Order

1. **Fix 1: Negative literal defaults** — trivial, self-contained
2. **Fix 5: IR type mapping completion** — targeted, low risk
3. **Fix 2: N-element destructuring** — moderate, self-contained
4. **Fix 4: Global pointer untracking** — moderate, runtime + codegen
5. **Fix 3: Recursive scope freeing** — moderate, runtime only
6. **Fix 7: Generic methods** — moderate, parser + typeck
7. **Fix 8: Nested generics** — moderate, parser + typeck
8. **Fix 6: Cross-module lambdas** — complex, touches IR + codegen calling convention

## Testing Strategy

Each fix gets at least one E2E test fixture:

1. `default_params_negative.sans` — `f(x:I=-1)` usage
2. `for_destructure_triple.sans` — 3-element tuple destructuring
3. `scope_nested_array.sans` — array of arrays, verify no leak/crash
4. `scope_global_escape.sans` — store heap value in global, use after function returns
5. `ir_opaque_param.sans` — pass JsonValue/Result through user functions
6. `lambda_cross_module.sans` — capturing lambda passed to another module
7. `generic_struct_method.sans` — methods on `Stack<I>`
8. `generic_nested.sans` — `Box<Pair<I S>>` construction + access

## Documentation Updates

After implementation, update all docs to remove the fixed limitations. The Known Limitations section should then only contain items deferred by design (bootstrap, thread safety) or be empty if those are also removed from the list.
