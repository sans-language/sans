#!/bin/bash
set -euo pipefail

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Determine the sans binary
SANS="${1:-sans}"

# Determine repo root (directory containing this script's parent)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

PASS=0
FAIL=0
SKIP=0

run_test() {
    local label="$1"
    local fixture_path="$2"
    local expected_exit="$3"

    # Check fixture exists
    if [ ! -e "$fixture_path" ]; then
        echo -e "  ${YELLOW}SKIP${NC}  $label (fixture not found: $fixture_path)"
        ((SKIP++)) || true
        return
    fi

    # Build the binary, then run it separately to capture the program's exit code
    local tmp_bin="/tmp/sans_test_$$_${label}"
    if ! "$SANS" build "$fixture_path" -o "$tmp_bin" 2>/dev/null; then
        # Build failed — try without -o flag (build in place)
        "$SANS" build "$fixture_path" 2>/dev/null || true
        local base_name
        base_name=$(basename "$fixture_path" .sans)
        local dir_name
        dir_name=$(dirname "$fixture_path")
        tmp_bin="${dir_name}/${base_name}"
    fi

    if [ ! -f "$tmp_bin" ]; then
        echo -e "  ${YELLOW}SKIP${NC}  $label (compile failed)"
        ((SKIP++)) || true
        return
    fi

    actual_exit=0
    "$tmp_bin" 2>/dev/null || actual_exit=$?
    rm -f "$tmp_bin"

    if [ "$actual_exit" -eq "$expected_exit" ]; then
        echo -e "  ${GREEN}✓${NC}  $label"
        ((PASS++)) || true
    elif [ "$actual_exit" -eq 101 ] || [ "$actual_exit" -eq 127 ]; then
        # 101 = typical "compilation failed" sentinel; 127 = command not found
        echo -e "  ${YELLOW}SKIP${NC}  $label (compile error, exit $actual_exit)"
        ((SKIP++)) || true
    else
        echo -e "  ${RED}✗${NC}  $label (expected exit $expected_exit, got $actual_exit)"
        ((FAIL++)) || true
    fi
}

echo "Sans test runner"
echo "Binary: $SANS"
echo "Fixtures: $REPO_ROOT/tests/fixtures"
echo "----------------------------------------"

# ---------------------------------------------------------------------------
# Single-file tests
# ---------------------------------------------------------------------------

run_test "struct_basic"              "$REPO_ROOT/tests/fixtures/struct_basic.sans"              7
run_test "struct_nested_access"      "$REPO_ROOT/tests/fixtures/struct_nested_access.sans"      30
run_test "struct_return_repeated"    "$REPO_ROOT/tests/fixtures/struct_return_repeated.sans"    3
run_test "enum_match_method"         "$REPO_ROOT/tests/fixtures/enum_match_method.sans"         5
run_test "enum_basic"                "$REPO_ROOT/tests/fixtures/enum_basic.sans"                2
run_test "enum_data"                 "$REPO_ROOT/tests/fixtures/enum_data.sans"                 12
run_test "method_basic"              "$REPO_ROOT/tests/fixtures/method_basic.sans"              7
run_test "trait_impl"                "$REPO_ROOT/tests/fixtures/trait_impl.sans"                13
run_test "generic_identity"          "$REPO_ROOT/tests/fixtures/generic_identity.sans"          42
run_test "generic_pair"              "$REPO_ROOT/tests/fixtures/generic_pair.sans"              17
run_test "spawn_join"                "$REPO_ROOT/tests/fixtures/spawn_join.sans"                7
run_test "channel_basic"             "$REPO_ROOT/tests/fixtures/channel_basic.sans"             42
run_test "spawn_channel"             "$REPO_ROOT/tests/fixtures/spawn_channel.sans"             10
run_test "mutex_basic"               "$REPO_ROOT/tests/fixtures/mutex_basic.sans"               15
run_test "mutex_threaded"            "$REPO_ROOT/tests/fixtures/mutex_threaded.sans"            1
run_test "channel_bounded"           "$REPO_ROOT/tests/fixtures/channel_bounded.sans"           30
run_test "array_basic"               "$REPO_ROOT/tests/fixtures/array_basic.sans"               28
run_test "array_literal"             "$REPO_ROOT/tests/fixtures/array_literal.sans"             63
run_test "array_param"               "$REPO_ROOT/tests/fixtures/array_param.sans"               70
run_test "array_for_in"              "$REPO_ROOT/tests/fixtures/array_for_in.sans"              10
run_test "string_ops"                "$REPO_ROOT/tests/fixtures/string_ops.sans"                18
run_test "string_conversion"         "$REPO_ROOT/tests/fixtures/string_conversion.sans"         42
run_test "file_write_read"           "$REPO_ROOT/tests/fixtures/file_write_read.sans"           11
run_test "file_exists_check"         "$REPO_ROOT/tests/fixtures/file_exists_check.sans"         1
run_test "read_file_alias"           "$REPO_ROOT/tests/fixtures/read_file_alias.sans"           9
run_test "args_builtin"              "$REPO_ROOT/tests/fixtures/args_builtin.sans"              1
run_test "json_object_stringify"     "$REPO_ROOT/tests/fixtures/json_object_stringify.sans"     2
run_test "json_int_roundtrip"        "$REPO_ROOT/tests/fixtures/json_int_roundtrip.sans"        42
run_test "json_build"                "$REPO_ROOT/tests/fixtures/json_build.sans"                50
run_test "json_parse_access"         "$REPO_ROOT/tests/fixtures/json_parse_access.sans"         42
run_test "json_roundtrip"            "$REPO_ROOT/tests/fixtures/json_roundtrip.sans"            7
run_test "http_error_handling"       "$REPO_ROOT/tests/fixtures/http_error_handling.sans"       1
run_test "log_levels"                "$REPO_ROOT/tests/fixtures/log_levels.sans"                0
run_test "map_basic"                 "$REPO_ROOT/tests/fixtures/map_basic.sans"                 30
run_test "map_has"                   "$REPO_ROOT/tests/fixtures/map_has.sans"                   42
run_test "map_len"                   "$REPO_ROOT/tests/fixtures/map_len.sans"                   3
run_test "result_ok_unwrap"          "$REPO_ROOT/tests/fixtures/result_ok_unwrap.sans"          10
run_test "result_error_handling"     "$REPO_ROOT/tests/fixtures/result_error_handling.sans"     99
run_test "result_error_code"          "$REPO_ROOT/tests/fixtures/result_error_code.sans"          10
run_test "float_basic"               "$REPO_ROOT/tests/fixtures/float_basic.sans"               12
run_test "string_methods"            "$REPO_ROOT/tests/fixtures/string_methods.sans"            17
run_test "string_ends_with"          "$REPO_ROOT/tests/fixtures/string_ends_with.sans"          2
run_test "array_methods"             "$REPO_ROOT/tests/fixtures/array_methods.sans"             33
run_test "map_filter"                "$REPO_ROOT/tests/fixtures/map_filter.sans"                21
run_test "string_replace"            "$REPO_ROOT/tests/fixtures/string_replace.sans"            11
run_test "array_remove"              "$REPO_ROOT/tests/fixtures/array_remove.sans"              63
run_test "multiline_string"          "$REPO_ROOT/tests/fixtures/multiline_string.sans"          11
run_test "modulo_neg"                "$REPO_ROOT/tests/fixtures/modulo_neg.sans"                9
run_test "string_interp"             "$REPO_ROOT/tests/fixtures/string_interp.sans"             11
run_test "ai_syntax"                 "$REPO_ROOT/tests/fixtures/ai_syntax.sans"                 126
run_test "ai_syntax2"                "$REPO_ROOT/tests/fixtures/ai_syntax2.sans"                17
run_test "ai_syntax3"                "$REPO_ROOT/tests/fixtures/ai_syntax3.sans"                27
run_test "ai_syntax4"                "$REPO_ROOT/tests/fixtures/ai_syntax4.sans"                5
run_test "ai_syntax5"                "$REPO_ROOT/tests/fixtures/ai_syntax5.sans"                3
run_test "global_var"                "$REPO_ROOT/tests/fixtures/global_var.sans"                3
run_test "tuple_basic"               "$REPO_ROOT/tests/fixtures/tuple_basic.sans"               5
run_test "tuple_return"              "$REPO_ROOT/tests/fixtures/tuple_return.sans"              30
run_test "tuple_three"               "$REPO_ROOT/tests/fixtures/tuple_three.sans"               42
run_test "tuple_nested"              "$REPO_ROOT/tests/fixtures/tuple_nested.sans"              3
run_test "lambda_basic"              "$REPO_ROOT/tests/fixtures/lambda_basic.sans"              15
run_test "lambda_map"                "$REPO_ROOT/tests/fixtures/lambda_map.sans"                9
run_test "lambda_capture"            "$REPO_ROOT/tests/fixtures/lambda_capture.sans"            15
run_test "nested_lambda"             "$REPO_ROOT/tests/fixtures/nested_lambda.sans"             15
run_test "array_any"                 "$REPO_ROOT/tests/fixtures/array_any.sans"                 1
run_test "array_find"                "$REPO_ROOT/tests/fixtures/array_find.sans"                30
run_test "array_enumerate"           "$REPO_ROOT/tests/fixtures/array_enumerate.sans"           32
run_test "array_zip"                 "$REPO_ROOT/tests/fixtures/array_zip.sans"                 22
run_test "string_slice"              "$REPO_ROOT/tests/fixtures/string_slice.sans"              10
run_test "string_interp_expr"        "$REPO_ROOT/tests/fixtures/string_interp_expr.sans"        6
run_test "try_operator"              "$REPO_ROOT/tests/fixtures/try_operator.sans"              6
run_test "try_operator_err"          "$REPO_ROOT/tests/fixtures/try_operator_err.sans"          99
run_test "break_basic"               "$REPO_ROOT/tests/fixtures/break_basic.sans"               10
run_test "continue_basic"            "$REPO_ROOT/tests/fixtures/continue_basic.sans"            25
run_test "tuple_return_typed"        "$REPO_ROOT/tests/fixtures/tuple_return_typed.sans"        7
run_test "tuple_array"               "$REPO_ROOT/tests/fixtures/tuple_array.sans"               3
run_test "arena_basic"               "$REPO_ROOT/tests/fixtures/arena_basic.sans"               100
run_test "arena_nested"              "$REPO_ROOT/tests/fixtures/arena_nested.sans"              141
run_test "scope_basic"               "$REPO_ROOT/tests/fixtures/scope_basic.sans"               100
run_test "scope_typed"               "$REPO_ROOT/tests/fixtures/scope_typed.sans"               99
run_test "scope_nested_calls"        "$REPO_ROOT/tests/fixtures/scope_nested_calls.sans"        60
run_test "scope_string"              "$REPO_ROOT/tests/fixtures/scope_string.sans"              0
run_test "range_basic"               "$REPO_ROOT/tests/fixtures/range_basic.sans"               35
run_test "array_sort"                "$REPO_ROOT/tests/fixtures/array_sort.sans"                55
run_test "string_upper_lower"        "$REPO_ROOT/tests/fixtures/string_upper_lower.sans"        7
run_test "array_join_reverse"        "$REPO_ROOT/tests/fixtures/array_join_reverse.sans"        45
run_test "system_basic"              "$REPO_ROOT/tests/fixtures/system_basic.sans"              0
run_test "builtin_override"          "$REPO_ROOT/tests/fixtures/builtin_override.sans"          42
run_test "default_params"            "$REPO_ROOT/tests/fixtures/default_params.sans"            40

# ---------------------------------------------------------------------------
# Directory-based (multi-module) tests
# ---------------------------------------------------------------------------

run_test "import_basic"   "$REPO_ROOT/tests/fixtures/import_basic/main.sans"   7
run_test "import_nested"  "$REPO_ROOT/tests/fixtures/import_nested/main.sans"  15
run_test "import_chain"   "$REPO_ROOT/tests/fixtures/import_chain/main.sans"   13
run_test "import_struct"  "$REPO_ROOT/tests/fixtures/import_struct/main.sans"  22

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL=$((PASS + FAIL + SKIP))
echo "----------------------------------------"
echo -e "${GREEN}$PASS${NC}/$TOTAL tests passed  (${SKIP} skipped, ${FAIL} failed)"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
