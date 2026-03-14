# Benchmark Suite Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a benchmark suite comparing Sans vs Python/Go/Node/Rust across 5 workloads with CLI runner and markdown report generator.

**Architecture:** Shell script orchestrator compiles and runs benchmarks, collects CSV timing data. Python script reads CSV and generates markdown report with tables. Each language gets its own directory with 5 benchmark programs.

**Tech Stack:** Bash, Python 3, Go, Node.js, Rust, Sans

---

## Chunk 1: Benchmark Programs

### Task 1: Sans benchmarks

**Files:**
- Create: `benchmarks/sans/fib.sans`
- Create: `benchmarks/sans/loop_sum.sans`
- Create: `benchmarks/sans/array_ops.sans`
- Create: `benchmarks/sans/string_concat.sans`
- Create: `benchmarks/sans/json_roundtrip.sans`

- [ ] **Step 1: Write fib.sans**

```sans
fib(n:I) I {
  n <= 1 ? n : fib(n - 1) + fib(n - 2)
}
p(fib(35))
```

- [ ] **Step 2: Write loop_sum.sans**

```sans
i := 0
sum := 0
while i < 1000000 {
  i += 1
  sum += i
}
p(sum)
```

- [ ] **Step 3: Write array_ops.sans**

```sans
double(x:I) I = x * 2
is_even(x:I) B = x % 2 == 0

a := [I]
i := 0
while i < 100000 {
  a.push(i)
  i += 1
}
b = a.map(double)
c = b.filter(is_even)
sum := 0
j := 0
while j < c.len() {
  sum += c[j]
  j += 1
}
p(sum)
```

- [ ] **Step 4: Write string_concat.sans**

```sans
s := ""
i := 0
while i < 100000 {
  s = s + "hello"
  i += 1
}
p(s.len())
```

- [ ] **Step 5: Write json_roundtrip.sans**

```sans
i := 0
result := 0
while i < 100 {
  j = jo()
  k := 0
  while k < 1000 {
    j.set(str(k), k)
    k += 1
  }
  s = json_stringify(j)
  parsed = json_parse(s)
  result = parsed.get("999").get_int()
  i += 1
}
p(result)
```

- [ ] **Step 6: Verify each Sans benchmark compiles and runs**

```bash
cd benchmarks
for f in sans/*.sans; do sans run "$f"; done
```

- [ ] **Step 7: Commit**

```bash
git add benchmarks/sans/
git commit -m "bench: add Sans benchmark programs"
```

### Task 2: Python benchmarks

**Files:**
- Create: `benchmarks/python/fib.py`
- Create: `benchmarks/python/loop_sum.py`
- Create: `benchmarks/python/array_ops.py`
- Create: `benchmarks/python/string_concat.py`
- Create: `benchmarks/python/json_roundtrip.py`

- [ ] **Step 1: Write all 5 Python benchmarks**

fib.py:
```python
import sys
sys.setrecursionlimit(100000)
def fib(n):
    return n if n <= 1 else fib(n - 1) + fib(n - 2)
print(fib(35))
```

loop_sum.py:
```python
s = 0
for i in range(1, 1000001):
    s += i
print(s)
```

array_ops.py:
```python
a = list(range(100000))
b = list(map(lambda x: x * 2, a))
c = list(filter(lambda x: x % 2 == 0, b))
print(sum(c))
```

string_concat.py:
```python
s = ""
for _ in range(100000):
    s += "hello"
print(len(s))
```

json_roundtrip.py:
```python
import json
result = 0
for _ in range(100):
    obj = {str(k): k for k in range(1000)}
    s = json.dumps(obj)
    parsed = json.loads(s)
    result = parsed["999"]
print(result)
```

- [ ] **Step 2: Verify correctness**

```bash
for f in python/*.py; do python3 "$f"; done
```

- [ ] **Step 3: Commit**

```bash
git add benchmarks/python/
git commit -m "bench: add Python benchmark programs"
```

### Task 3: Go benchmarks

**Files:**
- Create: `benchmarks/go/fib.go`
- Create: `benchmarks/go/loop_sum.go`
- Create: `benchmarks/go/array_ops.go`
- Create: `benchmarks/go/string_concat.go`
- Create: `benchmarks/go/json_roundtrip.go`

- [ ] **Step 1: Write all 5 Go benchmarks**

Each file is a standalone `package main` with `fmt` import. Idiomatic Go: slices, `encoding/json`, `strings.Builder` for concat.

- [ ] **Step 2: Verify each compiles and runs**

```bash
for f in go/*.go; do go run "$f"; done
```

- [ ] **Step 3: Commit**

```bash
git add benchmarks/go/
git commit -m "bench: add Go benchmark programs"
```

### Task 4: Node.js benchmarks

**Files:**
- Create: `benchmarks/node/fib.js`
- Create: `benchmarks/node/loop_sum.js`
- Create: `benchmarks/node/array_ops.js`
- Create: `benchmarks/node/string_concat.js`
- Create: `benchmarks/node/json_roundtrip.js`

- [ ] **Step 1: Write all 5 Node benchmarks**

Idiomatic JS: `Array.from`, `.map`, `.filter`, `.reduce`, `JSON.stringify`/`JSON.parse`.

- [ ] **Step 2: Verify correctness**

```bash
for f in node/*.js; do node "$f"; done
```

- [ ] **Step 3: Commit**

```bash
git add benchmarks/node/
git commit -m "bench: add Node.js benchmark programs"
```

### Task 5: Rust benchmarks

**Files:**
- Create: `benchmarks/rust/fib.rs`
- Create: `benchmarks/rust/loop_sum.rs`
- Create: `benchmarks/rust/array_ops.rs`
- Create: `benchmarks/rust/string_concat.rs`
- Create: `benchmarks/rust/json_roundtrip.rs`

- [ ] **Step 1: Write all 5 Rust benchmarks**

Standalone single-file programs. Use `serde_json` for json_roundtrip — but since these are standalone .rs files compiled with `rustc`, json_roundtrip will use manual string building/parsing or we skip external crates and use a simple approach. Actually: use a naive JSON approach (build string manually, parse character by character) to keep it dependency-free, OR note that Rust json benchmark requires `cargo`. Decision: use a minimal `Cargo.toml` in `benchmarks/rust/` with `serde_json` dependency for the json benchmark only.

- [ ] **Step 2: Verify each compiles and runs**

```bash
rustc -O fib.rs -o fib && ./fib
# For json_roundtrip: cd rust && cargo run --release --bin json_roundtrip
```

- [ ] **Step 3: Commit**

```bash
git add benchmarks/rust/
git commit -m "bench: add Rust benchmark programs"
```

## Chunk 2: Runner and Report

### Task 6: bench.sh orchestrator

**Files:**
- Create: `benchmarks/bench.sh`

- [ ] **Step 1: Write bench.sh**

Script structure:
1. Parse args (optional: specific benchmark, specific language, number of runs)
2. Detect available languages
3. Compile phase: `sans build`, `go build`, `rustc -O` (or `cargo build --release`)
4. Run phase: for each benchmark × language, run N times, capture wall-clock time via `date +%s%N` or `gtime -f %e` (macOS needs `gtime` from coreutils or python timing wrapper)
5. Output CSV to `results.csv`
6. Print summary table to terminal

Timing approach for macOS (no `date +%s%N`): use a small python one-liner wrapper:
```bash
python3 -c "import time,subprocess,sys; t=time.perf_counter(); subprocess.run(sys.argv[1:], stdout=subprocess.DEVNULL); print(f'{(time.perf_counter()-t)*1000:.2f}')" ./binary
```

- [ ] **Step 2: Make executable and test with one benchmark**

```bash
chmod +x bench.sh
./bench.sh
```

- [ ] **Step 3: Commit**

```bash
git add benchmarks/bench.sh
git commit -m "bench: add benchmark runner script"
```

### Task 7: report.py generator

**Files:**
- Create: `benchmarks/report.py`

- [ ] **Step 1: Write report.py**

- Read `results.csv`
- Calculate speedup relative to Python (Python = 1.0x)
- Generate markdown table
- If matplotlib available: generate bar chart PNGs in `benchmarks/charts/`
- Write `benchmarks/README.md`

- [ ] **Step 2: Test with sample CSV data**

- [ ] **Step 3: Commit**

```bash
git add benchmarks/report.py
git commit -m "bench: add report generator"
```

### Task 8: Integration test and first run

- [ ] **Step 1: Run full suite**

```bash
cd benchmarks && ./bench.sh
```

- [ ] **Step 2: Generate report**

```bash
python3 report.py
```

- [ ] **Step 3: Review README.md output**

- [ ] **Step 4: Add .gitignore for compiled artifacts**

```
benchmarks/build/
benchmarks/charts/
benchmarks/results.csv
benchmarks/rust/target/
```

- [ ] **Step 5: Final commit**

```bash
git add benchmarks/
git commit -m "bench: complete benchmark suite with initial results"
```
