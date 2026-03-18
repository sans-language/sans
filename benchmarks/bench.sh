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
        # Run from parent dir so self-hosted compiler can find runtime/
        (cd "$SCRIPT_DIR/.." && sans build "benchmarks/$f" 2>/dev/null) && mv "sans/$name" "$BUILD_DIR/sans_$name" || {
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
    if [[ -d rust/http_throughput ]]; then
        echo "  rust: http_throughput (cargo)"
        (cd rust/http_throughput && cargo build --release -q 2>/dev/null)
        cp rust/http_throughput/target/release/http_throughput "$BUILD_DIR/rust_http_throughput" 2>/dev/null || true
    fi
fi

echo ""

# ── Benchmark definitions ──────────────────────────────────────────
BENCHMARKS="fib loop_sum array_ops string_concat json_roundtrip concurrent file_io mixed"
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

# ── HTTP throughput benchmark ───────────────────────────────────────
if ! command -v wrk &>/dev/null; then
    echo "wrk not found — skipping HTTP throughput benchmark (brew install wrk)"
    exit 0
fi

HTTP_PORT=8765
HTTP_DURATION=10
HTTP_THREADS=8
HTTP_CONNS=100
HTTP_CSV="http_results.csv"

http_server_cmd() {
    local lang=$1
    case $lang in
        sans)   echo "$BUILD_DIR/sans_http_throughput" ;;
        python) echo "python3 python/http_throughput.py" ;;
        go)     echo "$BUILD_DIR/go_http_throughput" ;;
        node)   echo "node node/http_throughput.js" ;;
        rust)   echo "$BUILD_DIR/rust_http_throughput" ;;
    esac
}

http_server_exists() {
    local lang=$1
    case $lang in
        sans)   [[ -f "$BUILD_DIR/sans_http_throughput" ]] ;;
        python) [[ -f "python/http_throughput.py" ]] ;;
        go)     [[ -f "$BUILD_DIR/go_http_throughput" ]] ;;
        node)   [[ -f "node/http_throughput.js" ]] ;;
        rust)   [[ -f "$BUILD_DIR/rust_http_throughput" ]] ;;
    esac
}

wait_for_server() {
    local port=$1
    for i in $(seq 1 30); do
        if curl -sf "http://localhost:$port/" > /dev/null 2>&1; then
            return 0
        fi
        sleep 0.2
    done
    return 1
}

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}HTTP Throughput (req/sec, ${HTTP_DURATION}s, ${HTTP_THREADS}t/${HTTP_CONNS}c, higher is better)${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"

echo "benchmark,language,req_per_sec,latency_avg_ms,latency_p99_ms" > "$HTTP_CSV"

for lang in $LANG_ORDER; do
    [[ $(has_lang "$lang") -eq 0 ]] && continue
    http_server_exists "$lang" || continue

    cmd=$(http_server_cmd "$lang")

    # Start server
    eval "$cmd" > /dev/null 2>&1 &
    SERVER_PID=$!

    if ! wait_for_server "$HTTP_PORT"; then
        echo "  $lang: server failed to start"
        kill "$SERVER_PID" 2>/dev/null
        continue
    fi

    # Warmup
    wrk -t2 -c10 -d2s "http://localhost:$HTTP_PORT/" > /dev/null 2>&1

    # Benchmark
    wrk_out=$(wrk -t"$HTTP_THREADS" -c"$HTTP_CONNS" -d"${HTTP_DURATION}s" \
        --latency "http://localhost:$HTTP_PORT/" 2>&1)

    kill -9 "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true

    # Parse req/sec and latency from wrk output
    req_sec=$(echo "$wrk_out" | grep "Requests/sec:" | awk '{print $2}' | sed 's/k//')
    # wrk reports req/sec like "123456.78" or "12.34k" — normalize to integer
    req_sec_num=$(python3 -c "
s = '$req_sec'
if s.endswith('k') or s.endswith('K'):
    print(int(float(s[:-1]) * 1000))
else:
    print(int(float(s))) if s else print(0)
" 2>/dev/null)

    lat_avg=$(echo "$wrk_out" | grep "Latency" | head -1 | awk '{print $2}')
    lat_p99=$(echo "$wrk_out" | grep "99%" | awk '{print $2}')

    # Convert latency to ms
    to_ms() {
        python3 -c "
s = '$1'
if s.endswith('ms'): print(f'{float(s[:-2]):.2f}')
elif s.endswith('us'): print(f'{float(s[:-2])/1000:.3f}')
elif s.endswith('s') and not s.endswith('ms'): print(f'{float(s[:-1])*1000:.1f}')
else: print(s)
" 2>/dev/null
    }
    lat_avg_ms=$(to_ms "$lat_avg")
    lat_p99_ms=$(to_ms "$lat_p99")

    echo "http_throughput,$lang,$req_sec_num,$lat_avg_ms,$lat_p99_ms" >> "$HTTP_CSV"
    printf "  %-8s %8s req/sec  (avg: %s ms  p99: %s ms)\n" \
        "$lang" "$req_sec_num" "$lat_avg_ms" "$lat_p99_ms"

    sleep 0.5  # let port free up
done

echo ""
echo "HTTP results: $HTTP_CSV"
