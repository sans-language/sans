# Iterator Protocol Design

Lazy iterator type for Sans — avoids intermediate allocations by chaining operations that execute on demand.

## Type

`Iter<T>` — new opaque built-in type.

- Type tag: `TY_ITER` (add to `constants.sans`)
- IR type: `IRTY_ITER` (add to `constants.sans`)
- Type constructor: `make_iter_type(inner)` in `typeck.sans`
- Short name: `It<T>` (consistent with `R<T>`, `O<T>`, `M<K,V>`)

`T` tracks the element type after all transformations applied so far.

## Entry Points

### `array.iter()`

Method on `Array<T>`. Returns `Iter<T>` backed by the array's data pointer and length.

```sans
a = [1 2 3 4 5]
it = a.iter()  // Iter<I>
```

### `iter(n)` and `iter(a, b)`

Built-in functions that produce `Iter<I>` directly from a range — no intermediate array allocation.

- `iter(n)` — yields `0, 1, ..., n-1`
- `iter(a, b)` — yields `a, a+1, ..., b-1`

```sans
it = iter(10)       // 0..9, no array allocated
it = iter(5, 10)    // 5..9, no array allocated
```

## Lazy Combinators

Each returns a new `Iter<U>`. No work happens until a consumer is called.

| Method | Type | Description |
|--------|------|-------------|
| `.map(fn)` | `Iter<T> -> (T->U) -> Iter<U>` | Transform each element |
| `.filter(fn)` | `Iter<T> -> (T->B) -> Iter<T>` | Keep elements where fn returns true |
| `.enumerate()` | `Iter<T> -> Iter<(I,T)>` | Yield `(index, value)` tuples |
| `.take(n)` | `Iter<T> -> I -> Iter<T>` | Yield at most n elements |
| `.skip(n)` | `Iter<T> -> I -> Iter<T>` | Drop first n elements |
| `.zip(iter)` | `Iter<T> -> Iter<U> -> Iter<(T,U)>` | Pair elements from two iterators |
| `.flat_map(fn)` | `Iter<T> -> (T->Array<U>) -> Iter<U>` | Map then flatten |

## Consumers

Terminal operations that drive evaluation through the chain.

| Method | Type | Description |
|--------|------|-------------|
| `.collect()` | `Iter<T> -> Array<T>` | Materialize into array |
| `.find(fn)` | `Iter<T> -> (T->B) -> O<T>` | First element where fn is true |
| `.any(fn)` | `Iter<T> -> (T->B) -> B` | True if any element matches (short-circuits) |
| `.all(fn)` | `Iter<T> -> (T->B) -> B` | True if all elements match (short-circuits) |
| `.reduce(fn, init)` | `Iter<T> -> ((U,T)->U, U) -> U` | Fold elements into accumulator |
| `.count()` | `Iter<T> -> I` | Count elements (consumes iterator) |
| `.for_each(fn)` | `Iter<T> -> (T->Void) -> Void` | Execute fn on each element |

## Runtime Representation

### Operation Node

Each iterator is a heap-allocated node (32 bytes):

```
[kind: I, source: I, func: I, param: I]
```

- `kind` — operation tag constant:
  - `ITER_SOURCE_ARRAY = 0` — backed by array data ptr + length
  - `ITER_SOURCE_RANGE = 1` — backed by start/end integers
  - `ITER_MAP = 2`
  - `ITER_FILTER = 3`
  - `ITER_ENUMERATE = 4`
  - `ITER_TAKE = 5`
  - `ITER_SKIP = 6`
  - `ITER_ZIP = 7`
  - `ITER_FLAT_MAP = 8`
- `source` — pointer to upstream node (null for source nodes)
- `func` — closure pointer (for map/filter/flat_map), or 0
- `param` — overloaded:
  - For ARRAY source: pointer to `[data_ptr, len, index]` state (index is mutable cursor)
  - For RANGE source: pointer to `[current, end]` state
  - For TAKE/SKIP: the count `n`
  - For ZIP: pointer to second iterator node
  - For ENUMERATE: current index counter (mutable)
  - For FLAT_MAP: pointer to current sub-array state `[data_ptr, len, index]`

### Pull-Based Evaluation

`iter_next(node)` — recursive function that pulls one element at a time:

1. **SOURCE_ARRAY**: if `index < len`, yield `data[index++]`; else done
2. **SOURCE_RANGE**: if `current < end`, yield `current++`; else done
3. **MAP**: pull from source, apply func, yield result
4. **FILTER**: pull from source in loop until func returns true or source exhausted
5. **ENUMERATE**: pull from source, yield `(counter++, value)` as tuple
6. **TAKE**: if `yielded < n`, pull from source; else done
7. **SKIP**: on first call, pull and discard `n` elements; then pass through
8. **ZIP**: pull from both sources; done when either exhausts
9. **FLAT_MAP**: pull from source, apply func to get array, yield from array; when array exhausted, pull next from source

Returns a 2-value result: `(has_value: B, value: T)`. Implementations can use `O<T>` or a sentinel.

## Runtime Implementation

New file: `runtime/iter.sans`

Provides:
- `sans_iter_from_array(data_ptr, len)` — create ARRAY source node
- `sans_iter_from_range(start, end)` — create RANGE source node
- `sans_iter_map(source, func)` — wrap in MAP node
- `sans_iter_filter(source, func)` — wrap in FILTER node
- `sans_iter_enumerate(source)` — wrap in ENUMERATE node
- `sans_iter_take(source, n)` — wrap in TAKE node
- `sans_iter_skip(source, n)` — wrap in SKIP node
- `sans_iter_zip(source, other)` — wrap in ZIP node
- `sans_iter_flat_map(source, func)` — wrap in FLAT_MAP node
- `sans_iter_collect(source, elem_type)` — pull all, build array
- `sans_iter_find(source, func)` — pull until match, return `O<T>`
- `sans_iter_any(source, func)` — short-circuit boolean
- `sans_iter_all(source, func)` — short-circuit boolean
- `sans_iter_reduce(source, func, init)` — fold
- `sans_iter_count(source)` — consume and count
- `sans_iter_for_each(source, func)` — consume with side effects

## Compiler Pipeline

### 1. constants.sans

Add:
- `TY_ITER = 23` (next available type tag)
- `IRTY_ITER = 21` (next available IR type)
- IR opcodes: `IR_ITER_FROM_ARRAY`, `IR_ITER_FROM_RANGE`, `IR_ITER_MAP`, `IR_ITER_FILTER`, `IR_ITER_ENUMERATE`, `IR_ITER_TAKE`, `IR_ITER_SKIP`, `IR_ITER_ZIP`, `IR_ITER_FLAT_MAP`, `IR_ITER_COLLECT`, `IR_ITER_FIND`, `IR_ITER_ANY`, `IR_ITER_ALL`, `IR_ITER_REDUCE`, `IR_ITER_COUNT`, `IR_ITER_FOR_EACH`

### 2. typeck.sans

- Add `make_iter_type(inner)` type constructor
- Add `.iter()` method on `TY_ARRAY` — returns `Iter<elem_type>`
- Add `iter(n)` / `iter(a, b)` built-in function — returns `Iter<I>`
- Type check all combinator methods on `TY_ITER`:
  - `.map(fn)`: verify fn takes `T`, return `Iter<return_type_of_fn>`
  - `.filter(fn)`: verify fn takes `T` returns `B`, return `Iter<T>`
  - `.enumerate()`: return `Iter<(I, T)>`
  - `.take(n)` / `.skip(n)`: verify `n` is `I`, return `Iter<T>`
  - `.zip(other)`: verify other is `Iter<U>`, return `Iter<(T, U)>`
  - `.flat_map(fn)`: verify fn takes `T` returns `Array<U>`, return `Iter<U>`
- Type check all consumer methods:
  - `.collect()`: return `Array<T>`
  - `.find(fn)`: return `O<T>`
  - `.any(fn)` / `.all(fn)`: return `B`
  - `.reduce(fn, init)`: return type of `init`
  - `.count()`: return `I`
  - `.for_each(fn)`: return `Void`

### 3. ir.sans

Lower each method call to its corresponding IR opcode. Each opcode carries the source register and any argument registers (closure, count, etc.).

### 4. codegen.sans

Each IR opcode emits a call to the corresponding `sans_iter_*` runtime function. The iterator handle is passed as i64.

### 5. runtime/iter.sans

Implement all `sans_iter_*` functions. Uses `malloc` for node allocation. Nodes are tracked by scope GC like other opaque types.

## Scope GC Integration

Iterator nodes must be tracked for scope-based cleanup:
- Each `sans_iter_*` constructor calls `rc_track()` on the allocated node
- On scope exit, iterator nodes are freed
- `.collect()` result (the array) is promoted to caller scope as usual

## For-In Loop Integration

`for x in expr` where `expr` is `Iter<T>` should work — the loop pulls via `iter_next()` instead of indexing an array. This requires:
- typeck: allow `TY_ITER` as for-in iterable
- ir.sans: emit pull-based loop for iterator sources (vs index-based for arrays)

## Examples

```sans
// Basic lazy chain
evens = [1 2 3 4 5 6].iter().filter(|x| x % 2 == 0).collect()
// evens == [2 4 6], no intermediate array

// Range without allocation
squares = iter(10).map(|x| x * x).collect()
// squares == [0 1 4 9 16 25 36 49 64 81]

// Short-circuit — stops at first match > 100
has_big = iter(1000000).map(|x| x * x).any(|x| x > 100)
// has_big == true, only evaluated ~11 elements

// Chained operations
result = data.iter()
  .filter(|x| x > 0)
  .map(|x| x * 2)
  .take(5)
  .collect()

// Enumerate
indexed = ["a" "b" "c"].iter().enumerate().collect()
// indexed == [(0 "a") (1 "b") (2 "c")]

// Reduce
total = [1 2 3 4 5].iter().reduce(|acc x| acc + x, 0)
// total == 15

// For-in with iterator
for x in iter(10).filter(|x| x % 2 == 0) {
  p(x)  // prints 0 2 4 6 8
}

// Zip
pairs = iter(3).zip(["a" "b" "c"].iter()).collect()
// pairs == [(0 "a") (1 "b") (2 "c")]
```
