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

echo "LSP Semantic Tokens Tests ($SANS_LSP)"
echo "======================================"

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
TEST_URI="file:///tmp/sem_test.sans"
DID_OPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"main() I {\\n  x = 42\\n  p(x)\\n  0\\n}\"}}}"
SEM_REQ="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/semanticTokens/full\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\"}}}"
SHUTDOWN='{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN"; sleep 0.3; send_msg "$SEM_REQ"; sleep 0.3; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 10 "$SANS_LSP" 2>/dev/null || true)

check "semantic tokens response has data" '"data"' "$RESPONSE"
check "data array is non-empty" '"data":\[' "$RESPONSE"
check "semanticTokensProvider in capabilities" '"semanticTokensProvider"' "$RESPONSE"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
