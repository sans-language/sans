---
name: sans-test
type: skill
description: Write comprehensive Sans test fixtures — happy path, edge cases, and boundary conditions — then run the full suite and report results
---

# Sans Test Writing

You are writing comprehensive test fixtures for a Sans feature or fix. Tests are E2E — each fixture is a `.sans` file that is compiled, run, and its exit code checked.

## How Sans Tests Work

Each test fixture is a `.sans` file in `tests/fixtures/`. The test runner (`tests/run_tests.sh`) compiles it with `sans build`, runs the binary, and checks the exit code against an expected value.

```bash
# In run_tests.sh:
run_test "feature_name"  "$REPO_ROOT/tests/fixtures/feature_name.sans"  42
#        ^label           ^fixture path                                  ^expected exit code
```

**Exit code conventions:**
- Use the computed result as the exit code (e.g., a test for `abs(-42)` exits with `42`)
- Use `0` only for tests verifying side effects (printing, file I/O, logging)
- Exit codes must be 0-255 (unsigned byte)
- For tests that check multiple things, sum or combine results into the exit code

**File naming:** `<feature>_<scenario>.sans` (e.g., `array_sort_empty.sans`, `map_filter.sans`)

**Temp files:** If a fixture writes to disk, use a unique path: `/tmp/sans_test_<fixture_name>_<unique>`

## Step 1: Analyze the Change

Read the diff or description of what was added/modified. Identify:
- What feature or fix was implemented?
- What types/functions/methods are involved?
- What are the inputs and outputs?

## Step 2: Check Existing Coverage

Search `tests/fixtures/` and `tests/run_tests.sh` for existing tests covering this feature. Note gaps.

## Step 3: Write the Happy-Path Fixture

Write a test that exercises the primary use case. Use AI-optimized syntax:
- `I/S/B/F` for types, bare assignment, no commas in arrays
- `r!` for unwrap, `?` for try operator
- Expression bodies for simple functions

Example fixture structure:
```
// tests/fixtures/feature_basic.sans
main() = {
  result := 0
  // ... test the feature ...
  result += expected_value
  exit(result)
}
```

## Step 4: Identify Edge Cases

Systematically check each category:

### Empty/Zero/Null
- Empty string `""`
- Empty array `[]`
- Zero `0`
- What happens when the feature is called with "nothing"?

### Boundaries
- Single element arrays
- First/last element access
- Max values (large integers near i64 limits)
- Very long strings

### Error Paths
- Invalid input types (if the feature should reject them)
- Operations that should return `Result` — test both `ok` and `err` paths
- Division by zero, index out of bounds

### Type Interactions
- Does it work with `I`, `S`, `B`, `F`?
- Does it work with arrays of different types?
- Does it work with nested containers?

### Scope GC
- Call the feature in a function that returns — does memory get freed?
- Call it in a loop — does memory accumulate?
- Store the result in a container — is it promoted correctly?

### Cross-Feature
- Does it work inside a lambda?
- Does it work across module imports?
- Does it interact correctly with other features?

## Step 5: Write Edge Case Fixtures

One fixture per edge case (or group closely related edge cases). Keep each focused.

## Step 6: Register in run_tests.sh

Add entries to `tests/run_tests.sh` in the appropriate section (single-file or directory-based):

```bash
run_test "feature_basic"        "$REPO_ROOT/tests/fixtures/feature_basic.sans"        42
run_test "feature_empty"        "$REPO_ROOT/tests/fixtures/feature_empty.sans"         0
run_test "feature_boundary"     "$REPO_ROOT/tests/fixtures/feature_boundary.sans"      99
```

## Step 7: Build and Run

```bash
# Compile each new fixture:
sans build tests/fixtures/feature_basic.sans

# Run the full suite:
bash tests/run_tests.sh

# Verify: 0 failures, new tests show as passing
```

## Step 8: Report

```
## Test Report: [Feature Name]

### Fixtures Written
| Fixture | Tests | Expected Exit |
|---------|-------|---------------|
| feature_basic.sans | happy path | 42 |
| feature_empty.sans | empty input | 0 |

### Edge Cases Covered
- [x] Empty input
- [x] Boundary values
- [ ] Error paths (not applicable — feature doesn't fail)

### Suite Results
- Total: X tests
- Passed: Y
- Failed: Z
- Skipped: W

### Concerns
- [any issues, gaps, or things to watch]
```
