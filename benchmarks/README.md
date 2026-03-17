# Sans Benchmark Results

Comparing Sans against Python, Go, Node.js, and Rust across 6 workloads.

All times in milliseconds (lower is better). HTTP in req/sec (higher is better).

## Compute Results

| Benchmark | Sans (ms) | Python (ms) | Go (ms) | Node.js (ms) | Rust (ms) |
|-----------|---:|---:|---:|---:|---:|
| fib | 49.0 | 1570.8 | 78.9 | 107.4 | 41.7 |
| loop_sum | 17.6 | 103.5 | 23.4 | 51.2 | 14.9 |
| array_ops | 16.8 | 51.2 | 26.7 | 56.6 | 12.9 |
| string_concat | 22.0 | 51.5 | 22.9 | 51.0 | 21.0 |
| json_roundtrip | 47.9 | 79.6 | 88.2 | 58.1 | 41.1 |

## HTTP Throughput (req/sec, 8 threads, 100 connections, 10s)

| Language | Req/sec | Avg latency | p99 latency |
|----------|--------:|------------:|------------:|
| Sans | 45,423 | 2.11ms | 4.08ms |
| Python | 44,699 | 2.15ms | 4.18ms |
| Go | 44,407 | 2.15ms | 4.18ms |
| Node.js | 45,049 | 2.13ms | 4.13ms |
| Rust | 45,102 | 2.13ms | 4.12ms |

Sans matches Rust and Go on HTTP throughput — all languages top out around 45k req/sec
on this machine, bound by connection/OS overhead rather than language runtime.

## Speedup vs Python (compute)

| Benchmark | Sans | Python | Go | Node.js | Rust |
|-----------|---:|---:|---:|---:|---:|
| fib | 32.1x | 1.0x | 19.9x | 14.6x | 37.7x |
| loop_sum | 5.9x | 1.0x | 4.4x | 2.0x | 7.0x |
| array_ops | 3.0x | 1.0x | 1.9x | 0.9x | 4.0x |
| string_concat | 2.3x | 1.0x | 2.3x | 1.0x | 2.5x |
| json_roundtrip | 1.7x | 1.0x | 0.9x | 1.4x | 1.9x |

## Methodology

- Compute: each benchmark runs 10 times (1 warmup + 10 timed), wall-clock via `time.perf_counter()`
- HTTP: `wrk -t8 -c100 -d10s` with 2s warmup, endpoint returns `{"message":"hello","n":42}`
- Sans, Go, and Rust are compiled ahead of time with optimizations
- Python uses CPython, Node.js uses V8

## Workloads

| Benchmark | Description |
|-----------|-------------|
| fib | Recursive fibonacci(35), no memoization |
| loop_sum | Sum integers 1 to 1,000,000 |
| array_ops | Build 100k array, map (*2), filter (even), sum |
| string_concat | Concatenate 5 strings per iteration, 100k iterations |
| json_roundtrip | Build/stringify/parse JSON with 1k keys, 100 iterations |
| http_throughput | GET `/` → JSON response, 8 threads, 100 connections |
