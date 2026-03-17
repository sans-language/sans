# Sans Benchmark Results

Comparing Sans against Python, Go, Node.js, and Rust across 5 workloads.

All times in milliseconds (lower is better). Speedup relative to Python.

## Results

| Benchmark | Sans (ms) | Python (ms) | Go (ms) | Node.js (ms) | Rust (ms) |
|-----------|---:|---:|---:|---:|---:|
| fib | 48.4 | 1561.3 | 78.8 | 107.8 | 42.2 |
| loop_sum | 17.9 | 96.8 | 24.6 | 53.6 | 14.4 |
| array_ops | 16.2 | 52.7 | 27.2 | 59.2 | 13.2 |
| string_concat | 24.6 | 52.0 | 24.3 | 51.7 | 22.8 |
| json_roundtrip | 47.1 | 84.1 | 87.7 | 59.7 | 42.9 |

## Speedup vs Python

| Benchmark | Sans | Python | Go | Node.js | Rust |
|-----------|---:|---:|---:|---:|---:|
| fib | 32.3x | 1.0x | 19.8x | 14.5x | 37.0x |
| loop_sum | 5.4x | 1.0x | 3.9x | 1.8x | 6.7x |
| array_ops | 3.3x | 1.0x | 1.9x | 0.9x | 4.0x |
| string_concat | 2.1x | 1.0x | 2.1x | 1.0x | 2.3x |
| json_roundtrip | 1.8x | 1.0x | 1.0x | 1.4x | 2.0x |

## Details

### fib

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 48.4 | 1.9 | 46.1 | 51.3 |
| Python | 1561.3 | 11.2 | 1542.2 | 1576.7 |
| Go | 78.8 | 2.4 | 74.0 | 83.6 |
| Node.js | 107.8 | 3.1 | 103.2 | 112.4 |
| Rust | 42.2 | 5.3 | 37.4 | 56.3 |

### loop_sum

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 17.9 | 4.5 | 13.0 | 29.6 |
| Python | 96.8 | 5.3 | 82.9 | 102.8 |
| Go | 24.6 | 2.9 | 21.5 | 32.0 |
| Node.js | 53.6 | 2.6 | 50.4 | 58.6 |
| Rust | 14.4 | 4.2 | 9.2 | 25.4 |

### array_ops

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 16.2 | 3.0 | 13.0 | 22.8 |
| Python | 52.7 | 2.7 | 49.0 | 57.6 |
| Go | 27.2 | 2.1 | 23.5 | 29.9 |
| Node.js | 59.2 | 2.0 | 55.2 | 62.6 |
| Rust | 13.2 | 2.7 | 8.2 | 19.4 |

### string_concat

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 24.6 | 5.7 | 19.5 | 38.9 |
| Python | 52.0 | 3.8 | 46.2 | 56.8 |
| Go | 24.3 | 3.0 | 20.9 | 32.7 |
| Node.js | 51.7 | 2.6 | 48.6 | 57.7 |
| Rust | 22.8 | 2.6 | 19.7 | 29.3 |

### json_roundtrip

| Language | Mean (ms) | Std Dev | Min | Max |
|----------|--------:|--------:|----:|----:|
| Sans | 47.1 | 3.6 | 42.1 | 54.2 |
| Python | 84.1 | 3.9 | 76.9 | 90.5 |
| Go | 87.7 | 3.3 | 84.1 | 96.7 |
| Node.js | 59.7 | 2.0 | 56.7 | 62.7 |
| Rust | 42.9 | 2.7 | 39.9 | 49.6 |

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
