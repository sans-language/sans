# Architecture Review for Sans

Step-by-step guide for evaluating a design against Sans compiler architecture.

## Sans Architecture Quick Reference

**Pipeline:** Source → Lexer → Parser → Typeck → IR → Codegen → LLVM IR → Binary

| File | Purpose |
|------|---------|
| `compiler/lexer.sans` | Tokenizes source |
| `compiler/parser.sans` | Tokens → AST |
| `compiler/typeck.sans` | Type-checks AST, resolves types |
| `compiler/constants.sans` | IR opcodes, type/statement tags |
| `compiler/ir.sans` | AST → flat IR |
| `compiler/codegen.sans` | IR → LLVM IR text |
| `compiler/main.sans` | CLI, orchestration, linking |

**Runtime:** 13 `.sans` modules in `runtime/` — fully self-hosted, zero C code.

**Memory:** Scope-based GC — allocations freed on function return, return values promoted to caller.

## Review Checklist

For every proposed design, work through each section:

### Does It Fit the Register Model?
- All values must be storable as i64
- Pointers go in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue)
- New types need a type tag in `constants.sans`

### IR Design Decision Tree
```
Is this a compile-time operation?
├── Yes → Handle in typeck/codegen only (no IR instruction needed)
└── No → Runtime operation
    ├── Simple (1-2 LLVM instructions) → Inline in codegen
    └── Complex → Add sans_ runtime function
        ├── Add IR instruction constant to constants.sans
        ├── Add lowering in ir.sans
        └── Add codegen emission in codegen.sans
```

### Scope GC Interaction
- Does it allocate heap memory? → Must be tracked
- Does it return allocated memory? → Must promote to caller scope
- Nested containers? → Recursive promotion needed
- Hot path? → Consider arena support

### AI-Optimized Syntax Check
- Can the syntax use fewer tokens?
- Short type names: `I/S/B/F/R<T>`
- Bare assignment: `x = 42` not `let x = 42`
- Expression bodies: `f(x:I) = x*2`
- No commas: `[1 2 3]`

### Conflict Check
- Does it clash with existing builtins or methods?
- Does it introduce parser ambiguity?
- Does it break backward compatibility?

### Thread Safety
- Does it touch global state (`rc_alloc_head`, `rc_scope_head`)?
- Thread safety is a known limitation — flag concerns as "worth discussing", not blocking

## Output Template

```
### Architecture Review: [Feature Name]

**Verdict:** APPROVED / NEEDS CHANGES / BLOCKED

**Findings:**
- Blocking: [must fix]
- Worth discussing: [design questions]
- Minor nits: [suggestions]

**Recommendations:** [specific improvements]
```

**Next step:** [Skeptic Review](skeptic-review.md) (challenge the design before implementing)
