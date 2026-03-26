#!/bin/bash
set -euo pipefail

# LSP Diagnostics Integration Test
# Tests that diagnostics are published for files with errors

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

echo "LSP Diagnostics Tests ($SANS_LSP)"
echo "==================================="

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_test_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
echo 'main() { x = undefined_var }' > "$TMPFILE"
FILE_URI="file://$TMPFILE"

# --- JSON-RPC Messages ---
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() { x = undefined_var }\"}}}"
SHUTDOWN='{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run LSP with didOpen ---
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DIDOPEN"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT") | timeout 10 "$SANS_LSP" 2>/dev/null) || true

# Test 1: Server sends publishDiagnostics notification
if echo "$RESPONSE" | grep -q '"textDocument/publishDiagnostics"'; then
    pass "server sends publishDiagnostics notification"
else
    fail "server did not send publishDiagnostics notification"
fi

# Test 2: Diagnostics array is non-empty
if echo "$RESPONSE" | grep -q '"diagnostics":\[{'; then
    pass "diagnostics array is non-empty"
else
    fail "diagnostics array is empty or missing"
fi

# Test 3: Diagnostic mentions the undefined variable
if echo "$RESPONSE" | grep -q 'undefined.*variable\|undefined_var'; then
    pass "diagnostic mentions undefined variable"
else
    fail "diagnostic does not mention undefined variable"
fi

# Test 4: Diagnostic has severity
if echo "$RESPONSE" | grep -q '"severity"'; then
    pass "diagnostic includes severity"
else
    fail "diagnostic missing severity"
fi

# Test 5: Diagnostic has range
if echo "$RESPONSE" | grep -q '"range"'; then
    pass "diagnostic includes range"
else
    fail "diagnostic missing range"
fi

# Test 6: Diagnostic source is sans
if echo "$RESPONSE" | grep -q '"source":"sans"'; then
    pass "diagnostic source is sans"
else
    fail "diagnostic source is not sans"
fi

# --- Test with valid file (should have no/fewer diagnostics) ---
echo ""
echo "Testing valid file..."

VALID_DIDOPEN='{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/sans_lsp_valid_test.sans","languageId":"sans","version":1,"text":"main() = 0"}}}'

VALID_RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$VALID_DIDOPEN"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT") | timeout 10 "$SANS_LSP" 2>/dev/null) || true

# Test 7: Valid file gets diagnostics response (even if empty)
if echo "$VALID_RESPONSE" | grep -q '"textDocument/publishDiagnostics"'; then
    pass "valid file receives publishDiagnostics"
else
    fail "valid file did not receive publishDiagnostics"
fi

# Test 8: Valid file has empty diagnostics array
if echo "$VALID_RESPONSE" | grep -q '"diagnostics":\[\]'; then
    pass "valid file has empty diagnostics array"
else
    fail "valid file has non-empty diagnostics (or missing response)"
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
