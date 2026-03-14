# Benchmark Suite Design

**Goal:** Create a benchmark suite comparing Sans performance against Python, Go, Node.js, and Rust across 5 workloads, with a CLI runner and auto-generated markdown report.

## Benchmarks

| Name | Workload | What It Tests |
|------|----------|---------------|
| fib | `fib(35)` recursive, no memoization | Function call overhead, recursion |
| loop_sum | Sum 1 to 1,000,000 in while loop | Tight loop, integer arithmetic |
| array_ops | Build 100k array, map (*2), filter (even), sum | Array alloc, iteration, HOF |
| string_concat | Concatenate "hello" 100k times | String allocation/copying |
| json_roundtrip | Build JSON with 1k keys, stringify, parse, extract — 100 iterations | Serialization overhead |

## Directory Structure

```
benchmarks/
├── bench.sh              # orchestrator
├── report.py             # markdown generation
├── README.md             # auto-generated results
├── sans/                 # 5 .sans files
├── python/               # 5 .py files
├── go/                   # 5 .go files
├── node/                 # 5 .js files
└── rust/                 # 5 .rs files
```

## Runner (bench.sh)

- Compiles Sans, Go, Rust ahead of time
- Runs each benchmark 10 times per language
- Uses `hyperfine` if available, else raw `time` with millisecond precision
- Outputs CSV: `benchmark,language,mean_ms,stddev_ms,min_ms,max_ms`
- Prints summary table to terminal

## Report (report.py)

- Reads CSV from runner
- Generates markdown table normalized to Python as 1.0x baseline
- Generates bar chart PNGs per benchmark if matplotlib available
- Writes `benchmarks/README.md`

## Constraints

- Each program prints its result value for correctness verification
- No I/O in the timed portion (print once at end)
- Idiomatic code per language — no artificial handicapping
- Python as 1.0x baseline (slowest, makes speedups read well)
