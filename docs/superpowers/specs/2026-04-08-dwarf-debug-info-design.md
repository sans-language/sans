# DWARF Debug Info Design

**Date:** 2026-04-08
**Status:** Approved
**Goal:** Add `sans build --debug` that produces binaries with DWARF debug info, enabling line-level debugging in lldb/gdb.

---

## 1. Architecture

Source locations (line/col) exist on every AST node but are lost at IR creation. A side table in the IR module maps instruction indices to (line, col) pairs. When `--debug` is enabled, codegen reads this side table and emits `!dbg` metadata on LLVM IR instructions, plus module-level DWARF metadata nodes.

```
AST (has line/col) → IR lowering (records locations in side table) → Codegen (emits !dbg metadata)
```

## 2. IR Source Location Side Table

Add to the IR module structure a parallel array: `ir_loc_lines` and `ir_loc_cols`, indexed by instruction emission order. When `ctx_emit` is called during IR lowering, it also records the current source location (set from the AST node being lowered).

New globals in `ir.sans`:
- `g ir_current_line = 0` — set before emitting instructions for an AST node
- `g ir_current_col = 0` — set before emitting instructions for an AST node
- `g ir_loc_lines = 0` — array of line numbers, indexed by instruction index
- `g ir_loc_cols = 0` — array of col numbers, indexed by instruction index

`ctx_emit` appends the current line/col to the side table arrays each time an instruction is emitted.

IR lowering functions (`lower_expr`, `lower_stmt`, etc.) set `ir_current_line`/`ir_current_col` from the AST node before emitting instructions:
```
ir_current_line = node_line(expr)
ir_current_col = node_col(expr)
```

## 3. Codegen DWARF Emission

When debug mode is enabled (`g cg_debug = 1`), codegen emits:

### Module-level metadata (in `emit_header` or at end of module):
```llvm
!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!100, !101}
!100 = !{i32 2, !"Dwarf Version", i32 4}
!101 = !{i32 2, !"Debug Info Version", i32 3}
!0 = distinct !DICompileUnit(language: DW_LANG_C, file: !1, producer: "sans 0.8.6", isOptimized: false, emissionKind: FullDebug)
!1 = !DIFile(filename: "main.sans", directory: "/path/to/project")
```

### Per-function metadata:
```llvm
define i64 @function_name(...) !dbg !N {
```
Where `!N` references a `!DISubprogram`:
```llvm
!N = distinct !DISubprogram(name: "function_name", file: !1, line: 10, type: !FNTYPE, scopeLine: 10, unit: !0)
```

### Per-instruction metadata:
For each IR instruction with a known source location, emit `!dbg !M` on the LLVM instruction:
```llvm
  %5 = add i64 %3, %4, !dbg !M
```
Where `!M` references a `!DILocation`:
```llvm
!M = !DILocation(line: 12, column: 5, scope: !N)
```

### Metadata numbering:
Maintain a counter for metadata node IDs. Deduplicate DILocation nodes where possible (same line/col/scope).

## 4. CLI Integration

### `--debug` flag
Add to `sans build` and `sans run`:
```
sans build --debug main.sans        # produces binary with DWARF info
sans build --debug main.sans -o out # with output path
```

### Build pipeline changes when `--debug`:
- Set `cg_debug = 1` globally
- Pass `-g` to `llc` (in addition to existing flags)
- Use `-O0` instead of `-O2` for `llc` (better debug experience)
- Pass `-g` to `cc` linker invocation
- Do NOT delete the `.ll` file (useful for debugging codegen)

## 5. Scope

### In scope:
- Side table for IR source locations
- `!DICompileUnit`, `!DIFile`, `!DISubprogram`, `!DILocation` metadata
- `--debug` CLI flag
- `-g` and `-O0` passed to llc/cc
- lldb can show Sans function names in stack traces
- lldb can step through Sans source line-by-line
- lldb can set breakpoints by file:line

### Out of scope (deferred):
- Variable-level debug info (`DILocalVariable`, `llvm.dbg.declare`)
- Type debug info (`DIBasicType`, `DICompositeType`)
- Debug info for runtime modules (only user code gets debug info)
- Inlined function debug scopes
- Column-level stepping (lines only)

## 6. Success Criteria

- [ ] `sans build --debug main.sans` produces a binary with DWARF sections
- [ ] `lldb ./binary` shows Sans function names in `bt` (backtrace)
- [ ] `lldb` can set breakpoint by file:line: `b main.sans:5`
- [ ] `lldb` can step through Sans source with `n` (next line)
- [ ] Debug binary is larger than release binary (DWARF sections present)
- [ ] Regular `sans build` (no --debug) is unchanged — no performance impact
