#!/bin/sh
# Bootstrap the Sans compiler from a release binary.
#
# Downloads a known-good bootstrap compiler, builds the current source,
# and installs the result to ~/.local/bin/sans.
#
# Usage: bash scripts/bootstrap.sh
#
# Prerequisites: LLVM 17 (brew install llvm@17)

set -e

BOOTSTRAP_VERSION="v0.4.0"
BOOTSTRAP_URL="https://github.com/sans-language/sans/releases/download/${BOOTSTRAP_VERSION}/sans-macos-arm64.tar.gz"
INSTALL_DIR="${HOME}/.local/bin"
TMP_DIR=$(mktemp -d)

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "=== Sans Compiler Bootstrap ==="
echo "Bootstrap version: ${BOOTSTRAP_VERSION}"
echo "Install directory: ${INSTALL_DIR}"
echo ""

# 1. Check prerequisites
if ! command -v brew >/dev/null 2>&1; then
  echo "ERROR: Homebrew not found. Install it from https://brew.sh"
  exit 1
fi

LLVM_PREFIX=$(brew --prefix llvm@17 2>/dev/null || true)
if [ -z "$LLVM_PREFIX" ] || [ ! -d "$LLVM_PREFIX" ]; then
  echo "ERROR: LLVM 17 not found. Install with: brew install llvm@17"
  exit 1
fi

OPENSSL_PREFIX=$(brew --prefix openssl@3 2>/dev/null || true)
if [ -z "$OPENSSL_PREFIX" ] || [ ! -d "$OPENSSL_PREFIX" ]; then
  echo "ERROR: OpenSSL 3 not found. Install with: brew install openssl@3"
  exit 1
fi

echo "LLVM 17: ${LLVM_PREFIX}"
echo "OpenSSL: ${OPENSSL_PREFIX}"
echo ""

# 2. Download bootstrap compiler
echo "Downloading bootstrap compiler (${BOOTSTRAP_VERSION})..."
curl -fsSL "$BOOTSTRAP_URL" -o "$TMP_DIR/bootstrap.tar.gz"
cd "$TMP_DIR" && tar xzf bootstrap.tar.gz && chmod +x sans
cd - >/dev/null

echo "Bootstrap compiler: $($TMP_DIR/sans --version 2>&1 || echo 'failed to run')"

# 3. Build current source
echo ""
echo "Building compiler from source..."
export DYLD_LIBRARY_PATH="${OPENSSL_PREFIX}/lib:${DYLD_LIBRARY_PATH:-}"
export PATH="${LLVM_PREFIX}/bin:$PATH"
"$TMP_DIR/sans" build compiler/main.sans

if [ ! -f compiler/main ]; then
  echo "ERROR: Build failed — compiler/main not produced"
  exit 1
fi

echo "Build successful."

# 4. Fix library paths for portability
echo "Fixing library paths..."
for lib in libssl.3.dylib libcrypto.3.dylib; do
  old=$(otool -L compiler/main | grep "$lib" | sed 's/^[[:space:]]*//' | cut -d' ' -f1)
  if [ -n "$old" ] && [ "$old" != "@rpath/$lib" ]; then
    install_name_tool -change "$old" "@rpath/$lib" compiler/main 2>/dev/null || true
  fi
done
install_name_tool -add_rpath /opt/homebrew/opt/openssl@3/lib compiler/main 2>/dev/null || true
install_name_tool -add_rpath /usr/local/opt/openssl@3/lib compiler/main 2>/dev/null || true
install_name_tool -add_rpath "${OPENSSL_PREFIX}/lib" compiler/main 2>/dev/null || true

# 5. Smoke test
echo "Smoke testing..."
echo 'main() I { 0 }' > "$TMP_DIR/smoke.sans"
./compiler/main build "$TMP_DIR/smoke.sans" 2>&1
if [ $? -ne 0 ]; then
  echo "WARNING: Smoke test failed. Binary may still work on this system."
fi

# 6. Install
mkdir -p "$INSTALL_DIR"
cp compiler/main "$INSTALL_DIR/sans"
rm -f compiler/main
echo ""
echo "=== Installed sans to ${INSTALL_DIR}/sans ==="
"$INSTALL_DIR/sans" --version

# Check PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo ""
  echo "NOTE: ${INSTALL_DIR} is not in your PATH. Add it:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi
