# Self-Hosting Scope GC Analysis

**Date:** 2026-03-23
**Status:** Complete
**Goal:** Document the specific patterns that break scope GC during compiler self-compilation

---

## Background

The Sans compiler is self-hosted but cannot compile itself with scope GC enabled. The bootstrap chain uses `scope_disable()` in `build()` (compiler/main.sans:167) to prevent scope GC from running during the multi-step compilation pipeline.

## Root Cause

The compiler's build pipeline (`build()` → `build_step3()` → `do_link()`) passes data across function boundaries that scope_exit would free:

1. **IR data shared across codegen:** The IR module created in `build_step3` is consumed by `compile_to_ll` in the same scope. If scope_exit ran between these steps, the IR data would be freed.

2. **Runtime module compilation loop:** `do_link()` compiles 18 runtime modules sequentially. Each compilation creates AST, IR, and codegen data. Without scope_disable, each iteration's scope_exit frees data that subsequent iterations or the linker still reference.

3. **String accumulation:** The compiler builds LLVM IR as a single large string via repeated concatenation. Each concatenation allocates a new string. Without scope_disable, intermediate strings are freed.

## Known Issue: Bootstrapped Binary Scope GC

When the v0.6.1 bootstrap compiles current source, the resulting binary has `scope_disable()` working correctly. However, there was a critical bug (fixed in v0.7.2):

**Global variable init offset bug:** The v0.7.0 AST changes added a size field at offset 8 of all expression nodes, shifting literal values from offset 8 to offset 16. But `ir.sans` global init extraction still read offset 8, causing all global variables to be initialized to their node size (40) instead of their actual values. This caused:
- `CURLOPT_URL = 10002` → initialized to 40 → curl runtime crashed
- `g counter = 0` → initialized to 40 → test segfaults
- Fixed by updating `load64(gvalue + 8)` → `load64(gvalue + 16)` in ir.sans

## Patterns That Break Under Scope GC

### Pattern 1: Cross-function data sharing
```sans
build() I {
  modules = resolve_imports(path)  // allocates
  program = parse(src)              // allocates
  build_step3(modules, program)     // uses both
  // scope_exit here would free modules and program
}
```
**Why it breaks:** `modules` and `program` are allocated in `build()` scope. `build_step3` needs them but scope_exit runs before it returns.

### Pattern 2: Accumulator loops
```sans
do_link() I {
  ri := 0
  while ri < runtime_modules.len() {
    // Each iteration: parse → typecheck → lower → codegen → llc
    // Creates ~1MB of temporary data per module
    // Without scope_disable, this accumulates until function return
    ri += 1
  }
}
```
**Why it breaks:** The loop body allocates heavily. With scope_disable OFF, scope_enter/scope_exit should run per-iteration, but the compiler doesn't emit per-iteration scope boundaries. All 18 iterations accumulate ~18MB before the function returns.

### Pattern 3: String builder pattern
```sans
compile_to_ll() I {
  // Builds LLVM IR as one large string
  // Each emit() call: new_str = old_str + line + "\n"
  // Creates O(n^2) temporary strings
}
```
**Why it breaks:** Each concatenation creates a new string and the old one becomes garbage. Without scope GC, all intermediate strings accumulate.

## Current Mitigation

`scope_disable()` is called at the top of `build()`. This disables ALL scope GC for the entire compilation, relying on the OS to reclaim memory when the process exits. This works but means:
- Compiler memory usage is O(total allocations), not O(live data)
- Long compilation sessions accumulate memory indefinitely
- The compiler cannot be used as a long-running service (LSP)

## Potential Future Fixes

1. **Per-module scope boundaries:** Add scope_enter/scope_exit around each runtime module compilation iteration in `do_link()`. This requires ensuring no data leaks between iterations.

2. **Arena allocator for codegen:** Replace scope GC in codegen with a per-compilation arena that's freed all at once after LLVM IR is written to disk.

3. **Incremental compilation:** Only recompile changed modules. Reduces peak memory.

4. **String builder type:** Replace O(n^2) string concatenation with an O(n) builder that maintains a list of chunks and joins at the end.

None of these are required for 1.0 — scope_disable works. They're optimizations for post-1.0.
