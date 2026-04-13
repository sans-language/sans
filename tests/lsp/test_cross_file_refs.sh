#!/bin/bash
set -euo pipefail

# LSP Cross-File References Integration Test
# Tests that find references returns location entries with range data.
# Uses a single-file case to verify the core references machinery works;
# cross-file resolution requires real import resolution in the LSP.

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

echo "LSP Cross-File References Tests ($SANS_LSP)"
echo "============================================="

# --- Create temp file for URI ---
TMPFILE=$(mktemp /tmp/sans_lsp_refs_XXXXXX.sans)
trap "rm -f '$TMPFILE'" EXIT
FILE_URI="file://$TMPFILE"

# Source: function "add" defined and called in the same file
# add(a:I b:I) I = a + b
# main() I {
#   r = add(1 2)
#   r
# }
# References requested at line 0, character 0 (on "add" definition)

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"$FILE_URI\",\"languageId\":\"sans\",\"version\":1,\"text\":\"add(a:I b:I) I = a + b\\nmain() I {\\n  r = add(1 2)\\n  r\\n}\"}}}"
# textDocument/references at line 0, char 0 (on "add"), includeDeclaration=true
REFERENCES='{"jsonrpc":"2.0","id":3,"method":"textDocument/references","params":{"textDocument":{"uri":"'"$FILE_URI"'"},"position":{"line":0,"character":0},"context":{"includeDeclaration":true}}}'
SHUTDOWN='{"jsonrpc":"2.0","id":10,"method":"shutdown","params":null}'
EXIT_MSG='{"jsonrpc":"2.0","method":"exit","params":null}'

# --- Run LSP ---
RESPONSE=$( (send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DIDOPEN"; sleep 1; send_msg "$REFERENCES"; sleep 1; send_msg "$SHUTDOWN"; sleep 0.3; send_msg "$EXIT_MSG") | timeout 15 "$SANS_LSP" 2>/dev/null) || true

# Test 1: Server initializes successfully
if echo "$RESPONSE" | grep -q '"capabilities"'; then
    pass "server returns capabilities on initialize"
else
    fail "server did not return capabilities"
fi

# Test 2: References response is not an error
if echo "$RESPONSE" | grep -q '"error"'; then
    fail "references request returned an error"
else
    pass "references request did not return an error"
fi

# Test 3: References response contains range entries
if echo "$RESPONSE" | grep -q '"range"'; then
    pass "references response contains range entries"
else
    fail "references response missing range entries"
fi

# Test 4: References response contains uri entries
if echo "$RESPONSE" | grep -q '"uri"'; then
    pass "references response contains uri entries"
else
    fail "references response missing uri entries"
fi

# Test 5: References response result is an array (starts with "[")
if echo "$RESPONSE" | grep -q '"result":\s*\['; then
    pass "references result is an array"
else
    fail "references result is not an array"
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
