# Standard Library — Math Expansion Design

**Date:** 2026-04-09
**Status:** Approved
**Goal:** Add floating-point math builtins (floor, ceil, sqrt, pow, sin, cos, tan, log, exp, etc.) wrapping libc.

---

## 1. New Builtins

All functions take and return `F` (Float), except where noted.

### Rounding
| Function | Signature | Wraps |
|---|---|---|
| `floor(x:F)` | `F → F` | `floor()` |
| `ceil(x:F)` | `F → F` | `ceil()` |
| `round(x:F)` | `F → F` | `round()` |

### Roots & Powers
| Function | Signature | Wraps |
|---|---|---|
| `sqrt(x:F)` | `F → F` | `sqrt()` |
| `pow(x:F y:F)` | `(F, F) → F` | `pow()` |

### Trigonometry
| Function | Signature | Wraps |
|---|---|---|
| `sin(x:F)` | `F → F` | `sin()` |
| `cos(x:F)` | `F → F` | `cos()` |
| `tan(x:F)` | `F → F` | `tan()` |
| `asin(x:F)` | `F → F` | `asin()` |
| `acos(x:F)` | `F → F` | `acos()` |
| `atan(x:F)` | `F → F` | `atan()` |
| `atan2(y:F x:F)` | `(F, F) → F` | `atan2()` |

### Logarithms & Exponential
| Function | Signature | Wraps |
|---|---|---|
| `log(x:F)` | `F → F` | `log()` (natural) |
| `log10(x:F)` | `F → F` | `log10()` |
| `exp(x:F)` | `F → F` | `exp()` |

### Float Utilities
| Function | Signature | Wraps |
|---|---|---|
| `fabs(x:F)` | `F → F` | `fabs()` |
| `fmin(x:F y:F)` | `(F, F) → F` | `fmin()` |
| `fmax(x:F y:F)` | `(F, F) → F` | `fmax()` |

### Constants
| Name | Value |
|---|---|
| `PI` | 3.141592653589793 |
| `E` | 2.718281828459045 |

## 2. Implementation Pipeline

Per the CLAUDE.md pipeline for new builtins:

1. **typeck.sans** — Type check each function (arg types, return type `TY_FLOAT`)
2. **constants.sans** — Add IR instruction constants (`IR_FLOOR`, `IR_CEIL`, etc.)
3. **ir.sans** — Add IR lowering (map function name → instruction)
4. **codegen.sans** — Emit LLVM extern declarations and call instructions

### Codegen Approach

These are libc functions. In LLVM IR:
```llvm
declare double @floor(double)
declare double @ceil(double)
declare double @sqrt(double)
; ... etc
```

For each call:
```llvm
%ftmp = bitcast i64 %arg to double    ; unbox float from i64
%result = call double @floor(double %ftmp)
%out = bitcast double %result to i64   ; box result back to i64
```

Two-arg functions (pow, atan2, fmin, fmax):
```llvm
%a = bitcast i64 %arg0 to double
%b = bitcast i64 %arg1 to double
%result = call double @pow(double %a, double %b)
%out = bitcast double %result to i64
```

Constants (PI, E): Emitted as global constants in codegen, or as inline float literals at the call site.

### No Runtime Module Needed

These are direct libc calls — no `runtime/math.sans` changes needed. The existing `math.sans` handles integer math (`abs`, `min`, `max`, `random`). Float math goes through codegen directly.

## 3. Documentation Updates

Per CLAUDE.md checklist:
- `docs/reference.md` — add Math section with all functions
- `docs/ai-reference.md` — add compact math entry
- `website/docs/index.html` — add math section
- `editors/vscode-sans/syntaxes/sans.tmLanguage.json` — add new builtins to syntax highlighting
- `editors/vscode-sans/out/extension.js` — add hover data entries
- `editors/neovim-sans/syntax/sans.vim` — add new builtins
- `editors/emacs-sans/sans-mode.el` — add new builtins to font-lock
- `tests/fixtures/` — add test fixtures for math functions
- `README.md` — update if needed

## 4. Success Criteria

- [ ] All 19 math functions type-check, compile, and produce correct results
- [ ] `PI` and `E` constants available as globals
- [ ] `floor(3.7)` returns `3.0`, `ceil(3.2)` returns `4.0`, `sqrt(4.0)` returns `2.0`
- [ ] Trig functions match libc results (sin(PI) ≈ 0, cos(0) = 1)
- [ ] Test fixtures pass
- [ ] Documentation updated across all channels
- [ ] Editor syntax highlighting includes new builtins
