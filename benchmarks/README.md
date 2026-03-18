# Sans Benchmark Results

Comparing Sans against Python, Go, Node.js, and Rust across 8 workloads.

All times in milliseconds (lower is better). HTTP in req/sec (higher is better).

## Compute Results

| Benchmark | Sans (ms) | Python (ms) | Go (ms) | Node.js (ms) | Rust (ms) |
|-----------|---:|---:|---:|---:|---:|
| fib | 49.4 | 1566.3 | 80.1 | 109.8 | 42.7 |
| loop_sum | 18.8 | 100.2 | 23.6 | 53.4 | 14.1 |
| array_ops | 17.6 | 54.0 | 25.6 | 58.7 | 14.4 |
| string_concat | 24.9 | 53.9 | 22.1 | 51.9 | 22.9 |
| json_roundtrip | 48.5 | 81.9 | 88.0 | 58.3 | 42.4 |
| concurrent | 19.0 | 81.5 | 24.1 | 83.2 | 14.0 |
| file_io | 34.4 | 41.5 | 23.4 | 52.1 | 13.9 |
| mixed | 17.2 | 50.8 | 23.9 | 51.8 | 15.0 |

## HTTP Throughput (req/sec, 8 threads, 100 connections, 10s)

| Language | Req/sec | Avg latency | p99 latency |
|----------|--------:|------------:|------------:|
| Sans | 45,044 | 2.13ms | 4.13ms |
| Python | 44,577 | 2.15ms | 4.18ms |
| Go | 41,107 | 2.38ms | 7.33ms |
| Node.js | 44,493 | 2.18ms | 5.06ms |
| Rust | 45,447 | 2.11ms | 4.10ms |

Sans matches Rust on HTTP throughput — all languages top out around 45k req/sec
on this machine, bound by connection/OS overhead rather than language runtime.

## Speedup vs Python (compute)

| Benchmark | Sans | Python | Go | Node.js | Rust |
|-----------|---:|---:|---:|---:|---:|
| fib | 31.7x | 1.0x | 19.6x | 14.3x | 36.7x |
| loop_sum | 5.3x | 1.0x | 4.2x | 1.9x | 7.1x |
| array_ops | 3.1x | 1.0x | 2.1x | 0.9x | 3.8x |
| string_concat | 2.2x | 1.0x | 2.4x | 1.0x | 2.4x |
| json_roundtrip | 1.7x | 1.0x | 0.9x | 1.4x | 1.9x |
| concurrent | 4.3x | 1.0x | 3.4x | 1.0x | 5.8x |
| file_io | 1.2x | 1.0x | 1.8x | 0.8x | 3.0x |
| mixed | 3.0x | 1.0x | 2.1x | 1.0x | 3.4x |

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
| concurrent | 4 worker threads each summing 1M integers via channels |
| file_io | Write 1000 lines (70KB) to disk, read back, report length |
| mixed | 10k array map+filter + JSON serialize/deserialize + file write/read |
| http_throughput | GET `/` → JSON response, 8 threads, 100 connections |
