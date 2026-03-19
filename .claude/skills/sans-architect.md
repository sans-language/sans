---
name: sans-architect
type: skill
description: Evaluate a design against Sans compiler architecture — register model, opaque types, scope GC, and AI-optimized syntax conventions
---

# Sans Architecture Review

You are reviewing a proposed design for the Sans compiler. Evaluate it against the project's architecture and conventions.

## Sans Architecture Overview

**Compiler pipeline:** Source → Lexer → Parser → Typeck → IR → Codegen → LLVM IR → Binary

**Key files:**
| File | Purpose |
|------|---------|
| `compiler/lexer.sans` | Tokenizes source |
| `compiler/parser.sans` | Tokens → AST |
| `compiler/typeck.sans` | Type-checks AST, resolves types |
| `compiler/constants.sans` | IR opcodes, type/statement tags |
| `compiler/ir.sans` | AST → flat IR |
| `compiler/codegen.sans` | IR → LLVM IR text |
| `compiler/main.sans` | CLI, pipeline orchestration, linking |

**Runtime:** 13 `.sans` modules in `runtime/` — all self-hosted, no C.

**Register model:** All values stored as `i64`. Pointers stored in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue).

**Opaque types:** JsonValue, HttpResponse, Result, Map, etc. — backed by runtime with `sans_` prefix functions.

**Scope GC:** Allocations tracked per-function, freed on return. Return values promoted to caller scope (including nested container contents).

## Evaluation Checklist

For every proposed design, answer these questions:

### Fit with Register Model
- [ ] Does this use i64 for value storage?
- [ ] If it introduces pointers, are they stored in both `regs` and `ptrs`?
- [ ] Does it need a new type tag in `constants.sans`?

### IR Design
- [ ] Does this need a new IR instruction, or can it reuse an existing one?
- [ ] Is the operation compile-time (typeck/codegen) or runtime (needs `sans_` function)?
- [ ] If it adds an IR instruction, is the constant added to `constants.sans`?

### Scope GC Interaction
- [ ] Does this allocate heap memory?
- [ ] Will allocations be tracked by scope GC?
- [ ] What happens when this value crosses a function boundary?
- [ ] Are there nested containers that need recursive promotion?
- [ ] Does this need arena support for hot paths?

### AI-Optimized Syntax
- [ ] Can the syntax be expressed in fewer tokens?
- [ ] Are short type names used (`I/S/B/F/R<T>`)?
- [ ] Does it follow existing patterns (bare assignment, expression bodies, no commas)?

### Conflicts
- [ ] Does this conflict with existing builtins or methods?
- [ ] Does it introduce ambiguity in the parser?
- [ ] Does it break backward compatibility?

### Thread Safety
- [ ] Does this touch global state (`rc_alloc_head`, `rc_scope_head`)?
- [ ] If used from spawned threads, are there race conditions?
- (Note: thread safety is a known limitation — flag as "worth discussing", not blocking)

## Output Format

Present findings as:

### Architecture Review: [Feature Name]

**Verdict:** APPROVED / NEEDS CHANGES / BLOCKED

**Findings:**
- **Blocking:** [issues that must be resolved]
- **Worth discussing:** [design questions to consider]
- **Minor nits:** [suggestions, not required]

**Recommendations:** [specific suggestions for improvement]

## After Review

If the design needs iteration, work with the user to resolve issues. Once approved, invoke the `superpowers:brainstorming` skill to continue the design process with the architecture context established here.
