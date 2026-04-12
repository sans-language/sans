# Iterator Protocol Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add lazy `Iter<T>` type with pull-based map/filter/collect chains that avoid intermediate allocations.

**Architecture:** New opaque type `TY_ITER = 23` / `IRTY_ITER = 21` with runtime backing in `runtime/iter.sans`. Each combinator creates a linked operation node; consumers pull elements through the chain one at a time. Entry points: `.iter()` on arrays, `iter(n)` / `iter(a,b)` for ranges.

**Tech Stack:** Sans compiler pipeline (typeck -> ir -> codegen -> LLVM IR), Sans runtime functions with `sans_` prefix.

---

### Task 1: Add Constants

**Files:**
- Modify: `compiler/constants.sans:154` (after TY_DYN_TRAIT)
- Modify: `compiler/constants.sans:546` (after IR_STRING_REVERSE)
- Modify: `compiler/constants.sans:561` (after SCOPE_TAG_DYN_TRAIT)
- Modify: `compiler/ir.sans:43` (after IRTY_DYN_TRAIT)

- [ ] **Step 1: Add TY_ITER type tag**

In `compiler/constants.sans`, after line 154 (`g TY_DYN_TRAIT = 22`), add:

```sans
g TY_ITER = 23
```

- [ ] **Step 2: Add IRTY_ITER IR type tag**

In `compiler/ir.sans`, after line 43 (`g IRTY_DYN_TRAIT = 20`), add:

```sans
g IRTY_ITER = 21
```

- [ ] **Step 3: Add SCOPE_TAG_ITER**

In `compiler/constants.sans`, after line 561 (`g SCOPE_TAG_DYN_TRAIT = 7`), add:

```sans
g SCOPE_TAG_ITER = 8
```

- [ ] **Step 4: Add IR opcodes for iterator operations**

In `compiler/constants.sans`, after line 546 (`g IR_STRING_REVERSE = 372`), add:

```sans
// Iterator opcodes
g IR_ITER_FROM_ARRAY = 373
g IR_ITER_FROM_RANGE = 374
g IR_ITER_MAP = 375
g IR_ITER_FILTER = 376
g IR_ITER_ENUMERATE = 377
g IR_ITER_TAKE = 378
g IR_ITER_SKIP = 379
g IR_ITER_ZIP = 380
g IR_ITER_FLAT_MAP = 381
g IR_ITER_COLLECT = 382
g IR_ITER_FIND = 383
g IR_ITER_ANY = 384
g IR_ITER_ALL = 385
g IR_ITER_REDUCE = 386
g IR_ITER_COUNT = 387
g IR_ITER_FOR_EACH = 388
```

- [ ] **Step 5: Commit**

```bash
git add compiler/constants.sans compiler/ir.sans
git commit -m "feat(iter): add TY_ITER, IRTY_ITER, SCOPE_TAG_ITER, and IR_ITER_* opcodes"
```

---

### Task 2: Runtime Implementation

**Files:**
- Create: `runtime/iter.sans`

- [ ] **Step 1: Create the iterator runtime**

Create `runtime/iter.sans` with the full pull-based iterator implementation. Every iterator node is 32 bytes: `[kind, source, func, param]`.

```sans
// Iterator node kind constants
g ITER_SOURCE_ARRAY = 0
g ITER_SOURCE_RANGE = 1
g ITER_MAP = 2
g ITER_FILTER = 3
g ITER_ENUMERATE = 4
g ITER_TAKE = 5
g ITER_SKIP = 6
g ITER_ZIP = 7
g ITER_FLAT_MAP = 8

// Sentinel: iter_next returns this when exhausted
g ITER_DONE = -9223372036854775807

// --- Source constructors ---

// Create iterator from array. param -> [data_ptr, len, index]
sans_iter_from_array(arr:I) I {
  node = alloc(32)
  state = alloc(24)
  store64(state, load64(arr))        // data_ptr
  store64(state + 8, load64(arr + 8)) // len
  store64(state + 16, 0)             // index = 0
  store64(node, ITER_SOURCE_ARRAY)
  store64(node + 8, 0)               // no source
  store64(node + 16, 0)              // no func
  store64(node + 24, state)
  node
}

// Create iterator from range [start, end). param -> [current, end]
sans_iter_from_range(start:I end_val:I) I {
  node = alloc(32)
  state = alloc(16)
  store64(state, start)
  store64(state + 8, end_val)
  store64(node, ITER_SOURCE_RANGE)
  store64(node + 8, 0)
  store64(node + 16, 0)
  store64(node + 24, state)
  node
}

// --- Combinator constructors ---

sans_iter_map(source:I fn_p:I) I {
  node = alloc(32)
  store64(node, ITER_MAP)
  store64(node + 8, source)
  store64(node + 16, fn_p)
  store64(node + 24, 0)
  node
}

sans_iter_filter(source:I fn_p:I) I {
  node = alloc(32)
  store64(node, ITER_FILTER)
  store64(node + 8, source)
  store64(node + 16, fn_p)
  store64(node + 24, 0)
  node
}

sans_iter_enumerate(source:I) I {
  node = alloc(32)
  state = alloc(8)
  store64(state, 0)                   // counter = 0
  store64(node, ITER_ENUMERATE)
  store64(node + 8, source)
  store64(node + 16, 0)
  store64(node + 24, state)
  node
}

sans_iter_take(source:I n:I) I {
  node = alloc(32)
  state = alloc(16)
  store64(state, n)                   // limit
  store64(state + 8, 0)              // yielded count
  store64(node, ITER_TAKE)
  store64(node + 8, source)
  store64(node + 16, 0)
  store64(node + 24, state)
  node
}

sans_iter_skip(source:I n:I) I {
  node = alloc(32)
  state = alloc(16)
  store64(state, n)                   // skip count
  store64(state + 8, 0)              // skipped flag (0 = not yet skipped)
  store64(node, ITER_SKIP)
  store64(node + 8, source)
  store64(node + 16, 0)
  store64(node + 24, state)
  node
}

sans_iter_zip(source:I other:I) I {
  node = alloc(32)
  store64(node, ITER_ZIP)
  store64(node + 8, source)
  store64(node + 16, 0)
  store64(node + 24, other)           // second iterator
  node
}

sans_iter_flat_map(source:I fn_p:I) I {
  node = alloc(32)
  state = alloc(24)
  store64(state, 0)                   // current sub-array data ptr
  store64(state + 8, 0)              // current sub-array len
  store64(state + 16, 0)             // current sub-array index
  store64(node, ITER_FLAT_MAP)
  store64(node + 8, source)
  store64(node + 16, fn_p)
  store64(node + 24, state)
  node
}

// --- Core pull function ---
// Returns next value, or ITER_DONE if exhausted.

sans_iter_next(node:I) I {
  kind = load64(node)

  if kind == ITER_SOURCE_ARRAY {
    state = load64(node + 24)
    data = load64(state)
    len = load64(state + 8)
    idx = load64(state + 16)
    if idx >= len { return ITER_DONE }
    val = load64(data + idx * 8)
    store64(state + 16, idx + 1)
    return val
  }

  if kind == ITER_SOURCE_RANGE {
    state = load64(node + 24)
    cur = load64(state)
    end_val = load64(state + 8)
    if cur >= end_val { return ITER_DONE }
    store64(state, cur + 1)
    return cur
  }

  if kind == ITER_MAP {
    source = load64(node + 8)
    fn_p = load64(node + 16)
    val = sans_iter_next(source)
    if val == ITER_DONE { return ITER_DONE }
    return fcall(fn_p, val)
  }

  if kind == ITER_FILTER {
    source = load64(node + 8)
    fn_p = load64(node + 16)
    val := sans_iter_next(source)
    while val != ITER_DONE {
      if fcall(fn_p, val) != 0 { return val }
      val = sans_iter_next(source)
    }
    return ITER_DONE
  }

  if kind == ITER_ENUMERATE {
    source = load64(node + 8)
    state = load64(node + 24)
    val = sans_iter_next(source)
    if val == ITER_DONE { return ITER_DONE }
    idx = load64(state)
    store64(state, idx + 1)
    // Create tuple (idx, val) - 16 bytes
    tup = alloc(16)
    store64(tup, idx)
    store64(tup + 8, val)
    return tup
  }

  if kind == ITER_TAKE {
    state = load64(node + 24)
    limit = load64(state)
    yielded = load64(state + 8)
    if yielded >= limit { return ITER_DONE }
    source = load64(node + 8)
    val = sans_iter_next(source)
    if val == ITER_DONE { return ITER_DONE }
    store64(state + 8, yielded + 1)
    return val
  }

  if kind == ITER_SKIP {
    source = load64(node + 8)
    state = load64(node + 24)
    skip_n = load64(state)
    skipped = load64(state + 8)
    // Skip elements on first call
    if skipped == 0 {
      store64(state + 8, 1)
      i := 0
      while i < skip_n {
        val = sans_iter_next(source)
        if val == ITER_DONE { return ITER_DONE }
        i += 1
      }
    }
    return sans_iter_next(source)
  }

  if kind == ITER_ZIP {
    source = load64(node + 8)
    other = load64(node + 24)
    val_a = sans_iter_next(source)
    if val_a == ITER_DONE { return ITER_DONE }
    val_b = sans_iter_next(other)
    if val_b == ITER_DONE { return ITER_DONE }
    tup = alloc(16)
    store64(tup, val_a)
    store64(tup + 8, val_b)
    return tup
  }

  if kind == ITER_FLAT_MAP {
    source = load64(node + 8)
    fn_p = load64(node + 16)
    state = load64(node + 24)
    // Try current sub-array first
    sub_data = load64(state)
    sub_len = load64(state + 8)
    sub_idx = load64(state + 16)
    if sub_data != 0 && sub_idx < sub_len {
      val = load64(sub_data + sub_idx * 8)
      store64(state + 16, sub_idx + 1)
      return val
    }
    // Pull next from source, apply fn to get array
    src_val := sans_iter_next(source)
    while src_val != ITER_DONE {
      sub_arr = fcall(fn_p, src_val)
      sd = load64(sub_arr)
      sl = load64(sub_arr + 8)
      if sl > 0 {
        store64(state, sd)
        store64(state + 8, sl)
        store64(state + 16, 1)
        return load64(sd)
      }
      src_val = sans_iter_next(source)
    }
    return ITER_DONE
  }

  ITER_DONE
}

// --- Consumers ---

sans_iter_collect(node:I) I {
  result = alloc(24)
  cap := 8
  data = alloc(cap * 8)
  len := 0
  val := sans_iter_next(node)
  while val != ITER_DONE {
    if len >= cap {
      cap = cap * 2
      new_data = alloc(cap * 8)
      mcpy(new_data, data, len * 8)
      data = new_data
    }
    store64(data + len * 8, val)
    len += 1
    val = sans_iter_next(node)
  }
  store64(result, data)
  store64(result + 8, len)
  store64(result + 16, cap)
  result
}

sans_iter_find(node:I fn_p:I) I {
  val := sans_iter_next(node)
  while val != ITER_DONE {
    if fcall(fn_p, val) != 0 {
      // Return some(val) - Option with value
      opt = alloc(16)
      store64(opt, 1)        // is_some = 1
      store64(opt + 8, val)
      return opt
    }
    val = sans_iter_next(node)
  }
  // Return none()
  opt = alloc(16)
  store64(opt, 0)            // is_some = 0
  store64(opt + 8, 0)
  opt
}

sans_iter_any(node:I fn_p:I) I {
  val := sans_iter_next(node)
  while val != ITER_DONE {
    if fcall(fn_p, val) != 0 { return 1 }
    val = sans_iter_next(node)
  }
  0
}

sans_iter_all(node:I fn_p:I) I {
  val := sans_iter_next(node)
  while val != ITER_DONE {
    if fcall(fn_p, val) == 0 { return 0 }
    val = sans_iter_next(node)
  }
  1
}

sans_iter_reduce(node:I fn_p:I init:I) I {
  acc := init
  val := sans_iter_next(node)
  while val != ITER_DONE {
    acc = fcall2(fn_p, acc, val)
    val = sans_iter_next(node)
  }
  acc
}

sans_iter_count(node:I) I {
  n := 0
  val := sans_iter_next(node)
  while val != ITER_DONE {
    n += 1
    val = sans_iter_next(node)
  }
  n
}

sans_iter_for_each(node:I fn_p:I) I {
  val := sans_iter_next(node)
  while val != ITER_DONE {
    fcall(fn_p, val)
    val = sans_iter_next(node)
  }
  0
}
```

- [ ] **Step 2: Commit**

```bash
git add runtime/iter.sans
git commit -m "feat(iter): add pull-based iterator runtime"
```

---

### Task 3: Type Checker — `make_iter_type` and Entry Points

**Files:**
- Modify: `compiler/typeck.sans`

- [ ] **Step 1: Add `make_iter_type` constructor**

In `compiler/typeck.sans`, after the existing `make_option_type` function (around line 55), add:

```sans
make_iter_type(inner:I) I {
  t = alloc(32)
  store64(t, TY_ITER)
  store64(t + 8, inner)
  store64(t + 16, 0)
  store64(t + 24, 0)
  t
}
```

- [ ] **Step 2: Add `.iter()` method on arrays**

In `compiler/typeck.sans`, inside the `if ot == TY_ARRAY` method dispatch block (around line 2661+), add handling for `.iter()`. Find the array method section and add before its closing:

```sans
if mc_method == "iter" {
  if mc_nargs != 0 { return tc_error_at(expr, ".iter() takes no arguments") }
  return make_iter_type(arr_inner)
}
```

Where `arr_inner` is the array's inner type (already extracted in the array method dispatch block via `type_inner(obj_ty)`).

- [ ] **Step 3: Add `iter(n)` / `iter(a, b)` built-in functions**

In `compiler/typeck.sans`, in the built-in function dispatch (where `range()` is handled around line 1662), add nearby:

```sans
if name == "iter" {
  if nargs == 1 {
    at = check_expr(args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) != TY_INT { return tc_error_at(expr, "iter() argument must be Int") }
    return make_iter_type(make_type(TY_INT))
  }
  if nargs == 2 {
    at0 = check_expr(args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    at1 = check_expr(args[1], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at0) != TY_INT { return tc_error_at(expr, "iter() first argument must be Int") }
    if type_tag(at1) != TY_INT { return tc_error_at(expr, "iter() second argument must be Int") }
    return make_iter_type(make_type(TY_INT))
  }
  return tc_error_at(expr, "iter() takes 1 or 2 arguments")
}
```

- [ ] **Step 4: Add method dispatch for `TY_ITER` combinators and consumers**

In `compiler/typeck.sans`, in the method call dispatch section (after the existing type blocks for TY_ARRAY, TY_STRING, etc.), add a new block for `TY_ITER`:

```sans
if ot == TY_ITER {
  iter_inner = type_inner(obj_ty)

  // --- Lazy combinators (return Iter) ---

  if mc_method == "map" {
    if mc_nargs != 1 { return tc_error_at(expr, ".map() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 {
        return make_iter_type(fn_ret)
      }
    }
    return tc_error_at(expr, ".map() requires a function (" + type_to_string(iter_inner) + ") -> T")
  }

  if mc_method == "filter" {
    if mc_nargs != 1 { return tc_error_at(expr, ".filter() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 && type_tag(fn_ret) == TY_BOOL {
        return make_iter_type(iter_inner)
      }
    }
    return tc_error_at(expr, ".filter() requires a function (" + type_to_string(iter_inner) + ") -> Bool")
  }

  if mc_method == "enumerate" {
    if mc_nargs != 0 { return tc_error_at(expr, ".enumerate() takes no arguments") }
    tup_inner = make_tuple_type(iter_inner, make_type(TY_INT))
    return make_iter_type(tup_inner)
  }

  if mc_method == "take" {
    if mc_nargs != 1 { return tc_error_at(expr, ".take() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) != TY_INT { return tc_error_at(expr, ".take() argument must be Int") }
    return make_iter_type(iter_inner)
  }

  if mc_method == "skip" {
    if mc_nargs != 1 { return tc_error_at(expr, ".skip() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) != TY_INT { return tc_error_at(expr, ".skip() argument must be Int") }
    return make_iter_type(iter_inner)
  }

  if mc_method == "zip" {
    if mc_nargs != 1 { return tc_error_at(expr, ".zip() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) != TY_ITER { return tc_error_at(expr, ".zip() argument must be an iterator") }
    other_inner = type_inner(at)
    tup_inner = make_tuple_type(iter_inner, other_inner)
    return make_iter_type(tup_inner)
  }

  if mc_method == "flat_map" {
    if mc_nargs != 1 { return tc_error_at(expr, ".flat_map() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 && type_tag(fn_ret) == TY_ARRAY {
        return make_iter_type(type_inner(fn_ret))
      }
    }
    return tc_error_at(expr, ".flat_map() requires a function (" + type_to_string(iter_inner) + ") -> Array<U>")
  }

  // --- Consumers (terminal operations) ---

  if mc_method == "collect" {
    if mc_nargs != 0 { return tc_error_at(expr, ".collect() takes no arguments") }
    return make_array_type(iter_inner)
  }

  if mc_method == "find" {
    if mc_nargs != 1 { return tc_error_at(expr, ".find() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 && type_tag(fn_ret) == TY_BOOL {
        return make_option_type(iter_inner)
      }
    }
    return tc_error_at(expr, ".find() requires a function (" + type_to_string(iter_inner) + ") -> Bool")
  }

  if mc_method == "any" {
    if mc_nargs != 1 { return tc_error_at(expr, ".any() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 && type_tag(fn_ret) == TY_BOOL {
        return make_type(TY_BOOL)
      }
    }
    return tc_error_at(expr, ".any() requires a function (" + type_to_string(iter_inner) + ") -> Bool")
  }

  if mc_method == "all" {
    if mc_nargs != 1 { return tc_error_at(expr, ".all() takes exactly 1 argument") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      fn_params = type_data(at)
      fn_ret = type_data2(at)
      fn_plen = if fn_params != 0 { load64(fn_params + 8) } else { 0 }
      fn_params_d = if fn_params != 0 { load64(fn_params) } else { 0 }
      if fn_plen == 1 && type_eq(load64(fn_params_d), iter_inner) == 1 && type_tag(fn_ret) == TY_BOOL {
        return make_type(TY_BOOL)
      }
    }
    return tc_error_at(expr, ".all() requires a function (" + type_to_string(iter_inner) + ") -> Bool")
  }

  if mc_method == "reduce" {
    if mc_nargs != 2 { return tc_error_at(expr, ".reduce() takes exactly 2 arguments (function, initial)") }
    at = check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    init_ty = check_expr(mc_args[1], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    if type_tag(at) == TY_FN {
      return init_ty
    }
    return tc_error_at(expr, ".reduce() first argument must be a function")
  }

  if mc_method == "count" {
    if mc_nargs != 0 { return tc_error_at(expr, ".count() takes no arguments") }
    return make_type(TY_INT)
  }

  if mc_method == "for_each" {
    if mc_nargs != 1 { return tc_error_at(expr, ".for_each() takes exactly 1 argument") }
    check_expr(mc_args[0], locals, fn_env, structs, enums, fn_name, mod_fns, imports, generics, trait_impls, type_params)
    return make_type(TY_VOID)
  }

  return tc_error_at(expr, "unknown method ." + mc_method + "() on Iter type")
}
```

- [ ] **Step 5: Allow `TY_ITER` in for-in loops**

In `compiler/typeck.sans`, in the `ST_FOR_IN` handler (around line 825), change the validation from requiring only `TY_ARRAY` to also accepting `TY_ITER`:

Currently:
```sans
if type_tag(iter_ty) != TY_ARRAY {
  tc_error_at(stmt, "for-in requires Array, got " + type_to_string(iter_ty))
}
```

Change to:
```sans
if type_tag(iter_ty) != TY_ARRAY && type_tag(iter_ty) != TY_ITER {
  tc_error_at(stmt, "for-in requires Array or Iter, got " + type_to_string(iter_ty))
}
```

And update the element type extraction to work for both:
```sans
elem_ty = type_inner(iter_ty)
```

(This already works since both `TY_ARRAY` and `TY_ITER` store the inner type at offset 8.)

- [ ] **Step 6: Add `type_to_string` support for TY_ITER**

Find where `type_to_string` handles other types and add:

```sans
if tag == TY_ITER { return "Iter<" + type_to_string(type_inner(t)) + ">" }
```

- [ ] **Step 7: Commit**

```bash
git add compiler/typeck.sans
git commit -m "feat(iter): add type checking for Iter<T> type, methods, and built-in iter()"
```

---

### Task 4: IR Lowering

**Files:**
- Modify: `compiler/ir.sans`

- [ ] **Step 1: Add `iter(n)` / `iter(a,b)` built-in function lowering**

In `compiler/ir.sans`, near where `range()` is lowered (around line 808), add:

```sans
else if name == "iter" {
  if args.len() == 1 {
    lower_call_1(ctx, args, IR_ITER_FROM_RANGE, IRTY_ITER)
  } else {
    lower_call_2(ctx, args, IR_ITER_FROM_RANGE, IRTY_ITER)
  }
}
```

Note: For `iter(n)`, the runtime interprets a single-arg call as `range(0, n)`. We'll use `IR_ITER_FROM_RANGE` for both overloads, passing 1 or 2 args.

Actually, we need distinct handling since `lower_call_1` passes 1 arg and `lower_call_2` passes 2 args, and the runtime function signatures differ. Better approach:

```sans
else if name == "iter" {
  if args.len() == 1 {
    a0 = lower_expr(ctx, args.get(0))
    zero_reg = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_const(zero_reg, 0))
    ctx_set_reg_type(ctx, zero_reg, IRTY_INT)
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_FROM_RANGE, dest, zero_reg, a0))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  } else {
    lower_call_2(ctx, args, IR_ITER_FROM_RANGE, IRTY_ITER)
  }
}
```

- [ ] **Step 2: Add `.iter()` method lowering on arrays**

In the `lower_array_method` function in `compiler/ir.sans` (around line 2669+), add:

```sans
else if method == "iter" {
  dest = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst1(IR_ITER_FROM_ARRAY, dest, obj_reg))
  ctx_set_reg_type(ctx, dest, IRTY_ITER)
  dest
}
```

- [ ] **Step 3: Add iterator method dispatch in `lower_method_call`**

In `compiler/ir.sans`, in the `lower_method_call` function (around line 2264+), add a new branch for `IRTY_ITER`:

```sans
else if obj_type == IRTY_ITER {
  lower_iter_method(ctx, obj_reg, method, args)
}
```

- [ ] **Step 4: Implement `lower_iter_method`**

Add the new function somewhere in `compiler/ir.sans`:

```sans
lower_iter_method(ctx:I obj_reg:I method:I args:I) I {
  if method == "map" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_MAP, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "filter" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_FILTER, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "enumerate" {
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst1(IR_ITER_ENUMERATE, dest, obj_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "take" {
    n_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_TAKE, dest, obj_reg, n_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "skip" {
    n_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_SKIP, dest, obj_reg, n_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "zip" {
    other_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_ZIP, dest, obj_reg, other_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "flat_map" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_FLAT_MAP, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ITER)
    dest
  }
  else if method == "collect" {
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst1(IR_ITER_COLLECT, dest, obj_reg))
    ctx_set_reg_type(ctx, dest, IRTY_ARRAY)
    dest
  }
  else if method == "find" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_FIND, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_OPTION)
    dest
  }
  else if method == "any" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_ANY, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_BOOL)
    dest
  }
  else if method == "all" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_ALL, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_BOOL)
    dest
  }
  else if method == "reduce" {
    fn_reg = lower_expr(ctx, args.get(0))
    init_reg = lower_expr(ctx, args.get(1))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst3(IR_ITER_REDUCE, dest, obj_reg, fn_reg, init_reg))
    ctx_set_reg_type(ctx, dest, IRTY_INT)
    dest
  }
  else if method == "count" {
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst1(IR_ITER_COUNT, dest, obj_reg))
    ctx_set_reg_type(ctx, dest, IRTY_INT)
    dest
  }
  else if method == "for_each" {
    fn_reg = lower_expr(ctx, args.get(0))
    dest = ctx_fresh_reg(ctx)
    ctx_emit(ctx, ir_inst2(IR_ITER_FOR_EACH, dest, obj_reg, fn_reg))
    ctx_set_reg_type(ctx, dest, IRTY_INT)
    dest
  }
  else {
    p("error: unknown iterator method: " + method)
    0
  }
}
```

- [ ] **Step 5: Add for-in loop support for iterators**

In `compiler/ir.sans`, in `lower_for_in` (around line 1943), add a branch that checks the IRTY type of the iterable. If it's `IRTY_ITER`, emit a pull-based loop instead of an index-based loop:

After lowering the iterable expression (`arr_reg = lower_expr(ctx, iterable)`), check its type:

```sans
iter_type = ctx_get_reg_type(ctx, arr_reg)
if iter_type == IRTY_ITER {
  // Pull-based loop for iterators
  cond_label = ctx_fresh_label(ctx)
  body_label = ctx_fresh_label(ctx)
  end_label = ctx_fresh_label(ctx)

  old_break = ctx_break_label(ctx)
  old_continue = ctx_continue_label(ctx)
  ctx_set_break_label(ctx, end_label)
  ctx_set_continue_label(ctx, cond_label)
  fi_old_ld = ctx_loop_depth(ctx)
  ctx_set_loop_depth(ctx, fi_old_ld + 1)

  ctx_emit(ctx, ir_jump(cond_label))
  ctx_emit(ctx, ir_label(cond_label))

  // Pull next value: val = sans_iter_next(iter)
  val_reg = ctx_fresh_reg(ctx)
  fname_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(fname_reg, ir_s("sans_iter_next")))
  args_arr = array<I>()
  args_arr.push(arr_reg)
  ctx_emit(ctx, ir_call(val_reg, fname_reg, ptr(args_arr)))
  ctx_set_reg_type(ctx, val_reg, IRTY_INT)

  // Check if done: val == ITER_DONE
  done_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(done_reg, -9223372036854775807))
  ctx_set_reg_type(ctx, done_reg, IRTY_INT)
  cmp_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_cmpop(cmp_reg, IRCMP_NEQ, val_reg, done_reg))
  ctx_set_reg_type(ctx, cmp_reg, IRTY_BOOL)
  ctx_emit(ctx, ir_branch(cmp_reg, body_label, end_label))

  ctx_emit(ctx, ir_label(body_label))
  fi_lse_d = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst0(IR_LOOP_SCOPE_ENTER, fi_lse_d))

  ctx_set_local(ctx, var_name, val_reg)
  lower_stmts(ctx, body_stmts)

  fi_lsx_d = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst0(IR_LOOP_SCOPE_EXIT, fi_lsx_d))
  ctx_emit(ctx, ir_jump(cond_label))

  ctx_emit(ctx, ir_label(end_label))
  ctx_set_break_label(ctx, old_break)
  ctx_set_continue_label(ctx, old_continue)
  ctx_set_loop_depth(ctx, fi_old_ld)
  return 0
}
```

Place this before the existing array-based loop code so it takes priority when the iterable is an iterator.

- [ ] **Step 6: Commit**

```bash
git add compiler/ir.sans
git commit -m "feat(iter): add IR lowering for iterator operations and for-in loop"
```

---

### Task 5: Code Generation

**Files:**
- Modify: `compiler/codegen.sans`
- Modify: `compiler/main.sans`

- [ ] **Step 1: Add external declarations for iterator runtime functions**

In `compiler/codegen.sans`, in the `emit_externals` function (after the array section around line 599), add:

```sans
  // ------ Sans runtime: Iterators ------
  emit(cg, "declare i64 @sans_iter_from_array(i64)")
  emit(cg, "declare i64 @sans_iter_from_range(i64, i64)")
  emit(cg, "declare i64 @sans_iter_map(i64, i64)")
  emit(cg, "declare i64 @sans_iter_filter(i64, i64)")
  emit(cg, "declare i64 @sans_iter_enumerate(i64)")
  emit(cg, "declare i64 @sans_iter_take(i64, i64)")
  emit(cg, "declare i64 @sans_iter_skip(i64, i64)")
  emit(cg, "declare i64 @sans_iter_zip(i64, i64)")
  emit(cg, "declare i64 @sans_iter_flat_map(i64, i64)")
  emit(cg, "declare i64 @sans_iter_next(i64)")
  emit(cg, "declare i64 @sans_iter_collect(i64)")
  emit(cg, "declare i64 @sans_iter_find(i64, i64)")
  emit(cg, "declare i64 @sans_iter_any(i64, i64)")
  emit(cg, "declare i64 @sans_iter_all(i64, i64)")
  emit(cg, "declare i64 @sans_iter_reduce(i64, i64, i64)")
  emit(cg, "declare i64 @sans_iter_count(i64)")
  emit(cg, "declare i64 @sans_iter_for_each(i64, i64)")
```

- [ ] **Step 2: Add codegen for all IR_ITER_* opcodes**

In `compiler/codegen.sans`, in the main instruction dispatch (after existing opcode handling), add:

```sans
  // ------ Iterator operations ------
  if op == IR_ITER_FROM_ARRAY {
    compile_rt1(cg, inst, "sans_iter_from_array")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_FROM_RANGE {
    compile_rt2(cg, inst, "sans_iter_from_range")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_MAP {
    compile_rt2(cg, inst, "sans_iter_map")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_FILTER {
    compile_rt2(cg, inst, "sans_iter_filter")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_ENUMERATE {
    compile_rt1(cg, inst, "sans_iter_enumerate")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_TAKE {
    compile_rt2(cg, inst, "sans_iter_take")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_SKIP {
    compile_rt2(cg, inst, "sans_iter_skip")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_ZIP {
    compile_rt2(cg, inst, "sans_iter_zip")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_FLAT_MAP {
    compile_rt2(cg, inst, "sans_iter_flat_map")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ITER)
    return 0
  }
  if op == IR_ITER_COLLECT {
    compile_rt1(cg, inst, "sans_iter_collect")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_ARRAY)
    return 0
  }
  if op == IR_ITER_FIND {
    compile_rt2(cg, inst, "sans_iter_find")
    cg_set_ptr(cg, dest)
    emit_scope_track(cg, cg_get_val(cg, dest), SCOPE_TAG_OPTION)
    return 0
  }
  if op == IR_ITER_ANY {
    compile_rt2(cg, inst, "sans_iter_any")
    return 0
  }
  if op == IR_ITER_ALL {
    compile_rt2(cg, inst, "sans_iter_all")
    return 0
  }
  if op == IR_ITER_REDUCE {
    compile_rt3(cg, inst, "sans_iter_reduce")
    return 0
  }
  if op == IR_ITER_COUNT {
    compile_rt1(cg, inst, "sans_iter_count")
    return 0
  }
  if op == IR_ITER_FOR_EACH {
    compile_rt2(cg, inst, "sans_iter_for_each")
    return 0
  }
```

- [ ] **Step 3: Add TY_ITER -> IRTY_ITER mapping in main.sans**

In `compiler/main.sans`, in the type tag to IRTY mapping chain (around line 410), add before the final fallback `IRTY_INT`:

```sans
if rt_tag == TY_ITER { IRTY_ITER } else {
```

And add the closing `}` to match.

- [ ] **Step 4: Add IRTY_ITER to scope tracking in codegen**

In `compiler/codegen.sans`, in the scope tracking dispatch for function return values (around line 2841), add:

```sans
else if frt == IRTY_ITER { stag = SCOPE_TAG_ITER }
```

- [ ] **Step 5: Add scope_exit handling for SCOPE_TAG_ITER**

Check how `sans_scope_exit` handles scope tags in the runtime. The scope GC frees tracked allocations — iterator nodes are just malloc'd memory, so the existing `free()` path for `SCOPE_TAG_RAW` should work, but we should verify the scope system handles tag 8. If `sans_scope_exit` uses a switch on tag values, add `SCOPE_TAG_ITER = 8` handling that frees the node.

Look at `runtime/rc.sans` for the scope exit implementation and add iterator handling if needed. Iterator nodes are plain `alloc(32)` allocations, so freeing them with the default path should work.

- [ ] **Step 6: Commit**

```bash
git add compiler/codegen.sans compiler/main.sans
git commit -m "feat(iter): add LLVM codegen for iterator operations"
```

---

### Task 6: Test Fixtures

**Files:**
- Create: `tests/fixtures/iter_basic.sans`
- Create: `tests/fixtures/iter_range.sans`
- Create: `tests/fixtures/iter_chain.sans`
- Create: `tests/fixtures/iter_consumers.sans`
- Create: `tests/fixtures/iter_take_skip.sans`
- Create: `tests/fixtures/iter_for_in.sans`
- Modify: `tests/run_tests.sh`

- [ ] **Step 1: Create `iter_basic.sans` — basic map + filter + collect**

```sans
main() I {
  a = [1 2 3 4 5]
  b = a.iter().map(|x:I| I { x * 2 }).collect()
  // b == [2 4 6 8 10]
  // sum: 2+4+6+8+10 = 30
  b[0] + b[1] + b[2] + b[3] + b[4]
}
```

Expected exit code: 30

- [ ] **Step 2: Create `iter_range.sans` — range iterator without allocation**

```sans
main() I {
  a = iter(5).collect()
  // a == [0 1 2 3 4]
  // sum: 0+1+2+3+4 = 10
  a[0] + a[1] + a[2] + a[3] + a[4]
}
```

Expected exit code: 10

- [ ] **Step 3: Create `iter_chain.sans` — chained map + filter**

```sans
main() I {
  result = [1 2 3 4 5 6 7 8 9 10].iter()
    .filter(|x:I| B { x > 3 })
    .map(|x:I| I { x * 2 })
    .collect()
  // filtered: [4 5 6 7 8 9 10], mapped: [8 10 12 14 16 18 20]
  result.len()
}
```

Expected exit code: 7

- [ ] **Step 4: Create `iter_consumers.sans` — any, all, count, reduce**

```sans
main() I {
  has_big = [1 2 3 4 5].iter().any(|x:I| B { x > 3 })
  all_pos = [1 2 3].iter().all(|x:I| B { x > 0 })
  cnt = iter(10).count()
  total = [1 2 3 4 5].iter().reduce(|acc:I x:I| I { acc + x }, 0)
  // has_big=1, all_pos=1, cnt=10, total=15
  // result = 1 + 1 + 10 + 15 = 27
  has_big + all_pos + cnt + total
}
```

Expected exit code: 27

- [ ] **Step 5: Create `iter_take_skip.sans` — take and skip**

```sans
main() I {
  a = iter(10).skip(3).take(4).collect()
  // skip 0,1,2 -> take 3,4,5,6
  // a == [3 4 5 6]
  a[0] + a[1] + a[2] + a[3]
}
```

Expected exit code: 18

- [ ] **Step 6: Create `iter_for_in.sans` — for-in loop with iterator**

```sans
main() I {
  total := 0
  for x in iter(5).map(|x:I| I { x * 2 }) {
    total += x
  }
  // 0*2 + 1*2 + 2*2 + 3*2 + 4*2 = 0+2+4+6+8 = 20
  total
}
```

Expected exit code: 20

- [ ] **Step 7: Register tests in run_tests.sh**

Add to `tests/run_tests.sh` alongside the other test registrations:

```bash
run_test "iter_basic"                "$REPO_ROOT/tests/fixtures/iter_basic.sans"                30
run_test "iter_range"                "$REPO_ROOT/tests/fixtures/iter_range.sans"                10
run_test "iter_chain"                "$REPO_ROOT/tests/fixtures/iter_chain.sans"                7
run_test "iter_consumers"            "$REPO_ROOT/tests/fixtures/iter_consumers.sans"            27
run_test "iter_take_skip"            "$REPO_ROOT/tests/fixtures/iter_take_skip.sans"            18
run_test "iter_for_in"               "$REPO_ROOT/tests/fixtures/iter_for_in.sans"               20
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `bash tests/run_tests.sh`

Expected: All 6 new iter_* tests pass (along with existing tests).

- [ ] **Step 9: Commit**

```bash
git add tests/fixtures/iter_*.sans tests/run_tests.sh
git commit -m "test(iter): add E2E test fixtures for iterator protocol"
```

---

### Task 7: Documentation Update

**Files:**
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `website/docs/index.html`
- Modify: `editors/vscode-sans/src/extension.ts`
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json`
- Modify: `README.md`

- [ ] **Step 1: Update `docs/reference.md`**

Add a new section for Iterator type with full documentation of all methods, examples, and explanation of lazy evaluation.

- [ ] **Step 2: Update `docs/ai-reference.md`**

Add compact AI reference entries for `iter()`, `.iter()`, and all combinator/consumer methods.

- [ ] **Step 3: Update `website/docs/index.html`**

Add Iterator section to website documentation matching reference.md content.

- [ ] **Step 4: Update VS Code extension hover data**

In `editors/vscode-sans/src/extension.ts`, add `HOVER_DATA` entries for:
- `iter` (built-in function)
- All iterator methods: `map`, `filter`, `enumerate`, `take`, `skip`, `zip`, `flat_map`, `collect`, `find`, `any`, `all`, `reduce`, `count`, `for_each`

- [ ] **Step 5: Update syntax highlighting**

In `editors/vscode-sans/syntaxes/sans.tmLanguage.json`, add `iter` to the built-in function keywords pattern.

- [ ] **Step 6: Update README.md**

Add Iterator/lazy evaluation to the feature list.

- [ ] **Step 7: Commit**

```bash
git add docs/ website/ editors/ README.md
git commit -m "docs(iter): add iterator protocol documentation, hover data, and syntax highlighting"
```

---

### Task 8: Verify All Tests Pass

- [ ] **Step 1: Build the compiler**

```bash
sans build compiler/main.sans
```

- [ ] **Step 2: Run full test suite**

```bash
bash tests/run_tests.sh
```

Verify: All existing tests still pass (no regressions). All 6 new iter_* tests pass.

- [ ] **Step 3: Test edge cases manually**

Build and run each test fixture individually to verify correct exit codes:

```bash
sans build tests/fixtures/iter_basic.sans -o /tmp/iter_basic && /tmp/iter_basic; echo $?
sans build tests/fixtures/iter_range.sans -o /tmp/iter_range && /tmp/iter_range; echo $?
sans build tests/fixtures/iter_chain.sans -o /tmp/iter_chain && /tmp/iter_chain; echo $?
sans build tests/fixtures/iter_consumers.sans -o /tmp/iter_consumers && /tmp/iter_consumers; echo $?
sans build tests/fixtures/iter_take_skip.sans -o /tmp/iter_take_skip && /tmp/iter_take_skip; echo $?
sans build tests/fixtures/iter_for_in.sans -o /tmp/iter_for_in && /tmp/iter_for_in; echo $?
```
