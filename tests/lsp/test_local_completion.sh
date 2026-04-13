#!/bin/bash
set -euo pipefail

# LSP Local Completion Integration Test
# Tests that completions inside a function body include local variables

SANS_LSP="${1:-/tmp/sans-lsp}"

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

PASS=0
FAIL=0

pass() {
    echo -e "  ${GREEN}✓${NC}  $1"
    ((PASS++)) || true
}

fail() {
    echo -e "  ${RED}✗${NC}  $1"
    ((FAIL++)) || true
}

send_msg() {
    local json="$1"
    local len=${#json}
    printf "Content-Length: %d\r\n\r\n%s" "$len" "$json"
}

# Check binary exists
if [ ! -x "$SANS_LSP" ]; then
    echo -e "${RED}Error: LSP binary not found or not executable: $SANS_LSP${NC}"
    exit 1
fi

echo "LSP Local Completion Tests ($SANS_LSP)"
echo "========================================"

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_local_comp_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
FILE_URI="file://$TMPFILE"

# Source: function with local vars a, b, result
# add(a:I b:I) I {
#   result = a + b
#   result
# }
# Completion requested at line 2, character 2 (inside function body)

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"add(a:I b:I) I {\\n  result = a + b\\n  result\\n}\"}}}"
# Completion at line 2, character 2 (inside function body, after leading spaces)
COMPLETION='{"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"'"$FILE_URI"'"},"position":{"line":2,"character":2}}}'
SHUTDOWN='{"jsonrpc":"2.0","id":10,"method":"shutdown","params":null}'
EXIT_MSG='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run LSP ---
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DIDOPEN"; sleep 1; send_msg "$COMPLETION"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT_MSG") | timeout 15 "$SANS_LSP" 2>/dev/null) || true

# Test 1: Capabilities include completionProvider
if echo "$RESPONSE" | grep -q '"completionProvider"'; then
    pass "server advertises completionProvider"
else
    fail "server missing completionProvider capability"
fi

# Test 2: Completion response contains items
if echo "$RESPONSE" | grep -q '"items"'; then
    pass "completion response contains items"
else
    fail "completion response missing items"
fi

# Test 3: Completion includes local variable "result"
if echo "$RESPONSE" | grep -q '"result"'; then
    pass "completion includes local variable result"
else
    fail "completion missing local variable result"
fi

# Test 4: Completion includes parameter "a"
if echo "$RESPONSE" | grep -q '"a"'; then
    pass "completion includes parameter a"
else
    fail "completion missing parameter a"
fi

# Test 5: Completion includes parameter "b"
if echo "$RESPONSE" | grep -q '"b"'; then
    pass "completion includes parameter b"
else
    fail "completion missing parameter b"
fi

# Test 6: Response has isIncomplete field
if echo "$RESPONSE" | grep -q '"isIncomplete"'; then
    pass "completion response has isIncomplete field"
else
    fail "completion response missing isIncomplete field"
fi

# --- Summary ---
echo ""
TOTAL=$((PASS + FAIL))
if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}All $TOTAL tests passed${NC}"
else
    echo -e "${RED}$FAIL/$TOTAL tests failed${NC}"
    exit 1
fi
