#!/usr/bin/env bash
# Fuzz testing harness for the Sans compiler.
# Usage: ./tools/fuzz/fuzz_compiler.sh [iterations] [sans-binary]
#
# Generates random/malformed Sans files and feeds them to the compiler.
# The compiler must NEVER crash — it must always emit an error and exit cleanly.
# Crashes (signal kills) and hangs (timeouts) are saved for inspection.

set -euo pipefail

ITERATIONS="${1:-10000}"
SANS_BIN="${2:-./sans}"
GENERATOR="$(dirname "$0")/generators/gen_random_sans.py"
CRASHES_DIR="$(dirname "$0")/crashes"
HANGS_DIR="$(dirname "$0")/hangs"
TIMEOUT_SECS=10
PROGRESS_EVERY=1000

# Colors (only if terminal supports it)
if [ -t 1 ]; then
    RED='\033[0;31m'
    YELLOW='\033[1;33m'
    GREEN='\033[0;32m'
    CYAN='\033[0;36m'
    NC='\033[0m'
else
    RED='' YELLOW='' GREEN='' CYAN='' NC=''
fi

count_pass=0
count_crash=0
count_hang=0
count_compile_error=0  # clean compiler errors (expected)

if [ ! -f "$SANS_BIN" ]; then
    echo "ERROR: Sans binary not found: $SANS_BIN" >&2
    echo "Build it first with: sans build compiler/main.sans" >&2
    exit 1
fi

if [ ! -f "$GENERATOR" ]; then
    echo "ERROR: Generator script not found: $GENERATOR" >&2
    exit 1
fi

if ! command -v python3 &>/dev/null; then
    echo "ERROR: python3 is required for the fuzzer generator" >&2
    exit 1
fi

mkdir -p "$CRASHES_DIR" "$HANGS_DIR"

echo -e "${CYAN}Sans Fuzz Testing Harness${NC}"
echo "  Binary:     $SANS_BIN"
echo "  Iterations: $ITERATIONS"
echo "  Timeout:    ${TIMEOUT_SECS}s per run"
echo "  Crashes dir: $CRASHES_DIR"
echo "  Hangs dir:  $HANGS_DIR"
echo ""

TMPFILE=$(mktemp /tmp/sans_fuzz_XXXXXX.sans)
trap 'rm -f "$TMPFILE"' EXIT

for i in $(seq 1 "$ITERATIONS"); do
    # Alternate between random and structured modes
    if (( i % 2 == 0 )); then
        mode="structured"
    else
        mode="random"
    fi

    # Generate a fuzz input
    python3 "$GENERATOR" "$mode" > "$TMPFILE" 2>/dev/null || true

    # Run compiler under timeout, capturing exit code
    set +e
    timeout "$TIMEOUT_SECS" "$SANS_BIN" build "$TMPFILE" \
        --output /dev/null \
        >/dev/null 2>/dev/null
    exit_code=$?
    set -e

    if [ "$exit_code" -eq 124 ]; then
        # Timeout — compiler hung
        count_hang=$(( count_hang + 1 ))
        ts=$(date +%Y%m%d_%H%M%S)
        dest="$HANGS_DIR/hang_${ts}_${i}.sans"
        cp "$TMPFILE" "$dest"
        echo -e "${YELLOW}[HANG]${NC} iter=$i mode=$mode saved=$dest"

    elif [ "$exit_code" -ge 128 ] || [ "$exit_code" -eq 1 ] && kill -0 $$ 2>/dev/null; then
        # Exit codes >= 128 mean killed by signal (crash)
        # Also catch exit code 134 (SIGABRT), 139 (SIGSEGV), 136 (SIGFPE), etc.
        if [ "$exit_code" -ge 128 ]; then
            count_crash=$(( count_crash + 1 ))
            ts=$(date +%Y%m%d_%H%M%S)
            dest="$CRASHES_DIR/crash_${ts}_${i}_exit${exit_code}.sans"
            cp "$TMPFILE" "$dest"
            echo -e "${RED}[CRASH]${NC} iter=$i mode=$mode exit=${exit_code} saved=$dest"
        else
            # Non-zero but < 128: clean compiler error (expected behaviour)
            count_compile_error=$(( count_compile_error + 1 ))
        fi
    elif [ "$exit_code" -eq 0 ]; then
        # Compiler accepted the input (might be a valid file, that's fine)
        count_pass=$(( count_pass + 1 ))
    else
        # Non-zero but < 128 and not a signal: clean compiler error
        count_compile_error=$(( count_compile_error + 1 ))
    fi

    # Progress report
    if (( i % PROGRESS_EVERY == 0 )); then
        pct=$(( i * 100 / ITERATIONS ))
        echo -e "${CYAN}[Progress]${NC} ${i}/${ITERATIONS} (${pct}%) — pass=${count_pass} errors=${count_compile_error} crashes=${count_crash} hangs=${count_hang}"
    fi
done

echo ""
echo "======================================"
echo "  Fuzz Run Complete"
echo "======================================"
echo "  Iterations:     $ITERATIONS"
echo "  Accepted:       $count_pass"
echo "  Compile errors: $count_compile_error  (expected — compiler rejected input cleanly)"
echo "  Hangs:          $count_hang"
echo "  Crashes:        $count_crash"
echo ""

if [ "$count_crash" -gt 0 ] || [ "$count_hang" -gt 0 ]; then
    echo -e "${RED}FAIL${NC} — $count_crash crash(es), $count_hang hang(s) found."
    echo "  Crash inputs: $CRASHES_DIR"
    echo "  Hang inputs:  $HANGS_DIR"
    exit 1
else
    echo -e "${GREEN}PASS${NC} — compiler handled all inputs gracefully."
    exit 0
fi
