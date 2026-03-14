# Plan 2, Batch 1: Bool Type + Comparisons + If/Else

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add boolean type, comparison/boolean operators, and if/else expressions to the Sans compiler, from lexer through codegen.

**Architecture:** Changes flow through all 6 compiler stages. Bool is represented as `i1` in LLVM IR, with `zext` to `i64` when used as a return value. If/else compiles to LLVM conditional branches with phi nodes for value merging.

**Tech Stack:** Rust, inkwell 0.8 (llvm17-0), Pratt parser

---

## Chunk 1: Lexer + AST + Parser

### Task 1: Lexer — New tokens

**Files:**
- Modify: `crates/sans-lexer/src/token.rs`
- Modify: `crates/sans-lexer/src/lib.rs`

New `TokenKind` variants:
```rust
True, False, If, Else,           // keywords
EqEq, NotEq, Lt, Gt, LtEq, GtEq, // comparison operators
And, Or, Bang,                    // boolean operators
```

Lexer changes:
- `=` must now look ahead: `==` → `EqEq`, else `Eq`
- `!` must look ahead: `!=` → `NotEq`, else `Bang`
- `<` must look ahead: `<=` → `LtEq`, else `Lt`
- `>` must look ahead: `>=` → `GtEq`, else `Gt`
- `&` must look ahead: `&&` → `And` (single `&` is error for now)
- `|` must look ahead: `||` → `Or` (single `|` is error for now)
- Keywords: `true` → `True`, `false` → `False`, `if` → `If`, `else` → `Else`

Tests:
- Lex `true false` → `[True, False, Eof]`
- Lex `== != < > <= >=` → correct tokens
- Lex `&& ||` → `[And, Or, Eof]`
- Lex `!` → `[Bang, Eof]`
- Lex `if x { 1 } else { 2 }` → correct token sequence

### Task 2: AST — New nodes

**Files:**
- Modify: `crates/sans-parser/src/ast.rs`

New AST nodes:
```rust
// Add to Expr enum:
BoolLiteral { value: bool, span: Span },
If { condition: Box<Expr>, then_body: Vec<Stmt>, then_expr: Box<Expr>, else_body: Vec<Stmt>, else_expr: Box<Expr>, span: Span },
UnaryOp { op: UnaryOp, operand: Box<Expr>, span: Span },

// Add to BinOp enum:
Eq, NotEq, Lt, Gt, LtEq, GtEq, And, Or

// New enum:
pub enum UnaryOp { Not }
```

The `If` expression has:
- `condition`: must be Bool
- `then_body`/`then_expr`: statements + final expression in then branch
- `else_body`/`else_expr`: statements + final expression in else branch

Update `expr_span` in parser to handle new variants.

### Task 3: Parser — Parse new constructs

**Files:**
- Modify: `crates/sans-parser/src/lib.rs`

Changes:
1. **Atoms:** `true`/`false` → `BoolLiteral`, `if` → parse if/else expression
2. **Prefix:** `!` → `UnaryOp { op: Not, ... }` with prefix binding power 7
3. **Infix binding powers (left-associative):**
   - `||` → (1, 2) — lowest
   - `&&` → (3, 4)
   - `==`, `!=` → (5, 6)
   - `<`, `>`, `<=`, `>=` → (7, 8)
   - `+`, `-` → (9, 10)
   - `*`, `/` → (11, 12)

Wait — this changes existing binding powers for `+`, `-`, `*`, `/`. Currently they are (1,2) and (3,4). Update them to make room for comparison/boolean operators below arithmetic.

4. **If/else parsing:**
```
if <expr> { <body> <expr> } else { <body> <expr> }
```
Both branches required (if/else is an expression, must produce a value).

Tests:
- Parse `true` and `false` as BoolLiteral
- Parse `1 == 2` as BinaryOp with Eq
- Parse `1 < 2 && 3 > 1` with correct precedence (comparisons bind tighter than &&)
- Parse `if true { 1 } else { 2 }` as If expression
- Parse `!true` as UnaryOp
- Parse `if x == 1 { 10 } else { 20 }` — condition is comparison

## Chunk 2: Type Checker

### Task 4: Type checker — Bool type and rules

**Files:**
- Modify: `crates/sans-typeck/src/types.rs`
- Modify: `crates/sans-typeck/src/lib.rs`

Changes:
1. Add `Type::Bool` variant, update Display
2. `resolve_type("Bool")` → `Type::Bool`
3. `check_expr` for `BoolLiteral` → `Type::Bool`
4. `check_expr` for `BinaryOp`: split arithmetic ops (Int→Int) from comparison ops (Int→Bool) from boolean ops (Bool→Bool)
5. `check_expr` for `UnaryOp::Not`: operand must be Bool, result is Bool
6. `check_expr` for `If`: condition must be Bool, then_expr and else_expr must have same type, that's the result type

Tests:
- `fn main() Bool { true }` — valid (needs Bool return type to work)
- `fn main() Int { if true { 1 } else { 2 } }` — valid
- `fn main() Int { if 1 { 1 } else { 2 } }` — error: condition not Bool
- `fn main() Int { if true { 1 } else { true } }` — error: branch type mismatch
- `fn main() Bool { 1 == 2 }` — valid
- `fn main() Bool { !true }` — valid
- `fn main() Bool { true && false }` — valid

## Chunk 3: IR + Codegen

### Task 5: IR — Control flow instructions

**Files:**
- Modify: `crates/sans-ir/src/ir.rs`
- Modify: `crates/sans-ir/src/lib.rs`

New IR instructions:
```rust
BoolConst { dest: Reg, value: bool },
CmpOp { dest: Reg, op: IrCmpOp, left: Reg, right: Reg },
Not { dest: Reg, src: Reg },
// Control flow
Label { name: String },
Branch { cond: Reg, then_label: String, else_label: String },
Jump { target: String },
Phi { dest: Reg, a_val: Reg, a_label: String, b_val: Reg, b_label: String },
```

New `IrCmpOp`: `Eq, NotEq, Lt, Gt, LtEq, GtEq`

Lowering if/else:
```
<lower condition into cond_reg>
Branch { cond: cond_reg, then_label: "then_N", else_label: "else_N" }
Label { name: "then_N" }
<lower then body + expr into then_reg>
Jump { target: "merge_N" }
Label { name: "else_N" }
<lower else body + expr into else_reg>
Jump { target: "merge_N" }
Label { name: "merge_N" }
Phi { dest: result_reg, a_val: then_reg, a_label: "then_N", b_val: else_reg, b_label: "else_N" }
```

Lowering `&&`: short-circuit via branch (a && b = if a { b } else { false })
Lowering `||`: short-circuit via branch (a || b = if a { true } else { b })

Tests:
- Lower `if true { 1 } else { 2 }` produces Branch, Labels, Phi
- Lower `1 == 2` produces CmpOp
- Lower `!true` produces Not

### Task 6: Codegen — LLVM basic blocks + comparisons

**Files:**
- Modify: `crates/sans-codegen/src/lib.rs`

Changes:
1. Two value types: `i64` for Int, `i1` for Bool. Track which registers are bool vs int.
2. `BoolConst` → `context.bool_type().const_int(value as u64, false)`
3. `CmpOp` → `builder.build_int_compare(predicate, lhs, rhs, dest)`
4. `Not` → `builder.build_not(val, dest)`
5. **Label processing:** Split instruction stream at Labels to create LLVM basic blocks.
6. `Branch` → `builder.build_conditional_branch(cond, then_bb, else_bb)`
7. `Jump` → `builder.build_unconditional_branch(target_bb)`
8. `Phi` → `phi = builder.build_phi(type, dest); phi.add_incoming(&[(a_val, a_bb), (b_val, b_bb)])`
9. Return value: if function returns Bool, `zext i1` to `i64` before `ret` (since main returns exit code as i64).

Strategy for basic blocks from flat IR:
- First pass: scan for all Label instructions, create LLVM basic blocks
- Second pass: generate instructions, switching `builder.position_at_end()` when hitting a Label

Tests:
- Compile `if true { 1 } else { 2 }` and verify LLVM IR has `br`, basic blocks, `phi`
- Compile `1 == 2` and verify LLVM IR has `icmp`

### Task 7: E2E integration tests

**Files:**
- Create: `tests/fixtures/if_else.cy`
- Create: `tests/fixtures/comparison.cy`
- Create: `tests/fixtures/boolean_ops.cy`

Test programs:
```
// if_else.cy — exit code 1
fn main() Int {
  if 1 == 1 { 1 } else { 0 }
}

// comparison.cy — exit code 42
fn main() Int {
  let x Int = 10
  if x > 5 { 42 } else { 0 }
}

// boolean_ops.cy — exit code 1
fn main() Int {
  if true && !false { 1 } else { 0 }
}
```

Build each with `sans build` and verify exit codes.
