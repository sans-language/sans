#!/bin/bash
set -euo pipefail

# LSP Dot Completion Integration Test
# Tests that dot completion shows methods on a typed value

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

echo "LSP Dot Completion Tests ($SANS_LSP)"
echo "======================================"

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_dot_comp_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
FILE_URI="file://$TMPFILE"

# Source:
# main() I {
#   arr = array<I>()
#   arr.
#   0
# }
# Completion at line 2, character 6 (after "arr.") with trigger "."

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() I {\\n  arr = array<I>()\\n  arr.\\n  0\\n}\"}}}"
# Completion at line 2, character 6 (after "arr."), triggerKind 2 = TriggerCharacter
COMPLETION='{"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"'"$FILE_URI"'"},"position":{"line":2,"character":6},"context":{"triggerKind":2,"triggerCharacter":"."}}}'
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

# Test 3: Completion includes array method "push" or "len"
if echo "$RESPONSE" | grep -q '"push"\|"len"'; then
    pass "completion includes array method (push or len)"
else
    fail "completion missing array methods"
fi

# Test 4: Response has isIncomplete field
if echo "$RESPONSE" | grep -q '"isIncomplete"'; then
    pass "completion response has isIncomplete field"
else
    fail "completion response missing isIncomplete field"
fi

# Test 5: Completion items have kind field
if echo "$RESPONSE" | grep -q '"kind"'; then
    pass "completion items have kind field"
else
    fail "completion items missing kind field"
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
