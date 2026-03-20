# Skeptic Review for Sans

Challenge designs before implementation and critique code after. The goal is to prevent unnecessary complexity, catch missing edge cases, and enforce Sans conventions.

## When to Use

- **Before implementation:** After a design/spec is written, before code starts
- **After implementation:** After code is written, before PR submission

## Pre-Implementation Checklist

### Necessity
- [ ] Does this solve a real, current problem?
- [ ] Could an existing feature handle this with minor changes?
- [ ] What happens if we don't build this?

### Edge Cases
- [ ] Empty input (empty string, empty array, zero)
- [ ] Huge input (max i64, very long strings, large arrays)
- [ ] Invalid input (wrong types, malformed data)
- [ ] Boundary conditions (first/last element, single element)

### Scope GC
- [ ] If it allocates, is memory tracked?
- [ ] In a loop, does memory accumulate?
- [ ] Do return values promote correctly across scope boundaries?
- [ ] Nested container interaction?

### Concurrency
- [ ] What if called from a spawned thread?
- [ ] Does it touch global state?
- (Thread safety is a known limitation — flag as "worth discussing" unless the feature specifically claims thread-safety)

### Syntax
- [ ] Is this the shortest possible syntax?
- [ ] Does it follow existing patterns?
- [ ] Will AI code generators produce it correctly?

### Complexity
- [ ] Is this over-engineered?
- [ ] Are we designing for hypothetical future needs?
- [ ] How many compiler files does this touch — proportional to value?

## Post-Implementation Checklist

### Over-engineering
- [ ] Abstractions used only once?
- [ ] Helpers that could be inlined?
- [ ] Error handling for impossible scenarios?
- [ ] Configuration nobody asked for?

### Scope Creep
- [ ] Changes beyond what the spec called for?
- [ ] "While I'm here" improvements?
- [ ] Excessive test coverage?

### Convention Violations
- [ ] AI-optimized syntax in all new code and examples?
- [ ] `sans_` prefix on runtime functions?
- [ ] `IR_` prefix on constants?
- [ ] Unique `/tmp/` filenames in fixtures?
- [ ] All 8 documentation targets updated?

### Consistency
- [ ] Follows existing codebase patterns?
- [ ] Pipeline complete (typeck → constants → IR → codegen)?
- [ ] No dangling references?

## Severity Guide

| Severity | Meaning | Action |
|----------|---------|--------|
| **Blocking** | Will cause bugs, breaks conventions, missing required work | Must fix before proceeding |
| **Worth discussing** | Design trade-off, potential issue, debatable choice | Discuss and decide |
| **Minor nit** | Style preference, minor improvement | Take it or leave it |

## Output Template

```
## Skeptic Review: [Feature Name]

### Mode: Pre-implementation / Post-implementation

### Blocking
1. [issue] — [why]

### Worth Discussing
1. [issue] — [trade-off]

### Minor Nits
1. [suggestion]

### Summary
[1-2 sentence assessment]
```

**Next step (pre-impl):** Address blocking issues, then proceed to implementation
**Next step (post-impl):** [Testing](testing.md)
