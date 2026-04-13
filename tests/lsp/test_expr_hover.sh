#!/bin/bash
set -euo pipefail

# LSP Expression Hover Integration Test
# Tests that hovering on a variable shows its type information

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

echo "LSP Expression Hover Tests ($SANS_LSP)"
echo "========================================"

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_hover_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
FILE_URI="file://$TMPFILE"

# Source:
# main() I {
#   x = 42
#   x
# }
# Hover at line 1, character 2 (on "x" in "x = 42")

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() I {\\n  x = 42\\n  x\\n}\"}}}"
# Hover at line 1, character 2 (on "x")
HOVER='{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":"'"$FILE_URI"'"},"position":{"line":1,"character":2}}}'
SHUTDOWN='{"jsonrpc":"2.0","id":10,"method":"shutdown","params":null}'
EXIT_MSG='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run LSP ---
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DIDOPEN"; sleep 1; send_msg "$HOVER"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT_MSG") | timeout 15 "$SANS_LSP" 2>/dev/null) || true

# Test 1: Capabilities include hoverProvider
if echo "$RESPONSE" | grep -q '"hoverProvider"'; then
    pass "server advertises hoverProvider"
else
    fail "server missing hoverProvider capability"
fi

# Test 2: Hover response contains contents
if echo "$RESPONSE" | grep -q '"contents"'; then
    pass "hover response contains contents"
else
    fail "hover response missing contents"
fi

# Test 3: Hover response is not an error
if echo "$RESPONSE" | grep -q '"error"'; then
    fail "hover request returned an error"
else
    pass "hover request did not return an error"
fi

# Test 4: Hover response mentions "local" (local variable)
if echo "$RESPONSE" | grep -qi '"local\|local'; then
    pass "hover response mentions local variable"
else
    fail "hover response does not mention local"
fi

# Test 5: Hover response mentions the type (Int or I)
if echo "$RESPONSE" | grep -q '"Int\|: I\b\|:I\b'; then
    pass "hover response mentions Int type"
else
    fail "hover response does not mention Int type"
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
