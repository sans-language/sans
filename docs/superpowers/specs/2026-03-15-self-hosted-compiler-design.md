# Self-Hosted Sans Compiler — Design Spec

## Overview

Rewrite the Sans compiler from Rust to Sans. The existing Rust compiler (v0.3.25) compiles the new Sans compiler, which then compiles itself (bootstrap). Full language support — every feature, builtin, and syntax form.

**Branch:** `feature/self-hosting-0.4.0`
**Version:** `0.4.0` on merge

## Architecture

```
compiler/
  lexer.sans      — tokenizer
  parser.sans     — recursive descent parser → AST (heap structs)
  typeck.sans     — type checking + inference
  ir.sans         — IR instruction types + AST→IR lowering
  codegen.sans    — IR → LLVM IR text (.ll file)
  main.sans       — CLI driver, import resolution, pipeline
```

Mirrors the 6-crate Rust structure for 1:1 traceability during the port.

## Key Design Decisions

### 1. AST Representation

All AST/IR nodes are heap-allocated via `alloc()`. Variant types use integer tags with known memory layouts.

**Expr node layout (example):**
```
offset 0:  tag (I)        — 0=IntLit, 1=FloatLit, 2=BoolLit, 3=StringLit,
                             4=Ident, 5=BinOp, 6=UnaryOp, 7=Call, 8=MethodCall,
                             9=Index, 10=FieldAccess, 11=If, 12=Lambda,
                             13=ArrayLit, 14=StructLit, 15=TupleLit,
                             16=Match, 17=Ternary, 18=Assign, ...
offset 8:  data fields vary by tag
```

Child nodes are pointers (I) loaded with `load64`. Lists of nodes use Sans arrays (`Array<I>` where each element is a pointer to a node).

### 2. Symbol Tables

All name→value lookups use `M()` (the existing Map type):
- **Variable scopes:** Stack of Maps, push on block entry, pop on exit
- **Function registry:** Map from name to function signature struct
- **Type registry:** Map from name to type descriptor
- **Module exports:** Map from module path to export Map

### 3. Codegen Strategy: Emit LLVM IR Text

The codegen module builds LLVM IR as strings and writes a `.ll` file. Then shells out to:
```
llc -filetype=obj -o output.o output.ll
clang output.o runtime/*.o -o output
```

No LLVM C bindings needed. The `.ll` files are human-readable for debugging.

**Example output:**
```llvm
define i64 @main() {
entry:
  %0 = add i64 1, 2
  ret i64 %0
}
```

### 4. Type System Representation

Types represented as tagged structs, similar to AST nodes:
```
offset 0:  tag (I)    — 0=Int, 1=Float, 2=Bool, 3=String, 4=Array,
                         5=Map, 6=Struct, 7=Enum, 8=Function, 9=Tuple,
                         10=Result, 11=Void, 12=Generic, 13=JsonValue,
                         14=HttpResponse, ...
offset 8:  inner type pointer (for Array<T>, Result<T>, etc.)
offset 16: additional data (struct fields, enum variants, etc.)
```

### 5. Error Handling

Compiler errors reported via `wfd(2, msg)` (write to file descriptor 2 = stderr) and `exit(1)`. The existing `Result<T>` type used for recoverable operations (file I/O).

### 6. String Building Strategy

Codegen produces large LLVM IR text (thousands of lines). Naive string concatenation is O(N^2). Instead, use an `Array<S>` as a string builder — append lines to the array, then join at the end with a single allocation pass using `alloc` + `mcpy`.

Helper function pattern:
```
emit(buf:[S] line:S) = buf.push(line)
finish(buf:[S]) S { ... join with newlines ... }
```

### 7. Shell Command Execution (Prerequisite)

Sans currently has no `system()` or `exec()` built-in. **This must be added before the driver module can work.** Needed: a `system(cmd:S) I` built-in that calls libc `system()` and returns the exit code. This is required to invoke `llc` and `clang`.

### 8. Map Values Convention

All Map values store heap pointers as i64. Complex values (type descriptors, function signatures, AST nodes) are heap-allocated structs accessed via `load64`/`store64` on the pointer retrieved from the Map.

### 9. Float Constant Emission

LLVM IR requires float constants in hex format (e.g., `0x4029000000000000`). Since Sans stores floats as i64 via bitcast internally, the raw bits are already available. Codegen converts these bits to a hex string for emission.

### 10. Memory Management

Use arena allocation per compiler phase to limit memory growth:
- **Lexer phase:** Arena for tokens (freed after parsing)
- **Parser phase:** Arena for AST nodes (freed after IR lowering)
- **IR phase:** Arena for instructions (freed after codegen)
- **Codegen phase:** Arena for string buffer (freed after file write)

This maps to Sans's 8-deep arena nesting limit.

## Module Details

### lexer.sans (~400-600 LOC estimated)

Port of `crates/sans-lexer/src/lib.rs` (651 LOC Rust).

**Input:** Source string
**Output:** Array of Token structs

**Token layout:**
```
offset 0:  type (I)     — token type enum (0=Int, 1=Float, 2=String, ...)
offset 8:  value (I)    — string pointer for identifiers/literals
offset 16: line (I)     — line number
offset 24: col (I)      — column number
```

Key behaviors:
- Single-pass character-by-character scan
- String interpolation handling (nested `{expr}` in strings)
- All operators, keywords, and punctuation
- Multiline string support (`"""..."""`)
- Comment skipping (`//` and `/* */`)

### parser.sans (~2000-2500 LOC estimated)

Port of `crates/sans-parser/src/lib.rs` (2,500 LOC Rust).

**Input:** Array of Tokens
**Output:** Array of top-level AST nodes (Stmt pointers)

Recursive descent with Pratt parsing for expressions (operator precedence). Supports:
- All expression forms (binary, unary, call, method call, index, field access, if-expr, match, lambda, ternary, array/struct/tuple literals)
- All statement forms (assignment, mutable assignment, function def, struct def, enum def, trait def, impl block, for/while loops, return, import, break/continue)
- Type annotations with generics
- Expression-body functions (`f(x:I) = x*2`)

### typeck.sans (~2500-3500 LOC estimated)

Port of `crates/sans-typeck/src/lib.rs` (3,509 LOC Rust).

**Input:** AST + module context
**Output:** Type-annotated AST (types stored on nodes or in side table Map)

Key responsibilities:
- Type inference for all expressions
- Generic function instantiation
- Struct field type tracking
- Enum variant type checking
- Module export/import type resolution
- Built-in function type signatures (100+ functions)
- Method type checking on all types

### ir.sans (~2500-3500 LOC estimated)

Port of `crates/sans-ir/src/ir.rs` + `crates/sans-ir/src/lib.rs` (3,514 LOC Rust).

**IR instruction layout:**
```
offset 0:  opcode (I)   — 0=Const, 1=BinOp, 2=Call, 3=Ret, ...
offset 8:  dest (I)     — destination register
offset 16: operands vary by opcode
```

50+ instruction types covering all language features. Lowering pass walks the typed AST and emits IR instructions into an array per function.

### codegen.sans (~3000-4000 LOC estimated)

Port of `crates/sans-codegen/src/lib.rs` (4,151 LOC Rust).

**Input:** IR (array of functions, each with array of instructions)
**Output:** LLVM IR text string

Responsibilities:
- Emit LLVM function declarations for all runtime/extern functions
- Emit LLVM function definitions from IR
- Register allocation (virtual registers → LLVM SSA values)
- Struct layout computation (field offsets)
- String constant pool
- Global variable emission

### main.sans (~400-500 LOC estimated)

Port of `crates/sans-driver/src/main.rs` (346 LOC Rust) + `imports.rs` (98 LOC).

**Responsibilities:**
- Parse CLI args (`sans build file.sans`, `sans run file.sans`)
- Resolve imports recursively with cycle detection (track visited paths in a Map)
- Topological sort of modules (depth-first traversal, emit in post-order)
- Orchestrate: lex → parse → typecheck → lower → codegen → write .ll → invoke llc + clang
- Link runtime object files

## Build Order (Incremental)

Each phase produces testable, committable code:

0. **Prerequisites** — add `system()` built-in to Rust compiler (needed for driver)
1. **Lexer** — tokenize source, validate against test fixtures
2. **Parser** — parse tokens to AST, validate by printing AST back
3. **IR types** — define instruction structs and enums
4. **Type checker** — validate types, test against fixture errors
5. **IR lowering** — AST to IR instructions
6. **Codegen** — IR to LLVM IR text
7. **Driver** — full pipeline wired together
8. **Bootstrap** — compile the compiler with itself

## Testing Strategy

1. **Per-module tests:** Each module gets test functions that validate against known inputs
2. **Fixture comparison:** Run both Rust and Sans compilers on all 97 test fixtures, compare exit codes
3. **Bootstrap test:** The Sans compiler (stage 1) compiles itself to produce stage 2. Stage 2 compiles the test fixtures — results must match stage 1

## Dependencies

- `llc` (from LLVM) — compile `.ll` to `.o`
- `clang` — link `.o` files + runtime into binary
- Existing Sans runtime modules (map, string_ext, etc.) — linked into compiled programs
- The Rust compiler (v0.3.25) — used to build the Sans compiler initially

## Success Criteria

1. All 97 existing test fixtures produce identical results when compiled with the Sans compiler
2. The Sans compiler can compile itself (bootstrap)
3. Stage 1 and stage 2 compilers produce identical results on all test fixtures
4. Full language coverage — no features dropped

## Debugging Strategy

- `--dump-tokens` flag: print lexer output and exit
- `--dump-ast` flag: print parsed AST and exit
- `--dump-ir` flag: print IR instructions and exit
- `--emit-ll` flag: write `.ll` file without invoking llc/clang (inspect LLVM IR)
- **Comparison mode:** Shell script that runs both Rust and Sans compilers on all fixtures and diffs exit codes + stdout

## Risks

- **LOC will be higher than Rust** (~40-60% more) due to manual tag dispatch replacing Rust's pattern matching. Expect ~18,000-22,000 LOC total.
- **No GC** — leaked allocations during compilation. Mitigated by arena allocation per phase.
- **100-variant instruction enum** — the codegen match will be ~2,000+ LOC. Error-prone but feasible.
