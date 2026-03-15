# Self-Hosted Sans Compiler Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the Sans compiler from Rust to Sans, producing a fully self-hosted compiler that can compile itself.

**Architecture:** 6 Sans modules mirroring the Rust crates (lexer, parser, typeck, ir, codegen, main). AST/IR nodes are heap-allocated tagged structs. Codegen emits LLVM IR as text, shells out to llc+clang. All symbol tables use Map type.

**Tech Stack:** Sans language, LLVM (via text IR), clang linker

**Spec:** `docs/superpowers/specs/2026-03-15-self-hosted-compiler-design.md`

---

## Chunk 1: Prerequisites + Lexer

### Task 0: Add `system()` built-in to Rust compiler

The self-hosted compiler needs to shell out to `llc` and `clang`. Sans has no `system()` call yet.

**Files:**
- Modify: `crates/sans-typeck/src/lib.rs` (add type check for `system`)
- Modify: `crates/sans-ir/src/ir.rs` (add `System` instruction)
- Modify: `crates/sans-ir/src/lib.rs` (add IR lowering for `system`)
- Modify: `crates/sans-codegen/src/lib.rs` (add codegen for `system`)
- Create: `tests/fixtures/system_basic.sans`
- Modify: `crates/sans-driver/tests/e2e.rs` (add test)
- Modify: `docs/reference.md`, `docs/ai-reference.md`, `website/static/docs.html`
- Modify: `editors/vscode-sans/src/extension.ts` (hover data)
- Modify: All version files (bump to 0.3.26)

- [ ] **Step 1: Add type check for `system` and `sys` alias**

In `crates/sans-typeck/src/lib.rs`, in the `Expr::Call` match, add:
```rust
"system" | "sys" => {
    if args.len() != 1 { return Err(TypeError { message: "system() takes 1 argument".into(), span: *span }); }
    let arg_ty = check_expr(&args[0], ...)?;
    if arg_ty != Type::String { return Err(TypeError { message: "system() argument must be String".into(), span: *span }); }
    Ok(Type::Int)
}
```

- [ ] **Step 2: Add IR instruction**

In `crates/sans-ir/src/ir.rs`, add to the `Instruction` enum:
```rust
System { dest: Reg, command: Reg },
```

- [ ] **Step 3: Add IR lowering**

In `crates/sans-ir/src/lib.rs`, in the built-in function lowering section:
```rust
} else if function == "system" || function == "sys" {
    let cmd_reg = self.lower_expr(&args[0]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::System { dest: dest.clone(), command: cmd_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
}
```

- [ ] **Step 4: Add codegen**

In `crates/sans-codegen/src/lib.rs`:
1. Declare `system` extern: `fn system(i8*) -> i32`
2. In the instruction compile match, add:
```rust
Instruction::System { dest, command } => {
    let cmd_ptr = /* get command as pointer */;
    let result = builder.build_call(system_fn, &[cmd_ptr.into()], "sys_result");
    let result_i64 = builder.build_int_s_extend(result, i64_type, "sys_ext");
    regs.insert(dest.clone(), result_i64);
}
```

- [ ] **Step 5: Write test fixture**

Create `tests/fixtures/system_basic.sans`:
```
main() I {
  system("echo hello > /dev/null")
}
```

- [ ] **Step 6: Add E2E test**

In `crates/sans-driver/tests/e2e.rs`:
```rust
#[test]
fn e2e_system_basic() {
    assert_eq!(compile_and_run("system_basic.sans"), 0);
}
```

- [ ] **Step 7: Run tests**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test e2e_system
```

- [ ] **Step 8: Update docs + version**

Update `docs/reference.md`, `docs/ai-reference.md`, `website/static/docs.html`, hover data, and bump version to 0.3.26 in all version files.

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat: add system() built-in for shell command execution — v0.3.26"
```

---

### Task 1: Create compiler directory and shared constants

**Files:**
- Create: `compiler/constants.sans` — token type tags, expr tags, stmt tags, IR opcode tags, type tags

This file defines ALL integer tag constants used across every module. Single source of truth.

- [ ] **Step 1: Create the compiler directory**

```bash
mkdir -p compiler
```

- [ ] **Step 2: Write constants.sans**

Create `compiler/constants.sans` with all tag constants. This is the shared vocabulary for the entire compiler.

```sans
// Token types (55 total)
TK_INT_LIT = 0
TK_FLOAT_LIT = 1
TK_STRING_LIT = 2
TK_INTERP_STRING = 3
TK_IDENT = 4
TK_FN = 5
TK_LET = 6
TK_MUT = 7
TK_TRUE = 8
TK_FALSE = 9
TK_IF = 10
TK_ELSE = 11
TK_WHILE = 12
TK_RETURN = 13
TK_STRUCT = 14
TK_ENUM = 15
TK_MATCH = 16
TK_TRAIT = 17
TK_IMPL = 18
TK_FOR = 19
TK_SELF_VAL = 20
TK_SELF_TYPE = 21
TK_SPAWN = 22
TK_CHANNEL = 23
TK_MUTEX = 24
TK_ARRAY = 25
TK_IN = 26
TK_IMPORT = 27
TK_GLOBAL = 28
TK_BREAK = 29
TK_CONTINUE = 30
TK_PLUS = 31
TK_MINUS = 32
TK_STAR = 33
TK_SLASH = 34
TK_PERCENT = 35
TK_PLUS_EQ = 36
TK_MINUS_EQ = 37
TK_STAR_EQ = 38
TK_SLASH_EQ = 39
TK_PERCENT_EQ = 40
TK_EQ_EQ = 41
TK_NOT_EQ = 42
TK_LT = 43
TK_GT = 44
TK_LT_EQ = 45
TK_GT_EQ = 46
TK_AND = 47
TK_OR = 48
TK_BANG = 49
TK_EQ = 50
TK_COLON_EQ = 51
TK_FAT_ARROW = 52
TK_QUESTION = 53
TK_PIPE = 54
TK_LPAREN = 55
TK_RPAREN = 56
TK_LBRACE = 57
TK_RBRACE = 58
TK_LBRACKET = 59
TK_RBRACKET = 60
TK_COMMA = 61
TK_COLON = 62
TK_COLON_COLON = 63
TK_DOT = 64
TK_EOF = 65

// Expr tags (22 total)
EX_INT_LIT = 0
EX_FLOAT_LIT = 1
EX_BOOL_LIT = 2
EX_STRING_LIT = 3
EX_IDENT = 4
EX_BINOP = 5
EX_UNARYOP = 6
EX_CALL = 7
EX_METHOD_CALL = 8
EX_INDEX = 9
EX_FIELD_ACCESS = 10
EX_IF = 11
EX_LAMBDA = 12
EX_ARRAY_LIT = 13
EX_STRUCT_LIT = 14
EX_TUPLE_LIT = 15
EX_MATCH = 16
EX_SPAWN = 17
EX_CHANNEL = 18
EX_MUTEX = 19
EX_ENUM_VARIANT = 20
EX_ARRAY_CREATE = 21
// Note: Ternary desugars to EX_IF in parser. Index desugars to EX_METHOD_CALL (.get).
// Assignment is ST_ASSIGN (statement only, not expression).

// Stmt tags (10 total)
ST_LET = 0
ST_WHILE = 1
ST_RETURN = 2
ST_ASSIGN = 3
ST_IF = 4
ST_BREAK = 5
ST_CONTINUE = 6
ST_LET_DESTR = 7
ST_FOR_IN = 8
ST_EXPR = 9

// BinOp tags
OP_ADD = 0
OP_SUB = 1
OP_MUL = 2
OP_DIV = 3
OP_MOD = 4
OP_EQ = 5
OP_NEQ = 6
OP_LT = 7
OP_GT = 8
OP_LTEQ = 9
OP_GTEQ = 10
OP_AND = 11
OP_OR = 12

// UnaryOp tags
UOP_NOT = 0
UOP_NEG = 1

// Type tags (20 total)
TY_INT = 0
TY_FLOAT = 1
TY_BOOL = 2
TY_STRING = 3
TY_ARRAY = 4
TY_MAP = 5
TY_STRUCT = 6
TY_ENUM = 7
TY_FN = 8
TY_TUPLE = 9
TY_RESULT = 10
TY_RESULT_ERR = 11
TY_VOID = 12
TY_JSON = 13
TY_HTTP_RESP = 14
TY_HTTP_SERVER = 15
TY_HTTP_REQ = 16
TY_JOIN_HANDLE = 17
TY_SENDER = 18
TY_RECEIVER = 19
TY_MUTEX = 20
```

- [ ] **Step 3: Commit**

```bash
git add compiler/ && git commit -m "compiler: add shared constants for self-hosted compiler"
```

---

### Task 2: Lexer (compiler/lexer.sans)

Port of `crates/sans-lexer/src/lib.rs` (651 LOC Rust). The lexer takes a source string and produces an array of token structs.

**Files:**
- Create: `compiler/lexer.sans`
- Create: `tests/fixtures/selfhost_lexer_test.sans` (test harness)

**Token struct layout (32 bytes):**
```
offset 0:  kind (I)   — token type tag from constants.sans
offset 8:  value (I)  — string pointer (for idents/literals) or 0
offset 16: line (I)   — line number (1-based)
offset 24: col (I)    — column number (1-based)
```

**Lexer state layout (40 bytes):**
```
offset 0:  src (I)    — source string pointer
offset 8:  len (I)    — source length
offset 16: pos (I)    — current position
offset 24: line (I)   — current line
offset 32: col (I)    — current column
```

- [ ] **Step 1: Write lexer scaffolding with token constructors**

Create `compiler/lexer.sans` with:
- `make_token(kind:I val:I line:I col:I) I` — allocates 32-byte token struct
- `make_lexer(src:S) I` — allocates lexer state
- `lex_peek(lx:I) I` — peek current char without advancing
- `lex_advance(lx:I) I` — advance position, update line/col
- `lex_at_end(lx:I) I` — check if at end of source

- [ ] **Step 2: Implement whitespace and comment skipping**

- `lex_skip_ws(lx:I) I` — skip spaces, tabs, newlines, carriage returns
- `lex_skip_line_comment(lx:I) I` — skip from `//` to end of line
- `lex_skip_block_comment(lx:I) I` — skip from `/*` to `*/` (no nesting)

- [ ] **Step 3: Implement number lexing**

- `lex_number(lx:I) I` — lex integer or float literal
- Handle the float-vs-tuple-field-access edge case (check if previous char was `.`)

- [ ] **Step 4: Implement identifier and keyword lexing**

- `lex_ident(lx:I) I` — lex identifier, check against keyword table
- Keyword matching: compare string against all 25 keywords, return appropriate token kind

- [ ] **Step 5: Implement string lexing**

- `lex_string(lx:I) I` — lex regular string with escape sequences
- `lex_multiline_string(lx:I) I` — lex `"""..."""` strings
- Handle escape sequences: `\n`, `\t`, `\\`, `\"`, `\{`

- [ ] **Step 6: Implement string interpolation**

- `lex_interp_string(lx:I) I` — lex string with `{...}` interpolation
- Track brace nesting depth
- Classify interpolation parts as Ident vs Expr
- Store parts as array of (tag, string) pairs

**InterpolatedString parts stored as array of pairs:**
```
Each part: 16 bytes
  offset 0: tag (0=literal, 1=ident, 2=expr)
  offset 8: string pointer
```

- [ ] **Step 7: Implement operator and delimiter lexing**

- `lex_op(lx:I) I` — lex operators with lookahead for 2-char variants
- Handle: `+/+=`, `-/-=`, `*/*=`, `///=`, `%/%=`, `==/=>`, `!=/!`, `</<=/`, `>/>=/`, `&&`, `||`, `:/::/`:=`, `.`, `?`
- Single-char delimiters: `( ) { } [ ] ,`

- [ ] **Step 8: Implement main lex function**

- `lex(src:S) I` — main entry point, returns array of tokens
- Loop: skip whitespace, match first char, dispatch to appropriate lexer
- Append EOF token at end

- [ ] **Step 9: Write lexer test**

Create `tests/fixtures/selfhost_lexer_test.sans`:
```sans
import "compiler/lexer"
import "compiler/constants"

main() I {
  tokens = lex("x = 42")
  // Verify: IDENT("x"), EQ, INT_LIT(42), EOF
  t0 = load64(tokens.get(0))
  t1 = load64(tokens.get(1))
  t2 = load64(tokens.get(2))
  // Check token kinds
  ok := 1
  if t0 != TK_IDENT { ok = 0; 0 } else { 0 }
  if t1 != TK_EQ { ok = 0; 0 } else { 0 }
  if t2 != TK_INT_LIT { ok = 0; 0 } else { 0 }
  ok
}
```

- [ ] **Step 10: Run test**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --test e2e selfhost_lexer
```

- [ ] **Step 11: Commit**

```bash
git add compiler/lexer.sans tests/fixtures/selfhost_lexer_test.sans && git commit -m "compiler: add self-hosted lexer"
```

---

## Chunk 2: Parser

### Task 3: Parser (compiler/parser.sans)

Port of `crates/sans-parser/src/lib.rs` (2,500 LOC Rust). Recursive descent parser with Pratt operator precedence.

**Files:**
- Create: `compiler/parser.sans`
- Create: `tests/fixtures/selfhost_parser_test.sans`

**Key AST node layouts (all heap-allocated, tag at offset 0):**

**Expr node (variable size by tag):**
```
All exprs: offset 0 = tag (I)

EX_INT_LIT:      [tag, value:I]                    — 16 bytes
EX_FLOAT_LIT:    [tag, value:I (bitcast f64)]       — 16 bytes
EX_BOOL_LIT:     [tag, value:I (0/1)]               — 16 bytes
EX_STRING_LIT:   [tag, str_ptr:I]                    — 16 bytes
EX_IDENT:        [tag, name_ptr:I]                   — 16 bytes
EX_BINOP:        [tag, op:I, left:I, right:I]        — 32 bytes
EX_UNARYOP:      [tag, op:I, operand:I]              — 24 bytes
EX_CALL:         [tag, name_ptr:I, args_array:I]     — 24 bytes
EX_METHOD_CALL:  [tag, object:I, method_ptr:I, args_array:I] — 32 bytes
EX_FIELD_ACCESS: [tag, object:I, field_ptr:I]        — 24 bytes
EX_IF:           [tag, cond:I, then_stmts:I, then_expr:I, else_stmts:I, else_expr:I] — 48 bytes
EX_ARRAY_LIT:    [tag, elements_array:I]             — 16 bytes
EX_STRUCT_LIT:   [tag, name_ptr:I, field_names:I, field_values:I] — 32 bytes
EX_TUPLE_LIT:    [tag, elements_array:I]             — 16 bytes
EX_MATCH:        [tag, scrutinee:I, arms_array:I]    — 24 bytes
EX_LAMBDA:       [tag, params:I, ret_type_ptr:I, body_stmts:I] — 32 bytes
EX_SPAWN:        [tag, func_name:I, args_array:I]    — 24 bytes
EX_CHANNEL:      [tag, elem_type_ptr:I, capacity:I]  — 24 bytes
EX_MUTEX:        [tag, value_expr:I]                  — 16 bytes
EX_ENUM_VARIANT: [tag, enum_name:I, variant_name:I, args_array:I] — 32 bytes
EX_ARRAY_CREATE: [tag, elem_type_ptr:I]              — 16 bytes

// Note: No EX_TERNARY — parser desugars `cond ? a : b` to EX_IF.
// Note: No EX_INDEX — parser desugars `arr[i]` to EX_METHOD_CALL(.get(i)).
// Note: No EX_ASSIGN — assignment is ST_ASSIGN (statement only).
// Note: Spans are omitted from AST nodes. Error messages use token line/col instead.
```

**Stmt node (variable size by tag):**
```
ST_LET:       [tag, name_ptr:I, mutable:I, type_ptr:I, value_expr:I]  — 40 bytes
ST_WHILE:     [tag, cond_expr:I, body_stmts:I]                         — 24 bytes
ST_RETURN:    [tag, value_expr:I]                                       — 16 bytes
ST_ASSIGN:    [tag, name_ptr:I, value_expr:I]                          — 24 bytes
ST_IF:        [tag, cond_expr:I, body_stmts:I]                         — 24 bytes
ST_BREAK:     [tag]                                                     — 8 bytes
ST_CONTINUE:  [tag]                                                     — 8 bytes
ST_LET_DESTR: [tag, names_array:I, value_expr:I]                       — 24 bytes
ST_FOR_IN:    [tag, var_name:I, iterable_expr:I, body_stmts:I]        — 32 bytes
ST_EXPR:      [tag, expr:I]                                             — 16 bytes
```

**Program struct layout:**
```
offset 0:  imports (Array<I>)        — array of import path strings
offset 8:  globals (Array<I>)        — array of GlobalDef pointers
offset 16: functions (Array<I>)      — array of Function pointers
offset 24: structs (Array<I>)        — array of StructDef pointers
offset 32: enums (Array<I>)          — array of EnumDef pointers
offset 40: traits (Array<I>)         — array of TraitDef pointers
offset 48: impls (Array<I>)          — array of ImplBlock pointers
```

**Function struct layout:**
```
offset 0:  name (I)           — string pointer
offset 8:  params (Array<I>)  — array of Param structs
offset 16: return_type (I)    — type name string pointer
offset 24: body (Array<I>)    — array of Stmt pointers
offset 32: type_params (Array<I>) — array of TypeParam structs
offset 40: is_method (I)      — 0 or 1
```

- [ ] **Step 1: Write parser state and token access helpers**

Parser state struct:
```
offset 0:  tokens (I)    — array of token pointers
offset 8:  pos (I)       — current position (mutable via alloca)
offset 16: last_try (I)  — last try expr for guard generation
```

Helper functions:
- `make_parser(tokens:I) I`
- `parser_peek(p:I) I` — get current token without consuming
- `parser_advance(p:I) I` — consume and return current token
- `parser_expect(p:I kind:I) I` — consume token of expected kind, or error
- `parser_at(p:I kind:I) I` — check if current token matches kind

- [ ] **Step 2: Implement atom parsing (literals, identifiers, calls)**

- `parse_atom(p:I) I` — returns Expr pointer
- Handle: int/float/bool/string literals, identifiers
- Handle: function calls (`name(args)`)
- Handle: struct literals (`Name { field: value }`)
- Handle: enum variants (`Name::Variant(args)`)
- Handle: parenthesized expressions and tuple literals
- Handle: array literals (`[1 2 3]`)
- Handle: `array<Type>()` creation
- Handle: `if` expression, `match` expression
- Handle: lambda (`|params| Type { body }`)
- Handle: `spawn`, `channel<T>()`, `mutex(val)`

- [ ] **Step 3: Implement Pratt expression parser**

- `parse_expr(p:I min_bp:I) I` — Pratt parser
- `infix_bp(kind:I) I` — returns (left_bp, right_bp, op) packed or 0 if not infix
- Binding powers: `||`=1/2, `&&`=3/4, `==/!=`=5/6, `<><=>=`=7/8, `+-`=9/10, `*/%`=11/12
- Prefix: `!` and `-` at bp=13
- Postfix: `.field`, `.method()`, `[index]`, `[start:end]`, `!` (unwrap)
- Handle ternary: `cond ? expr : expr` (at min_bp==0)
- Handle try: `expr?` (at min_bp==0, no `:` follows)

- [ ] **Step 4: Implement statement parsing**

- `parse_stmt(p:I) I` — returns Stmt pointer
- Handle: `let name = expr`, `name := expr` (mutable)
- Handle: `name = expr` (assignment), `name += expr` (compound)
- Handle: `name[idx] = expr` (index assignment → `.set()`)
- Handle: `while cond { body }`
- Handle: `for var in iterable { body }`
- Handle: `if cond { body }`
- Handle: `return expr`
- Handle: `break`, `continue`
- Handle: `let (a, b) = expr` (destructuring)
- Handle: bare expression statements
- Try guard generation: if `last_try` set, emit `if result.is_err() { return result }` guard

- [ ] **Step 5: Implement type annotation parsing**

- `parse_type_name(p:I) I` — returns type name string
- Handle: simple types (`Int`, `I`, `String`, `S`, etc.)
- Handle: array types (`[Int]` → `"Array<Int>"`)
- Handle: tuple types (`(Int String)` → `"(Int String)"`)
- Handle: parameterized types (`Result<Int>` → `"Result<Int>"`)

- [ ] **Step 6: Implement function definition parsing**

- `parse_function(p:I) I` — returns Function struct pointer
- `parse_params(p:I) I` — returns array of Param structs
- Handle: `fn` keyword optional
- Handle: type parameters (`<T>`, `<T: Bound>`)
- Handle: expression-body (`f(x:I) = x*2`)
- Handle: block-body (`f(x:I) I { x*2 }`)
- Default return type to `"I"` when next token is `{` or `=`

- [ ] **Step 7: Implement struct/enum/trait/impl parsing**

- `parse_struct_def(p:I) I` — `struct Name { field Type, ... }`
- `parse_enum_def(p:I) I` — `enum Name { Variant, Variant(Type), ... }`
- `parse_trait_def(p:I) I` — `trait Name { fn method(self) Type }`
- `parse_impl_block(p:I) I` — `impl [Trait for] Type { methods }`

- [ ] **Step 8: Implement top-level program parsing**

- `parse(src:S) I` — entry point: lex → parse → return Program pointer
- `parse_program(p:I) I` — collect imports, globals, functions, structs, enums, traits, impls
- Import parsing: `import "path"`
- Global parsing: `g name = expr`

- [ ] **Step 9: Implement string interpolation desugaring**

When parser encounters `TK_INTERP_STRING`, desugar to chain of string concatenation:
- Literal parts → `EX_STRING_LIT`
- Ident parts → `EX_CALL("str", [EX_IDENT(name)])`
- Expr parts → re-lex and re-parse the expression text, wrap in `EX_CALL("str", [expr])`
- Chain with `EX_BINOP(OP_ADD, left, right)` (string concat)

- [ ] **Step 10: Write parser test**

Create `tests/fixtures/selfhost_parser_test.sans`:
```sans
import "compiler/lexer"
import "compiler/parser"
import "compiler/constants"

main() I {
  prog = parse("main() I { 42 }")
  funcs = load64(prog + 16)  // functions array
  f0 = funcs.get(0)          // first function
  name = load64(f0)          // function name
  // Verify function name is "main"
  ok := 1
  if char_at(name, 0) != 109 { ok = 0; 0 } else { 0 }  // 'm'
  ok
}
```

- [ ] **Step 11: Run test and commit**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --test e2e selfhost_parser
git add compiler/parser.sans tests/fixtures/selfhost_parser_test.sans
git commit -m "compiler: add self-hosted parser"
```

---

## Chunk 3: Type Checker

### Task 4: Type checker (compiler/typeck.sans)

Port of `crates/sans-typeck/src/lib.rs` (3,509 LOC) + `types.rs`. Type checks the AST, infers types, resolves generics.

**Files:**
- Create: `compiler/typeck.sans`
- Create: `tests/fixtures/selfhost_typeck_test.sans`

**Type struct layout:**
```
offset 0:  tag (I)       — TY_INT, TY_FLOAT, etc.
offset 8:  inner (I)     — inner type pointer (for Array<T>, Result<T>, Sender<T>, etc.)
offset 16: data (I)      — extra data pointer (struct fields map, enum variants map, fn params array)
offset 24: data2 (I)     — secondary data (fn return type, tuple element types array)
```

**FunctionSignature layout:**
```
offset 0:  params (I)      — Array of Type pointers
offset 8:  return_type (I) — Type pointer
```

**ModuleExports layout:**
```
offset 0:  functions (I)   — Map: name → FunctionSignature pointer
offset 8:  structs (I)     — Map: name → fields Map (name → Type pointer)
offset 16: enums (I)       — Map: name → variants Map (name → Array of Type pointers)
```

- [ ] **Step 1: Implement type constructors and utilities**

- `make_type(tag:I) I` — basic type with no inner
- `make_array_type(inner:I) I` — Array<T>
- `make_result_type(inner:I) I` — Result<T>
- `make_fn_type(params:I ret:I) I` — Fn type
- `make_struct_type(name:I fields:I) I` — Struct type
- `make_enum_type(name:I variants:I) I` — Enum type
- `make_tuple_type(elements:I) I` — Tuple type
- `type_eq(a:I b:I) I` — type equality check
- `types_compatible(actual:I expected:I) I` — covariance (ResultErr matches Result<T>, etc.)

- [ ] **Step 2: Implement resolve_type**

- `resolve_type(name:S structs:I enums:I module_exports:I) I` — parse type name string → Type pointer
- Handle short aliases: `I`→Int, `F`→Float, `B`→Bool, `S`→String, `M`→Map
- Handle `Array<T>`, `[T]`, `Result<T>`, `R<T>`
- Handle `Sender<T>`, `Receiver<T>`, `Mutex<T>`
- Handle struct/enum names from registry
- Handle tuple types `(T1 T2 T3)`

- [ ] **Step 3: Implement check_expr — literals and identifiers**

- `check_expr(expr:I locals:I fn_env:I ...) I` — returns Type pointer
- Handle: `EX_INT_LIT` → Int, `EX_FLOAT_LIT` → Float, `EX_BOOL_LIT` → Bool, `EX_STRING_LIT` → String
- Handle: `EX_IDENT` — look up in locals (stack of Maps), then globals
- Handle: `EX_ARRAY_LIT` — check all elements same type, return Array<T>
- Handle: `EX_TUPLE_LIT` — check each element, return Tuple type

- [ ] **Step 4: Implement check_expr — operators**

- Handle: `EX_BINOP` — type check left and right, verify compatible
  - Arithmetic (`+-*/%`): Int+Int→Int, Float+Float→Float, String+String→String
  - Comparison (`== != < > <= >=`): same type → Bool
  - Logical (`&& ||`): Bool+Bool→Bool
- Handle: `EX_UNARYOP` — `!` on Bool, `-` on Int/Float

- [ ] **Step 5: Implement check_expr — calls (built-in functions)**

This is the largest section. Must type-check all 100+ built-in functions.

- Handle each built-in by name (including all aliases):
  - Validate argument count
  - Validate argument types
  - Return correct return type
- Group by category: I/O, conversion, JSON, HTTP, memory, logging, socket, curl, result, arena, fptr, misc

- [ ] **Step 6: Implement check_expr — calls (user functions + generics)**

- Look up function in `fn_env` (Map: name → FunctionSignature)
- Check argument types match parameter types
- For generic functions: infer type parameters from arguments, instantiate
- Check cross-module function calls via `module_exports`

- [ ] **Step 7: Implement check_expr — method calls**

- Look up method in `methods` registry (Map: "Type.method" → FunctionSignature)
- Built-in methods on types:
  - Array: `.len()`, `.push()`, `.get()`, `.set()`, `.map()`, `.filter()`, `.any()`, `.find()`, `.enumerate()`, `.zip()`, `.pop()`, `.contains()`, `.remove()`
  - String: `.len()`, `.trim()`, `.split()`, `.replace()`, `.starts_with()`, `.ends_with()`, `.contains()`, `.substring()`, `.add()`
  - Map: `.get()`, `.set()`, `.has()`, `.len()`, `.keys()`, `.vals()`
  - Result: `.unwrap()`, `.is_ok()`, `.is_err()`, `.error()`, `.unwrap_or()`
  - JSON: `.get()`, `.get_index()`, `.get_string()`, `.get_int()`, `.get_bool()`, `.set()`, `.push()`, `.len()`, `.type_of()`, `.stringify()`
  - Http: `.status()`, `.body()`, `.header()`, `.ok()`, `.path()`, `.method()`
  - Channel: `.send()`, `.recv()`
  - Mutex: `.lock()`, `.unlock()`
  - Thread: `.join()`

- [ ] **Step 8: Implement check_expr — remaining forms**

- Handle: `EX_IF` — check condition is Bool, check both branches return same type
- Handle: `EX_MATCH` — check scrutinee, check all arms return same type
- Handle: `EX_LAMBDA` — check body with params as locals, return Fn type
- Handle: `EX_FIELD_ACCESS` — look up struct field type
- Handle: `EX_STRUCT_LIT` — check all fields present and correct types
- Handle: `EX_ENUM_VARIANT` — check variant exists and args match
- Handle: `EX_SPAWN` — return JoinHandle
- Handle: `EX_CHANNEL` — return Tuple(Sender<T>, Receiver<T>)
- Handle: `EX_MUTEX` — return Mutex<T>

- [ ] **Step 9: Implement check_stmt**

- `check_stmt(stmt:I locals:I ...) I`
- Handle: `ST_LET` — check value type, add to locals
- Handle: `ST_ASSIGN` — check value type matches existing binding
- Handle: `ST_WHILE` — check condition is Bool, check body
- Handle: `ST_FOR_IN` — check iterable is Array, add loop var to locals
- Handle: `ST_IF` — check condition is Bool, check body
- Handle: `ST_RETURN` — check value matches function return type
- Handle: `ST_EXPR` — check expression

- [ ] **Step 10: Implement top-level check function**

- `check(program:I module_exports:I) I` — entry point
- Register all struct definitions
- Register all enum definitions
- Register all function signatures
- Register all trait methods
- Register all impl methods
- Type-check each function body
- Return ModuleExports

- [ ] **Step 11: Write test and commit**

```bash
git add compiler/typeck.sans tests/fixtures/selfhost_typeck_test.sans
git commit -m "compiler: add self-hosted type checker"
```

---

## Chunk 4: IR Types + Lowering

### Task 5: IR module (compiler/ir.sans)

Port of `crates/sans-ir/src/ir.rs` (instruction types) + `crates/sans-ir/src/lib.rs` (lowering) — 3,514 LOC Rust.

**Files:**
- Create: `compiler/ir.sans`
- Create: `tests/fixtures/selfhost_ir_test.sans`

**IR instruction layout (variable size, opcode at offset 0):**

All instructions follow the pattern: `[opcode:I, dest:I, ...operands]`

The opcode values (0-110+) must be defined in constants.sans. Key opcodes grouped:

```
// IR opcodes (add to constants.sans)
IR_CONST = 0
IR_FLOAT_CONST = 1
IR_BOOL_CONST = 2
IR_STRING_CONST = 3
IR_BINOP = 4
IR_FLOAT_BINOP = 5
IR_CMPOP = 6
IR_FLOAT_CMPOP = 7
IR_STRING_CMPOP = 8
IR_NOT = 9
IR_NEG = 10
IR_COPY = 11
IR_CALL = 12
IR_RET = 13
IR_LABEL = 14
IR_BRANCH = 15
IR_JUMP = 16
IR_PHI = 17
IR_PRINT_INT = 18
IR_PRINT_FLOAT = 19
IR_PRINT_STRING = 20
IR_PRINT_BOOL = 21
IR_ALLOCA = 22
IR_STORE = 23
IR_LOAD = 24
IR_STRUCT_ALLOC = 25
IR_FIELD_STORE = 26
IR_FIELD_LOAD = 27
IR_ENUM_ALLOC = 28
IR_ENUM_TAG = 29
IR_ENUM_DATA = 30
IR_GLOBAL_LOAD = 31
IR_GLOBAL_STORE = 32
IR_ALLOC = 33
IR_DEALLOC = 34
IR_RALLOC = 35
// Array ops (36-49)
IR_ARRAY_CREATE = 36
IR_ARRAY_PUSH = 37
IR_ARRAY_GET = 38
IR_ARRAY_SET = 39
IR_ARRAY_LEN = 40
IR_ARRAY_MAP = 41
IR_ARRAY_FILTER = 42
IR_ARRAY_ANY = 43
IR_ARRAY_FIND = 44
IR_ARRAY_ENUMERATE = 45
IR_ARRAY_ZIP = 46
IR_ARRAY_POP = 47
IR_ARRAY_CONTAINS = 48
IR_ARRAY_REMOVE = 49
// String ops (50-61)
IR_STRING_LEN = 50
IR_STRING_CONCAT = 51
IR_STRING_SUBSTRING = 52
IR_INT_TO_STRING = 53
IR_STRING_TO_INT = 54
IR_FLOAT_TO_STRING = 55
IR_STRING_TRIM = 56
IR_STRING_STARTS_WITH = 57
IR_STRING_ENDS_WITH = 58
IR_STRING_CONTAINS = 59
IR_STRING_SPLIT = 60
IR_STRING_REPLACE = 61
// File I/O (62-65)
IR_FILE_READ = 62
IR_FILE_WRITE = 63
IR_FILE_APPEND = 64
IR_FILE_EXISTS = 65
// JSON ops (66-82)
IR_JSON_PARSE = 66
IR_JSON_OBJECT = 67
IR_JSON_ARRAY = 68
IR_JSON_STRING = 69
IR_JSON_INT = 70
IR_JSON_BOOL = 71
IR_JSON_NULL = 72
IR_JSON_GET = 73
IR_JSON_GET_INDEX = 74
IR_JSON_GET_STRING = 75
IR_JSON_GET_INT = 76
IR_JSON_GET_BOOL = 77
IR_JSON_LEN = 78
IR_JSON_TYPE_OF = 79
IR_JSON_SET = 80
IR_JSON_PUSH = 81
IR_JSON_STRINGIFY = 82
// HTTP server (83-89)
IR_HTTP_LISTEN = 83
IR_HTTP_ACCEPT = 84
IR_HTTP_REQUEST_PATH = 85
IR_HTTP_REQUEST_METHOD = 86
IR_HTTP_REQUEST_BODY = 87
IR_HTTP_RESPOND = 88
IR_HTTP_RESPOND_CT = 89
// HTTP client (90-95)
IR_HTTP_GET = 90
IR_HTTP_POST = 91
IR_HTTP_STATUS = 92
IR_HTTP_BODY = 93
IR_HTTP_HEADER = 94
IR_HTTP_OK = 95
// Concurrency (96-105)
IR_THREAD_SPAWN = 96
IR_THREAD_JOIN = 97
IR_CHANNEL_CREATE = 98
IR_CHANNEL_CREATE_BOUNDED = 99
IR_CHANNEL_SEND = 100
IR_CHANNEL_RECV = 101
IR_MUTEX_CREATE = 102
IR_MUTEX_LOCK = 103
IR_MUTEX_UNLOCK = 104
// Function refs (105-108)
IR_FN_REF = 105
IR_FPTR_NAMED = 106
IR_FCALL = 107
IR_FCALL2 = 108
IR_FCALL3 = 109
// Float conversion (110-111)
IR_INT_TO_FLOAT = 110
IR_FLOAT_TO_INT = 111
// Result ops (112-118)
IR_RESULT_OK = 112
IR_RESULT_ERR = 113
IR_RESULT_IS_OK = 114
IR_RESULT_IS_ERR = 115
IR_RESULT_UNWRAP = 116
IR_RESULT_UNWRAP_OR = 117
IR_RESULT_ERROR = 118
// Logging (119-123)
IR_LOG_DEBUG = 119
IR_LOG_INFO = 120
IR_LOG_WARN = 121
IR_LOG_ERROR = 122
IR_LOG_SET_LEVEL = 123
IR_GET_LOG_LEVEL = 124
IR_SET_LOG_LEVEL = 125
// Low-level memory (126-135)
IR_LOAD8 = 126
IR_STORE8 = 127
IR_LOAD16 = 128
IR_STORE16 = 129
IR_LOAD32 = 130
IR_STORE32 = 131
IR_LOAD64 = 132
IR_STORE64 = 133
IR_BSWAP16 = 134
IR_SLEN = 135
// Socket/network (136-143)
IR_SOCK = 136
IR_SBIND = 137
IR_SLISTEN = 138
IR_SACCEPT = 139
IR_SRECV = 140
IR_SSEND = 141
IR_SCLOSE = 142
IR_RBIND = 143
IR_RSETSOCKOPT = 144
IR_STRSTR = 145
// libcurl (146-153)
IR_CINIT = 146
IR_CSETS = 147
IR_CSETI = 148
IR_CPERF = 149
IR_CCLEAN = 150
IR_CINFO = 151
IR_CURL_SLIST_APPEND = 152
IR_CURL_SLIST_FREE = 153
// Map (154-160)
IR_MAP_CREATE = 154
IR_MAP_GET = 155
IR_MAP_SET = 156
IR_MAP_HAS = 157
IR_MAP_LEN = 158
IR_MAP_KEYS = 159
IR_MAP_VALS = 160
// Arena (161-163)
IR_ARENA_BEGIN = 161
IR_ARENA_ALLOC = 162
IR_ARENA_END = 163
// Misc (164-167)
IR_ARGS = 164
IR_EXIT = 165
IR_PRINT_ERR = 166
IR_WRITE_FD = 167
IR_SYSTEM = 168
IR_MCPY = 169
IR_MZERO = 170
IR_MCMP = 171
IR_CHAR_AT = 172
```

**IrFunction layout:**
```
offset 0:  name (I)            — function name string
offset 8:  params (Array<I>)   — parameter name strings
offset 16: instructions (Array<I>) — array of instruction pointers
offset 24: return_type (I)     — IrType tag
offset 32: param_struct_sizes (Array<I>) — struct field count per param (0 = scalar)
```

**IrModule layout:**
```
offset 0:  functions (Array<I>)   — array of IrFunction pointers
offset 8:  globals (Array<I>)     — array of global name strings
offset 16: struct_defs (I)        — Map: name → field names array
```

- [ ] **Step 1: Define IR instruction constructors**

Create helper functions for each instruction type:
- `ir_const(dest:I value:I) I` — allocate Const instruction
- `ir_binop(dest:I op:I left:I right:I) I`
- `ir_call(dest:I fname:I args:I) I`
- ... (one constructor per opcode)

Use a consistent layout: `[opcode, dest, field1, field2, ...]`

- [ ] **Step 2: Implement lowering state**

Lowering context struct:
```
offset 0:  instructions (I)  — Array of instruction pointers (current function)
offset 8:  reg_counter (I)   — next register number (alloca for mutability)
offset 16: label_counter (I) — next label number
offset 24: locals (I)        — Map: name → LocalVar (value reg or alloca ptr)
offset 32: reg_types (I)     — Map: reg_name → IrType tag
offset 40: struct_defs (I)   — Map: struct_name → field names array
offset 48: enum_defs (I)     — Map: enum_name → variants info
offset 56: fn_ret_types (I)  — Map: func_name → IrType tag
```

- `fresh_reg(ctx:I) I` — return next register name (e.g., "r0", "r1", ...)
- `fresh_label(ctx:I) I` — return next label name

- [ ] **Step 3: Implement lower_expr — literals and identifiers**

- `lower_expr(ctx:I expr:I) I` — returns register name string
- `EX_INT_LIT` → emit `IR_CONST`
- `EX_FLOAT_LIT` → emit `IR_FLOAT_CONST`
- `EX_BOOL_LIT` → emit `IR_BOOL_CONST`
- `EX_STRING_LIT` → emit `IR_STRING_CONST`
- `EX_IDENT` → look up in locals, emit `IR_COPY` or `IR_LOAD` (for mutable vars)

- [ ] **Step 4: Implement lower_expr — operators**

- `EX_BINOP` → lower left and right, check types, emit `IR_BINOP` / `IR_FLOAT_BINOP` / `IR_STRING_CMPOP`
- Short-circuit `&&`/`||` → emit branches + phi nodes
- `EX_UNARYOP` → emit `IR_NOT` or `IR_NEG`

- [ ] **Step 5: Implement lower_expr — function calls**

This is the largest section (~1500 LOC in Rust). Every built-in function maps to a specific IR instruction.

- Lower all args to registers first
- Match function name against all built-ins (including aliases)
- Emit corresponding IR instruction
- For user-defined functions: emit `IR_CALL`

- [ ] **Step 6: Implement lower_expr — method calls**

- Lower object to register, determine type from `reg_types`
- Dispatch based on type + method name
- Array methods → `IR_ARRAY_MAP`, `IR_ARRAY_FILTER`, etc.
- String methods → `IR_STRING_TRIM`, `IR_STRING_SPLIT`, etc.
- Map methods → `IR_MAP_GET`, `IR_MAP_SET`, etc.
- Struct methods → `IR_CALL` with mangled name
- Result methods → `IR_RESULT_UNWRAP`, etc.

- [ ] **Step 7: Implement lower_expr — remaining forms**

- `EX_IF` → emit branch + labels + phi for value
- `EX_MATCH` → emit tag check chain + branches
- `EX_LAMBDA` → capture variables, emit as separate function
- `EX_FIELD_ACCESS` → emit `IR_FIELD_LOAD`
- `EX_STRUCT_LIT` → emit `IR_STRUCT_ALLOC` + `IR_FIELD_STORE` for each field
- `EX_TUPLE_LIT` → emit `IR_STRUCT_ALLOC` + `IR_FIELD_STORE`
- `EX_ARRAY_LIT` → emit `IR_ARRAY_CREATE` + `IR_ARRAY_PUSH` per element
- `EX_ENUM_VARIANT` → emit `IR_ENUM_ALLOC` + store data fields
- `EX_SPAWN` → emit `IR_THREAD_SPAWN`
- `EX_CHANNEL` → emit `IR_CHANNEL_CREATE`
- `EX_MUTEX` → emit `IR_MUTEX_CREATE`

- [ ] **Step 8: Implement lower_stmt**

- `lower_stmt(ctx:I stmt:I) I`
- `ST_LET` → lower value, store in locals (Value for immutable, Alloca+Store for mutable)
- `ST_ASSIGN` → lower value, emit `IR_STORE`
- `ST_WHILE` → emit loop label, condition branch, body, jump back
- `ST_FOR_IN` → desugar to while loop with index counter
- `ST_IF` → emit branch + labels
- `ST_RETURN` → emit `IR_RET`
- `ST_BREAK` / `ST_CONTINUE` → emit `IR_JUMP` to loop exit/header
- `ST_EXPR` → lower expression (discard result)

- [ ] **Step 9: Implement top-level lower function**

- `lower(program:I module_name:I module_fn_ret_types:I) I` — returns IrModule pointer
- Collect struct definitions → struct_defs Map
- Collect enum definitions → enum_defs Map
- For each function: create lowering context, lower all statements, return IrFunction
- Handle global variables: emit `IR_GLOBAL_STORE` in init
- Handle function name mangling for modules

- [ ] **Step 10: Write test and commit**

```bash
git add compiler/ir.sans tests/fixtures/selfhost_ir_test.sans
git commit -m "compiler: add self-hosted IR types and lowering"
```

---

## Chunk 5: Codegen

### Task 6: Codegen (compiler/codegen.sans)

Port of `crates/sans-codegen/src/lib.rs` (4,151 LOC Rust). Instead of inkwell LLVM bindings, emit LLVM IR as text strings.

**Files:**
- Create: `compiler/codegen.sans`
- Create: `tests/fixtures/selfhost_codegen_test.sans`

**Strategy:** Build an array of strings (lines of LLVM IR), join at the end, write to `.ll` file.

**Codegen state layout:**
```
offset 0:  lines (I)         — Array<S> of LLVM IR lines
offset 8:  reg_counter (I)   — SSA value counter for LLVM
offset 16: string_pool (I)   — Map: string content hash → global name
offset 24: struct_defs (I)   — Map: struct name → field count
offset 32: label_map (I)     — Map: IR label → LLVM basic block name
offset 40: reg_map (I)       — Map: IR register name → LLVM SSA value name
offset 48: ptr_set (I)       — Map: register name → 1 (tracks which regs hold pointers)
```

- [ ] **Step 1: Implement LLVM IR text helpers**

- `emit(cg:I line:S) I` — append line to lines array
- `llvm_reg(cg:I) S` — return next `%N` register name
- `llvm_label(cg:I name:S) S` — return sanitized label name
- `join_lines(cg:I) S` — join all lines with newlines (use alloc+mcpy for efficiency)

String building strategy for `join_lines`:
1. Sum all string lengths + newlines
2. `alloc(total + 1)`
3. `mcpy` each string into buffer at correct offset
4. Null-terminate

- [ ] **Step 2: Emit LLVM IR header and external declarations**

- `emit_header(cg:I) I` — target triple, data layout
- `emit_externals(cg:I) I` — declare all external functions:
  - C stdlib: `printf`, `malloc`, `free`, `strlen`, `memcpy`, `memset`, `memcmp`, `realloc`, `strcmp`, `snprintf`, `strtol`, `write`, `exit`, `system`, `access`
  - File I/O: `fopen`, `fclose`, `fread`, `fwrite`, `fseek`, `ftell`
  - Threading: `pthread_create`, `pthread_join`, `pthread_mutex_init/lock/unlock`, `pthread_cond_init/wait/signal`
  - Curl: `curl_easy_init/setopt/perform/cleanup/getinfo`, `curl_slist_append/free_all`
  - Sans runtime: all `sans_*` functions

Each declaration is a string like:
```
declare i8* @malloc(i64)
declare void @free(i8*)
declare i64 @sans_map_create()
```

- [ ] **Step 3: Emit global variables and string constants**

- `emit_globals(cg:I module:I) I` — emit `@global_name = global i64 0` for each global
- `emit_string_const(cg:I value:S) S` — emit string constant, return global name
  - Use FNV-1a hash for dedup (same as Rust version)
  - Format: `@str.HASH.N = private unnamed_addr constant [LEN x i8] c"ESCAPED\00"`
  - Escape special chars in LLVM IR string constants

- [ ] **Step 4: Emit main wrapper**

If module has `main` function:
- Rename user's `main` to `@__sans_main`
- Emit C-compatible `@main(i32, i8**)`:
  - Store argc/argv to globals `@__sans_argc`, `@__sans_argv`
  - Call `@__sans_main()`
  - Truncate i64 result to i32 for C return

- [ ] **Step 5: Compile IR instructions — constants and arithmetic**

For each function, iterate instructions and emit LLVM IR:

- `IR_CONST` → `%N = add i64 VALUE, 0` (or just use constant inline)
- `IR_FLOAT_CONST` → `%N = bitcast i64 BITS to double` then back
- `IR_BOOL_CONST` → `%N = add i64 0/1, 0`
- `IR_STRING_CONST` → emit global string, `%N = ptrtoint [LEN x i8]* @str.HASH to i64`
- `IR_BINOP` → `%N = add/sub/mul/sdiv/srem i64 %L, %R`
- `IR_FLOAT_BINOP` → bitcast to double, `fadd/fsub/fmul/fdiv/frem`, bitcast back
- `IR_CMPOP` → `%tmp = icmp eq/ne/slt/sgt/sle/sge i64 %L, %R` then `%N = zext i1 %tmp to i64`
- `IR_NOT` → `%N = xor i64 %V, 1`
- `IR_NEG` → `%N = sub i64 0, %V`

- [ ] **Step 6: Compile IR instructions — control flow**

- `IR_LABEL` → emit label: `LABEL_NAME:`
- `IR_BRANCH` → `%cond = trunc i64 %V to i1` then `br i1 %cond, label %THEN, label %ELSE`
- `IR_JUMP` → `br label %TARGET`
- `IR_PHI` → `%N = phi i64 [%A, %LABEL_A], [%B, %LABEL_B]`
- `IR_RET` → `ret i64 %V`

- [ ] **Step 7: Compile IR instructions — memory and structs**

- `IR_ALLOCA` → `%ptr = alloca i64` then `%N = ptrtoint i64* %ptr to i64`
- `IR_STORE` → `%ptr = inttoptr i64 %P to i64*` then `store i64 %V, i64* %ptr`
- `IR_LOAD` → `%ptr = inttoptr i64 %P to i64*` then `%N = load i64, i64* %ptr`
- `IR_STRUCT_ALLOC` → call `@malloc(N*8)`, cast result to i64
- `IR_FIELD_STORE` → GEP into struct, store
- `IR_FIELD_LOAD` → GEP into struct, load
- `IR_ENUM_ALLOC` → malloc, store tag at offset 0
- `IR_ENUM_TAG` / `IR_ENUM_DATA` → load from enum struct

- [ ] **Step 8: Compile IR instructions — function calls**

- `IR_CALL` → `%N = call i64 @FUNC_NAME(i64 %arg1, i64 %arg2, ...)`
- `IR_FPTR_NAMED` → `%N = ptrtoint i64 (i64, ...)* @FUNC to i64`
- `IR_FCALL` → cast fn ptr, `%N = call i64 %fp(i64 %arg)`
- `IR_FCALL2` / `IR_FCALL3` — same with 2/3 args

- [ ] **Step 9: Compile IR instructions — arrays, strings, print**

- `IR_ARRAY_CREATE` → call `@malloc(24)`, init [ptr, 0, 8]
- `IR_ARRAY_PUSH` → inline growth check + store (or call runtime)
- `IR_ARRAY_GET` / `IR_ARRAY_SET` → GEP into data buffer
- `IR_ARRAY_LEN` → load len field
- `IR_ARRAY_MAP/FILTER/...` → `call i64 @sans_array_map(i64 %arr, i64 %fn)`
- `IR_STRING_CONCAT` → call `@malloc`, `@memcpy`, etc.
- `IR_STRING_LEN` → call `@strlen`
- `IR_INT_TO_STRING` → call `@snprintf`
- `IR_PRINT_INT/FLOAT/STRING/BOOL` → call `@printf`

- [ ] **Step 10: Compile IR instructions — all remaining**

- JSON ops → call `@sans_json_*` functions
- HTTP ops → call `@sans_http_*` functions
- Channel/Mutex/Thread ops → emit pthread-based inline code (channels are complex — inline buffer management)
- Socket ops → call `@socket`, `@bind`, etc.
- Curl ops → call `@curl_*` functions
- Result ops → call `@sans_result_*` functions
- Log ops → call `@sans_log_*` functions
- Map ops → call `@sans_map_*` functions
- Arena ops → call `@sans_arena_*` functions
- Memory ops (load8/16/32/64, store8/16/32/64) → LLVM load/store with appropriate types
- `IR_SYSTEM` → call `@system`

- [ ] **Step 11: Implement compile_to_ll top-level function**

- `compile_to_ll(module:I output_path:S) I`
  1. Create codegen state
  2. `emit_header(cg)`
  3. `emit_externals(cg)`
  4. `emit_globals(cg, module)`
  5. First pass: declare all functions
  6. Second pass: compile each function body
  7. Emit main wrapper if needed
  8. `join_lines(cg)` → full LLVM IR string
  9. `file_write(output_path, ll_string)`

- [ ] **Step 12: Write test**

Create `tests/fixtures/selfhost_codegen_test.sans` — compile a trivial program through the full pipeline (lex → parse → typecheck → lower → codegen → write .ll), then read the .ll file and check it contains expected strings.

- [ ] **Step 13: Commit**

```bash
git add compiler/codegen.sans tests/fixtures/selfhost_codegen_test.sans
git commit -m "compiler: add self-hosted codegen (LLVM IR text emission)"
```

---

## Chunk 6: Driver + Bootstrap

### Task 7: Driver (compiler/main.sans)

Port of `crates/sans-driver/src/main.rs` (346 LOC) + `imports.rs` (98 LOC).

**Files:**
- Create: `compiler/main.sans`

- [ ] **Step 1: Implement import resolution**

- `resolve_imports(entry_path:S) I` — returns Array of ResolvedModule structs
- Recursive depth-first traversal of import graph
- Cycle detection via `in_progress` Map (path → 1)
- Deduplication via `visited` Map (path → 1)
- Topological order: dependencies emitted before dependents
- Path resolution: relative to entry file's directory

**ResolvedModule layout:**
```
offset 0:  name (I)      — module name string (last path component)
offset 8:  path (I)      — full path string
offset 16: program (I)   — parsed Program pointer
```

- [ ] **Step 2: Implement CLI argument parsing**

- Parse `args()` array
- Commands: `build <file.sans>`, `run <file.sans>`
- Flags: `--version`, `-V`, `--dump-tokens`, `--dump-ast`, `--dump-ir`, `--emit-ll`

- [ ] **Step 3: Implement build pipeline**

- `build(source_path:S output_path:S) I`:
  1. Resolve imports
  2. Parse entry point
  3. Type-check modules in dependency order, collecting ModuleExports
  4. Type-check entry point
  5. Build module_fn_ret_types Map
  6. Build extra_struct_defs Map
  7. Lower each module to IR (with module name prefix)
  8. Lower entry point to IR
  9. Merge all IR functions into single module
  10. Codegen: emit `.ll` file
  11. Shell out: `system("llc -filetype=obj -o OUTPUT.o OUTPUT.ll")`
  12. Compile each runtime `.sans` module to `.o` (parse → typecheck → lower → codegen for each)
  13. Shell out: `system("clang OUTPUT.o runtime_objs... -lcurl -lpthread -o OUTPUT")`
  14. Clean up `.o` and `.ll` files

**Arena allocation per phase** (spec section 10):
- Wrap each phase in `arena_begin()` / `arena_end()` to free intermediate allocations:
  - Phase 1 arena: lexer tokens (freed after parsing)
  - Phase 2 arena: AST nodes (freed after IR lowering)
  - Phase 3 arena: IR instructions (freed after codegen)
  - Phase 4 arena: codegen string buffer (freed after file write)

**Multi-module compilation of the compiler itself:**
- The compiler is 6+ `.sans` files that import each other
- The Rust compiler's existing import resolution + multi-module compilation handles this
- When building `compiler/main.sans`, it resolves `import "compiler/lexer"` etc.
- Each module is compiled, type-checked, lowered, and linked like any other Sans program
- No special handling needed — the existing import system works

**Runtime compilation:**
- Runtime is self-hosted (`.sans` files, not C)
- Each runtime module must be compiled to `.o` before linking
- Use same pipeline: parse → typecheck → lower → codegen for each `.sans` file
- Pre-compile at build time, cache `.o` files for reuse

- [ ] **Step 4: Implement run command**

- `run(source_path:S) I`:
  1. Call `build()` with temp output path
  2. `system("./OUTPUT")`
  3. Clean up

- [ ] **Step 5: Implement main function**

- `main() I`:
  1. Parse args
  2. Dispatch to build/run/version
  3. Return exit code

- [ ] **Step 6: Write integration test**

Test the full pipeline: compile a simple `.sans` file using the self-hosted compiler, then run the output.

Create `tests/selfhost_e2e.sh`:
```bash
#!/bin/bash
# Build the self-hosted compiler using the Rust compiler
sans build compiler/main.sans -o sans2
# Use sans2 to compile a test fixture
./sans2 build tests/fixtures/hello.sans -o hello_test
# Run and check output
RESULT=$(./hello_test)
echo "Exit code: $?"
```

- [ ] **Step 7: Commit**

```bash
git add compiler/main.sans tests/selfhost_e2e.sh
git commit -m "compiler: add self-hosted driver — full compilation pipeline"
```

---

### Task 8: Bootstrap — Compile the compiler with itself

- [ ] **Step 1: Build the self-hosted compiler using Rust compiler**

```bash
# Stage 0: Rust compiler builds Sans compiler
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo run -- build compiler/main.sans -o sans-stage1
```

- [ ] **Step 2: Test stage 1 against all fixtures**

```bash
# Run all 97 test fixtures through stage 1
for f in tests/fixtures/*.sans; do
  RUST_RESULT=$(sans run "$f" 2>/dev/null; echo $?)
  SANS_RESULT=$(./sans-stage1 run "$f" 2>/dev/null; echo $?)
  if [ "$RUST_RESULT" != "$SANS_RESULT" ]; then
    echo "MISMATCH: $f (rust=$RUST_RESULT, sans=$SANS_RESULT)"
  fi
done
```

Fix any mismatches before proceeding.

- [ ] **Step 3: Stage 1 compiles itself (bootstrap)**

```bash
# Stage 1: Sans compiler builds Sans compiler
./sans-stage1 build compiler/main.sans -o sans-stage2
```

- [ ] **Step 4: Test stage 2 against all fixtures**

```bash
# Verify stage 2 produces same results as stage 1
for f in tests/fixtures/*.sans; do
  S1_RESULT=$(./sans-stage1 run "$f" 2>/dev/null; echo $?)
  S2_RESULT=$(./sans-stage2 run "$f" 2>/dev/null; echo $?)
  if [ "$S1_RESULT" != "$S2_RESULT" ]; then
    echo "BOOTSTRAP MISMATCH: $f (s1=$S1_RESULT, s2=$S2_RESULT)"
  fi
done
```

- [ ] **Step 5: Verify .ll output stability**

```bash
# Stage 1 and Stage 2 should produce identical .ll for the same input
./sans-stage1 build --emit-ll tests/fixtures/hello.sans -o /tmp/s1.ll
./sans-stage2 build --emit-ll tests/fixtures/hello.sans -o /tmp/s2.ll
diff /tmp/s1.ll /tmp/s2.ll
```

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat: self-hosted Sans compiler — bootstrap verified v0.4.0"
```

---

## Summary

| Task | Module | Est. LOC | Depends On |
|------|--------|----------|------------|
| 0 | system() builtin | ~50 (Rust) | — |
| 1 | constants.sans | ~300 | — |
| 2 | lexer.sans | 600-900 | constants |
| 3 | parser.sans | 2500-3500 | lexer, constants |
| 4 | typeck.sans | 3500-5000 | parser, constants |
| 5 | ir.sans | 3500-5000 | parser, typeck, constants |
| 6 | codegen.sans | 4000-6000 | ir, constants |
| 7 | main.sans | 500-700 | all modules |
| 8 | bootstrap | — | all modules |

**Total estimated:** ~15,000-21,000 LOC Sans + ~50 LOC Rust (system() builtin)

**Debugging/dump functions:** Each module should include a dump function for the `--dump-*` flags:
- `lexer.sans`: `dump_tokens(tokens:I) I` — print each token's kind and value
- `parser.sans`: `dump_ast(program:I) I` — recursive pretty-print of AST
- `ir.sans`: `dump_ir(module:I) I` — print each function's instructions
- `codegen.sans`: `--emit-ll` writes `.ll` without invoking llc/clang

**Version strategy:** Tasks 1-7 are development on the feature branch — no version bumps. Version 0.4.0 applied at Task 8 (bootstrap verified) before merge to main.
