# Limitations Fixes Implementation Plan (v0.5.3)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 8 known limitations: negative defaults, N-tuple destructuring, recursive scope GC, global escape untracking, IR type mapping, cross-module lambdas, generic methods, nested generics. Remove 4 limitations as by-design.

**Architecture:** Each fix follows the Sans compiler pipeline (parser → typeck → IR → codegen → runtime). Fixes are independent and ordered by complexity. The compiler is self-hosted — all changes in `.sans` files. Bootstrap binary at `/tmp/sans040` required for building.

**Tech Stack:** Sans compiler, Sans runtime (rc.sans), LLVM 17

**Spec:** `docs/superpowers/specs/2026-03-19-limitations-fixes-design.md`

---

## Chunk 1: Easy Fixes (Negative Defaults + N-Tuple Destructuring)

### Task 1: Negative literal defaults

**Files:**
- Create: `tests/fixtures/default_params_negative.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/parser.sans:528-548`

- [ ] **Step 1: Write test fixture**

```sans
clamp(x:I lo:I=-100 hi:I=100) I {
  if x < lo { lo } else { if x > hi { hi } else { x } }
}

main() {
  a = clamp(50)
  b = clamp(-200)
  a + b + 100
}
```

Expected exit: 50 + (-100) + 100 = 50.

Write to `tests/fixtures/default_params_negative.sans`.

- [ ] **Step 2: Register test**

Add to `tests/run_tests.sh`:
```bash
run_test "default_params_negative"    "$REPO_ROOT/tests/fixtures/default_params_negative.sans"    50
```

- [ ] **Step 3: Update parse_params to handle negative defaults**

In `compiler/parser.sans`, the `parse_params` function (lines 528-548) currently calls `parse_atom(p)` for defaults. `parse_atom` doesn't handle prefix `-`.

Change the default parsing to check for `TK_MINUS` before calling `parse_atom`, mirroring how `parse_pattern` handles negative ints (lines 1398-1405):

```sans
def_expr = if p_at(p, TK_EQ) == 1 {
  p_advance(p)
  if p_at(p, TK_MINUS) == 1 {
    p_advance(p)
    t = p_advance(p)
    val = tok_val(t)
    // Check if float or int
    if p_prev_kind(p) == TK_FLOAT_LIT {
      make_float_lit(0 - val)
    } else {
      make_int_lit(0 - val)
    }
  } else {
    parse_atom(p)
  }
} else { 0 }
```

NOTE: Check how `parse_pattern` determines token kind. It may use `tok_kind` or check the token type. The key is: consume `-`, consume the next token (int or float literal), negate the value, create the appropriate literal node. Look at the existing negative pattern code at lines 1398-1405 and mirror it.

- [ ] **Step 4: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add compiler/parser.sans tests/fixtures/default_params_negative.sans tests/run_tests.sh
git commit -m "feat: allow negative literals as default parameter values"
```

### Task 2: N-element for-loop destructuring

**Files:**
- Create: `tests/fixtures/for_destructure_triple.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/parser.sans:323-331` (make_st_for_in_destr)
- Modify: `compiler/parser.sans:745-755` (for parsing)
- Modify: `compiler/typeck.sans` (ST_FOR_IN_DESTR handling)
- Modify: `compiler/ir.sans` (lower_for_in_destr)

- [ ] **Step 1: Write test fixture**

```sans
main() {
  a = [(1 2 3) (4 5 6)]
  total := 0
  for (x y z) in a {
    total += x + y + z
  }
  total
}
```

Expected exit: (1+2+3) + (4+5+6) = 21.

Write to `tests/fixtures/for_destructure_triple.sans`.

- [ ] **Step 2: Register test**

```bash
run_test "for_destructure_triple"     "$REPO_ROOT/tests/fixtures/for_destructure_triple.sans"     21
```

- [ ] **Step 3: Change AST node to use variable array**

In `compiler/parser.sans`, replace `make_st_for_in_destr` (lines 323-331) which currently stores two fixed vars. Change to store an array of variable names:

```sans
make_st_for_in_destr(vars:I iterable_expr:I body_stmts:I) I {
  n = alloc(32)
  store64(n, ST_FOR_IN_DESTR)
  store64(n + 8, ptr(vars))
  store64(n + 16, iterable_expr)
  store64(n + 24, ptr(body_stmts))
  n
}
```

- [ ] **Step 4: Update parser to collect N variables**

In the for-loop parsing (lines 745-755), change from parsing exactly 2 idents to parsing N idents into an array:

```sans
if p_at(p, TK_LPAREN) == 1 {
  p_advance(p)
  vars = array<I>()
  while p_at(p, TK_RPAREN) == 0 && p_at(p, TK_EOF) == 0 {
    vars.push(p_expect_ident(p))
  }
  p_expect(p, TK_RPAREN)
  p_expect(p, TK_IN)
  iterable = parse_expr(p, 0)
  p_expect(p, TK_LBRACE)
  body = parse_body(p)
  p_expect(p, TK_RBRACE)
  return make_st_for_in_destr(vars, iterable, body)
}
```

- [ ] **Step 5: Update typeck for N-element destructuring**

In `compiler/typeck.sans`, the ST_FOR_IN_DESTR handler currently reads `var1` at offset 8 and `var2` at offset 16. Update to read the vars array:

```sans
if tag == ST_FOR_IN_DESTR {
  vars_arr = load64(stmt + 8)
  vars_p = load64(vars_arr)
  num_vars = load64(vars_arr + 8)
  iterable = load64(stmt + 16)
  body_arr = load64(stmt + 24)
  iter_ty = check_expr(iterable, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if type_tag(iter_ty) != TY_ARRAY { tc_error("for-in destructuring requires Array") }
  push_scope(locals)
  inner_scope = locals[locals.len() - 1]
  // Bind all variables — use INT type for all elements for now
  vi := 0
  while vi < num_vars {
    vname = load64(vars_p + vi * 8)
    mset(inner_scope, vname, make_type(TY_INT))
    vi += 1
  }
  body_len = if body_arr != 0 { load64(body_arr + 8) } else { 0 }
  body_p = if body_arr != 0 { load64(body_arr) } else { 0 }
  check_stmts(body_p, body_len, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  pop_scope(locals)
  return make_type(TY_VOID)
}
```

NOTE: The first variable in map entries destructuring should still be TY_STRING. Check if the existing code handles this. For map entries `(k v)`, k should be String and v should be Int. For generic tuple arrays, all elements might be Int. The safest approach for now: check if num_vars == 2 and iterable comes from `.entries()` — if so, use String+Int. Otherwise use INT for all. Or just use INT for all and let the runtime handle it (values are all i64 anyway).

- [ ] **Step 6: Update IR lower_for_in_destr for N elements**

In `compiler/ir.sans`, the `lower_for_in_destr` function (lines 1543-1627) currently extracts exactly 2 fields. Update to loop over N fields:

Read the vars array from the new AST layout:
```sans
vars_arr = load64(stmt + 8)
vars_p = load64(vars_arr)
num_vars = load64(vars_arr + 8)
iterable = load64(stmt + 16)
body_arr = load64(stmt + 24)
```

Then in the body section, instead of hardcoded 2 field loads, loop:
```sans
vi := 0
while vi < num_vars {
  vname = load64(vars_p + vi * 8)
  field_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_field_load(field_reg, elem_reg, vi, num_vars))
  ctx_set_reg_type(ctx, field_reg, IRTY_INT)
  ctx_set_local(ctx, vname, field_reg)
  vi += 1
  0
}
```

- [ ] **Step 7: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

Verify both `for_destructure_map` (existing, exit 6) and `for_destructure_triple` (new, exit 21) pass.

- [ ] **Step 8: Commit**

```bash
git add compiler/parser.sans compiler/typeck.sans compiler/ir.sans tests/fixtures/for_destructure_triple.sans tests/run_tests.sh
git commit -m "feat: N-element for-loop destructuring — for (a b c) in arr"
```

---

## Chunk 2: IR Type Mapping Completion

### Task 3: Complete parameter type mappings

**Files:**
- Create: `tests/fixtures/ir_opaque_param.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/ir.sans:3595-3640`

- [ ] **Step 1: Write test fixture**

Create a test that passes a Result through a user function:

```sans
check_result(r:R<I>) I {
  if r.is_ok() { r! } else { 0 }
}

main() {
  r = ok(42)
  check_result(r)
}
```

Expected exit: 42.

Write to `tests/fixtures/ir_opaque_param.sans`.

- [ ] **Step 2: Register test**

```bash
run_test "ir_opaque_param"            "$REPO_ROOT/tests/fixtures/ir_opaque_param.sans"            42
```

- [ ] **Step 3: Add missing type mappings in lower_function_body**

In `compiler/ir.sans`, the parameter type setup in `lower_function_body` (around lines 3595-3640) is missing `JoinHandle`. Add it:

```sans
else if ptype == "JoinHandle" { ctx_set_reg_type(ctx, arg_reg, IRTY_JOIN_HANDLE) }
```

Add this after the existing `HttpServer`/`HS` checks.

Also verify `ir_type_for_return` (lines 3488-3522) has all types. Based on research, it's mostly complete. The only gap is `JoinHandle` in parameter mapping.

- [ ] **Step 4: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add compiler/ir.sans tests/fixtures/ir_opaque_param.sans tests/run_tests.sh
git commit -m "feat: complete IR type mapping for all opaque types (JoinHandle)"
```

---

## Chunk 3: Scope GC Fixes (Recursive Freeing + Global Escape)

### Task 4: Add sans_scope_untrack to runtime

**Files:**
- Modify: `runtime/rc.sans`

- [ ] **Step 1: Add sans_scope_untrack function**

In `runtime/rc.sans`, add after `sans_scope_track` (line 41):

```sans
sans_scope_untrack(ptr:I) I {
  // Walk tracking list, remove node matching ptr
  prev := 0
  node := rc_alloc_head
  while node != 0 {
    node_ptr = load64(node)
    next = load64(node + 16)
    if node_ptr == ptr {
      // Found — unlink from list
      if prev == 0 {
        rc_alloc_head = next
      } else {
        store64(prev + 16, next)
      }
      dealloc(node)
      return 0
    }
    prev = node
    node = next
  }
  0
}
```

- [ ] **Step 2: Commit runtime change**

```bash
git add runtime/rc.sans
git commit -m "feat(runtime): add sans_scope_untrack for global pointer escape"
```

### Task 5: Emit scope_untrack on global store

**Files:**
- Create: `tests/fixtures/scope_global_escape.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/codegen.sans:1367-1373` (IR_GLOBAL_STORE)
- Modify: `compiler/codegen.sans` (extern declarations, around line 584)

- [ ] **Step 1: Write test fixture**

```sans
g result = 0

store_global() {
  a = [10 20 30]
  result = a
}

main() {
  store_global()
  result.get(1)
}
```

Expected exit: 20 (array survives function return via global).

NOTE: This test may already work if scope GC doesn't track global stores. Check first. If it already passes, the fix is still needed for correctness but the test verifies the existing behavior.

- [ ] **Step 2: Register test**

```bash
run_test "scope_global_escape"        "$REPO_ROOT/tests/fixtures/scope_global_escape.sans"        20
```

- [ ] **Step 3: Declare sans_scope_untrack in codegen**

In `compiler/codegen.sans`, find the extern declarations section (around line 584 where `sans_scope_track` is declared). Add:

```
declare i64 @sans_scope_untrack(i64)
```

- [ ] **Step 4: Emit untrack call on IR_GLOBAL_STORE**

In `compiler/codegen.sans`, the IR_GLOBAL_STORE handler (lines 1367-1373) currently just emits a store. After the store, emit a call to untrack the value:

```sans
if op == IR_GLOBAL_STORE {
  gname = dest
  val = cg_get_val(cg, ir_field(inst, 16))
  emit(cg, "  store i64 " + val + ", ptr @" + gname)
  // Untrack the value from scope GC so globals survive scope_exit
  if cg_is_runtime_flag == 0 {
    ut = cg_fresh_reg(cg)
    emit(cg, "  " + ut + " = call i64 @sans_scope_untrack(i64 " + val + ")")
  }
  return 0
}
```

- [ ] **Step 5: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add compiler/codegen.sans tests/fixtures/scope_global_escape.sans tests/run_tests.sh
git commit -m "feat: untrack heap values on global store to prevent premature freeing"
```

### Task 6: Recursive scope freeing for nested containers

**Files:**
- Create: `tests/fixtures/scope_nested_array.sans`
- Modify: `tests/run_tests.sh`
- Modify: `runtime/rc.sans` (scope_free function)

- [ ] **Step 1: Write test fixture**

```sans
make_nested() [I] {
  inner1 = [1 2 3]
  inner2 = [4 5 6]
  outer = [inner1 inner2]
  outer
}

main() {
  a = make_nested()
  10
}
```

Expected exit: 10 (doesn't crash from use-after-free or double-free).

- [ ] **Step 2: Register test**

```bash
run_test "scope_nested_array"         "$REPO_ROOT/tests/fixtures/scope_nested_array.sans"         10
```

- [ ] **Step 3: Update scope_free for recursive array freeing**

In `runtime/rc.sans`, the `scope_free` function for tag==1 (SCOPE_TAG_ARRAY) currently does:
```sans
dealloc(load64(ptr))   // free data buffer
dealloc(ptr)           // free array struct
```

Update to iterate elements and untrack any that are in the scope list:

```sans
} else if tag == 1 {
  // Array: iterate elements and untrack inner heap values before freeing
  data = load64(ptr)
  len = load64(ptr + 8)
  ei := 0
  while ei < len {
    elem = load64(data + ei * 8)
    if elem != 0 { sans_scope_untrack(elem) }
    ei += 1
    0
  }
  dealloc(data)
  dealloc(ptr)
```

NOTE: This calls `sans_scope_untrack` which removes the element from the tracking list, preventing double-free when scope_exit later encounters the same pointer. The inner arrays will be freed by scope_exit when it reaches their tracking nodes (or they've already been untracked and won't be freed again). Actually — the issue is that `scope_exit` walks from head to watermark. If inner arrays were allocated BEFORE the outer array, they'd have tracking nodes that appear AFTER the outer array in the list. So untracking is the right approach: remove inner pointers from the list so they don't get double-freed, then explicitly free them here.

Actually, the safer approach: for each element that looks like a heap pointer, call `scope_free` on it recursively with the appropriate tag. But we don't know the element's tag. The simplest safe fix: just `sans_scope_untrack` each element. This prevents double-free. The actual freeing of inner arrays happens when their own tracking nodes are reached during scope_exit.

Wait — that's the opposite. If inner arrays are tracked and we DON'T untrack them, scope_exit will free them. The problem described is that scope_exit frees the outer array's data buffer (which contains pointers to inner arrays), and those inner arrays are ALSO freed individually by their own tracking nodes. That should actually work fine — each tracked allocation is freed exactly once.

Let me re-read the limitation: "Nested heap values in containers (array of arrays) — inner values not recursively freed." This means inner values are NOT freed. Let me reconsider...

The issue is likely: the outer array is tracked, but when scope_exit frees the outer array, it frees the data buffer (`dealloc(load64(ptr))`). The INNER arrays are also tracked separately (they have their own tracking nodes), so they SHOULD be freed. Unless the inner arrays were created in a different scope...

The real scenario: a function creates inner arrays, puts them in an outer array, and returns the outer array. The return value (outer array) is promoted to the caller's scope. But the inner arrays are still freed by scope_exit in the callee because they're NOT promoted — only the outer array is promoted (via the `keep` parameter).

**THIS IS THE REAL FIX:** In `sans_scope_exit`, when promoting a return value, also promote any heap pointers contained within it. For an array return value, iterate its elements and promote any that are tracked.

Updated approach for scope_exit:
```sans
// After re-linking kept_node, also promote inner values
if kept_node != 0 {
  kept_ptr = load64(kept_node)
  kept_tag = load64(kept_node + 8)
  if kept_tag == 1 {
    // Array — promote inner elements
    data = load64(kept_ptr)
    len = load64(kept_ptr + 8)
    ei := 0
    while ei < len {
      elem = load64(data + ei * 8)
      // Check if elem is tracked in current scope (between head and watermark)
      // If so, re-link to caller's scope
      promote_if_tracked(elem)
      ei += 1
      0
    }
  }
}
```

Actually this is getting complex. Let me simplify. The implementer should:

1. Read scope_exit carefully
2. After the `if kept_node != 0` block that re-links the kept node, add a loop that checks if the kept value is an array (tag==1) and promotes its elements too
3. "Promoting" means: walk from head to watermark, find nodes matching element pointers, re-link them past the watermark instead of freeing them

This is the key insight: **promotion needs to be recursive for containers.**

- [ ] **Step 4: Update sans_scope_exit to recursively promote container contents**

In `runtime/rc.sans`, in `sans_scope_exit` (lines 95-123), the current promotion logic:
1. Walks tracking list from head to watermark
2. Frees everything except the `keep` pointer
3. Re-links the `keep` node at the new head

Update to also promote elements of arrays/maps that are being kept:

After the main walk loop but before re-linking kept_node, collect additional pointers to promote:

```sans
// Build set of pointers to also promote (elements of kept containers)
promote_set = 0  // Will be populated if keep is a container
if kept_node != 0 {
  kept_tag = load64(kept_node + 8)
  if kept_tag == 1 {
    // Array return value — collect inner element pointers
    arr_data = load64(keep)
    arr_len = load64(keep + 8)
    // Create simple linked list of extra keep pointers
    // ... iterate elements, mark as "also keep" ...
  }
}
```

The exact implementation is complex. The implementer should study the existing scope_exit flow and extend the `keep` mechanism to also preserve inner pointers. The simplest approach: change the walk loop to check `if ptr == keep || is_inner_of(ptr, keep, kept_tag)` instead of just `if ptr == keep`.

- [ ] **Step 5: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add runtime/rc.sans tests/fixtures/scope_nested_array.sans tests/run_tests.sh
git commit -m "feat(runtime): recursively promote container contents in scope_exit"
```

---

## Chunk 4: Generic Methods on Generic Structs

### Task 7: Generic impl blocks

**Files:**
- Create: `tests/fixtures/generic_struct_method.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/parser.sans:1712-1733` (parse_impl_block)
- Modify: `compiler/typeck.sans:2833-2931` (impl collection)
- Modify: `compiler/typeck.sans:2268-2280` (method dispatch)
- Modify: `compiler/ir.sans` (method registration + dispatch)

- [ ] **Step 1: Write test fixture**

```sans
struct Box<T> {
  value:T
}

impl Box<T> {
  get(self:Box<T>) I = self.value
}

main() {
  b = Box<I>{value:42}
  b.get()
}
```

Expected exit: 42.

Write to `tests/fixtures/generic_struct_method.sans`.

- [ ] **Step 2: Register test**

```bash
run_test "generic_struct_method"      "$REPO_ROOT/tests/fixtures/generic_struct_method.sans"      42
```

- [ ] **Step 3: Parse type params on impl blocks**

In `compiler/parser.sans`, `parse_impl_block` (lines 1712-1733) currently parses `impl Name { methods }`. After parsing the target type name, check for `<...>` and parse type params:

```sans
// After parsing target_type name...
impl_type_params = array<I>()
if p_at(p, TK_LT) == 1 {
  p_advance(p)
  while p_at(p, TK_GT) == 0 && p_at(p, TK_EOF) == 0 {
    impl_type_params.push(p_expect_ident(p))
    if p_at(p, TK_COMMA) == 1 { p_advance(p) }
  }
  p_expect(p, TK_GT)
}
```

Store the type params on the impl block AST. Use offset 24 (currently reserved/0):
```sans
store64(impl_block + 24, if impl_type_params.len() > 0 { ptr(impl_type_params) } else { 0 })
```

- [ ] **Step 4: Register generic methods in typeck**

In `compiler/typeck.sans`, the impl collection pass (lines 2833-2931) registers methods with key `"Type.method"`. For generic impls (`impl Box<T>`), store as a template:

When `impl_type_params != 0`:
- Store the method signatures as generic templates keyed by base name
- When a method is called on `Box$$I`, look up the template for `Box`, substitute type params, and cache

The approach: when registering methods for `impl Box<T>`, store them under the base name `Box` in a `generic_methods` map. When method dispatch encounters `Box$$I.get`, parse the base name (`Box`), look up generic methods, monomorphize by substituting `T→I`.

Add a `generic_methods` map alongside `methods_map`. Key: `"Box.get"` → value: method template (unresolved param types with type param names).

During method dispatch (lines 2268-2280), if `method_key` not found in `methods_map`:
1. Parse base name from struct name (split on `$$`)
2. Look up `base_name + "." + method` in `generic_methods`
3. Build type substitution map from the mangled struct name
4. Resolve method param/return types with substitution
5. Cache the resolved signature in `methods_map`

- [ ] **Step 5: Register generic impl methods in IR**

In `compiler/ir.sans`, method registration in `lower_full` (around lines 4040-4045) mangles method names as `Type_method`. For generic structs, the concrete mangled name should be `Box$$I_get`.

When processing impl blocks, if the impl has type params:
- For each monomorphized instantiation of the struct (found in struct_defs), create a mangled method function
- Or: lazily resolve during `lower_method_fallback` when the struct name contains `$$`

The simpler approach: In `lower_method_fallback` (lines 1954-1980), when `type_name` contains `$$`, look up the template method using the base name and lower it on demand.

- [ ] **Step 6: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 7: Commit**

```bash
git add compiler/parser.sans compiler/typeck.sans compiler/ir.sans tests/fixtures/generic_struct_method.sans tests/run_tests.sh
git commit -m "feat: generic methods on generic structs — impl Box<T> { get(self) }"
```

---

## Chunk 5: Nested Generics

### Task 8: Recursive type argument parsing and monomorphization

**Files:**
- Create: `tests/fixtures/generic_nested.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/parser.sans` (parse_type_name, struct literal parsing)
- Modify: `compiler/typeck.sans` (instantiation)

- [ ] **Step 1: Write test fixture**

```sans
struct Box<T> {
  value:T
}

struct Pair<A B> {
  first:A
  second:B
}

main() {
  inner = Pair<I I>{first:3 second:7}
  outer = Box<Pair<I I>>{value:inner}
  outer.value.first + outer.value.second
}
```

Expected exit: 10 (3 + 7).

Write to `tests/fixtures/generic_nested.sans`.

- [ ] **Step 2: Register test**

```bash
run_test "generic_nested"             "$REPO_ROOT/tests/fixtures/generic_nested.sans"             10
```

- [ ] **Step 3: Update parse_type_name for nested angle brackets**

In `compiler/parser.sans`, `parse_type_name` (lines 465-506) already handles `Name<Inner>` recursively — it calls itself for the inner type. Check if it handles `Box<Pair<I I>>`. The issue: when parsing `Pair<I I>`, the `>` closing Pair is consumed, then the parser needs to see another `>` for Box.

Current code (around line 498):
```sans
if p_at(p, TK_LT) {
  p_advance(p)
  inner = parse_type_name(p)   // recursive
  p_expect(p, TK_GT)
  return name + "<" + inner + ">"
}
```

This only parses ONE type arg. For `Pair<I I>`, it would parse `I` as the only inner type. We need to parse MULTIPLE type args separated by spaces.

Update to parse multiple type args:
```sans
if p_at(p, TK_LT) == 1 {
  p_advance(p)
  inner_parts = ""
  while p_at(p, TK_GT) == 0 && p_at(p, TK_EOF) == 0 {
    part = parse_type_name(p)  // recursive — handles nested <...>
    inner_parts = if inner_parts == "" { part } else { inner_parts + " " + part }
    if p_at(p, TK_COMMA) == 1 { p_advance(p) }
  }
  p_expect(p, TK_GT)
  return name + "<" + inner_parts + ">"
}
```

This way `Box<Pair<I I>>` parses as:
1. name="Box", sees `<`
2. Recursively parse type arg: name="Pair", sees `<`, parse "I" and "I", closing `>` → `"Pair<I I>"`
3. No more args before `>` → closing `>`
4. Returns `"Box<Pair<I I>>"`

- [ ] **Step 4: Update generic struct literal parsing for nested types**

In `compiler/parser.sans`, the generic struct literal parsing in `parse_atom` currently mangles type args by joining with `$$`. For nested generics, the type arg itself may contain `<...>`. The mangling should recursively mangle inner types:

`Box<Pair<I I>>` should mangle to `Box$$Pair$$I$$I`.

Update the mangling loop to recursively replace `<` with `$$` and strip `>`:
```sans
// Build mangled name from type args
mangled = name
tai := 0
while tai < type_args.len() {
  ta = type_args.get(tai)
  // Replace any < and > in the type arg with $$ mangling
  mangled_ta = mangle_type_arg(ta)  // "Pair<I I>" → "Pair$$I$$I"
  mangled = mangled + "$$" + mangled_ta
  tai += 1
  0
}
```

Helper function `mangle_type_arg`: replace `<` with `$$`, remove `>`, replace spaces with `$$`:
```sans
mangle_type_arg(ta:S) S {
  result := ""
  i := 0
  while i < ta.len() {
    c = ta.char_at(i)
    if c == "<" { result = result + "$$" }
    else if c == ">" { 0 }  // skip
    else if c == " " { result = result + "$$" }
    else { result = result + c }
    i += 1
    0
  }
  result
}
```

- [ ] **Step 5: Update typeck instantiation for nested generics**

In `compiler/typeck.sans`, `instantiate_generic_struct` (lines 618-680) parses mangled names by splitting on `$$`. For `Box$$Pair$$I$$I`, it needs to know that `Box` has 1 type param, so the first arg is `Pair$$I$$I` (not just `Pair`).

The key: use the template's type param count to determine how many `$$`-separated segments belong to each type arg. For `Box<T>` (1 type param), everything after `Box$$` is one type arg: `Pair$$I$$I`.

But if we have `Pair<A B>` (2 type params) with `Pair$$I$$S`, we need 2 args: `I` and `S`.

The mangling is ambiguous if we just split by `$$`. The fix: use bracket depth in mangling to keep nested types together. Instead of `Box$$Pair$$I$$I`, use `Box$$Pair.I.I` (dots for inner separators) or count-prefix.

Actually, the simplest fix: when instantiating, use the template's param count to greedily consume the right number of `$$`-separated tokens. For `Box` (1 param), consume all remaining tokens as one arg. For `Pair` (2 params), consume tokens as separate args.

The implementer will need to figure out the exact splitting logic based on the template's param count.

- [ ] **Step 6: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 7: Commit**

```bash
git add compiler/parser.sans compiler/typeck.sans tests/fixtures/generic_nested.sans tests/run_tests.sh
git commit -m "feat: nested generics — Box<Pair<I I>> with recursive monomorphization"
```

---

## Chunk 6: Cross-Module Capturing Lambdas

### Task 9: Heap-allocated closure objects

**Files:**
- Create: `tests/fixtures/lambda_cross_module/main.sans`
- Create: `tests/fixtures/lambda_cross_module/utils.sans`
- Modify: `tests/run_tests.sh`
- Modify: `compiler/ir.sans` (lower_lambda, lower_call_user)
- Modify: `compiler/codegen.sans` (IR_FCALL handling)

- [ ] **Step 1: Write cross-module lambda test**

`tests/fixtures/lambda_cross_module/main.sans`:
```sans
import "utils"

main() {
  offset = 10
  f = |x:I| I { x + offset }
  utils.apply(f 5)
}
```

`tests/fixtures/lambda_cross_module/utils.sans`:
```sans
apply(f:I arg:I) I = fcall(f arg)
```

Expected exit: 15 (5 + 10).

- [ ] **Step 2: Register test**

```bash
run_test "lambda_cross_module"        "$REPO_ROOT/tests/fixtures/lambda_cross_module/main.sans"   15
```

- [ ] **Step 3: Change closure representation to heap-allocated object**

This is the most complex change. In `compiler/ir.sans`, `lower_lambda` (lines 3011-3158):

Currently: emits `IR_FN_REF` pointing to the lifted function name, stores capture metadata in `closure_info` map (IR context only).

Change to: allocate a heap closure object and store function pointer + captures in it.

After lifting the lambda function and before returning:

```sans
// Instead of just IR_FN_REF:
if captures.len() > 0 {
  // Allocate closure object: [fn_ptr, num_captures, cap0, cap1, ...]
  closure_size = 16 + captures.len() * 8
  size_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(size_reg, closure_size))
  closure_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_alloc(closure_reg, size_reg))

  // Store function pointer
  fn_ptr_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_fn_ref(fn_ptr_reg, lambda_name))
  ctx_emit(ctx, ir_inst2(IR_STORE64, closure_reg, 0, fn_ptr_reg))  // offset 0

  // Store num_captures
  ncap_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(ncap_reg, captures.len()))
  ctx_emit(ctx, ir_inst2(IR_STORE64, closure_reg, 8, ncap_reg))  // offset 8

  // Store each capture value
  ci := 0
  while ci < captures.len() {
    cap_name = captures.get(ci)
    cap_reg = ctx_get_local(ctx, cap_name)
    ctx_emit(ctx, ir_inst2(IR_STORE64, closure_reg, 16 + ci * 8, cap_reg))
    ci += 1
    0
  }

  // Closure register holds the heap object pointer
  ctx_set_reg_type(ctx, closure_reg, IRTY_INT)
  // Still store closure_info for intra-module direct calls (optimization)
  mset(ci_map, closure_reg, cinfo)
  dest = closure_reg
} else {
  // Non-capturing lambda — use direct fn ref as before
  dest = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_fn_ref(dest, lambda_name))
  ctx_set_reg_type(ctx, dest, IRTY_INT)
}
```

NOTE: The `IR_STORE64` instruction may not exist. Check constants.sans — there is `IR_STORE64 = 133`. If it works differently (it's a memory store, not a field store), use the appropriate instruction. The implementer should check how `store64` at a specific offset works in the IR. It may need to use `IR_FIELD_STORE` or a combination of `IR_ALLOC` + manual offset stores.

- [ ] **Step 4: Update lambda call path for cross-module closures**

In `lower_call_user` (lines 864-928), the lambda call path currently looks up `closure_info` in the IR context. For cross-module calls, this info won't be there.

Add a fallback: when calling a local variable that is NOT in closure_info, check if it might be a closure object by emitting runtime closure unpacking:

For the `IR_FCALL`/`IR_FCALL2`/`IR_FCALL3` path (non-lambda fn ref calls), these use indirect function pointer calls. For closures, the pointer is a closure object, not a raw function pointer.

The fix: add a new IR instruction `IR_CLOSURE_CALL` or modify the fcall path to:
1. Load function pointer from `closure_ptr + 0`
2. Load num_captures from `closure_ptr + 8`
3. Load captures from `closure_ptr + 16+`
4. Build full arg list and call

This is complex. The simpler approach: keep the intra-module path as-is (direct call with captures prepended). For the cross-module path, the `fcall` function in utils.sans calls `fcall(f, arg)` which becomes `IR_FCALL`. In codegen, `IR_FCALL` does `inttoptr + call`.

The key insight: we need `fcall` to understand closure objects. One approach: add a `closure_call(closure, arg)` builtin that:
1. Loads fn_ptr from closure + 0
2. Loads num_captures from closure + 8
3. Builds call with captures + arg

This would be a runtime function `sans_closure_call(closure, arg)` in a new `runtime/closure.sans` or in `runtime/functional.sans`.

The implementer will need to decide the exact approach. The test should guide the implementation.

- [ ] **Step 5: Build and test**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add compiler/ir.sans compiler/codegen.sans compiler/constants.sans tests/fixtures/lambda_cross_module/ tests/run_tests.sh
git commit -m "feat: heap-allocated closure objects for cross-module capturing lambdas"
```

---

## Chunk 7: Documentation Cleanup

### Task 10: Remove fixed limitations and update docs

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `docs/reference.md`
- Modify: `website/docs/index.html`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Remove "by design" items from CLAUDE.md Known Limitations**

Remove from CLAUDE.md:
- "Float stored as i64 via bitcast in register map"
- "Typeck is relaxed for bootstrap"

These are design choices, not limitations.

- [ ] **Step 2: Update CLAUDE.md with remaining limitations**

The Known Limitations in CLAUDE.md should now only contain:
- Scope GC gaps that are NOT fixed (thread safety of rc globals, compiler must use bootstrap)
- Any remaining gaps after the fixes

- [ ] **Step 3: Update all docs to remove fixed limitations**

Remove from README.md, reference.md, and website/docs/index.html:
- "Nested heap values not recursively freed" → FIXED
- "Global pointer escape" → FIXED
- "Capturing lambdas across modules" → FIXED
- "No generic methods on generic structs" → FIXED
- "No nested generics" → FIXED
- "Default params limited to literals" → FIXED (negative literals added)
- "For-loop destructuring limited to 2-element tuples" → FIXED

- [ ] **Step 4: Add [Unreleased] section to CHANGELOG**

```markdown
## [Unreleased]

### Fixed
- Recursive promotion of nested container contents in scope GC
- Global pointer escape — heap values stored in globals no longer freed prematurely
- IR type tracking for JoinHandle parameters
- Cross-module capturing lambdas via heap-allocated closure objects
- Generic methods on generic structs: `impl Box<T> { get(self) ... }`
- Nested generics: `Box<Pair<I I>>` now supported

### Improved
- Default parameters now support negative literals: `f(x:I=-1)`
- For-loop destructuring supports N-element tuples: `for (a b c) in arr`

### Removed from Known Limitations (by design)
- Float stored as i64 via bitcast (design choice)
- Type checker relaxed for bootstrap (design choice)
```

- [ ] **Step 5: Run full test suite**

```bash
/tmp/sans040 build compiler/main.sans
bash tests/run_tests.sh ./compiler/main
```

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md README.md docs/reference.md website/docs/index.html CHANGELOG.md
git commit -m "docs: update known limitations for v0.5.3 — most limitations resolved"
```
