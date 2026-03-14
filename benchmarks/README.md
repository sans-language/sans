# Sans Benchmark Results

Comparing Sans against Python, Go, Node.js, and Rust across 5 workloads.

All times in milliseconds (lower is better). Speedup relative to Python.

## Results

| Benchmark | Sans (ms) | Python (ms) | Go (ms) | Node.js (ms) | Rust (ms) |
|-----------|---:|---:|---:|---:|---:|
| fib | 49.4 | 1504.7 | 78.2 | 109.7 | 41.8 |
| loop_sum | 14.8 | 98.3 | 21.7 | 52.4 | 13.2 |
| array_ops | 14.9 | 47.4 | 26.2 | 57.3 | 13.8 |
| string_concat | 22.4 | 48.8 | 21.8 | 50.8 | 22.8 |
| json_roundtrip | 325.2 | 77.7 | 89.0 | 61.7 | 41.4 |

## Speedup vs Python

| Benchmark | Sans | Python | Go | Node.js | Rust |
|-----------|---:|---:|---:|---:|---:|
| fib | 30.4x | 1.0x | 19.3x | 13.7x | 36.0x |
| loop_sum | 6.6x | 1.0x | 4.5x | 1.9x | 7.5x |
| array_ops | 3.2x | 1.0x | 1.8x | 0.8x | 3.4x |
| string_concat | 2.2x | 1.0x | 2.2x | 1.0x | 2.1x |
| json_roundtrip | 0.2x | 1.0x | 0.9x | 1.3x | 1.9x |

## Details

### fib

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 49.4 | 2.5 | 45.2 | 54.5 |
| Python | 1504.7 | 32.1 | 1459.2 | 1550.1 |
| Go | 78.2 | 1.7 | 75.1 | 79.9 |
| Node.js | 109.7 | 3.2 | 103.5 | 115.2 |
| Rust | 41.8 | 3.5 | 37.0 | 49.2 |

### loop_sum

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 14.8 | 4.2 | 11.1 | 26.1 |
| Python | 98.3 | 4.9 | 90.5 | 109.8 |
| Go | 21.7 | 2.5 | 19.1 | 28.2 |
| Node.js | 52.4 | 3.5 | 46.5 | 58.7 |
| Rust | 13.2 | 3.1 | 9.9 | 18.4 |

### array_ops

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 14.9 | 2.7 | 10.3 | 18.8 |
| Python | 47.4 | 1.9 | 44.9 | 51.5 |
| Go | 26.2 | 2.9 | 23.1 | 31.8 |
| Node.js | 57.3 | 3.8 | 47.2 | 60.5 |
| Rust | 13.8 | 2.7 | 10.2 | 19.7 |

### string_concat

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 22.4 | 5.5 | 18.0 | 36.7 |
| Python | 48.8 | 2.2 | 45.8 | 52.7 |
| Go | 21.8 | 2.5 | 18.0 | 27.1 |
| Node.js | 50.8 | 2.8 | 45.5 | 55.0 |
| Rust | 22.8 | 3.5 | 18.1 | 29.4 |

### json_roundtrip

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 325.2 | 3.9 | 320.1 | 332.9 |
| Python | 77.7 | 2.1 | 74.4 | 80.1 |
| Go | 89.0 | 1.3 | 87.2 | 91.2 |
| Node.js | 61.7 | 3.7 | 56.6 | 69.5 |
| Rust | 41.4 | 3.7 | 37.7 | 51.5 |

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
