# Planning a Sans Feature

Step-by-step guide for planning a feature, fix, or change to the Sans compiler.

## 1. Identify the Feature Type

Determine which pipeline your change follows:

### New Built-in Function (e.g., `abs`, `strlen`)
| Step | File | What to do |
|------|------|------------|
| 1 | `compiler/typeck.sans` | Add type signature in builtin call dispatch |
| 2 | `compiler/constants.sans` | Add IR instruction constant (`IR_NAME`) |
| 3 | `compiler/ir.sans` | Lower the call to the IR instruction |
| 4 | `compiler/codegen.sans` | Emit LLVM IR for the instruction |
| 5 | `runtime/*.sans` | Add `sans_` runtime function (if needed) |
| 6 | `tests/fixtures/` | Add E2E test fixture |
| 7 | All docs | See Documentation Checklist below |

### New Method on a Type (e.g., `arr.reverse()`)
Same as above, but use method dispatch in typeck and IR lowering.

### New Type (e.g., `Set`, `Queue`)
| Step | File | What to do |
|------|------|------------|
| 1 | `compiler/typeck.sans` | Add type resolution |
| 2 | `compiler/ir.sans` | Add IR type handling |
| 3 | `compiler/codegen.sans` | Add codegen support |
| 4 | `runtime/*.sans` | Add runtime backing with `sans_` prefix (if opaque) |
| 5 | `tests/fixtures/` | Add E2E test fixture |
| 6 | All docs | See Documentation Checklist below |

## 2. Check Conventions

Before writing code, verify your design follows:
- **i64 register model** — all values stored as i64 in IR/codegen
- **Pointer dual storage** — pointers in both `regs` (i64) and `ptrs` (PointerValue)
- **Opaque types** — runtime-backed types use `sans_` prefix functions
- **Scope GC** — allocations tracked per-function, freed on return, return values promoted
- **AI-optimized syntax** — use the shortest form possible (`I/S/B/F`, bare assignment, no commas)

## 3. Documentation Checklist

Copy this into your plan — every feature must update ALL items:

```
- [ ] docs/reference.md — human-readable reference
- [ ] docs/ai-reference.md — compact AI reference
- [ ] website/docs/index.html — website documentation
- [ ] editors/vscode-sans/src/extension.ts — HOVER_DATA entry
- [ ] editors/vscode-sans/syntaxes/sans.tmLanguage.json — syntax highlighting
- [ ] tests/fixtures/ — E2E test fixture
- [ ] examples/ — update or add example (if significant)
- [ ] README.md — update feature list (if user-facing)
```

## 4. Scope GC Considerations

For every feature that allocates memory, answer:
- Is the allocation tracked by scope GC?
- Does the return value get promoted to the caller's scope?
- Are nested containers (array of arrays, map of strings) recursively promoted?
- Should this use arenas for hot paths?

## 5. Write Your Plan

Structure your plan as a series of tasks, each with:
- Exact file paths to create or modify
- The specific changes needed
- Test fixtures with expected exit codes
- Documentation updates

**Next step:** [Architecture Review](architecture-review.md)
