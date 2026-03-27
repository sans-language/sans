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

check_not() {
  local desc="$1" pattern="$2" response="$3"
  printf "  "
  if echo "$response" | grep -q "$pattern"; then
    echo -e "${RED}\xE2\x9C\x97${NC}  $desc"
    ((FAIL++)) || true
  else
    echo -e "${GREEN}\xE2\x9C\x93${NC}  $desc"
    ((PASS++)) || true
  fi
}

echo "LSP Multi-file Tests ($SANS_LSP)"
echo "================================="

# Create test project with two files
TEST_DIR="/tmp/sans_lsp_multifile_$$"
mkdir -p "$TEST_DIR"

cat > "$TEST_DIR/utils.sans" << 'SANS'
add(a:I b:I) I = a + b
greet(name:S) S = "hello " + name
SANS

cat > "$TEST_DIR/main.sans" << 'SANS'
import "utils"

main() I {
  x = add(1, 2)
  p(x)
  0
}
SANS

MAIN_URI="file://${TEST_DIR}/main.sans"
MAIN_CONTENT=$(python3 -c 'import sys,json; print(json.dumps(open(sys.argv[1]).read())[1:-1])' "$TEST_DIR/main.sans")

INIT="{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"rootUri\":\"file://${TEST_DIR}\",\"capabilities\":{}}}"
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
DID_OPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${MAIN_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"${MAIN_CONTENT}\"}}}"
HOVER="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{\"textDocument\":{\"uri\":\"${MAIN_URI}\"},\"position\":{\"line\":3,\"character\":6}}}"
SHUTDOWN='{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN"; sleep 1.5; send_msg "$HOVER"; sleep 0.5; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 15 "$SANS_LSP" 2>/dev/null || true)

# Test 1: No "undefined" error for imported function
check_not "imported add() not reported as undefined" "undefined.*add" "$RESPONSE"

# Test 2: Diagnostics were published
check "diagnostics published for main.sans" "publishDiagnostics" "$RESPONSE"

# Test 3: Hover shows something for add (contents field present)
check "hover returns content for imported function" '"contents"' "$RESPONSE"

# --- Test with bad import ---
cat > "$TEST_DIR/bad.sans" << 'SANS'
import "nonexistent"

main() I { 0 }
SANS

BAD_URI="file://${TEST_DIR}/bad.sans"
BAD_CONTENT=$(python3 -c 'import sys,json; print(json.dumps(open(sys.argv[1]).read())[1:-1])' "$TEST_DIR/bad.sans")
DID_OPEN_BAD="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${BAD_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"${BAD_CONTENT}\"}}}"

RESPONSE2=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN_BAD"; sleep 1.5; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 15 "$SANS_LSP" 2>/dev/null || true)

# Test 4: Bad import doesn't crash
check "LSP handles missing import without crash" "publishDiagnostics" "$RESPONSE2"

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
