# Sans Security Best Practices

A practical guide for writing secure Sans programs. See [SECURITY.md](../SECURITY.md) for the vulnerability disclosure policy.

---

## Input Validation

Always validate user input before processing. Never assume request bodies, query parameters, or headers are well-formed.

**Use Result types for parsing — never unwrap user input directly:**

```sans
handle(req:HR) I {
  body = req.body()
  j = json_parse(body)?   // propagate parse error, don't crash
  name = j.get("name").unwrap_or("anonymous")
  req.respond(200 "Hello " + name)
}
```

**JSON depth limit is 512 — safe by default.** Deeply nested payloads return an error from `json_parse()` rather than stack-overflowing.

**Set explicit request size limits** to prevent memory exhaustion:

```sans
main() I {
  set_max_body(65536)       // 64 KB body limit
  set_max_headers(8192)     // 8 KB header block limit
  set_max_header_count(50)  // max 50 individual headers
  set_max_url(2048)         // 2 KB URL limit
  serve(8080 handler)
}
```

---

## HTTP Server Hardening

`serve()` includes a bounded thread pool, connection timeouts, and keep-alive by default. Tune these for your workload:

```sans
main() I {
  set_max_workers(32)         // cap concurrent threads (default: 16)
  set_read_timeout(30)        // 30-second read timeout (default: 60)
  set_max_body(1048576)       // 1 MB body limit
  serve(8080 handler)
}
```

**CORS** — allow specific origins or use wildcard for public APIs:

```sans
handler(req:HR) I {
  cors(req "https://myapp.example.com")   // specific origin
  // or: cors_all(req)                    // wildcard (public API)
  req.respond(200 "OK")
}
```

**TLS** — use `serve_tls()` for HTTPS in production:

```sans
main() I {
  serve_tls(443 "cert.pem" "key.pem" handler)
}
```

Generate a self-signed cert for development with `./examples/gen_cert.sh`.

---

## Error Handling

**Never use `!` (unwrap) on user-controlled data in server handlers.** Panic recovery catches per-request crashes and returns HTTP 500, but relying on it is a code smell — it hides bugs and leaks no useful error to the client.

```sans
// BAD — panics if body is not valid JSON or field is missing
handler(req:HR) I {
  j = json_parse(req.body())!
  id = j.get("id")!
  req.respond(200 id)
}

// GOOD — explicit error handling
handler(req:HR) I {
  r = json_parse(req.body())
  if r.is_err() { return req.respond(400 "invalid JSON") }
  j = r!
  id = j.get("id").unwrap_or("")
  slen(id) == 0 ? req.respond(400 "missing id") : req.respond(200 id)
}
```

Use `?` for propagation in helper functions that return `R<T>`:

```sans
parse_user(body:S) R<J> {
  j = json_parse(body)?
  j
}
```

---

## Memory Safety

Sans uses scope-based GC: all allocations made in a function are freed when it returns, and return values are promoted to the caller's scope.

- **Per-request isolation** — each request handler runs in its own thread with its own scope. A memory leak in one handler does not accumulate across requests.
- **Known limitation** — `rc_alloc_head` / `rc_scope_head` globals are not thread-safe. Do not share mutable global state across request handlers without external synchronization.
- Avoid large global mutable variables that grow unboundedly across requests.

---

## Shell Injection

`sh()` and `system()` pass strings directly to the shell. **Never pass user input to these functions without sanitization** — there is no built-in escaping.

```sans
// DANGEROUS — user controls filename
filename = req.body()
result = sh("cat " + filename)   // attacker sends "; rm -rf /"

// SAFER — validate before use
filename = req.body()
if scontains(filename "/") || scontains(filename "..") {
  return req.respond(400 "invalid filename")
}
result = sh("cat /data/" + filename)
```

When possible, avoid `sh()`/`system()` entirely with untrusted input. Prefer built-in file I/O functions (`fr()`, `fw()`) with explicit path validation.

Note: `mkdir()`, `is_dir()`, and `listdir()` in the runtime single-quote paths internally to reduce injection risk, but `sh()` itself provides no such protection.

---

## File I/O

Sans has no built-in path traversal prevention. Validate paths before passing them to `fr()`, `fw()`, or `fe()`.

```sans
safe_read(user_path:S) S {
  // Reject paths containing traversal sequences
  if scontains(user_path "..") { return "" }
  if scontains(user_path "//") { return "" }
  // Restrict to a known base directory
  full = "/var/app/data/" + user_path
  fe(full) ? fr(full) : ""
}
```

Do not construct file paths from user input without these checks.

---

## Package Manager

`sans pkg` downloads packages directly from git repositories. There is **no integrity verification or lockfile** in the current version. Until supply chain hardening ships (planned post-1.0):

- Review the source of every dependency before adding it.
- Pin to a specific git commit hash rather than a branch name where possible.
- Audit `sans_modules/` in your project for unexpected changes after updates.

---

## Summary Checklist

- [ ] Set request size limits (`set_max_body`, `set_max_headers`, `set_max_url`)
- [ ] Never `!`-unwrap user-controlled data in handlers
- [ ] Use `?` or `is_err()` for all parsing paths
- [ ] Validate file paths before `fr()`/`fw()`/`fe()`
- [ ] Never pass unsanitized user input to `sh()`/`system()`
- [ ] Use `serve_tls()` with valid certificates in production
- [ ] Set `set_max_workers()` and `set_read_timeout()` for your workload
- [ ] Review all `sans pkg` dependencies manually
