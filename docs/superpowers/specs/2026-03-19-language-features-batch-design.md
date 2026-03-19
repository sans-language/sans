# Language Features Batch — Design Spec (v0.5.x)

Five language features for AI-generated code, all shipping under 0.5.x.

## 1. Default Function Parameters

**Syntax:** Trailing params can have `=literal` defaults.

```sans
greet(name:S greeting:S="Hello") = p(greeting + " " + name)
greet("World")           // "Hello World"
greet("World" "Hi")      // "Hi World"

connect(host:S port:I=8080 tls:B=false) {
  // ...
}
connect("example.com")
connect("example.com" 443 true)
```

**Supported defaults:** Int, Float, String, Bool literals only. Lambdas cannot have default params.

**Rules:**
- Non-default params must come before default params
- Caller fills args left-to-right; omitted trailing args get defaults
- Default literal type must match param's declared type

**Implementation pipeline:**
1. **Parser:** After param type, check for `=` token → parse literal as an AST expression node (via `parse_primary`) → store at offset 16 (the reserved slot) in the 24-byte param layout. Null means no default.
2. **Typeck:** Validate default expression type matches param type. Enforce ordering (no required params after a default param). Accept calls with `required_count <= argc <= total_count`.
3. **IR:** Add a function-name → param-defaults map in IR context. When lowering a call with fewer args than params, look up the callee's defaults and call `lower_expr` on each default AST node to fill missing arg registers.
4. **Codegen:** No changes — IR supplies all arg registers.

## 2. Error Codes on Result

**Syntax:** `err()` gains an optional integer code. Disambiguated by first-arg type: if Int, treat as `(code, msg)`; if String, treat as `(msg)` with code=0.

```sans
err("not found")           // code=0 (backwards compatible)
err(404 "not found")       // code=404

r = do_thing()
if r.is_err() {
  match r.code() {
    404 => p("missing")
    500 => p("server error")
    _ => p(r.error())
  }
}
```

**New method:** `.code() -> I` — returns error code (0 if unset).

**Backwards compatibility:** Existing `err("msg")` continues to work with implicit code 0.

**Implementation pipeline:**
1. **Runtime (`result.sans`):** Expand Result layout from 24 → 32 bytes for BOTH `sans_result_ok` and `sans_result_err`. New field: error code at offset 24. `sans_result_ok` must zero-initialize offset 24. `sans_result_err(code, msg)` always takes 2 args. Add `sans_result_code(r)` to read offset 24.
2. **Typeck:** Add `.code()` method on Result returning `I`. `err()` accepts 1 arg (string, code=0) or 2 args (int + string).
3. **IR:** Always emit 2-arg calls to `sans_result_err`. When user writes `err("msg")`, synthesize `ir_const(0)` as the first arg. Add `IR_RESULT_CODE` instruction constant. Lower `.code()` calls to it.
4. **Codegen:** Compile `IR_RESULT_CODE` — load i64 from result pointer + 24.

## 3. User-Defined Generic Structs

**Syntax:** Struct definitions with type params in angle brackets.

```sans
struct Pair<A B> {
  first:A
  second:B
}

pair = Pair<I S>{first:1 second:"hello"}
p(pair.first)    // 1
p(pair.second)   // hello

struct Box<T> {
  value:T
}
b = Box<I>{value:42}
```

**Monomorphization:** Each unique instantiation (e.g. `Pair<I S>`) creates a concrete struct type with mangled name (`Pair__I_S`). Same approach as existing function generics.

**Rules:**
- Multiple type params supported: `<A B>`, `<A B C>`
- Type params resolve to concrete types at use site
- `Pair<I S>` and `Pair<S I>` are distinct types
- No generic methods in this iteration — just field access and construction
- No nested generics (e.g. `Box<Pair<I S>>` is not supported in this iteration)
- Generic struct literal syntax requires `{` immediately after closing `>` (no intervening operators)

**Implementation pipeline:**
1. **Parser:** After struct name, check for `<...>` → parse type param names → store at offset 16 (the reserved slot) in `make_struct_def`. Null means non-generic struct.
2. **Typeck — two-phase approach:**
   - **Phase 1 (registration):** When walking struct definitions, if type params are present, register as a generic struct *template* (store name + type params + unresolved fields). Do not create a concrete struct entry yet.
   - **Phase 2 (instantiation):** When a usage like `Pair<I S>` appears (struct literal or type annotation), look up the template, substitute type params with concrete types, create a monomorphized struct entry (`Pair__I_S`) with resolved field types.
3. **IR:** Monomorphized structs lower identically to regular structs — no new instructions.
4. **Codegen:** No changes — mangled struct names produce normal LLVM struct types.

## 4. Pattern Match Guards

**Syntax:** `if` condition after pattern, before `=>`.

```sans
match x {
  n if n > 0 => p("positive")
  n if n < 0 => p("negative")
  _ => p("zero")
}
```

Works with all pattern types:

```sans
match result {
  Ok(v) if v > 100 => p("big value")
  Ok(v) => p("small: " + v.to_s())
  Err(e) => p(e)
}

match cmd {
  "get" if authenticated => handle_get()
  "get" => err(401 "unauthorized")
  _ => err(400 "unknown command")
}
```

**Rules:**
- Guard expression must resolve to Bool
- Guard can reference bindings introduced by the pattern (e.g. `n`, `v`)
- If guard is false, fall through to next arm
- Guards do not affect exhaustiveness — wildcard `_` still catches all

**Prerequisite — variable binding patterns for value match:**
Currently, value-match patterns only support literals (`42`, `"hello"`) and wildcards (`_`). The syntax `n if n > 0` requires a new pattern type: a bare identifier that binds the scrutinee value to a name. This "binding pattern" must be added alongside guards:
- **Parser:** When parsing a match pattern, if the token is a lowercase identifier (not a keyword, not an enum name), parse it as a binding pattern that captures the scrutinee.
- **Typeck:** Binding pattern introduces the variable with the scrutinee's type into the arm's scope.
- **IR (value match):** Binding pattern always matches (like wildcard) but also stores the scrutinee register under the bound name in the local context.
- **IR (enum match):** Enum patterns already support bindings — no change needed. Guard is evaluated after `lower_match_bindings` sets up the bindings, before the body.

**Implementation pipeline:**
1. **Parser:** After parsing pattern, check for `if` token → parse expression → store at offset 16 (reserved slot) in `make_match_arm`. Also add binding-pattern parsing (bare lowercase ident).
2. **Typeck:** Type-check guard in scope that includes pattern bindings. Must resolve to Bool.
3. **IR:** After pattern match succeeds and bindings are set up, emit guard condition check. If false, jump to next arm. Each arm becomes: check pattern → set up bindings → check guard → execute body, with fallthrough on guard failure.
4. **Codegen:** No changes — conditional branching already exists in IR.

## 5. Destructuring in For Loops (Map Entries)

**Syntax:** `for (k v) in m.entries()` for map key-value iteration.

```sans
m = {"a": 1 "b": 2 "c": 3}
for (k v) in m.entries() {
  p(k + ": " + v.to_s())
}
```

**Existing support:** `.entries()` method and `sans_map_entries` runtime function already exist. They return `[(K V)]` — an array of 16-byte tuples (key at offset 0, value at offset 8). No runtime changes needed.

**Rules:**
- `(k v)` destructures 2-element tuples from the iterable
- Types inferred from the iterable's inner tuple type
- Only 2-element destructuring (matches existing tuple support)

**Implementation pipeline:**
1. **Parser:** When parsing `for`, check for `(ident ident)` pattern. Create a new AST node type `ST_FOR_IN_DESTR` (distinct from `ST_FOR_IN`) with two name slots — var1 at offset 8, var2 at offset 16, iterable at offset 24, body at offset 32.
2. **Typeck:** When `for (a b) in expr` is seen, resolve iterable type. It must be an array of tuples. Infer element types from the tuple's inner types. Bind `a` to tuple field 0 type, `b` to tuple field 1 type.
3. **IR:** Add `lower_for_in_destr`. Same loop structure as `lower_for_in` (index alloca, len check, increment). After getting each element (a tuple pointer), load field 0 (offset 0) and field 1 (offset 8) into separate registers and bind to the two variable names. Track element types as tuple inner types, not `IRTY_INT`.
4. **Codegen:** No changes — tuple field extraction already exists.

## Implementation Order

Recommended order based on dependencies and complexity:

1. **Default params** — self-contained, no dependencies
2. **Error codes on Result** — self-contained, extends existing Result
3. **Match guards** (includes binding patterns) — self-contained, extends existing match
4. **Generic structs** — extends existing generic infrastructure
5. **For-loop destructuring** — uses existing .entries() runtime

## Testing Strategy

Each feature gets at least one E2E fixture in `tests/fixtures/`:

1. `default_params.sans` — basic defaults, partial calls, all literal types
2. `result_error_code.sans` — err with code, .code() method, match on code, backwards compat
3. `match_guard.sans` — guards on binding patterns, enum patterns, literal patterns
4. `generic_struct.sans` — Pair<A B>, Box<T>, field access, multiple instantiations
5. `for_destructure_map.sans` — map entries iteration with (k v)
