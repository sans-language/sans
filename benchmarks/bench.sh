#!/bin/bash
set -euo pipefail

# Sans Benchmark Suite Runner
# Compiles and runs benchmarks across Sans, Python, Go, Node.js, and Rust
# Outputs CSV to results.csv and prints summary table

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

RUNS=${1:-10}
CSV="results.csv"
BUILD_DIR="build"

mkdir -p "$BUILD_DIR"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}Sans Benchmark Suite${NC}"
echo "Runs per benchmark: $RUNS"
echo ""

# ── Detect available languages ──────────────────────────────────────
HAS_SANS=0; HAS_PYTHON=0; HAS_GO=0; HAS_NODE=0; HAS_RUST=0

echo "Detecting languages..."
for pair in "sans:sans" "python:python3" "go:go" "node:node" "rust:rustc"; do
    name="${pair%%:*}"
    cmd="${pair##*:}"
    if command -v "$cmd" &>/dev/null; then
        eval "HAS_$(echo "$name" | tr '[:lower:]' '[:upper:]')=1"
        echo -e "  ${GREEN}✓${NC} $name ($cmd)"
    else
        echo -e "  ${YELLOW}✗${NC} $name (not found: $cmd)"
    fi
done
echo ""

has_lang() { eval "echo \$HAS_$(echo "$1" | tr '[:lower:]' '[:upper:]')"; }

# ── Timing helper (macOS compatible) ────────────────────────────────
time_ms() {
    python3 -c "
import time, subprocess, sys
t = time.perf_counter()
subprocess.run(sys.argv[1:], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
print(f'{(time.perf_counter() - t) * 1000:.2f}')
" "$@"
}

# ── Compile phase ───────────────────────────────────────────────────
echo "Compiling..."

if [[ $(has_lang sans) -eq 1 ]]; then
    for f in sans/*.sans; do
        name=$(basename "$f" .sans)
        echo "  sans: $name"
        sans build "$f" 2>/dev/null && mv "sans/$name" "$BUILD_DIR/sans_$name" || {
            echo "    WARN: sans build failed for $name"
        }
    done
fi

if [[ $(has_lang go) -eq 1 ]]; then
    for f in go/*.go; do
        name=$(basename "$f" .go)
        echo "  go: $name"
        go build -o "$BUILD_DIR/go_$name" "$f"
    done
fi

if [[ $(has_lang rust) -eq 1 ]]; then
    for f in rust/*.rs; do
        name=$(basename "$f" .rs)
        echo "  rust: $name"
        rustc -O "$f" -o "$BUILD_DIR/rust_$name" 2>/dev/null
    done
    if [[ -d rust/json_roundtrip ]]; then
        echo "  rust: json_roundtrip (cargo)"
        (cd rust/json_roundtrip && cargo build --release -q 2>/dev/null)
        cp rust/json_roundtrip/target/release/json_roundtrip "$BUILD_DIR/rust_json_roundtrip" 2>/dev/null || true
    fi
fi

echo ""

# ── Benchmark definitions ──────────────────────────────────────────
BENCHMARKS="fib loop_sum array_ops string_concat json_roundtrip"
LANG_ORDER="sans python go node rust"

bench_cmd() {
    local bench=$1 lang=$2
    case $lang in
        sans)   echo "$BUILD_DIR/sans_$bench" ;;
        python) echo "python3 python/$bench.py" ;;
        go)     echo "$BUILD_DIR/go_$bench" ;;
        node)   echo "node node/$bench.js" ;;
        rust)   echo "$BUILD_DIR/rust_$bench" ;;
    esac
}

bench_exists() {
    local bench=$1 lang=$2
    case $lang in
        sans)   [[ -f "$BUILD_DIR/sans_$bench" ]] ;;
        python) [[ -f "python/$bench.py" ]] ;;
        go)     [[ -f "$BUILD_DIR/go_$bench" ]] ;;
        node)   [[ -f "node/$bench.js" ]] ;;
        rust)   [[ -f "$BUILD_DIR/rust_$bench" ]] ;;
    esac
}

# ── Run phase ──────────────────────────────────────────────────────
echo "benchmark,language,mean_ms,stddev_ms,min_ms,max_ms" > "$CSV"

for bench in $BENCHMARKS; do
    echo -e "${CYAN}Running: $bench${NC}"
    for lang in $LANG_ORDER; do
        [[ $(has_lang "$lang") -eq 0 ]] && continue
        bench_exists "$bench" "$lang" || continue

        cmd=$(bench_cmd "$bench" "$lang")
        times=""

        # Warmup run
        eval "$cmd" > /dev/null 2>&1 || true

        for r in $(seq 1 "$RUNS"); do
            ms=$(time_ms $cmd)
            times="$times $ms"
        done

        # Calculate stats
        stats=$(python3 -c "
import sys
times = [float(x) for x in sys.argv[1:]]
n = len(times)
mean = sum(times) / n
variance = sum((t - mean) ** 2 for t in times) / n
stddev = variance ** 0.5
print(f'{mean:.2f},{stddev:.2f},{min(times):.2f},{max(times):.2f}')
" $times)

        echo "$bench,$lang,$stats" >> "$CSV"

        mean=$(echo "$stats" | cut -d, -f1)
        printf "  %-8s %8s ms\n" "$lang" "$mean"
    done
    echo ""
done

# ── Summary table ──────────────────────────────────────────────────
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}Results Summary (mean ms, ${RUNS} runs)${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
printf "%-16s" "Benchmark"
for lang in $LANG_ORDER; do
    [[ $(has_lang "$lang") -eq 0 ]] && continue
    printf "%10s" "$lang"
done
echo ""
printf "%-16s" "────────────────"
for lang in $LANG_ORDER; do
    [[ $(has_lang "$lang") -eq 0 ]] && continue
    printf "%10s" "──────────"
done
echo ""

for bench in $BENCHMARKS; do
    printf "%-16s" "$bench"
    for lang in $LANG_ORDER; do
        [[ $(has_lang "$lang") -eq 0 ]] && continue
        val=$(grep "^$bench,$lang," "$CSV" 2>/dev/null | cut -d, -f3)
        if [[ -n "$val" ]]; then
            printf "%10s" "${val}"
        else
            printf "%10s" "-"
        fi
    done
    echo ""
done
echo ""
echo "Full results: $CSV"
