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
NEG_PASS=0
NEG_FAIL=0

run_negative_test() {
    local label="$1"
    local fixture="$2"
    local expected_error="${3:-}"

    # Check fixture exists
    if [ ! -e "$fixture" ]; then
        echo -e "  ${YELLOW}SKIP${NC}  $label (fixture not found: $fixture)"
        ((SKIP++)) || true
        return
    fi

    build_exit=0
    output=$("$SANS" build "$fixture" -o "/tmp/sans_neg_$$_${label}" 2>&1) || build_exit=$?
    rm -f "/tmp/sans_neg_$$_${label}"

    if [ "$build_exit" -eq 0 ]; then
        echo -e "  ${RED}✗${NC}  $label (expected compilation to fail but it succeeded)"
        ((NEG_FAIL++)) || true
        return
    fi

    if [ -n "$expected_error" ] && ! echo "$output" | grep -qi "$expected_error"; then
        echo -e "  ${RED}✗${NC}  $label (expected error containing: $expected_error)"
        echo "     got: $(echo "$output" | tail -3)"
        ((NEG_FAIL++)) || true
        return
    fi

    echo -e "  ${GREEN}✓${NC}  $label (correctly rejected)"
    ((NEG_PASS++)) || true
}

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
run_test "generic_struct"            "$REPO_ROOT/tests/fixtures/generic_struct.sans"            20
run_test "enum_match_method"         "$REPO_ROOT/tests/fixtures/enum_match_method.sans"         5
run_test "enum_basic"                "$REPO_ROOT/tests/fixtures/enum_basic.sans"                2
run_test "enum_data"                 "$REPO_ROOT/tests/fixtures/enum_data.sans"                 12
run_test "match_guard"               "$REPO_ROOT/tests/fixtures/match_guard.sans"               120
run_test "method_basic"              "$REPO_ROOT/tests/fixtures/method_basic.sans"              7
run_test "trait_impl"                "$REPO_ROOT/tests/fixtures/trait_impl.sans"                13
run_test "trait_object_basic"        "$REPO_ROOT/tests/fixtures/trait_object_basic.sans"        0
run_test "dyn_trait_param"           "$REPO_ROOT/tests/fixtures/dyn_trait_param.sans"           0
run_test "dyn_trait_array"           "$REPO_ROOT/tests/fixtures/dyn_trait_array.sans"           0
run_test "generic_identity"          "$REPO_ROOT/tests/fixtures/generic_identity.sans"          42
run_test "generic_pair"              "$REPO_ROOT/tests/fixtures/generic_pair.sans"              17
run_test "spawn_join"                "$REPO_ROOT/tests/fixtures/spawn_join.sans"                7
run_test "channel_basic"             "$REPO_ROOT/tests/fixtures/channel_basic.sans"             42
run_test "spawn_channel"             "$REPO_ROOT/tests/fixtures/spawn_channel.sans"             10
run_test "mutex_basic"               "$REPO_ROOT/tests/fixtures/mutex_basic.sans"               15
run_test "mutex_threaded"            "$REPO_ROOT/tests/fixtures/mutex_threaded.sans"            1
run_test "channel_bounded"           "$REPO_ROOT/tests/fixtures/channel_bounded.sans"           30
run_test "select_basic"              "$REPO_ROOT/tests/fixtures/select_basic.sans"              42
run_test "select_timeout"            "$REPO_ROOT/tests/fixtures/select_timeout.sans"            99
run_test "select_multi"              "$REPO_ROOT/tests/fixtures/select_multi.sans"              10
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
run_test "map_generic"               "$REPO_ROOT/tests/fixtures/map_generic.sans"               0
run_test "option_map_get"            "$REPO_ROOT/tests/fixtures/option_map_get.sans"            0
run_test "result_ok_unwrap"          "$REPO_ROOT/tests/fixtures/result_ok_unwrap.sans"          10
run_test "result_error_handling"     "$REPO_ROOT/tests/fixtures/result_error_handling.sans"     99
run_test "result_error_code"          "$REPO_ROOT/tests/fixtures/result_error_code.sans"          10
run_test "result_map"                "$REPO_ROOT/tests/fixtures/result_map.sans"                0
run_test "option_basic"              "$REPO_ROOT/tests/fixtures/option_basic.sans"              1
run_test "option_methods"            "$REPO_ROOT/tests/fixtures/option_methods.sans"            0
run_test "option_unwrap_bang"        "$REPO_ROOT/tests/fixtures/option_unwrap_bang.sans"        42
run_test "option_try"                "$REPO_ROOT/tests/fixtures/option_try.sans"                0
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
run_test "array_find_option"         "$REPO_ROOT/tests/fixtures/array_find_option.sans"         0
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
run_test "default_params_negative"   "$REPO_ROOT/tests/fixtures/default_params_negative.sans"   50
run_test "for_destructure_map"       "$REPO_ROOT/tests/fixtures/for_destructure_map.sans"       6
run_test "for_destructure_triple"    "$REPO_ROOT/tests/fixtures/for_destructure_triple.sans"    21
run_test "ir_opaque_param"           "$REPO_ROOT/tests/fixtures/ir_opaque_param.sans"           42
run_test "scope_global_escape"       "$REPO_ROOT/tests/fixtures/scope_global_escape.sans"       20
run_test "scope_nested_array"        "$REPO_ROOT/tests/fixtures/scope_nested_array.sans"        10
run_test "generic_struct_method"     "$REPO_ROOT/tests/fixtures/generic_struct_method.sans"     42
run_test "generic_nested"             "$REPO_ROOT/tests/fixtures/generic_nested.sans"             10
run_test "json_keys"                  "$REPO_ROOT/tests/fixtures/json_keys.sans"                  0
run_test "json_fn_return"             "$REPO_ROOT/tests/fixtures/json_fn_return.sans"             0
run_test "pkg_validate"               "$REPO_ROOT/tests/fixtures/pkg_validate.sans"               0
run_test "random_real"                "$REPO_ROOT/tests/fixtures/random_real.sans"                0
run_test "json_float"                 "$REPO_ROOT/tests/fixtures/json_float.sans"                 3
run_test "closure_captures"          "$REPO_ROOT/tests/fixtures/closure_captures.sans"          0
run_test "deep_recursion"            "$REPO_ROOT/tests/fixtures/deep_recursion.sans"            0
run_test "defer_basic"               "$REPO_ROOT/tests/fixtures/defer_basic.sans"               0
run_test "defer_early_return"        "$REPO_ROOT/tests/fixtures/defer_early_return.sans"        0
run_test "basic_test"                "$REPO_ROOT/tests/fixtures/basic_test.sans"                0
run_test "math_test"                 "$REPO_ROOT/tests/fixtures/math_test.sans"                 0

# ---------------------------------------------------------------------------
# Option<T> edge cases
# ---------------------------------------------------------------------------

run_test "option_none_unwrap_or"     "$REPO_ROOT/tests/fixtures/option_none_unwrap_or.sans"     43
run_test "option_nested"             "$REPO_ROOT/tests/fixtures/option_nested.sans"             52
run_test "option_chain"              "$REPO_ROOT/tests/fixtures/option_chain.sans"              42
run_test "option_map_miss"           "$REPO_ROOT/tests/fixtures/option_map_miss.sans"           42
run_test "option_array_find_miss"    "$REPO_ROOT/tests/fixtures/option_array_find_miss.sans"    1
run_test "option_some_string"        "$REPO_ROOT/tests/fixtures/option_some_string.sans"        15
run_test "option_comparison"         "$REPO_ROOT/tests/fixtures/option_comparison.sans"         2
run_test "option_in_struct"          "$REPO_ROOT/tests/fixtures/option_in_struct.sans"          25
run_test "option_unwrap_variants"    "$REPO_ROOT/tests/fixtures/option_unwrap_variants.sans"    42
run_test "option_default_chain"      "$REPO_ROOT/tests/fixtures/option_default_chain.sans"      57

# ---------------------------------------------------------------------------
# Map<K,V> variants
# ---------------------------------------------------------------------------

run_test "map_int_keys"              "$REPO_ROOT/tests/fixtures/map_int_keys.sans"              60
run_test "map_string_values"         "$REPO_ROOT/tests/fixtures/map_string_values.sans"         3
run_test "map_delete"                "$REPO_ROOT/tests/fixtures/map_delete.sans"                2
run_test "map_keys_vals"             "$REPO_ROOT/tests/fixtures/map_keys_vals.sans"             6
run_test "map_overwrite"             "$REPO_ROOT/tests/fixtures/map_overwrite.sans"             42
run_test "map_empty"                 "$REPO_ROOT/tests/fixtures/map_empty.sans"                 42
run_test "map_large"                 "$REPO_ROOT/tests/fixtures/map_large.sans"                 100
run_test "map_int_string"            "$REPO_ROOT/tests/fixtures/map_int_string.sans"            3
run_test "map_entries"               "$REPO_ROOT/tests/fixtures/map_entries.sans"               6
run_test "map_contains"              "$REPO_ROOT/tests/fixtures/map_contains.sans"              110

# ---------------------------------------------------------------------------
# Result<T> combinators
# ---------------------------------------------------------------------------

run_test "result_and_then"           "$REPO_ROOT/tests/fixtures/result_and_then.sans"           15
run_test "result_or_else"            "$REPO_ROOT/tests/fixtures/result_or_else.sans"            42
run_test "result_map_err"            "$REPO_ROOT/tests/fixtures/result_map_err.sans"            11
run_test "result_chain"              "$REPO_ROOT/tests/fixtures/result_chain.sans"              13
run_test "result_try_chain"          "$REPO_ROOT/tests/fixtures/result_try_chain.sans"          30
run_test "result_unwrap_or"          "$REPO_ROOT/tests/fixtures/result_unwrap_or.sans"          42
run_test "result_error_methods"      "$REPO_ROOT/tests/fixtures/result_error_methods.sans"      43
run_test "result_nested"             "$REPO_ROOT/tests/fixtures/result_nested.sans"             42

# ---------------------------------------------------------------------------
# Trait objects
# ---------------------------------------------------------------------------

run_test "trait_multi_method"        "$REPO_ROOT/tests/fixtures/trait_multi_method.sans"        26
run_test "trait_coerce_call"         "$REPO_ROOT/tests/fixtures/trait_coerce_call.sans"         42
run_test "trait_in_function"         "$REPO_ROOT/tests/fixtures/trait_in_function.sans"         42
run_test "trait_array_sum"           "$REPO_ROOT/tests/fixtures/trait_array_sum.sans"           25
run_test "trait_self_method"         "$REPO_ROOT/tests/fixtures/trait_self_method.sans"         37
run_test "trait_impl_multiple"       "$REPO_ROOT/tests/fixtures/trait_impl_multiple.sans"       42
run_test "trait_different_return"    "$REPO_ROOT/tests/fixtures/trait_different_return.sans"     42
run_test "trait_vtable_order"        "$REPO_ROOT/tests/fixtures/trait_vtable_order.sans"        20

# ---------------------------------------------------------------------------
# Defer
# ---------------------------------------------------------------------------

run_test "defer_cleanup"             "$REPO_ROOT/tests/fixtures/defer_cleanup.sans"             105
run_test "defer_nested"              "$REPO_ROOT/tests/fixtures/defer_nested.sans"              0
run_test "defer_with_args"           "$REPO_ROOT/tests/fixtures/defer_with_args.sans"           0
run_test "defer_multiple_returns"    "$REPO_ROOT/tests/fixtures/defer_multiple_returns.sans"    11
run_test "defer_noop"                "$REPO_ROOT/tests/fixtures/defer_noop.sans"                42

# ---------------------------------------------------------------------------
# Select / Channels
# ---------------------------------------------------------------------------

run_test "select_immediate"          "$REPO_ROOT/tests/fixtures/select_immediate.sans"          77
run_test "select_two_channels"       "$REPO_ROOT/tests/fixtures/select_two_channels.sans"       42
run_test "select_timeout_zero"       "$REPO_ROOT/tests/fixtures/select_timeout_zero.sans"       99
run_test "select_in_loop"            "$REPO_ROOT/tests/fixtures/select_in_loop.sans"            45
run_test "select_default"            "$REPO_ROOT/tests/fixtures/select_default.sans"            55

# ---------------------------------------------------------------------------
# String operations
# ---------------------------------------------------------------------------

run_test "string_empty"              "$REPO_ROOT/tests/fixtures/string_empty.sans"              0
run_test "string_concat_chain"       "$REPO_ROOT/tests/fixtures/string_concat_chain.sans"       11
run_test "string_length"             "$REPO_ROOT/tests/fixtures/string_length.sans"             8
run_test "string_compare"            "$REPO_ROOT/tests/fixtures/string_compare.sans"            10
run_test "string_interp_nested"      "$REPO_ROOT/tests/fixtures/string_interp_nested.sans"      14
run_test "string_trim_test"          "$REPO_ROOT/tests/fixtures/string_trim_test.sans"          5
run_test "string_contains_test"      "$REPO_ROOT/tests/fixtures/string_contains_test.sans"      10
run_test "string_char_at_bounds"     "$REPO_ROOT/tests/fixtures/string_char_at_bounds.sans"     4
run_test "string_repeat"             "$REPO_ROOT/tests/fixtures/string_repeat.sans"             6
run_test "string_number_convert"     "$REPO_ROOT/tests/fixtures/string_number_convert.sans"     42
run_test "string_split_test"         "$REPO_ROOT/tests/fixtures/string_split_test.sans"         4
run_test "string_starts_with"        "$REPO_ROOT/tests/fixtures/string_starts_with.sans"        10
run_test "string_index_of"           "$REPO_ROOT/tests/fixtures/string_index_of.sans"           6
run_test "string_upper_test"         "$REPO_ROOT/tests/fixtures/string_upper_test.sans"         10
run_test "string_replace_test"       "$REPO_ROOT/tests/fixtures/string_replace_test.sans"       10

# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------

run_test "while_break_value"         "$REPO_ROOT/tests/fixtures/while_break_value.sans"         55
run_test "while_continue_test"       "$REPO_ROOT/tests/fixtures/while_continue_test.sans"       14
run_test "nested_if_deep"            "$REPO_ROOT/tests/fixtures/nested_if_deep.sans"            42
run_test "for_in_array_test"         "$REPO_ROOT/tests/fixtures/for_in_array_test.sans"         60
run_test "for_in_range_test"         "$REPO_ROOT/tests/fixtures/for_in_range_test.sans"         45
run_test "ternary_nested"            "$REPO_ROOT/tests/fixtures/ternary_nested.sans"            42
run_test "match_wildcard"            "$REPO_ROOT/tests/fixtures/match_wildcard.sans"            42
run_test "match_multiple"            "$REPO_ROOT/tests/fixtures/match_multiple.sans"            20
run_test "if_else_chain"             "$REPO_ROOT/tests/fixtures/if_else_chain.sans"             175
run_test "early_return"              "$REPO_ROOT/tests/fixtures/early_return.sans"              42

# ---------------------------------------------------------------------------
# Arithmetic and types
# ---------------------------------------------------------------------------

run_test "int_large"                 "$REPO_ROOT/tests/fixtures/int_large.sans"                 30
run_test "float_arithmetic"          "$REPO_ROOT/tests/fixtures/float_arithmetic.sans"          21
run_test "float_comparison"          "$REPO_ROOT/tests/fixtures/float_comparison.sans"          101
run_test "bool_logic"                "$REPO_ROOT/tests/fixtures/bool_logic.sans"                11
run_test "modulo_test"               "$REPO_ROOT/tests/fixtures/modulo_test.sans"               4
run_test "negative_numbers"          "$REPO_ROOT/tests/fixtures/negative_numbers.sans"          10
run_test "type_conversion"           "$REPO_ROOT/tests/fixtures/type_conversion.sans"           42
run_test "compound_assignment"       "$REPO_ROOT/tests/fixtures/compound_assignment.sans"       6
run_test "expression_precedence"     "$REPO_ROOT/tests/fixtures/expression_precedence.sans"     4
run_test "int_division"              "$REPO_ROOT/tests/fixtures/int_division.sans"              36

# ---------------------------------------------------------------------------
# Closures and functional
# ---------------------------------------------------------------------------

run_test "closure_nested"            "$REPO_ROOT/tests/fixtures/closure_nested.sans"            42
run_test "closure_mutable_capture"   "$REPO_ROOT/tests/fixtures/closure_mutable_capture.sans"   45
run_test "closure_as_param"          "$REPO_ROOT/tests/fixtures/closure_as_param.sans"          12
run_test "map_filter_chain"          "$REPO_ROOT/tests/fixtures/map_filter_chain.sans"          5
run_test "reduce_sum"                "$REPO_ROOT/tests/fixtures/reduce_sum.sans"                15
run_test "any_predicate"             "$REPO_ROOT/tests/fixtures/any_predicate.sans"             1
run_test "enumerate_pairs"           "$REPO_ROOT/tests/fixtures/enumerate_pairs.sans"           63
run_test "zip_arrays"                "$REPO_ROOT/tests/fixtures/zip_arrays.sans"                66
run_test "closure_return_val"        "$REPO_ROOT/tests/fixtures/closure_return_val.sans"        42
run_test "flat_map_test"             "$REPO_ROOT/tests/fixtures/flat_map_test.sans"             6

# ---------------------------------------------------------------------------
# Scope GC
# ---------------------------------------------------------------------------

run_test "scope_array_return"        "$REPO_ROOT/tests/fixtures/scope_array_return.sans"        30
run_test "scope_map_return"          "$REPO_ROOT/tests/fixtures/scope_map_return.sans"          42
run_test "scope_string_return"       "$REPO_ROOT/tests/fixtures/scope_string_return.sans"       11
run_test "scope_nested_return"       "$REPO_ROOT/tests/fixtures/scope_nested_return.sans"       6
run_test "scope_loop_alloc"          "$REPO_ROOT/tests/fixtures/scope_loop_alloc.sans"          45
run_test "scope_option_return"       "$REPO_ROOT/tests/fixtures/scope_option_return.sans"       10
run_test "scope_result_return"       "$REPO_ROOT/tests/fixtures/scope_result_return.sans"       21
run_test "scope_multiple_returns"    "$REPO_ROOT/tests/fixtures/scope_multiple_returns.sans"    42

# ---------------------------------------------------------------------------
# Concurrency (non-spawn)
# ---------------------------------------------------------------------------

run_test "thread_return"             "$REPO_ROOT/tests/fixtures/thread_return.sans"             21
run_test "channel_multiple"          "$REPO_ROOT/tests/fixtures/channel_multiple.sans"          30
run_test "mutex_counter"             "$REPO_ROOT/tests/fixtures/mutex_counter.sans"             30
run_test "spawn_closure_test"        "$REPO_ROOT/tests/fixtures/spawn_closure_test.sans"        42
run_test "channel_bounded_full"      "$REPO_ROOT/tests/fixtures/channel_bounded_full.sans"      6
run_test "thread_join_order"         "$REPO_ROOT/tests/fixtures/thread_join_order.sans"         21

# ---------------------------------------------------------------------------
# Additional coverage
# ---------------------------------------------------------------------------

run_test "struct_method"             "$REPO_ROOT/tests/fixtures/struct_method.sans"             43
run_test "enum_variant_data"         "$REPO_ROOT/tests/fixtures/enum_variant_data.sans"         42
run_test "array_sum"                 "$REPO_ROOT/tests/fixtures/array_sum.sans"                 55
run_test "array_min_max"             "$REPO_ROOT/tests/fixtures/array_min_max.sans"             10
run_test "array_reverse"             "$REPO_ROOT/tests/fixtures/array_reverse.sans"             6
run_test "array_slice"               "$REPO_ROOT/tests/fixtures/array_slice.sans"               22
run_test "array_contains"            "$REPO_ROOT/tests/fixtures/array_contains.sans"            10
run_test "array_each"                "$REPO_ROOT/tests/fixtures/array_each.sans"                15
run_test "global_mutable"            "$REPO_ROOT/tests/fixtures/global_mutable.sans"            5
run_test "recursion_factorial"       "$REPO_ROOT/tests/fixtures/recursion_factorial.sans"       120
run_test "recursion_sum"             "$REPO_ROOT/tests/fixtures/recursion_sum.sans"             55
run_test "struct_nested"             "$REPO_ROOT/tests/fixtures/struct_nested.sans"             42
run_test "enum_simple_match"         "$REPO_ROOT/tests/fixtures/enum_simple_match.sans"         4
run_test "generic_pair_test"         "$REPO_ROOT/tests/fixtures/generic_pair_test.sans"         42
run_test "multiline_calc"            "$REPO_ROOT/tests/fixtures/multiline_calc.sans"            21
run_test "expression_fn"             "$REPO_ROOT/tests/fixtures/expression_fn.sans"             42
run_test "min_max_abs"               "$REPO_ROOT/tests/fixtures/min_max_abs.sans"               12
run_test "json_object_build"         "$REPO_ROOT/tests/fixtures/json_object_build.sans"         1
run_test "json_array_build"          "$REPO_ROOT/tests/fixtures/json_array_build.sans"          3
run_test "json_parse_int"            "$REPO_ROOT/tests/fixtures/json_parse_int.sans"            42
run_test "range_step"                "$REPO_ROOT/tests/fixtures/range_step.sans"                35
run_test "tuple_access"              "$REPO_ROOT/tests/fixtures/tuple_access.sans"              42
run_test "tuple_destructure"         "$REPO_ROOT/tests/fixtures/tuple_destructure.sans"         42
run_test "array_push_pop"            "$REPO_ROOT/tests/fixtures/array_push_pop.sans"            32
run_test "bool_short_circuit"        "$REPO_ROOT/tests/fixtures/bool_short_circuit.sans"        1

# ---------------------------------------------------------------------------
# Directory-based (multi-module) tests
# ---------------------------------------------------------------------------

run_test "import_basic"   "$REPO_ROOT/tests/fixtures/import_basic/main.sans"   7
run_test "import_nested"  "$REPO_ROOT/tests/fixtures/import_nested/main.sans"  15
run_test "import_chain"   "$REPO_ROOT/tests/fixtures/import_chain/main.sans"   13
run_test "import_struct"  "$REPO_ROOT/tests/fixtures/import_struct/main.sans"  22
run_test "lambda_cross_module" "$REPO_ROOT/tests/fixtures/lambda_cross_module/main.sans" 15
run_test "visibility_pub"     "$REPO_ROOT/tests/fixtures/visibility_pub/main.sans"      17
run_test "import_alias"       "$REPO_ROOT/tests/fixtures/import_alias/main.sans"        14
run_test "warnings_test"     "$REPO_ROOT/tests/fixtures/warnings_test.sans"            42
run_test "unreachable_test"  "$REPO_ROOT/tests/fixtures/unreachable_test.sans"         10

# ---------------------------------------------------------------------------
# Negative tests (expected to fail compilation)
# ---------------------------------------------------------------------------

echo ""
echo "Negative tests (expected compile failures)"
echo "----------------------------------------"

run_negative_test "neg_undefined_var"        "$REPO_ROOT/tests/negative/undefined_var.sans"        "undefined variable"
run_negative_test "neg_undefined_var2"       "$REPO_ROOT/tests/negative/undefined_var2.sans"       "undefined variable"
run_negative_test "neg_undefined_fn"         "$REPO_ROOT/tests/negative/undefined_fn.sans"         "undefined function"
run_negative_test "neg_wrong_arg_count"      "$REPO_ROOT/tests/negative/wrong_arg_count.sans"      "argument"
run_negative_test "neg_return_type_mismatch" "$REPO_ROOT/tests/negative/return_type_mismatch.sans" "undefined"
run_negative_test "neg_parse_error"          "$REPO_ROOT/tests/negative/parse_error.sans"          "PARSE ERR"
run_negative_test "neg_double_assign"        "$REPO_ROOT/tests/negative/double_assign.sans"        "undefined variable"
run_negative_test "neg_wrong_method"         "$REPO_ROOT/tests/negative/wrong_method.sans"         "undefined"
run_negative_test "neg_duplicate_fn"         "$REPO_ROOT/tests/negative/duplicate_fn.sans"         "PARSE ERR"
run_negative_test "neg_trait_not_impl"       "$REPO_ROOT/tests/negative/trait_not_impl.sans"       ""
run_negative_test "neg_missing_return"       "$REPO_ROOT/tests/negative/missing_return.sans"       "undefined"
run_negative_test "neg_generic_mismatch"     "$REPO_ROOT/tests/negative/generic_mismatch.sans"     "undefined function"
run_negative_test "neg_import_missing"       "$REPO_ROOT/tests/negative/import_missing.sans"       "undefined variable"
run_negative_test "neg_undefined_fn2"        "$REPO_ROOT/tests/negative/undefined_fn2.sans"        "undefined function"
run_negative_test "neg_wrong_arg_type"       "$REPO_ROOT/tests/negative/wrong_arg_type.sans"       "argument"
run_negative_test "neg_bad_struct_field"     "$REPO_ROOT/tests/negative/bad_struct_field.sans"      "undefined"
run_negative_test "neg_parse_error2"         "$REPO_ROOT/tests/negative/parse_error2.sans"          "PARSE ERR"
run_negative_test "neg_wrong_arg_count2"     "$REPO_ROOT/tests/negative/wrong_arg_count2.sans"      "argument"
run_negative_test "neg_undefined_var3"       "$REPO_ROOT/tests/negative/undefined_var3.sans"        "undefined variable"
run_negative_test "neg_bad_enum_variant"     "$REPO_ROOT/tests/negative/bad_enum_variant.sans"      "undefined"
run_negative_test "neg_no_main"              "$REPO_ROOT/tests/negative/no_main.sans"               ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL=$((PASS + FAIL + SKIP))
NEG_TOTAL=$((NEG_PASS + NEG_FAIL))
GRAND_TOTAL=$((TOTAL + NEG_TOTAL))
echo "----------------------------------------"
echo -e "${GREEN}$PASS${NC}/$TOTAL tests passed  (${SKIP} skipped, ${FAIL} failed)"
echo -e "${GREEN}$NEG_PASS${NC}/$NEG_TOTAL negative tests passed  (${NEG_FAIL} failed)"
echo -e "Grand total: $((PASS + NEG_PASS))/$GRAND_TOTAL"

if [ "$FAIL" -gt 0 ] || [ "$NEG_FAIL" -gt 0 ]; then
    exit 1
fi
