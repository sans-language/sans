# Sans Self-Hosting Roadmap

**Goal:** Add language features needed to rewrite the Sans compiler in Sans instead of Rust.

**Constraint:** All syntax must be AI-optimized — fewest tokens possible. Sans is an AI-first language.

---

## Current State

The Sans compiler is ~11K lines of Rust across 6 crates. It uses these Rust features that Sans doesn't yet support:
- HashMap (pervasive — symbol tables, registries, type tracking)
- Tuples (compound keys, multi-return, grouped values)
- Closures with captures (iterator chains, callbacks)
- Iterator chains (map/filter/collect patterns)
- String slicing and format expressions
- Error propagation (`?` operator)

## Version Plan

### v0.3.3 — Tuples
```sans
t = (1 "hi" true)           // literal, no commas
x = t.0                      // positional access
f(a:I) (I S) = (a str(a))   // multi-return
```
**Type syntax:** `(I S B)`
**Why first:** Needed by Map (compound keys) and throughout compiler for grouped values. No dependencies.

### v0.3.4 — Closures with Captures
```sans
offset = 10
add = |x:I| I { x + offset }          // captures offset implicitly
nums.map(|n:I| I { n * factor })       // captures factor
```
**Change:** Existing lambda syntax gains implicit capture — no new syntax needed.
**Why second:** Needed for iterator chains (v0.3.5). No dependencies.

### v0.3.5 — Iterator Chains (auto-materialized)
```sans
a.map(|x:I| I { x * 2 }).filter(|x:I| B { x > 5 })  // returns [I]
a.enumerate()     // returns [(I T)]
a.find(|x:I| B { x > 3 })    // returns first match or 0
a.any(|x:I| B { x > 3 })     // returns B
a.zip(b)          // returns [(T T)]
```
**Key decision:** Chains auto-materialize to arrays. No `.collect()` needed — saves tokens.
**Depends on:** Closures (v0.3.4), Tuples (v0.3.3 — for enumerate/zip return types).

### v0.3.6 — String Slicing + Expression Interpolation
```sans
s = "hello world"
s[0:5]                        // "hello"
s[6:]                         // "world"
s[:5]                         // "hello"
name = "reg_{n+1}"            // expression interpolation
msg = "got {items.len()} items"
```
**Why here:** Compiler needs substring extraction and name generation. No dependencies on prior features.

### v0.3.7 — Map Type
```sans
m = M<S I>{}                  // empty map
m = {"a":1 "b":2}             // literal, no commas
m.set("c" 3)                  // set
v = m.get("a")                // get
m.has("b")                    // returns B
m.keys()                      // returns [S]
m.vals()                      // returns [I]
m.each(|k:S v:I| I { p("{k}={v}") })  // iterate
m.len()                       // count
```
**Implementation:** Built on the hash table from json.sans (open-addressing, djb2 hash).
**Depends on:** Tuples (compound keys, `.each()` pairs), Closures (`.each()` callback).

### v0.3.8 — Error Propagation (`?` operator)
```sans
parse(s:S) R<I> {
  n = to_int(s)?              // unwraps or early-returns err
  n > 0 ? ok(n) : err("must be positive")
}
```
**Sugar:** `x?` expands to `if x.is_err() { return x } else { x! }`.
**Depends on:** Existing Result type.

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tuple separators | Spaces, no commas | Matches `[1 2 3]` array style, fewer tokens |
| Closure captures | Implicit | AI shouldn't need capture lists |
| Iterator materialization | Auto (no `.collect()`) | Compiler never uses lazy iteration |
| Map type name | `M<K V>` | Short alias consistent with `I/S/B/F/R<T>` |
| Map literal syntax | `{"k":v}` | Familiar JSON-like, no commas |
| String slice syntax | `s[0:5]` | Python-style, compact |
| Expression interpolation | `"{expr}"` | Extends existing `"hello {name}"` |
| Error propagation | `?` postfix | Familiar from Rust, saves if/else boilerplate |

## After v0.3.8

With all six features, Sans has enough capability to begin self-hosting:
- HashMap → symbol tables, registries
- Tuples → compound types, multi-return
- Closures → callbacks, transforms
- Iterator chains → collection processing
- String ops → name generation, parsing
- Error propagation → clean error handling

The self-hosting effort itself would be a separate project, likely starting with the lexer (simplest crate) and working up through parser → typeck → IR → codegen.
