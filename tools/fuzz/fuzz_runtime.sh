#!/usr/bin/env bash
# Runtime fuzz testing harness for Sans.
# Usage: ./tools/fuzz/fuzz_runtime.sh [iterations] [sans-binary]
#
# Generates valid Sans programs that exercise JSON parsing, string ops,
# and map operations with adversarial inputs. The program must compile
# and run without crashing (segfault/abort). Non-zero exit codes from
# the generated program are OK (e.g., error handling paths).

set -euo pipefail

ITERATIONS="${1:-1000}"
SANS_BIN="${2:-./sans}"
GENERATOR="$(dirname "$0")/generators/gen_runtime_fuzz.py"
CRASHES_DIR="$(dirname "$0")/crashes"
HANGS_DIR="$(dirname "$0")/hangs"
COMPILE_TIMEOUT=30
RUN_TIMEOUT=10
PROGRESS_EVERY=100

# Colors
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
count_compile_error=0
count_runtime_error=0

if [ ! -f "$SANS_BIN" ]; then
    echo "ERROR: Sans binary not found: $SANS_BIN" >&2
    echo "Build it first or pass the path as second argument" >&2
    exit 1
fi

if [ ! -f "$GENERATOR" ]; then
    echo "ERROR: Generator script not found: $GENERATOR" >&2
    exit 1
fi

if ! command -v python3 &>/dev/null; then
    echo "ERROR: python3 is required" >&2
    exit 1
fi

mkdir -p "$CRASHES_DIR" "$HANGS_DIR"

echo -e "${CYAN}Sans Runtime Fuzz Testing Harness${NC}"
echo "  Binary:      $SANS_BIN"
echo "  Iterations:  $ITERATIONS"
echo "  Compile timeout: ${COMPILE_TIMEOUT}s"
echo "  Run timeout:     ${RUN_TIMEOUT}s"
echo ""

TMPFILE=$(mktemp /tmp/sans_rtfuzz_XXXXXX.sans)
TMPBIN=$(mktemp /tmp/sans_rtfuzz_bin_XXXXXX)
trap 'rm -f "$TMPFILE" "$TMPBIN"' EXIT

MODES=("json" "string" "map" "random")

for i in $(seq 1 "$ITERATIONS"); do
    mode="${MODES[$((i % ${#MODES[@]}))]}"

    # Generate a fuzz test program
    python3 "$GENERATOR" "$mode" > "$TMPFILE" 2>/dev/null || true

    # Compile it
    set +e
    timeout "$COMPILE_TIMEOUT" "$SANS_BIN" build "$TMPFILE" \
        --output "$TMPBIN" \
        >/dev/null 2>/dev/null
    compile_exit=$?
    set -e

    if [ "$compile_exit" -eq 124 ]; then
        count_hang=$(( count_hang + 1 ))
        ts=$(date +%Y%m%d_%H%M%S)
        dest="$HANGS_DIR/rt_compile_hang_${ts}_${i}.sans"
        cp "$TMPFILE" "$dest"
        echo -e "${YELLOW}[COMPILE HANG]${NC} iter=$i mode=$mode saved=$dest"
        continue
    fi

    if [ "$compile_exit" -ge 128 ]; then
        count_crash=$(( count_crash + 1 ))
        ts=$(date +%Y%m%d_%H%M%S)
        dest="$CRASHES_DIR/rt_compile_crash_${ts}_${i}_exit${compile_exit}.sans"
        cp "$TMPFILE" "$dest"
        echo -e "${RED}[COMPILE CRASH]${NC} iter=$i mode=$mode exit=${compile_exit} saved=$dest"
        continue
    fi

    if [ "$compile_exit" -ne 0 ]; then
        count_compile_error=$(( count_compile_error + 1 ))
        continue
    fi

    # Run the compiled program
    if [ ! -f "$TMPBIN" ]; then
        count_compile_error=$(( count_compile_error + 1 ))
        continue
    fi

    set +e
    timeout "$RUN_TIMEOUT" "$TMPBIN" >/dev/null 2>/dev/null
    run_exit=$?
    set -e

    if [ "$run_exit" -eq 124 ]; then
        count_hang=$(( count_hang + 1 ))
        ts=$(date +%Y%m%d_%H%M%S)
        dest="$HANGS_DIR/rt_run_hang_${ts}_${i}.sans"
        cp "$TMPFILE" "$dest"
        echo -e "${YELLOW}[RUN HANG]${NC} iter=$i mode=$mode saved=$dest"
    elif [ "$run_exit" -ge 128 ]; then
        count_crash=$(( count_crash + 1 ))
        ts=$(date +%Y%m%d_%H%M%S)
        dest="$CRASHES_DIR/rt_run_crash_${ts}_${i}_exit${run_exit}.sans"
        cp "$TMPFILE" "$dest"
        echo -e "${RED}[RUN CRASH]${NC} iter=$i mode=$mode exit=${run_exit} saved=$dest"
    elif [ "$run_exit" -ne 0 ]; then
        # Non-zero exit is fine (error handling code paths)
        count_runtime_error=$(( count_runtime_error + 1 ))
    else
        count_pass=$(( count_pass + 1 ))
    fi

    # Progress report
    if (( i % PROGRESS_EVERY == 0 )); then
        pct=$(( i * 100 / ITERATIONS ))
        echo -e "${CYAN}[Progress]${NC} ${i}/${ITERATIONS} (${pct}%) — pass=${count_pass} compile_err=${count_compile_error} runtime_err=${count_runtime_error} crashes=${count_crash} hangs=${count_hang}"
    fi
done

echo ""
echo "======================================"
echo "  Runtime Fuzz Run Complete"
echo "======================================"
echo "  Iterations:      $ITERATIONS"
echo "  Passed:          $count_pass"
echo "  Compile errors:  $count_compile_error  (expected — generated code may not type-check)"
echo "  Runtime errors:  $count_runtime_error  (expected — error handling paths)"
echo "  Hangs:           $count_hang"
echo "  Crashes:         $count_crash"
echo ""

if [ "$count_crash" -gt 0 ] || [ "$count_hang" -gt 0 ]; then
    echo -e "${RED}FAIL${NC} — $count_crash crash(es), $count_hang hang(s) found."
    echo "  Crash inputs: $CRASHES_DIR"
    echo "  Hang inputs:  $HANGS_DIR"
    exit 1
else
    echo -e "${GREEN}PASS${NC} — runtime handled all inputs gracefully."
    exit 0
fi
