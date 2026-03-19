---
name: sans-plan
type: skill
description: Plan a Sans compiler feature — maps compiler pipeline stages, doc checklist, and AI syntax rules before creating the implementation plan
---

# Sans Feature Planning

You are planning a feature for the Sans compiler. Before creating the implementation plan, gather project-specific context.

## Step 1: Identify the Feature Type

Determine which pipeline applies:

**New built-in function** (e.g., `abs`, `exit`):
1. `compiler/typeck.sans` — add type signature in builtin call dispatch
2. `compiler/constants.sans` — add IR instruction constant (e.g., `IR_ABS`)
3. `compiler/ir.sans` — lower the call to the new IR instruction
4. `compiler/codegen.sans` — emit LLVM IR for the instruction
5. `runtime/*.sans` — add backing function with `sans_` prefix (if needed)

**New method on a type** (e.g., `arr.reverse()`):
1. `compiler/typeck.sans` — add method in type method dispatch
2. `compiler/constants.sans` — add IR instruction constant
3. `compiler/ir.sans` — lower via method dispatch
4. `compiler/codegen.sans` — emit LLVM IR
5. `runtime/*.sans` — add backing function (if needed)

**New type** (e.g., `Set`):
1. `compiler/typeck.sans` — add type resolution
2. `compiler/ir.sans` — add IR type handling
3. `compiler/codegen.sans` — add codegen support
4. `runtime/*.sans` — add runtime backing with `sans_` prefix (if opaque)

## Step 2: Check Conventions

Before planning, verify the design follows Sans conventions:
- **i64 register model**: all values stored as i64 in IR/codegen register map
- **Pointer dual storage**: pointers in both `regs` (i64 via ptr_to_int) and `ptrs` (PointerValue)
- **Opaque types**: backed by runtime functions with `sans_` prefix
- **Scope GC**: allocations tracked per-function, freed on return, return values promoted to caller
- **AI-optimized syntax**: can this be expressed in fewer tokens? Use short forms (`I/S/B/F`, bare assignment, no commas)

## Step 3: Pre-populate Documentation Checklist

Every feature plan MUST include tasks for ALL of these:
1. `docs/reference.md` — human-readable reference
2. `docs/ai-reference.md` — compact AI reference
3. `website/docs/index.html` — website documentation
4. `editors/vscode-sans/src/extension.ts` — hover docs in `HOVER_DATA`
5. `editors/vscode-sans/syntaxes/sans.tmLanguage.json` — syntax highlighting
6. `tests/fixtures/` — E2E test fixture(s)
7. `examples/` — update or add example if significant
8. `README.md` — update feature list if user-facing

## Step 4: Scope GC Implications

Consider:
- Does this feature allocate heap memory? If so, it must be tracked by scope GC.
- Does the return value need promotion to the caller's scope?
- Are there nested containers (e.g., array of arrays) that need recursive promotion?
- Does this interact with arenas?

## Step 5: Create the Plan

Now invoke the `superpowers:writing-plans` skill with all the context gathered above. The plan should include:
- The specific compiler files and line ranges to modify
- The exact pipeline steps for this feature type
- All 8 documentation update tasks
- Test fixtures with expected exit codes
- Scope GC considerations noted in relevant tasks
