#!/bin/bash
set -euo pipefail

# LSP Scope-Aware Rename Integration Test
# Tests that rename produces workspace edits with the new name

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

echo "LSP Scope Rename Tests ($SANS_LSP)"
echo "====================================="

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_rename_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
FILE_URI="file://$TMPFILE"

# Source:
# main() I {
#   x = 1
#   if x > 0 {
#     x = 2
#     p(x)
#   }
#   x
# }
# Rename at line 3, character 4 (on "x" inside if block) to "y"

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() I {\\n  x = 1\\n  if x > 0 {\\n    x = 2\\n    p(x)\\n  }\\n  x\\n}\"}}}"
# Rename at line 3, character 4 (on "x" in "    x = 2") to "y"
RENAME='{"jsonrpc":"2.0","id":3,"method":"textDocument/rename","params":{"textDocument":{"uri":"'"$FILE_URI"'"},"position":{"line":3,"character":4},"newName":"y"}}'
SHUTDOWN='{"jsonrpc":"2.0","id":10,"method":"shutdown","params":null}'
EXIT_MSG='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run LSP ---
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DIDOPEN"; sleep 1; send_msg "$RENAME"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT_MSG") | timeout 15 "$SANS_LSP" 2>/dev/null) || true

# Test 1: Capabilities include renameProvider
if echo "$RESPONSE" | grep -q '"renameProvider"'; then
    pass "server advertises renameProvider"
else
    fail "server missing renameProvider capability"
fi

# Test 2: Rename response is not an error
if echo "$RESPONSE" | grep -q '"error"'; then
    fail "rename request returned an error"
else
    pass "rename request did not return an error"
fi

# Test 3: Rename response contains "changes" (WorkspaceEdit)
if echo "$RESPONSE" | grep -q '"changes"'; then
    pass "rename response contains changes"
else
    fail "rename response missing changes"
fi

# Test 4: Rename response contains the new name "y"
if echo "$RESPONSE" | grep -q '"y"'; then
    pass "rename response contains new name y"
else
    fail "rename response missing new name y"
fi

# Test 5: Rename response contains range entries (text edits have ranges)
if echo "$RESPONSE" | grep -q '"range"'; then
    pass "rename response contains range entries"
else
    fail "rename response missing range entries"
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
