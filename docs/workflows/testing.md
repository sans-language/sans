# Writing Tests for Sans

Guide for writing comprehensive E2E test fixtures for the Sans compiler.

## How Tests Work

Each test is a `.sans` file in `tests/fixtures/`. The test runner compiles it, runs the binary, and checks the exit code.

```bash
# In tests/run_tests.sh:
run_test "label"  "$REPO_ROOT/tests/fixtures/label.sans"  42
#        ^name    ^path                                     ^expected exit code
```

**Build and run:**
```bash
# Compile a fixture:
sans build tests/fixtures/my_test.sans

# Run full suite:
bash tests/run_tests.sh
```

## Exit Code Conventions

- Use the **computed result** as the exit code (e.g., `abs(-42)` → exit `42`)
- Use `0` only for side-effect tests (printing, file I/O, logging)
- Exit codes are 0-255 (unsigned byte)
- For multi-check tests, sum or combine results

**Example:**
```
// tests/fixtures/array_basic.sans
main() = {
  a = [10 8 10]
  exit(a[0] + a[1] + a[2])  // exits 28
}
```

## Naming Convention

`<feature>_<scenario>.sans`

Examples:
- `array_sort.sans` — basic sort test
- `array_sort_empty.sans` — sort an empty array
- `map_filter.sans` — filter a map
- `lambda_capture_nested.sans` — nested lambda captures

## Temp Files

If a fixture writes to disk, use a unique path:
```
"/tmp/sans_test_<fixture_name>"
```
This prevents races when tests run in parallel.

## Edge Case Checklist

For every feature, write tests covering:

### Empty/Zero/Null
- [ ] Empty string `""`
- [ ] Empty array `[]`
- [ ] Zero `0`
- [ ] Feature called with minimal input

### Boundaries
- [ ] Single-element array
- [ ] First/last element access
- [ ] Large values

### Error Paths
- [ ] Invalid input (if feature should handle it)
- [ ] Result `ok` and `err` paths
- [ ] Graceful failure scenarios

### Scope GC
- [ ] Feature used inside a function (memory freed on return?)
- [ ] Feature used in a loop (memory accumulates?)
- [ ] Return value stored in container (promoted correctly?)

### Cross-Feature
- [ ] Works inside a lambda?
- [ ] Works across module imports?
- [ ] Interacts with other features correctly?

## Registering Tests

Add to `tests/run_tests.sh` in the appropriate section:

```bash
# Single-file tests (most common):
run_test "feature_basic"    "$REPO_ROOT/tests/fixtures/feature_basic.sans"    42

# Directory-based (multi-module) tests:
run_test "feature_import"   "$REPO_ROOT/tests/fixtures/feature_import/main.sans"   7
```

## Good Test Examples

Look at these existing fixtures for reference:
- `tests/fixtures/array_basic.sans` — simple computed exit code
- `tests/fixtures/file_write_read.sans` — side-effect test with temp file
- `tests/fixtures/try_operator.sans` — Result/error path testing
- `tests/fixtures/scope_basic.sans` — scope GC verification
- `tests/fixtures/import_basic/main.sans` — multi-module test

**Next step:** [PR Review](pr-review.md)
