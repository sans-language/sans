# Sans Benchmark Results

Comparing Sans against Python, Go, Node.js, and Rust across 5 workloads.

All times in milliseconds (lower is better). Speedup relative to Python.

## Results

| Benchmark | Sans (ms) | Python (ms) | Go (ms) | Node.js (ms) | Rust (ms) |
|-----------|---:|---:|---:|---:|---:|
| fib | 51.5 | 1547.3 | 78.2 | 108.9 | 42.5 |
| loop_sum | 16.4 | 96.3 | 22.1 | 52.7 | 12.3 |
| array_ops | 15.0 | 48.6 | 25.1 | 53.8 | 12.8 |
| string_concat | 21.8 | 47.4 | 22.2 | 50.2 | 22.0 |
| json_roundtrip | 129.8 | 80.5 | 86.1 | 58.5 | 42.1 |

## Speedup vs Python

| Benchmark | Sans | Python | Go | Node.js | Rust |
|-----------|---:|---:|---:|---:|---:|
| fib | 30.0x | 1.0x | 19.8x | 14.2x | 36.4x |
| loop_sum | 5.9x | 1.0x | 4.4x | 1.8x | 7.8x |
| array_ops | 3.2x | 1.0x | 1.9x | 0.9x | 3.8x |
| string_concat | 2.2x | 1.0x | 2.1x | 0.9x | 2.2x |
| json_roundtrip | 0.6x | 1.0x | 0.9x | 1.4x | 1.9x |

## Details

### fib

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 51.5 | 2.6 | 47.3 | 55.5 |
| Python | 1547.3 | 19.7 | 1525.5 | 1579.7 |
| Go | 78.2 | 3.0 | 71.4 | 82.5 |
| Node.js | 108.9 | 2.7 | 103.8 | 112.0 |
| Rust | 42.5 | 2.0 | 40.0 | 47.0 |

### loop_sum

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 16.4 | 4.1 | 13.7 | 27.6 |
| Python | 96.3 | 3.0 | 92.9 | 102.0 |
| Go | 22.1 | 3.0 | 19.4 | 30.0 |
| Node.js | 52.7 | 1.5 | 50.3 | 56.0 |
| Rust | 12.3 | 3.5 | 9.5 | 22.1 |

### array_ops

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 15.0 | 4.8 | 11.3 | 27.8 |
| Python | 48.6 | 1.7 | 46.2 | 51.0 |
| Go | 25.1 | 1.9 | 21.3 | 28.0 |
| Node.js | 53.8 | 4.8 | 49.0 | 61.1 |
| Rust | 12.8 | 4.0 | 9.7 | 23.9 |

### string_concat

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 21.8 | 3.7 | 17.9 | 30.0 |
| Python | 47.4 | 2.5 | 42.7 | 51.0 |
| Go | 22.2 | 3.4 | 19.2 | 31.1 |
| Node.js | 50.2 | 3.1 | 44.5 | 54.3 |
| Rust | 22.0 | 4.3 | 16.9 | 33.1 |

### json_roundtrip

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 129.8 | 19.4 | 79.0 | 145.6 |
| Python | 80.5 | 2.0 | 77.1 | 83.1 |
| Go | 86.1 | 1.7 | 83.2 | 88.9 |
| Node.js | 58.5 | 1.9 | 55.4 | 62.8 |
| Rust | 42.1 | 2.6 | 39.1 | 48.9 |

## Methodology

- Each benchmark runs 10 times (1 warmup + 10 timed)
- Wall-clock time measured via Python `time.perf_counter()`
- Sans, Go, and Rust are compiled ahead of time with optimizations
- Python uses CPython, Node.js uses V8
- All programs produce identical output for correctness verification

## Workloads

| Benchmark | Description |
|-----------|-------------|
| fib | Recursive fibonacci(35), no memoization |
| loop_sum | Sum integers 1 to 1,000,000 |
| array_ops | Build 100k array, map (*2), filter (even), sum |
| string_concat | Concatenate 5 strings per iteration, 100k iterations |
| json_roundtrip | Build/stringify/parse JSON with 1k keys, 100 iterations |
