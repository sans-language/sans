# Bootstrap Dependency Status

**Date:** 2026-03-23
**Compiler version:** 0.7.3
**Bootstrap binary version:** v0.6.1 (used in CI release workflow)

---

## Summary

The Sans compiler is self-hosted and requires a bootstrap binary to start the compilation chain. The current CI pipeline downloads v0.6.1 as the bootstrap compiler. The compiler disables scope GC via `scope_disable()` in `build()` (compiler/main.sans:176) because its internal data structures use patterns that conflict with automatic scope-based memory management.

## Can v0.7.3 self-compile with scope GC enabled?

**Partially tested. Compilation pipeline completes; runtime behavior untested.**

### Test methodology

1. Removed `scope_disable()` from `compiler/main.sans:build()`
2. Ran `sans build compiler/main.sans` using the installed v0.7.3 binary
3. Compared output with normal build (scope_disable present)

### Results

- **Parse (STEP 0):** Completed identically with and without `scope_disable()`
- **Typecheck + IR (STEP 1):** Completed identically
- **Codegen + runtime modules (STEP 2):** All 18 runtime modules compiled identically
- **Link (STEP 3):** Failed on both builds due to platform-specific PIE relocation error (unrelated to scope GC)

The compilation pipeline produced identical output in both cases. This is expected because `scope_disable()` is a **runtime** call -- it affects the *compiled binary's* behavior, not the compilation process itself. The v0.7.3 bootstrap binary already has its own `scope_disable()` baked in, so removing it from the source only affects the newly built binary.

### What remains untested

The critical test is whether a binary built **without** `scope_disable()` can then compile programs without crashing. This requires:
1. Successfully linking the binary (blocked by PIE issue on this Linux x86_64 environment)
2. Using that binary to compile a non-trivial program (e.g., `sans build compiler/main.sans`)
3. Verifying the output is correct

This test can only be performed on macOS (the primary development platform) where the linker configuration is known to work.

## Patterns that break under scope GC (from analysis)

Per `docs/superpowers/specs/2026-03-23-self-hosting-scope-gc-analysis.md`:

### 1. Cross-function data sharing
`build()` allocates `modules` and `program`, then passes them to `build_step3()`. Without `scope_disable()`, scope_exit in `build()` would free these before `build_step3` returns. However, since `build_step3` is called as a tail expression, the data may survive if scope_exit runs after the call returns.

### 2. Accumulator loops in do_link()
`do_link()` compiles 18 runtime modules in a while loop. Each iteration allocates AST, IR, and codegen data (~1MB each). Without per-iteration scope boundaries, all 18 iterations accumulate ~18MB. The compiler does not emit scope_enter/scope_exit per loop iteration, so this is a memory accumulation issue rather than a use-after-free. The data is valid but never freed until the function returns.

### 3. String builder pattern in codegen
`compile_to_ll()` builds LLVM IR via O(n^2) string concatenation. Each concatenation allocates a new string. Without scope GC these accumulate; with scope GC the old strings would be freed but the new concatenation references the result, not the old string. This pattern may actually be safe under scope GC since each concatenation produces a new allocation that replaces the old reference.

### Assessment

Patterns 2 and 3 are primarily **memory efficiency** concerns, not correctness issues. Pattern 1 (cross-function data sharing) is the only potential correctness issue, and it depends on whether scope_exit runs before or after the tail call in `build()`.

## Bootstrap version recommendation

### Current state
- **CI bootstrap:** v0.6.1 (set in `.github/workflows/release.yml:72`)
- **Installed binary:** v0.7.3
- **Source on main:** v0.7.3

### Recommendation: Update bootstrap to v0.7.3

The v0.6.1 bootstrap is over a year old. v0.7.3 includes:
- Scope GC depth-2 fix (v0.7.2)
- Global variable init offset fix (v0.7.2)
- Unused variable/unreachable code warnings (v0.7.2)
- Assert builtins (v0.7.3)
- Various hardening (v0.7.3)

Updating the bootstrap to v0.7.3 would:
1. Reduce the gap between bootstrap and source
2. Ensure the bootstrap understands all current language features
3. Make future self-compilation more reliable

**Action items:**
- Update `.github/workflows/release.yml` line 72: change `v0.6.1` to `v0.7.3`
- Verify v0.7.3 release binary exists on GitHub
- Test that v0.7.3 can compile the current source in CI

### Regarding scope_disable() removal

**Do not remove `scope_disable()` yet.** While compilation succeeds, the runtime behavior of a compiler built without it has not been verified. The analysis identifies real patterns (cross-function data sharing) that could cause use-after-free at runtime. The safe path is:

1. Keep `scope_disable()` for now
2. If/when loop-scoped allocation cleanup is implemented (Task 2), re-evaluate
3. A string builder type would address Pattern 3
4. Per-module scope boundaries in `do_link()` would address Pattern 2
5. Pattern 1 requires careful analysis of tail-call scope_exit ordering

## Environment note

Self-compilation testing on Linux x86_64 is blocked by a PIE relocation error in the LLVM codegen output. The `llc-17` on this system produces non-PIC object files, but the system linker defaults to PIE mode. This affects all Sans programs, not just self-compilation. Fix: add `-relocation-model=pic` to `llc` invocations or add `-no-pie` to linker flags on Linux. This is tracked separately from the bootstrap investigation.
