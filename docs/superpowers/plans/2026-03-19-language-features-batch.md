# Language Features Batch Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add five language features to the Sans compiler: default function parameters, error codes on Result, pattern match guards (with binding patterns), user-defined generic structs, and for-loop destructuring for map entries.

**Architecture:** Each feature follows the Sans compiler pipeline: parser → typeck → IR → codegen. All features are independent and can be implemented in sequence. The compiler is self-hosted (written in Sans), so all changes are in `.sans` files.

**Tech Stack:** Sans compiler (parser.sans, typeck.sans, ir.sans, codegen.sans, constants.sans), Sans runtime (runtime/result.sans), LLVM 17

**Spec:** `docs/superpowers/specs/2026-03-19-language-features-batch-design.md`

---

## Chunk 1: Default Function Parameters

### Task 1: Test fixture for default params

**Files:**
- Create: `tests/fixtures/default_params.sans`
- Modify: `tests/run_tests.sh`

- [ ] **Step 1: Write test fixture**

```sans
add(a:I b:I=10) I = a + b
greet(name:S greeting:S="Hi") S = greeting + " " + name

main() {
  x = add(5)
  y = add(5 20)
  x + y
}
```

Expected: `add(5)` → 15 (5+10), `add(5 20)` → 25 (5+20), total = 40.

Write to `tests/fixtures/default_params.sans`.

- [ ] **Step 2: Register test in run_tests.sh**

Add line after the last `run_test` entry in the single-file tests section:

```bash
run_test "default_params"             "$REPO_ROOT/tests/fixtures/default_params.sans"             40
```

- [ ] **Step 3: Run test to verify it fails**

Run: `bash tests/run_tests.sh 2>&1 | grep default_params`
Expected: SKIP or FAIL (feature not implemented yet)

- [ ] **Step 4: Commit test fixture**

```bash
git add tests/fixtures/default_params.sans tests/run_tests.sh
git commit -m "test: add fixture for default function parameters"
```

### Task 2: Parser — default param values

**Files:**
- Modify: `compiler/parser.sans:331-337` (make_param)
- Modify: `compiler/parser.sans:518-531` (parse_params)

- [ ] **Step 1: Update `make_param` to store default at offset 16**

In `compiler/parser.sans`, the current `make_param` (lines 331-337) stores 0 at offset 16 (reserved). The default expression pointer will go there. No layout change needed — just semantic: offset 16 = default AST expr pointer (0 means no default).

No code change to `make_param` itself (it already stores 0 at offset 16). We need a new helper:

```sans
make_param_default(name:I type_name:I default_expr:I) I {
  n = alloc(24)
  store64(n, name)
  store64(n + 8, type_name)
  store64(n + 16, default_expr)
  n
}
```

Add this directly after `make_param` (after line 337).

- [ ] **Step 2: Update `parse_params` to parse `=literal` defaults**

In `compiler/parser.sans`, replace the param-building line in `parse_params` (line 526). After parsing the type name, check for `TK_EQ`:

```sans
parse_params(p:I) [I] {
  params = array<I>()
  if p_at(p, TK_RPAREN) == 1 { return params }
  while 1 {
    name = p_expect_ident(p)
    if p_at(p, TK_COLON) == 1 { p_advance(p) }
    type_name = parse_type_name(p)
    def_expr = if p_at(p, TK_EQ) == 1 {
      p_advance(p)
      parse_atom(p)
    } else { 0 }
    params.push(make_param_default(name, type_name, def_expr))
    if p_at(p, TK_COMMA) == 1 { p_advance(p) }
    if p_at(p, TK_RPAREN) == 1 { return params }
  }
  params
}
```

Key: We use `parse_atom(p)` which accepts any primary expression. However, defaults must be **literals only** (Int, Float, String, Bool). Typeck will enforce this by checking the AST node type of the default expression (EX_INT_LIT, EX_FLOAT_LIT, EX_STRING_LIT, EX_BOOL_LIT). If no `=`, default is 0 (null). Lambdas cannot have default params.

- [ ] **Step 3: Verify TK_EQ exists in the lexer**

Search for `TK_EQ` in the lexer/constants. The `=` token for assignment should already exist. Confirm it's distinct from `==` (TK_EQ_EQ or similar).

Run: grep for `TK_EQ` in `compiler/constants.sans` or `compiler/lexer.sans`.

- [ ] **Step 4: Commit parser changes**

```bash
git add compiler/parser.sans
git commit -m "feat(parser): parse default parameter values (=literal)"
```

### Task 3: Typeck — validate default params and allow partial calls

**Files:**
- Modify: `compiler/typeck.sans:1484-1499` (fn_env call checking)
- Modify: `compiler/typeck.sans:2798-2809` (fn_env registration — store default count)

- [ ] **Step 1: Count required params during fn_env registration and validate literal-only defaults**

In `compiler/typeck.sans` around line 2798-2809, when building the fn_env signature, also count how many params have no default (required params). Store this in the fn_sig. Additionally, validate that default expressions are literal AST nodes only (EX_INT_LIT, EX_FLOAT_LIT, EX_STRING_LIT, EX_BOOL_LIT) — if not, emit a type error.

Expand `make_fn_sig` (line 2499) from 16 to 24 bytes to add required_count:

```sans
make_fn_sig(params_ptr:I ret_type:I required_count:I) I {
  sig = alloc(24)
  store64(sig, params_ptr)
  store64(sig + 8, ret_type)
  store64(sig + 16, required_count)
  sig
}
```

Update all callers of `make_fn_sig` to pass the required count. For functions without defaults, `required_count = num_params`. For functions with defaults, count params where `load64(p_node + 16) == 0`.

In the registration loop (~line 2798-2809):

```sans
num_fparams = load64(fparams_arr + 8)
param_types = array<I>()
required_count := num_fparams
pi := 0
while pi < num_fparams {
  p_node = load64(fparams_p + pi * 8)
  p_type_name = load64(p_node + 8)
  p_default = load64(p_node + 16)
  param_types.push(resolve_type(p_type_name, structs_map, enums_map, mod_exports))
  if p_default != 0 && required_count == num_fparams {
    required_count = pi
  }
  pi += 1
}
ret_ty = resolve_type(fret_name, structs_map, enums_map, mod_exports)
mset(fn_env, fname, make_fn_sig(ptr(param_types), ret_ty, required_count))
```

- [ ] **Step 2: Update call-site arg count check to allow range**

In `compiler/typeck.sans` around line 1491, change the exact match to a range check:

```sans
sig_required = load64(sig + 16)
if nargs < sig_required || nargs > sig_params_len {
  tc_error("wrong argument count calling '" + name + "': expected " + str(sig_required) + " to " + str(sig_params_len) + " argument(s) but got " + str(nargs))
}
```

Keep the per-arg type checking loop (lines 1492-1498) but only check up to `nargs` (it already does this since `ci < nargs`).

- [ ] **Step 3: Update all other `make_fn_sig` callers**

Search for all calls to `make_fn_sig` in typeck.sans. Each one needs a third argument. For methods and imported functions, pass `num_params` as required_count (no defaults for those):

- Line 2765: `make_fn_sig(ptr(method_param_types), mret)` → add `, method_param_types.len()`
- Line 2777: `make_fn_sig(ptr(all_param_types), mret)` → add `, all_param_types.len()`

- [ ] **Step 4: Commit typeck changes**

```bash
git add compiler/typeck.sans
git commit -m "feat(typeck): validate default params, allow partial call arg counts"
```

### Task 4: IR — fill missing args with default values

**Files:**
- Modify: `compiler/ir.sans:914-945` (lower_call_user)
- Modify: `compiler/ir.sans:246-266` (make_lower_ctx — add fn_param_defaults map)
- Modify: `compiler/ir.sans:3596-3620` (lower_full — populate fn_param_defaults)
- Modify: `compiler/ir.sans:3394-3402` (lower_function_body — pass fn_param_defaults)

- [ ] **Step 1: Add fn_param_defaults to IR context**

Expand `make_lower_ctx` (line 246) from 168 to 176 bytes. Add at offset 168: `fn_param_defaults` — a Map from function name to array of default AST expr nodes.

```sans
store64(ctx + 168, ptr(M()))   // fn_param_defaults: fname -> [default_expr_0, default_expr_1, ...]
```

Add accessor:
```sans
ctx_fn_param_defaults(ctx:I) I = load64(ctx + 168)
```

- [ ] **Step 2: Populate fn_param_defaults in lower_full**

In `compiler/ir.sans` around line 3596-3620, after building `fn_ret_types`, build the defaults map:

```sans
// 3d. Build fn_param_defaults from program functions
fn_param_defaults = M()
fpdi := 0
while fpdi < nfns {
  func = prog_fns.get(fpdi)
  fname = load64(func)
  fparams_arr = load64(func + 8)
  fparams_p = load64(fparams_arr)
  num_fparams = load64(fparams_arr + 8)
  defaults = array<I>()
  has_defaults := 0
  dpi := 0
  while dpi < num_fparams {
    p_node = load64(fparams_p + dpi * 8)
    p_default = load64(p_node + 16)
    defaults.push(p_default)
    if p_default != 0 { has_defaults = 1 }
    dpi += 1
    0
  }
  if has_defaults == 1 {
    mset(fn_param_defaults, fname, defaults)
    if module_name != 0 { mset(fn_param_defaults, module_name + "__" + fname, defaults) }
  }
  fpdi += 1
  0
}
```

Store on ctx: `store64(ctx + 168, ptr(fn_param_defaults))` in `make_lower_ctx` and pass through `lower_function_body`.

- [ ] **Step 3: Fill missing args in lower_call_user**

In `compiler/ir.sans` around lines 914-945, after lowering explicit args, check if more are needed:

```sans
// Regular user-defined function call
arg_regs = array<I>()
i := 0
while i < args.len() {
  arg_regs.push(lower_expr(ctx, args.get(i)))
  i = i + 1
  0
}
// Fill missing args with defaults
fn_defaults_map = ctx_fn_param_defaults(ctx)
defaults_arr = if mhas(fn_defaults_map, call_name) == 1 { mget(fn_defaults_map, call_name) } else {
  if mhas(fn_defaults_map, name) == 1 { mget(fn_defaults_map, name) } else { 0 }
}
if defaults_arr != 0 {
  total_params = defaults_arr.len()
  while arg_regs.len() < total_params {
    di = arg_regs.len()
    def_expr = defaults_arr.get(di)
    if def_expr != 0 {
      arg_regs.push(lower_expr(ctx, def_expr))
    }
    0
  }
}
```

Note: This code goes BEFORE the `ctx_emit(ctx, ir_call(...))` line. The `call_name` variable is computed just before (line 929-933), so insert the defaults-filling between the name resolution and the call emission.

- [ ] **Step 4: Commit IR changes**

```bash
git add compiler/ir.sans
git commit -m "feat(ir): fill missing args with default values from param declarations"
```

### Task 5: Build, test, and verify default params

**Files:** None (verification only)

- [ ] **Step 1: Build the compiler**

Run: `sans build compiler/main.sans`
Expected: Successful compilation

- [ ] **Step 2: Run the default_params test**

Run: `bash tests/run_tests.sh 2>&1 | grep default_params`
Expected: `✓  default_params`

- [ ] **Step 3: Run full test suite**

Run: `bash tests/run_tests.sh`
Expected: All existing tests still pass (no regressions), plus the new test passes.

- [ ] **Step 4: Commit any fixes if needed**

---

## Chunk 2: Error Codes on Result

### Task 6: Test fixture for error codes

**Files:**
- Create: `tests/fixtures/result_error_code.sans`
- Modify: `tests/run_tests.sh`

- [ ] **Step 1: Write test fixture**

```sans
fetch(url:S) R<I> {
  if url == "bad" { err(404 "not found") } else { ok(42) }
}

main() {
  r = fetch("bad")
  code = r.code()
  if code == 404 { code - 404 + 10 } else { 0 }
}
```

Expected exit: 10 (404 - 404 + 10).

Write to `tests/fixtures/result_error_code.sans`.

- [ ] **Step 2: Register test**

```bash
run_test "result_error_code"          "$REPO_ROOT/tests/fixtures/result_error_code.sans"          10
```

- [ ] **Step 3: Verify test fails**

Run: `bash tests/run_tests.sh 2>&1 | grep result_error_code`
Expected: SKIP or FAIL

- [ ] **Step 4: Commit test**

```bash
git add tests/fixtures/result_error_code.sans tests/run_tests.sh
git commit -m "test: add fixture for Result error codes"
```

### Task 7: Runtime — expand Result layout to 32 bytes

**Files:**
- Modify: `runtime/result.sans:1-18` (sans_result_ok and sans_result_err)

- [ ] **Step 1: Update sans_result_ok to alloc 32 bytes**

```sans
sans_result_ok(value:I) I {
  r = alloc(32)
  store64(r, 0)
  store64(r + 8, value)
  store64(r + 16, 0)
  store64(r + 24, 0)
  r
}
```

- [ ] **Step 2: Update sans_result_err to take code + msg, alloc 32 bytes**

```sans
sans_result_err(code:I msg:I) I {
  r = alloc(32)
  store64(r, 1)
  store64(r + 8, 0)
  len = slen(msg)
  buf = alloc(len + 1)
  mcpy(buf, msg, len + 1)
  store64(r + 16, buf)
  store64(r + 24, code)
  r
}
```

- [ ] **Step 3: Add sans_result_code function**

```sans
sans_result_code(r:I) I {
  load64(r + 24)
}
```

Add after `sans_result_error` at the end of the file.

- [ ] **Step 4: Commit runtime changes**

```bash
git add runtime/result.sans
git commit -m "feat(runtime): expand Result to 32 bytes with error code at offset 24"
```

### Task 8: Constants + IR — add IR_RESULT_CODE, update err() lowering

**Files:**
- Modify: `compiler/constants.sans` (add IR_RESULT_CODE = 245)
- Modify: `compiler/ir.sans:701` (err lowering — change to 2-arg)
- Modify: `compiler/ir.sans` (add .code() method lowering)

- [ ] **Step 1: Add IR_RESULT_CODE constant**

In `compiler/constants.sans`, after `IR_MAP_ENTRIES = 244` (line 381), add:

```sans
g IR_RESULT_CODE = 245
```

- [ ] **Step 2: Update err() lowering to always emit 2 args**

In `compiler/ir.sans` line 701, change:

```sans
else if name == "err" { lower_call_1(ctx, args, IR_RESULT_ERR, IRTY_RESULT) }
```

To handle 1-arg (synthesize code=0) and 2-arg cases. For consistency, both paths should use `lower_call_2` to ensure identical IR emission and scope tracking. For the 1-arg case, build a synthetic args array with a zero literal prepended:

```sans
else if name == "err" {
  if args.len() == 2 {
    lower_call_2(ctx, args, IR_RESULT_ERR, IRTY_RESULT)
  } else {
    // 1-arg err("msg") — synthesize code=0 as first arg
    // Build a 2-element args array: [make_int_lit(0), original_msg_expr]
    synth_args = array<I>()
    synth_args.push(make_int_lit(0))
    synth_args.push(args.get(0))
    lower_call_2(ctx, synth_args, IR_RESULT_ERR, IRTY_RESULT)
  }
}
```

- [ ] **Step 3: Add .code() method lowering**

Find where Result methods are lowered in ir.sans (search for `IR_RESULT_IS_OK` or `is_ok`). Add `.code()` handling alongside the other Result methods:

```sans
else if method == "code" { lower_method_0(ctx, obj_reg, IR_RESULT_CODE, IRTY_INT) }
```

The pattern follows the existing `.is_ok()`, `.is_err()`, `.error()` method lowering pattern using `lower_method_0` (0-arg method call that takes the object register).

- [ ] **Step 4: Commit IR changes**

```bash
git add compiler/constants.sans compiler/ir.sans
git commit -m "feat(ir): add IR_RESULT_CODE, update err() to always emit 2-arg call"
```

### Task 9: Typeck — update err() and add .code() method

**Files:**
- Modify: `compiler/typeck.sans:1410-1415` (err() checking)
- Modify: `compiler/typeck.sans:1950-1980` (Result methods)

- [ ] **Step 1: Update err() to accept 1 or 2 args**

Replace the err() checking block (lines 1410-1414):

```sans
if name == "err" {
  if nargs == 1 {
    at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
    if is_i64_compat(at) != 1 { tc_error("err() requires String argument, got " + type_to_string(at)) }
    return make_type(TY_RESULT_ERR)
  }
  if nargs == 2 {
    at0 = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
    at1 = check_expr(args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
    if type_tag(at0) != TY_INT { tc_error("err() first arg must be Int error code, got " + type_to_string(at0)) }
    if is_i64_compat(at1) != 1 { tc_error("err() second arg must be String message, got " + type_to_string(at1)) }
    return make_type(TY_RESULT_ERR)
  }
  tc_error("err() takes 1 or 2 arguments, got " + str(nargs))
}
```

- [ ] **Step 2: Add .code() method on Result**

In `compiler/typeck.sans` inside the Result methods block (after line 1953 where `.is_ok()` is handled, or alongside the other methods), add:

```sans
if mc_method == "code" {
  if mc_nargs != 0 { tc_error("code() takes no arguments") }
  return make_type(TY_INT)
}
```

- [ ] **Step 3: Commit typeck changes**

```bash
git add compiler/typeck.sans
git commit -m "feat(typeck): err() accepts optional error code, add .code() method"
```

### Task 10: Codegen — compile IR_RESULT_CODE and update IR_RESULT_ERR

**Files:**
- Modify: `compiler/codegen.sans:2849-2852` (IR_RESULT_ERR)
- Modify: `compiler/codegen.sans:2858` (add IR_RESULT_CODE after IR_RESULT_ERROR)

**IMPORTANT:** Steps 1 and 3 must be done atomically — the `compile_rt2` change REQUIRES the declaration change, or LLVM will emit a 2-arg call to a 1-arg declaration, corrupting the stack.

- [ ] **Step 1: Update IR_RESULT_ERR codegen to use compile_rt2 AND update declaration**

Change line 2849-2852:

```sans
if op == IR_RESULT_ERR {
  compile_rt2(cg, inst, "sans_result_err")
  emit_scope_track(cg, cg_get_val(cg, ir_dest(inst)), 4)
  return 0
}
```

(`compile_rt1` → `compile_rt2` since err now always takes 2 args: code + msg)

Also, find the LLVM extern declaration for `sans_result_err` (search for `declare i64 @sans_result_err` in codegen.sans, around line 493-500). Change from:

```
declare i64 @sans_result_err(i64)
```

To:

```
declare i64 @sans_result_err(i64, i64)
```

And add the new declaration:

```
declare i64 @sans_result_code(i64)
```

- [ ] **Step 2: Add IR_RESULT_CODE codegen**

After the `IR_RESULT_ERROR` line (line 2858), add:

```sans
if op == IR_RESULT_CODE { return compile_rt1(cg, inst, "sans_result_code") }
```

- [ ] **Step 4: Commit codegen changes**

```bash
git add compiler/codegen.sans
git commit -m "feat(codegen): compile IR_RESULT_CODE, update IR_RESULT_ERR to 2-arg"
```

### Task 11: Build, test, and verify error codes

- [ ] **Step 1: Build the compiler**

Run: `sans build compiler/main.sans`

- [ ] **Step 2: Run the error code test**

Run: `bash tests/run_tests.sh 2>&1 | grep result_error_code`
Expected: `✓  result_error_code`

- [ ] **Step 3: Run full test suite — check for regressions**

Run: `bash tests/run_tests.sh`
Expected: All tests pass. Especially verify `result_ok_unwrap` (exit 10), `result_error_handling` (exit 99), `try_operator` and `try_operator_err` still pass — these use the old `err("msg")` 1-arg form.

- [ ] **Step 4: Commit any fixes**

---

## Chunk 3: Pattern Match Guards (with Binding Patterns)

### Task 12: Test fixture for match guards

**Files:**
- Create: `tests/fixtures/match_guard.sans`
- Modify: `tests/run_tests.sh`

- [ ] **Step 1: Write test fixture**

```sans
classify(x:I) I {
  match x {
    n if n > 0 => 1
    n if n < 0 => 2
    _ => 0
  }
}

main() {
  a = classify(5)
  b = classify(-3)
  c = classify(0)
  a * 100 + b * 10 + c
}
```

Expected exit: 120 (1*100 + 2*10 + 0).

- [ ] **Step 2: Register test**

```bash
run_test "match_guard"                "$REPO_ROOT/tests/fixtures/match_guard.sans"                120
```

- [ ] **Step 3: Verify test fails**

Run: `bash tests/run_tests.sh 2>&1 | grep match_guard`

- [ ] **Step 4: Commit test**

```bash
git add tests/fixtures/match_guard.sans tests/run_tests.sh
git commit -m "test: add fixture for match guards with binding patterns"
```

### Task 13: Parser — binding patterns and guards

**Files:**
- Modify: `compiler/constants.sans` (add PAT_BIND = 4 or similar)
- Modify: `compiler/parser.sans:1307-1312` (parse_match_arm — check for guard)
- Modify: `compiler/parser.sans:1314-1377` (parse_pattern — add binding pattern)

- [ ] **Step 1: Determine pattern type constants**

Check how pattern types are stored. The research shows `pat_type` at offset 24 in pattern nodes:
- 1 = int literal
- 2 = string literal
- 3 = wildcard
- 0 = enum variant

Add a new constant for binding patterns. In `compiler/constants.sans`, add:

```sans
g PAT_BIND = 4
```

after the statement type constants (or wherever pattern constants live, if they're in constants.sans — they may be inline in parser.sans).

Note: Pattern types may be hardcoded as magic numbers in parser.sans. If so, just use `4` directly and add a comment.

- [ ] **Step 2: Add binding pattern parsing**

In `parse_pattern` (line 1314-1377), the wildcard case handles `_` at lines 1339-1346. Before the wildcard check (or in the else/fallthrough for unrecognized identifiers), add binding pattern recognition:

When the token is a lowercase identifier that is NOT `_`, `true`, or `false`, and is NOT an enum name (uppercase first char), treat it as a binding pattern.

**Placement:** This must come AFTER the bool literal checks (`true`/`false` at lines 1365-1372) and AFTER the wildcard `_` check (lines 1339-1346), but BEFORE the enum variant check (uppercase first char). Insert it as a fallthrough case for lowercase identifiers:

```sans
// Binding pattern: bare lowercase identifier captures scrutinee
// e.g. "n if n > 0 =>"
// Must come after true/false/_ checks
if kind == TK_IDENT {
  name = tok_val(p_peek(p))
  first_char = load8(name)
  // lowercase letter (a-z), not underscore, not true/false (already handled above)
  if first_char >= 97 && first_char <= 122 && name != "true" && name != "false" {
    p_advance(p)
    pat = alloc(32)
    store64(pat, 0)        // no enum name
    store64(pat + 8, name) // binding name at offset 8
    store64(pat + 16, 0)   // no bindings array
    store64(pat + 24, 4)   // pat_type = 4 (PAT_BIND)
    return pat
  }
}
```

- [ ] **Step 3: Add guard parsing to parse_match_arm**

In `parse_match_arm` (lines 1307-1312), after parsing the pattern and before expecting `TK_FAT_ARROW`, check for `if`:

```sans
parse_match_arm(p:I) I {
  pattern = parse_pattern(p)
  guard = if p_at(p, TK_IF) == 1 {
    p_advance(p)
    parse_expr(p, 0)
  } else { 0 }
  p_expect(p, TK_FAT_ARROW)
  body = parse_expr(p, 0)
  arm = make_match_arm(pattern, body)
  store64(arm + 16, guard)
  arm
}
```

The guard expression is stored at offset 16 in the match arm (the reserved slot).

- [ ] **Step 4: Commit parser changes**

```bash
git add compiler/parser.sans compiler/constants.sans
git commit -m "feat(parser): parse binding patterns and match arm guards"
```

### Task 14: Typeck — type-check guards

**Files:**
- Modify: `compiler/typeck.sans:2315-2383` (match type checking)

- [ ] **Step 1: Handle binding patterns in value match**

In the value-match path (lines 2322-2333), when checking arms, add support for binding patterns. Currently only literal and wildcard patterns are handled. When a binding pattern is encountered (pat_type == 4), add the binding to the arm's scope:

After checking the scrutinee type and before checking arm bodies, add guard checking. For each arm:

```sans
// Read guard expression from arm offset 16
guard_expr = load64(arm + 16)
// If pattern is a binding pattern (pat_type == 4), add binding to scope
pat_type = load64(pattern + 24)
if pat_type == 4 {
  push_scope(locals)
  bind_name = load64(pattern + 8)
  inner_scope = locals[locals.len() - 1]
  mset(inner_scope, bind_name, scrutinee_ty)
}
// Type-check guard if present
if guard_expr != 0 {
  guard_ty = check_expr(guard_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if type_tag(guard_ty) != TY_BOOL { tc_error("match guard must be Bool") }
}
// Type-check body
arm_ty = check_expr(arm_body, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
if pat_type == 4 { pop_scope(locals) }
```

- [ ] **Step 2: Handle guards in enum match path**

In the enum match path (lines 2335-2382), after binding pattern variables with `push_scope` (line 2364), check the guard before checking the body:

```sans
// After push_scope and binding pattern vars...
guard_expr = load64(arm + 16)
if guard_expr != 0 {
  guard_ty = check_expr(guard_expr, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if type_tag(guard_ty) != TY_BOOL { tc_error("match guard must be Bool") }
}
arm_ty = check_expr(arm_body, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
```

- [ ] **Step 3: Commit typeck changes**

```bash
git add compiler/typeck.sans
git commit -m "feat(typeck): type-check match guards and binding patterns"
```

### Task 15: IR — lower binding patterns and guards

**Files:**
- Modify: `compiler/ir.sans:2723-2794` (lower_match_value — add binding + guard)
- Modify: `compiler/ir.sans:2631-2720` (lower_match — add guard to enum match)

- [ ] **Step 1: Add binding pattern + guard to lower_match_value**

In `lower_match_value` (lines 2723-2794), currently each arm does:
- pat_type==1 (int): compare, branch
- pat_type==2 (string): compare, branch
- pat_type==3 (wildcard): always match

Add pat_type==4 (binding): always match (like wildcard) but bind the scrutinee to a local. Then for ALL pattern types, after the pattern matches, check the guard:

For each non-last arm, the current flow is:
1. Check pattern → branch to arm_label or next_label
2. Emit arm_label, lower body, jump to merge

With guards, it becomes:
1. Check pattern → branch to arm_label or next_label
2. Emit arm_label
3. If binding pattern: `ctx_set_local(ctx, bind_name, scrutinee_reg)`
4. If guard present: lower guard expr, branch on result to body_label or next_label (fallthrough)
5. Emit body_label, lower body, jump to merge

The key change: when a guard is present AND the guard fails, we need to jump to the next arm's pattern check, not just fall through. This means each arm with a guard needs an additional label.

```sans
// For binding pattern (pat_type == 4):
if pt == 4 {
  bind_name = load64(pat + 8)
  ctx_set_local(ctx, bind_name, scrutinee_reg)
}

// Check guard (for any pattern type)
guard_expr = load64(arm + 16)
if guard_expr != 0 {
  guard_reg = lower_expr(ctx, guard_expr)
  body_label = ctx_fresh_label(ctx)
  ctx_emit(ctx, ir_branch(guard_reg, body_label, next_label))
  ctx_emit(ctx, ir_label(body_label))
}
```

- [ ] **Step 2: Add guard to enum match (lower_match)**

In `lower_match` (lines 2631-2720), after `lower_match_bindings` sets up bindings, check the guard. If guard is false, jump to next arm:

```sans
// After lower_match_bindings...
guard_expr = load64(arm + 16)
if guard_expr != 0 {
  guard_reg = lower_expr(ctx, guard_expr)
  body_label = ctx_fresh_label(ctx)
  ctx_emit(ctx, ir_branch(guard_reg, body_label, next_label))
  ctx_emit(ctx, ir_label(body_label))
}
// Lower body expression...
```

For the last arm in enum match, if it has a guard, we need a fallthrough target. Since the last arm is supposed to be exhaustive, a guard on the last arm is unusual. Create a `trap_label` that emits `unreachable` (LLVM intrinsic) if the guard fails — this prevents invalid IR. Same applies to `lower_match_value`.

```sans
// For last arm with guard:
guard_expr = load64(arm + 16)
if guard_expr != 0 {
  guard_reg = lower_expr(ctx, guard_expr)
  body_label = ctx_fresh_label(ctx)
  trap_label = ctx_fresh_label(ctx)
  ctx_emit(ctx, ir_branch(guard_reg, body_label, trap_label))
  ctx_emit(ctx, ir_label(body_label))
  // ... lower body ...
  // After merge label, emit trap:
  ctx_emit(ctx, ir_label(trap_label))
  ctx_emit(ctx, ir_inst0(IR_EXIT, 0))  // or unreachable
}
```

- [ ] **Step 3: Commit IR changes**

```bash
git add compiler/ir.sans
git commit -m "feat(ir): lower binding patterns and match guards with fallthrough"
```

### Task 16: Build, test, and verify match guards

- [ ] **Step 1: Build the compiler**

Run: `sans build compiler/main.sans`

- [ ] **Step 2: Run the match guard test**

Run: `bash tests/run_tests.sh 2>&1 | grep match_guard`
Expected: `✓  match_guard`

- [ ] **Step 3: Run full test suite**

Run: `bash tests/run_tests.sh`
Expected: All tests pass, including existing match tests (`enum_basic`, `enum_data`, `enum_match_method`).

- [ ] **Step 4: Commit any fixes**

---

## Chunk 4: User-Defined Generic Structs

### Task 17: Test fixture for generic structs

**Files:**
- Create: `tests/fixtures/generic_struct.sans`
- Modify: `tests/run_tests.sh`

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
  b = Box<I>{value:10}
  p = Pair<I I>{first:3 second:7}
  b.value + p.first + p.second
}
```

Expected exit: 20 (10 + 3 + 7).

- [ ] **Step 2: Register test**

```bash
run_test "generic_struct"             "$REPO_ROOT/tests/fixtures/generic_struct.sans"             20
```

- [ ] **Step 3: Verify test fails, commit**

```bash
git add tests/fixtures/generic_struct.sans tests/run_tests.sh
git commit -m "test: add fixture for generic structs"
```

### Task 18: Parser — parse type params on struct definitions and generic struct literals

**Files:**
- Modify: `compiler/parser.sans:1527-1544` (parse_struct_def)
- Modify: `compiler/parser.sans:358-364` (make_struct_def)
- Modify: `compiler/parser.sans:1069-1086` (struct literal parsing in parse_atom)

- [ ] **Step 1: Store type params in make_struct_def**

The reserved slot at offset 16 will hold the type params array (or 0 for non-generic).

No change to `make_struct_def` layout needed — offset 16 is already there and set to 0. We just store the type params there.

- [ ] **Step 2: Parse type params in parse_struct_def**

In `parse_struct_def` (lines 1527-1544), after parsing the name and before expecting `{`, check for `<...>`:

```sans
parse_struct_def(p:I) I {
  p_expect(p, TK_STRUCT)
  name = p_expect_ident(p)

  // Parse optional type params <A, B, ...>
  type_params = array<I>()
  if p_at(p, TK_LT) == 1 {
    p_advance(p)
    while p_at(p, TK_GT) == 0 && p_at(p, TK_EOF) == 0 {
      tp = p_expect_ident(p)
      type_params.push(tp)
      if p_at(p, TK_COMMA) == 1 { p_advance(p) }
    }
    p_expect(p, TK_GT)
  }

  p_expect(p, TK_LBRACE)
  // ... existing field parsing ...
  p_expect(p, TK_RBRACE)

  sdef = make_struct_def(name, fields)
  if type_params.len() > 0 { store64(sdef + 16, ptr(type_params)) }
  sdef
}
```

- [ ] **Step 3: Update struct literal parsing for generic types**

In `parse_atom` (lines 1069-1086), the struct literal detection checks for `Name{`. For generic structs like `Pair<I S>{...}`, after parsing the identifier name, check for `<`:

```sans
// After getting 'name' from identifier...
// Check for generic struct literal: Name<T1, T2>{field: value}
// IMPORTANT: `<` is also the less-than operator. To disambiguate, we use
// a lookahead heuristic: after `Name<`, the next token should be an uppercase
// type name (I, S, B, F, Int, String, etc.) — not a number or lowercase var.
// This works because generic struct literals always have type names as arguments.
if p_at(p, TK_LT) == 1 && is_upper_start(name) == 1 && p_peek_is_type_name(p, 1) == 1 {
  // Save parser position in case we need to bail out
  // Parse type args: <I, S, ...>
  p_advance(p)  // consume <
  type_args = array<I>()
  while p_at(p, TK_GT) == 0 && p_at(p, TK_EOF) == 0 {
    ta = parse_type_name(p)
    type_args.push(ta)
    if p_at(p, TK_COMMA) == 1 { p_advance(p) }
  }
  p_expect(p, TK_GT)  // consume >

  // Require { immediately after > for struct literal
  if p_at(p, TK_LBRACE) == 1 {
    // Build mangled name: Pair$$I$$S (using $$ as separator to avoid
    // collision with underscores in type names)
    mangled = name
    tai := 0
    while tai < type_args.len() {
      mangled = mangled + "$$" + type_args.get(tai)
      tai += 1
      0
    }
    // Parse fields normally
    p_advance(p)  // consume {
    field_names = array<I>()
    field_values = array<I>()
    while p_at(p, TK_RBRACE) == 0 && p_at(p, TK_EOF) == 0 {
      fname = p_expect_ident(p)
      p_expect(p, TK_COLON)
      fval = parse_expr(p, 0)
      field_names.push(fname)
      field_values.push(fval)
      if p_at(p, TK_COMMA) == 1 { p_advance(p) }
    }
    p_expect(p, TK_RBRACE)
    return make_struct_lit(mangled, field_names, field_values)
  }
}
```

This approach mangles the name at parse time so the rest of the pipeline sees a concrete struct name like `Pair$$I$$S`.

- [ ] **Step 4: Commit parser changes**

```bash
git add compiler/parser.sans
git commit -m "feat(parser): parse generic struct definitions and generic struct literals"
```

### Task 19: Typeck — two-phase generic struct registration

**Files:**
- Modify: `compiler/typeck.sans:2583-2605` (struct registration pass)
- Modify: `compiler/typeck.sans:2239-2281` (struct literal checking)

- [ ] **Step 1: Phase 1 — register generic struct templates**

In the struct registration pass (lines 2583-2605), check for type params at offset 16. If present, store as a template instead of a concrete struct:

```sans
// In the struct registration loop...
sdef = load64(structs_p + si * 8)
sname = load64(sdef)
type_params_ptr = load64(sdef + 16)

if type_params_ptr != 0 {
  // Generic struct — store template for later instantiation
  // Template: the raw struct def with unresolved field types
  mset(generic_structs, sname, sdef)
} else {
  // Non-generic — register as before
  // ... existing field resolution code ...
  mset(structs_map, sname, fields_m)
}
```

Add `generic_structs = map()` alongside the other maps at the top of the typeck main function.

- [ ] **Step 2: Phase 2 — instantiate on use**

In struct literal checking (lines 2239-2281), when a struct name like `Pair$$I$$S` is not found in `structs_map`, check if the base name (`Pair`) is in `generic_structs`. If so, parse the type args from the mangled name and instantiate:

```sans
if mhas(structs, sname) == 0 {
  // Try generic struct instantiation
  // Parse mangled name: "Pair$$I$$S" -> base="Pair", args=["I", "S"]
  base_name = split_generic_name(sname)  // extract base name before first "$$"
  if mhas(generic_structs, base_name) {
    template = mget(generic_structs, base_name)
    type_args = parse_mangled_type_args(sname)  // extract ["I", "S"] from after "__"
    type_params = load64(template + 16)
    // Build type param -> concrete type map
    tp_map = map()
    tpi := 0
    while tpi < type_args.len() {
      tp_name = load64(load64(type_params) + tpi * 8)
      tp_concrete = resolve_type(type_args.get(tpi), structs_map, enums_map, mod_exports)
      mset(tp_map, tp_name, tp_concrete)
      tpi += 1
      0
    }
    // Resolve fields with substituted types
    fields_flat_arr = load64(template + 8)
    fields_flat = load64(fields_flat_arr)
    num_flat = load64(fields_flat_arr + 8)
    fields_m = map()
    fi := 0
    while fi < num_flat {
      fname = load64(fields_flat + fi * 8)
      ftype_name = load64(fields_flat + (fi + 1) * 8)
      fty = if mhas(tp_map, ftype_name) { mget(tp_map, ftype_name) } else { resolve_type(ftype_name, structs_map, enums_map, mod_exports) }
      mset(fields_m, fname, fty)
      fi += 2
    }
    mset(structs_map, sname, fields_m)
  }
}
```

You'll need helper functions `split_generic_name` and `parse_mangled_type_args` that parse the mangled name. These are simple string operations using `$$` as separator:
- `split_generic_name("Pair$$I$$S")` → `"Pair"` (everything before first `"$$"`)
- `parse_mangled_type_args("Pair$$I$$S")` → `["I", "S"]` (split after `"__"` by `"_"`)

- [ ] **Step 3: Commit typeck changes**

```bash
git add compiler/typeck.sans
git commit -m "feat(typeck): two-phase generic struct registration and instantiation"
```

### Task 20: Build, test, and verify generic structs

- [ ] **Step 1: Build the compiler**

Run: `sans build compiler/main.sans`

- [ ] **Step 2: Run the generic struct test**

Run: `bash tests/run_tests.sh 2>&1 | grep generic_struct`
Expected: `✓  generic_struct`

- [ ] **Step 3: Run full test suite**

Run: `bash tests/run_tests.sh`
Expected: All tests pass.

- [ ] **Step 4: Commit any fixes**

---

## Chunk 5: For-Loop Destructuring (Map Entries)

### Task 21: Test fixture for for-loop destructuring

**Files:**
- Create: `tests/fixtures/for_destructure_map.sans`
- Modify: `tests/run_tests.sh`

- [ ] **Step 1: Write test fixture**

```sans
main() {
  m = {"a": 1 "b": 2 "c": 3}
  total := 0
  for (k v) in m.entries() {
    total += v
  }
  total
}
```

Expected exit: 6 (1 + 2 + 3).

- [ ] **Step 2: Register test**

```bash
run_test "for_destructure_map"        "$REPO_ROOT/tests/fixtures/for_destructure_map.sans"        6
```

- [ ] **Step 3: Verify test fails, commit**

```bash
git add tests/fixtures/for_destructure_map.sans tests/run_tests.sh
git commit -m "test: add fixture for for-loop destructuring with map entries"
```

### Task 22: Parser — parse destructured for-in

**Files:**
- Modify: `compiler/constants.sans` (add ST_FOR_IN_DESTR = 10)
- Modify: `compiler/parser.sans:726-735` (for-in parsing)
- Add: new `make_st_for_in_destr` function in parser.sans

- [ ] **Step 1: Add ST_FOR_IN_DESTR constant**

In `compiler/constants.sans`, after `ST_EXPR = 9` (line 102), add:

```sans
g ST_FOR_IN_DESTR = 10
```

- [ ] **Step 2: Add make_st_for_in_destr in parser.sans**

After `make_st_for_in` (line 321), add:

```sans
make_st_for_in_destr(var1:I var2:I iterable_expr:I body_stmts:I) I {
  n = alloc(40)
  store64(n, ST_FOR_IN_DESTR)
  store64(n + 8, var1)
  store64(n + 16, var2)
  store64(n + 24, iterable_expr)
  store64(n + 32, ptr(body_stmts))
  n
}
```

- [ ] **Step 3: Update for-in parsing to detect (k v) pattern**

In the for-loop parsing (lines 726-735), after consuming `TK_FOR`, check for `(`:

```sans
if kind == TK_FOR {
  p_advance(p)
  if p_at(p, TK_LPAREN) == 1 {
    // Destructuring: for (k v) in expr { ... }
    p_advance(p)  // consume (
    var1 = p_expect_ident(p)
    var2 = p_expect_ident(p)
    p_expect(p, TK_RPAREN)
    p_expect(p, TK_IN)
    iterable = parse_expr(p, 0)
    p_expect(p, TK_LBRACE)
    body = parse_body(p)
    p_expect(p, TK_RBRACE)
    return make_st_for_in_destr(var1, var2, iterable, body)
  } else {
    // Regular: for x in expr { ... }
    var_name = p_expect_ident(p)
    p_expect(p, TK_IN)
    iterable = parse_expr(p, 0)
    p_expect(p, TK_LBRACE)
    body = parse_body(p)
    p_expect(p, TK_RBRACE)
    return make_st_for_in(var_name, iterable, body)
  }
}
```

- [ ] **Step 4: Commit parser changes**

```bash
git add compiler/parser.sans compiler/constants.sans
git commit -m "feat(parser): parse destructured for-in loops with (k v) pattern"
```

### Task 23: Typeck — type-check destructured for-in

**Files:**
- Modify: `compiler/typeck.sans:548-563` (add ST_FOR_IN_DESTR handling)

- [ ] **Step 1: Add ST_FOR_IN_DESTR case in check_stmt**

After the existing `ST_FOR_IN` case (line 548-563), add:

```sans
if tag == ST_FOR_IN_DESTR {
  var1 = load64(stmt + 8)
  var2 = load64(stmt + 16)
  iterable = load64(stmt + 24)
  body_arr = load64(stmt + 32)
  iter_ty = check_expr(iterable, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  if type_tag(iter_ty) != TY_ARRAY { tc_error("for-in destructuring requires Array, got " + type_to_string(iter_ty)) }
  inner = type_inner(iter_ty)
  if type_tag(inner) != TY_TUPLE { tc_error("for-in destructuring requires Array of tuples, got Array of " + type_to_string(inner)) }
  push_scope(locals)
  inner_scope = locals[locals.len() - 1]
  // For map entries, tuples are (String, V) — first element is key type, second is value type
  // Since type_inner of TY_TUPLE doesn't carry detailed field types currently,
  // use TY_STRING for key and TY_INT for value (map entries are always String keys)
  mset(inner_scope, var1, make_type(TY_STRING))
  mset(inner_scope, var2, make_type(TY_INT))
  body_len = if body_arr != 0 { load64(body_arr + 8) } else { 0 }
  body_p = if body_arr != 0 { load64(body_arr) } else { 0 }
  check_stmts(body_p, body_len, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
  pop_scope(locals)
  return make_type(TY_VOID)
}
```

Note: The exact types of k and v depend on the map type. Since Sans maps currently have String keys and Int values in most use cases, and the tuple type doesn't carry sub-type info, we use TY_STRING for the key and TY_INT for the value. This may need refinement based on how the existing `.entries()` typing works. Check the actual type_inner behavior for TY_TUPLE.

- [ ] **Step 2: Commit typeck changes**

```bash
git add compiler/typeck.sans
git commit -m "feat(typeck): type-check destructured for-in loops"
```

### Task 24: IR — lower destructured for-in

**Files:**
- Modify: `compiler/ir.sans` (add lower_for_in_destr, add ST_FOR_IN_DESTR dispatch)

- [ ] **Step 1: Add lower_for_in_destr function**

Add near `lower_for_in` (after line 1502):

```sans
lower_for_in_destr(ctx:I stmt:I) I {
  var1 = load64(stmt + 8)
  var2 = load64(stmt + 16)
  iterable = load64(stmt + 24)
  body_arr = load64(stmt + 32)
  body_p = load64(body_arr)
  body_len = load64(body_arr + 8)

  // Lower iterable
  arr_reg = lower_expr(ctx, iterable)

  // Get array length
  len_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst1(IR_ARRAY_LEN, len_reg, arr_reg))
  ctx_set_reg_type(ctx, len_reg, IRTY_INT)

  // Allocate index
  idx_ptr = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_alloca(idx_ptr))
  zero_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(zero_reg, 0))
  ctx_emit(ctx, ir_store(idx_ptr, zero_reg))

  // Labels
  cond_label = ctx_fresh_label(ctx)
  body_label = ctx_fresh_label(ctx)
  end_label = ctx_fresh_label(ctx)

  old_break = ctx_break_label(ctx)
  old_continue = ctx_continue_label(ctx)
  ctx_set_break_label(ctx, end_label)
  ctx_set_continue_label(ctx, cond_label)

  // Condition
  ctx_emit(ctx, ir_jump(cond_label))
  ctx_emit(ctx, ir_label(cond_label))
  idx_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_load(idx_reg, idx_ptr))
  cmp_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst3(IR_CMPOP, cmp_reg, 0, idx_reg, len_reg))
  ctx_emit(ctx, ir_branch(cmp_reg, body_label, end_label))

  // Body
  ctx_emit(ctx, ir_label(body_label))
  idx_reg2 = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_load(idx_reg2, idx_ptr))
  elem_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst2(IR_ARRAY_GET, elem_reg, arr_reg, idx_reg2))
  ctx_set_reg_type(ctx, elem_reg, IRTY_TUPLE)

  // Destructure tuple: field 0 = key, field 1 = value
  key_reg = ctx_fresh_reg(ctx)
  val_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_field_load(key_reg, elem_reg, 0, 2))
  ctx_emit(ctx, ir_field_load(val_reg, elem_reg, 1, 2))
  ctx_set_reg_type(ctx, key_reg, IRTY_STR)
  ctx_set_reg_type(ctx, val_reg, IRTY_INT)

  // Bind loop variables
  ctx_set_local(ctx, var1, key_reg)
  ctx_set_local(ctx, var2, val_reg)

  // Lower body
  bi := 0
  while bi < body_len {
    lower_stmt(ctx, load64(body_p + bi * 8))
    bi += 1
    0
  }

  // Increment index
  idx_reg3 = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_load(idx_reg3, idx_ptr))
  one_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_const(one_reg, 1))
  inc_reg = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst3(IR_BINOP, inc_reg, 0, idx_reg3, one_reg))
  ctx_emit(ctx, ir_store(idx_ptr, inc_reg))
  ctx_emit(ctx, ir_jump(cond_label))

  // End
  ctx_emit(ctx, ir_label(end_label))
  ctx_set_break_label(ctx, old_break)
  ctx_set_continue_label(ctx, old_continue)
  0
}
```

- [ ] **Step 2: Add ST_FOR_IN_DESTR dispatch in lower_stmt**

Find where `ST_FOR_IN` is dispatched in `lower_stmt` (search for `ST_FOR_IN` in ir.sans). Add:

```sans
if tag == ST_FOR_IN_DESTR { return lower_for_in_destr(ctx, stmt) }
```

Right after the existing `ST_FOR_IN` dispatch.

- [ ] **Step 3: Commit IR changes**

```bash
git add compiler/ir.sans
git commit -m "feat(ir): lower destructured for-in loops with tuple field extraction"
```

### Task 25: Build, test, and verify for-loop destructuring

- [ ] **Step 1: Build the compiler**

Run: `sans build compiler/main.sans`

- [ ] **Step 2: Run the destructuring test**

Run: `bash tests/run_tests.sh 2>&1 | grep for_destructure_map`
Expected: `✓  for_destructure_map`

- [ ] **Step 3: Run full test suite**

Run: `bash tests/run_tests.sh`
Expected: All 90+ existing tests pass, plus all 5 new tests pass.

- [ ] **Step 4: Commit any fixes**

---

## Chunk 6: Documentation and Final Verification

### Task 26: Update documentation

**Files:**
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `website/docs/index.html`
- Modify: `editors/vscode-sans/src/extension.ts` (HOVER_DATA)
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update docs/reference.md**

Add sections for each new feature with examples:
- Default parameters: `f(x:I y:I=0)` syntax, rules
- Error codes: `err(404 "msg")`, `.code()` method
- Generic structs: `struct Pair<A B> { ... }`, instantiation
- Match guards: `n if n > 0 => ...`, binding patterns
- For-loop destructuring: `for (k v) in m.entries()`

- [ ] **Step 2: Update docs/ai-reference.md**

Add compact entries using short-form syntax:
```
f(x:I y:I=0) = x+y         // default params
err(404 "msg")              // error code
r.code()                    // get error code
struct Pair<A B>{a:A b:B}   // generic struct
match x { n if n>0 => n }   // guard
for (k v) in m.entries()     // destructure
```

- [ ] **Step 3: Update HOVER_DATA in vscode extension**

Add entries for: `err` (update to mention error codes), `code` (new method), generic struct syntax.

- [ ] **Step 4: Update CHANGELOG.md**

Add a new `## [Unreleased]` section at the top with all five features listed. Do NOT set a version number — version is managed by CI when a git tag is pushed.

- [ ] **Step 5: Update website docs**

Mirror reference.md changes in `website/docs/index.html`.

- [ ] **Step 6: Commit all docs**

```bash
git add docs/ website/ editors/ CHANGELOG.md
git commit -m "docs: add documentation for v0.5.2 language features"
```

### Task 27: Final verification

- [ ] **Step 1: Run full test suite**

Run: `bash tests/run_tests.sh`
Expected: All tests pass (90+ existing + 5 new).

- [ ] **Step 2: Test each feature manually with a combined example**

Create a quick manual test combining multiple features:

```sans
struct Pair<A B> { first:A second:B }

safe_div(a:I b:I=1) R<I> {
  if b == 0 { err(400 "div by zero") } else { ok(a / b) }
}

main() {
  r = safe_div(10 0)
  result = match r.code() {
    c if c > 0 => c
    _ => 0
  }
  m = {"x": 1 "y": 2}
  total := 0
  for (k v) in m.entries() { total += v }
  p = Pair<I I>{first:result second:total}
  p.first + p.second
}
```

Run: `sans build test.sans -o /tmp/test_combined && /tmp/test_combined; echo $?`
Expected: 403 (400 + 3)

- [ ] **Step 3: Clean up temp files**
