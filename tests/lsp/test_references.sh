#!/bin/bash
set -euo pipefail

SANS_LSP="${1:-./sans-lsp}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
PASS=0
FAIL=0

send_msg() {
  local json="$1"
  local len=${#json}
  printf "Content-Length: %d\r\n\r\n%s" "$len" "$json"
}

check() {
  local desc="$1" pattern="$2" response="$3"
  printf "  "
  if echo "$response" | grep -q "$pattern"; then
    echo -e "${GREEN}\xE2\x9C\x93${NC}  $desc"
    ((PASS++)) || true
  else
    echo -e "${RED}\xE2\x9C\x97${NC}  $desc"
    ((FAIL++)) || true
  fi
}

echo "LSP References & Rename Tests ($SANS_LSP)"
echo "=========================================="

TEST_URI="file:///tmp/ref_test.sans"
# Source: "main() I {\n  x = 42\n  y = x + 1\n  p(x)\n  0\n}"
# "x" appears at: line 1 col 2 (def), line 2 col 6 (use), line 3 col 4 (use)
DID_OPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() I {\\n  x = 42\\n  y = x + 1\\n  p(x)\\n  0\\n}\"}}}"
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
# Find references for "x" at line 1, col 2 (0-based)
REFS="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/references\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\"},\"position\":{\"line\":1,\"character\":2},\"context\":{\"includeDeclaration\":true}}}"
# Rename "x" to "val"
RENAME="{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"textDocument/rename\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\"},\"position\":{\"line\":1,\"character\":2},\"newName\":\"val\"}}"
SHUTDOWN='{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN"; sleep 0.5; send_msg "$REFS"; sleep 0.3; send_msg "$RENAME"; sleep 0.3; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 10 "$SANS_LSP" 2>/dev/null || true)

check "references response contains locations" '"range"' "$RESPONSE"
check "found reference on line 2" '"line":2' "$RESPONSE"
check "rename response has changes" '"changes"' "$RESPONSE"
check "rename uses new name" '"val"' "$RESPONSE"
check "referencesProvider in capabilities" '"referencesProvider"' "$RESPONSE"
check "renameProvider in capabilities" '"renameProvider"' "$RESPONSE"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
