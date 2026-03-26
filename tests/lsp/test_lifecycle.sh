#!/bin/bash
set -euo pipefail

# LSP Lifecycle Integration Test
# Tests: initialize -> initialized -> shutdown -> exit

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

echo "LSP Lifecycle Tests ($SANS_LSP)"
echo "================================"

# --- JSON-RPC Messages ---
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
SHUTDOWN='{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run full lifecycle ---
EXIT_CODE=0
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT") | timeout 5 "$SANS_LSP" 2>/dev/null) || EXIT_CODE=$?

# Test 1: Process exits cleanly
if [ "$EXIT_CODE" -eq 0 ]; then
    pass "process exits with code 0"
else
    fail "process exited with code $EXIT_CODE (expected 0)"
fi

# Test 2: Initialize response contains capabilities
if echo "$RESPONSE" | grep -q '"capabilities"'; then
    pass "initialize response contains capabilities"
else
    fail "initialize response missing capabilities"
fi

# Test 3: Initialize response has textDocumentSync
if echo "$RESPONSE" | grep -q '"textDocumentSync"'; then
    pass "server advertises textDocumentSync"
else
    fail "server missing textDocumentSync capability"
fi

# Test 4: Initialize response has hoverProvider
if echo "$RESPONSE" | grep -q '"hoverProvider"'; then
    pass "server advertises hoverProvider"
else
    fail "server missing hoverProvider capability"
fi

# Test 5: Initialize response has definitionProvider
if echo "$RESPONSE" | grep -q '"definitionProvider"'; then
    pass "server advertises definitionProvider"
else
    fail "server missing definitionProvider capability"
fi

# Test 6: Initialize response has serverInfo
if echo "$RESPONSE" | grep -q '"serverInfo"'; then
    pass "initialize response contains serverInfo"
else
    fail "initialize response missing serverInfo"
fi

# Test 7: Shutdown returns null result
if echo "$RESPONSE" | grep -q '"id":2.*"result":null\|"result":null.*"id":2'; then
    pass "shutdown returns null result"
else
    fail "shutdown did not return null result"
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
