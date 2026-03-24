# Memory Safety Audit — v0.8.1

## Scope GC

### Fixed: JSON Reference Walk (Critical)
`scope_should_keep` in `runtime/rc.sans` did not walk JSON values (tag 3). Returning a JSON object from a function inside a Result caused use-after-free — the JSON's internal hash table metadata was freed while the outer node survived.

**Fix:** Added JSON inner type walking (string, array, object) to `scope_should_keep`.

### Fixed: Result<JSON> Unwrap Type Loss (Critical)
`IR_RESULT_UNWRAP` always set the inner type to `IRTY_INT`, causing method dispatch to use Map operations instead of JSON operations on unwrapped JSON values. This caused segfaults when calling `.keys()`, `.get()`, etc. on JSON values extracted from Results.

**Fix:** Added inner type propagation through struct_name annotations on Result registers. Unwrap now correctly resolves `R<J>` → `IRTY_JSON`.

### Known Limitation: Thread Safety
`rc_alloc_head`, `rc_scope_head`, `rc_kept_head` are global variables with no synchronization. Concurrent scope_enter/scope_exit from multiple threads will corrupt the linked lists. Documented in CLAUDE.md.

**Recommendation:** Thread-local storage for scope GC heads (deferred to future release).

## JSON Parser

### Fixed: No Recursion Depth Limit (High)
`sans_json_p_value` recursed through objects/arrays with no depth counter. Deeply nested JSON caused stack overflow.

**Fix:** Added depth counter at parser context offset +40. Max depth 512. Returns error Result on overflow.

### Fixed: Parse Failure Returns Null (High)
`sans_json_parse` returned `null` on failure, indistinguishable from literal `null` in input. Breaking change.

**Fix:** Now returns `Result<JsonValue>`. Errors include descriptive messages.

## Extern Function Boundaries

### LibC (Low Risk)
- `malloc`/`free` balance: All `alloc()` calls in runtime use `malloc`, freed via `dealloc()` (which calls `free`). Scope GC tracks and frees allocations. No detected leaks in normal paths.
- `snprintf` in `itos()` codegen: Buffer is 21 bytes for i64 — correct (max 20 digits + sign).
- `strlen`/`memcpy`: Used correctly in string operations with computed lengths.

### LibCurl (Medium Risk)
- `curl_easy_init`/`curl_easy_cleanup` are paired in `IR_CINIT`/`IR_CCLEAN`. User code must call cleanup manually.
- `curl_slist_append`/`curl_slist_free_all` are paired in `IR_CURL_SLIST_APPEND`/`IR_CURL_SLIST_FREE`. User code must free manually.
- **No automatic cleanup on error paths.** If `curl_easy_perform` fails, user code may skip cleanup.

### OpenSSL (Medium Risk)
- `SSL_new`/`SSL_free` are paired in IR ops. Same manual cleanup requirement.
- No certificate verification configuration — TLS server only, client verification not exposed.

### Zlib (Low Risk)
- `deflateInit2`/`deflateEnd` are paired in HTTP compression code. Buffer sizes computed from `deflateBound`.

## HTTP Server Buffers

### Single-Read Request Assembly (Medium)
`runtime/server.sans:sans_http_accept_read` uses a single `recv(cfd, buf, 8192)` call. If the request is larger than 8KB or arrives in multiple TCP segments, it will be truncated. No Content-Length–based multi-read.

**Recommendation:** Fix in v0.8.2 (Server Production-Ready).

### No Request Size Limits (Medium)
No maximum body size, header count, or URL length limits. A malicious client can send arbitrarily large requests.

**Recommendation:** Fix in v0.8.2.

## Summary

| Finding | Severity | Status |
|---------|----------|--------|
| scope_should_keep missing JSON walk | Critical | Fixed |
| Result unwrap loses inner IRTY | Critical | Fixed |
| JSON parser no depth limit | High | Fixed |
| json_parse returns null on error | High | Fixed |
| Scope GC not thread-safe | Medium | Known limitation |
| Curl/SSL no automatic cleanup on error | Medium | Document |
| HTTP single-read request assembly | Medium | Deferred to v0.8.2 |
| No request size limits | Medium | Deferred to v0.8.2 |
