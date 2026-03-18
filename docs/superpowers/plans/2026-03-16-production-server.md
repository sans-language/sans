# Production Server Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `serve(port, handler)` with auto-threading, HTTP/1.1 keep-alive, and chunked streaming to the Sans HTTP server — all purely in Sans.

**Architecture:** `serve()` accepts connections in a loop and spawns a thread per connection. Each thread runs a keep-alive loop: parse request → call handler → check Connection header → read next request or close. `respond_stream()` sends chunked Transfer-Encoding headers and returns a writer for incremental output.

**Tech Stack:** Sans runtime (`runtime/server.sans`), self-hosted compiler (`compiler/*.sans`), existing `spawn`/`fcall`/socket primitives.

**Spec:** `docs/superpowers/specs/2026-03-16-production-server-design.md`

---

## Task 1: Update `respond()` to support keep-alive (don't always close)

Currently `sans_http_respond_plain` and the SSL variant always close the connection after sending. We need to make closing conditional — only close when explicitly requested or when not in a keep-alive context.

**Files:**
- Modify: `runtime/server.sans`

- [ ] **Step 1: Add a "should_close" flag to the request struct**

At offset 56 (extending struct from 56 to 64 bytes), add a `keep_alive` flag. When 0 (default), respond closes the connection as before. When 1, respond does NOT close — the keep-alive loop handles it.

Update `sans_make_empty_request` to allocate 64 bytes.
Update `sans_http_accept_parse` to allocate 64 bytes with offset 56 = 0.
Update `sans_https_accept_parse` in `runtime/ssl.sans` similarly.

- [ ] **Step 2: Change `sans_http_respond_plain` to check keep-alive flag**

```sans
sans_http_respond_plain(req:I fd:I resp:I total:I) I {
  sans_send_all(fd, resp, total)
  ka = load64(req + 56)
  ka == 0 ? { sclose(fd); store64(req, -1) } : 0
  1
}
```

Do the same for the SSL respond path — check offset 56 before calling `ssl_close`/`sclose`.

- [ ] **Step 3: Change Connection header from "close" to "keep-alive"**

In `sans_lit_conn_close`, rename to `sans_lit_conn_header` and check the request's keep-alive flag. If keep-alive: send `Connection: keep-alive\r\n\r\n`. If not: send `Connection: close\r\n\r\n`.

Actually, simpler: add a new function `sans_lit_conn_keepalive()` that returns `"\r\nConnection: keep-alive\r\n\r\n"`. Then in `sans_http_respond_build`, choose which to use based on offset 56.

- [ ] **Step 4: Add `sans_check_close(req)` helper**

Returns 1 if the connection should close after this response:
- If client sent `Connection: close` header → 1
- If user set `Connection: close` via `set_header` → 1
- If HTTP/1.0 (check first line) → 1 (but we can skip this for now)
- Otherwise → 0

```sans
sans_check_close(req:I) I {
  headers = load64(req + 32)
  headers == 0 ? 0 : {
    conn = sans_http_request_header(req, "connection")
    slen(conn) > 0 ? conn == "close" ? 1 : 0 : 0
  }
}
```

- [ ] **Step 5: Verify existing tests still pass**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --test e2e 2>&1 | tail -3
```

- [ ] **Step 6: Commit**

```bash
git add runtime/server.sans runtime/ssl.sans && git commit -m "feat: keep-alive support in HTTP respond (conditional close)"
```

---

## Task 2: Implement `serve()` and `serve_tls()` with keep-alive loop

**Files:**
- Modify: `runtime/server.sans`
- Modify: `compiler/typeck.sans`
- Modify: `compiler/ir.sans`
- Modify: `compiler/codegen.sans`
- Modify: `compiler/constants.sans`

- [ ] **Step 1: Add `sans_serve` runtime function**

```sans
sans_serve(port:I handler:I) I {
  server = sans_http_listen(port)
  load64(server) < 0 ? { p("serve: failed to listen"); exit(1); 0 } : 0
  while 1 {
    req = sans_http_accept(server)
    fd = load64(req)
    fd >= 0 ? spawn sans_serve_worker(handler, req, server) : 0
    0
  }
  0
}
```

- [ ] **Step 2: Add `sans_serve_worker` — keep-alive loop**

```sans
sans_serve_worker(handler:I req:I server:I) I {
  fd = load64(req)
  ssl = load64(req + 48)
  // Mark as keep-alive mode
  store64(req + 56, 1)
  current_req := req
  done := 0
  while done == 0 {
    // Call user handler
    fcall(handler, current_req)
    // Check if should close
    if sans_check_close(current_req) {
      done = 1
    } else {
      // Read next request on same connection
      next = sans_read_next_request(fd, ssl)
      next_fd = load64(next)
      if next_fd < 0 {
        done = 1
      } else {
        current_req = next
        store64(current_req + 56, 1)  // keep-alive mode
      }
    }
    0
  }
  // Close connection
  ssl != 0 ? ssl_close(ssl) : 0
  sclose(fd)
  0
}
```

- [ ] **Step 3: Add `sans_read_next_request` — parse on existing fd**

```sans
sans_read_next_request(fd:I ssl:I) I {
  buf = alloc(65536)
  n = ssl != 0 ? ssl_read(ssl, buf, 65535) : srecv(fd, buf, 65535)
  n <= 0 ? sans_make_empty_request() : sans_parse_existing_conn(fd, ssl, buf, n)
}

sans_parse_existing_conn(fd:I ssl:I buf:I n:I) I {
  store8(buf + n, 0)
  mend = sans_find_space(buf, n, 0)
  mlen = mend
  pstart = mend < n ? mend + 1 : mend
  pend = sans_find_space(buf, n, pstart)
  plen = pend - pstart
  method = alloc(mlen + 1)
  mlen > 0 ? mcpy(method, buf, mlen) : 0
  store8(method + mlen, 0)
  path = alloc(plen + 1)
  plen > 0 ? mcpy(path, buf + pstart, plen) : 0
  store8(path + plen, 0)
  bstart = sans_find_body(buf, n)
  body = sans_extract_body(buf, n, bstart)
  req = alloc(64)
  store64(req, fd)
  store64(req + 8, method)
  store64(req + 16, path)
  store64(req + 24, body)
  store64(req + 32, 0)
  store64(req + 40, 0)
  store64(req + 48, ssl)
  store64(req + 56, 0)
  sans_parse_headers(req, buf, n)
  req
}
```

- [ ] **Step 4: Add `sans_serve_tls` runtime function**

```sans
sans_serve_tls(port:I cert:S key:S handler:I) I {
  server = sans_https_listen(port, cert, key)
  load64(server) < 0 ? { p("serve_tls: failed"); exit(1); 0 } : 0
  while 1 {
    req = sans_https_accept(server)
    fd = load64(req)
    fd >= 0 ? spawn sans_serve_worker(handler, req, server) : 0
    0
  }
  0
}
```

- [ ] **Step 5: Add type checking in `compiler/typeck.sans`**

Add `serve` (2 args → Int) and `serve_tls` (4 args → Int) to the built-in function chain.

- [ ] **Step 6: Add IR lowering in `compiler/ir.sans`**

Lower `serve` → `IR_CALL` to `sans_serve`. Lower `serve_tls` → `IR_CALL` to `sans_serve_tls`.

- [ ] **Step 7: Add extern declarations in `compiler/codegen.sans`**

Add `sans_serve` and `sans_serve_tls` to `emit_externals`.

- [ ] **Step 8: Verify tests pass, commit**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --test e2e 2>&1 | tail -3
git add runtime/server.sans compiler/*.sans && git commit -m "feat: serve() and serve_tls() with auto-threading and keep-alive"
```

---

## Task 3: Implement chunked streaming (`respond_stream`, `stream_write`, `stream_end`)

**Files:**
- Modify: `runtime/server.sans`
- Modify: `compiler/typeck.sans`
- Modify: `compiler/ir.sans`
- Modify: `compiler/codegen.sans`
- Modify: `compiler/constants.sans`

- [ ] **Step 1: Add `sans_int_to_hex(n)` helper**

Converts an integer to a lowercase hex string (for chunk size header).

```sans
sans_int_to_hex(n:I) I {
  n == 0 ? { buf = alloc(2); store8(buf, 48); store8(buf + 1, 0); buf } : {
    // Count hex digits
    digits := 0
    tmp := n
    while tmp > 0 { digits += 1; tmp = tmp / 16; 0 }
    buf = alloc(digits + 1)
    store8(buf + digits, 0)
    i := digits - 1
    tmp = n
    while tmp > 0 {
      rem = tmp % 16
      ch = rem < 10 ? 48 + rem : 87 + rem  // 0-9 or a-f
      store8(buf + i, ch)
      tmp = tmp / 16
      i -= 1
      0
    }
    buf
  }
}
```

- [ ] **Step 2: Add `sans_respond_stream(req, status)` function**

Sends HTTP headers with `Transfer-Encoding: chunked`, returns a stream writer struct.

```sans
sans_respond_stream(req:I status:I) I {
  fd = load64(req)
  ssl = load64(req + 48)
  // Build response header
  status_str = sans_int_to_buf(status)
  st = sans_status_text(status)
  // Build header: "HTTP/1.1 {status} {text}\r\nTransfer-Encoding: chunked\r\n{custom headers}\r\n"
  header = "HTTP/1.1 " + status_str + " " + st + "\r\nTransfer-Encoding: chunked"
  // Add custom response headers
  rh = load64(req + 40)
  rh_str = rh != 0 ? sans_build_resp_headers_str(rh) : ""
  header = header + rh_str + "\r\n\r\n"
  // Send header
  hlen = slen(header)
  ssl != 0 ? ssl_write(ssl, header, hlen) : ssend(fd, header, hlen)
  // Create writer struct [fd, ssl, req]
  w = alloc(24)
  store64(w, fd)
  store64(w + 8, ssl)
  store64(w + 16, req)
  w
}
```

- [ ] **Step 3: Add `sans_stream_write(w, data)` function**

```sans
sans_stream_write(w:I data:I) I {
  fd = load64(w)
  ssl = load64(w + 8)
  dlen = slen(data)
  hex = sans_int_to_hex(dlen)
  hlen = slen(hex)
  // Send: hex_len \r\n data \r\n
  chunk_header = hex + "\r\n"
  chlen = slen(chunk_header)
  ssl != 0 ? ssl_write(ssl, chunk_header, chlen) : ssend(fd, chunk_header, chlen)
  ssl != 0 ? ssl_write(ssl, data, dlen) : ssend(fd, data, dlen)
  crlf = "\r\n"
  ssl != 0 ? ssl_write(ssl, crlf, 2) : ssend(fd, crlf, 2)
  dlen
}
```

- [ ] **Step 4: Add `sans_stream_end(w)` function**

```sans
sans_stream_end(w:I) I {
  fd = load64(w)
  ssl = load64(w + 8)
  final_chunk = "0\r\n\r\n"
  ssl != 0 ? ssl_write(ssl, final_chunk, 5) : ssend(fd, final_chunk, 5)
  0
}
```

- [ ] **Step 5: Add type checking for new functions**

In `compiler/typeck.sans`:
- `respond_stream` — method on HttpRequest, 1 arg (status), returns Int
- `stream_write` — built-in function, 2 args (writer, data), returns Int
- `stream_end` — built-in function, 1 arg (writer), returns Int

- [ ] **Step 6: Add IR lowering**

- `req.respond_stream(status)` → `IR_CALL` to `sans_respond_stream(req, status)`
- `stream_write(w, data)` → `IR_CALL` to `sans_stream_write(w, data)`
- `stream_end(w)` → `IR_CALL` to `sans_stream_end(w)`

- [ ] **Step 7: Add extern declarations in codegen**

- [ ] **Step 8: Verify and commit**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test --test e2e 2>&1 | tail -3
git add runtime/server.sans compiler/*.sans && git commit -m "feat: chunked streaming (respond_stream, stream_write, stream_end)"
```

---

## Task 4: Examples + Documentation + Version Bump

**Files:**
- Create: `examples/concurrent_server.sans`
- Create: `examples/streaming_server.sans`
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `website/static/docs.html`
- Modify: `editors/vscode-sans/src/extension.ts`
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json`
- Modify: All version files (→ 0.3.35)
- Modify: `CLAUDE.md`

- [ ] **Step 1: Create examples**

`examples/concurrent_server.sans`:
```sans
handle(req:I) I {
  path = req.path_only()
  path == "/" ? req.respond(200, "Hello from Sans!") :
  path == "/json" ? req.respond_json(200, "{\"ok\":true}") :
  req.respond(404, "Not Found")
}

main() I {
  p("Server on http://localhost:8080")
  serve(8080, fptr("handle"))
}
```

`examples/streaming_server.sans`:
```sans
handle(req:I) I {
  w = req.respond_stream(200)
  stream_write(w, "line 1\n")
  stream_write(w, "line 2\n")
  stream_write(w, "line 3\n")
  stream_end(w)
}

main() I {
  p("Streaming on http://localhost:8080")
  serve(8080, fptr("handle"))
}
```

- [ ] **Step 2: Update docs**

Add to reference.md, ai-reference.md, docs.html:
- `serve(port, handler)` — concurrent HTTP server
- `serve_tls(port, cert, key, handler)` — concurrent HTTPS server
- `req.respond_stream(status)` — start chunked response
- `stream_write(writer, data)` — send chunk
- `stream_end(writer)` — end chunked response
- Keep-alive behavior documentation

- [ ] **Step 3: Update editor tooling**

Add HOVER_DATA and syntax highlighting for: `serve`, `serve_tls`, `respond_stream`, `stream_write`, `stream_end`.

- [ ] **Step 4: Version bump to 0.3.35**

- [ ] **Step 5: Commit**

```bash
git add examples/ docs/ website/ editors/ CLAUDE.md crates/*/Cargo.toml && git commit -m "feat: production server — serve(), keep-alive, chunked streaming — v0.3.35"
```

---

## Summary

| Task | What | Est. |
|------|------|------|
| 1 | Keep-alive support (conditional close) | Runtime only |
| 2 | `serve()`/`serve_tls()` + keep-alive loop | Runtime + compiler |
| 3 | Chunked streaming | Runtime + compiler |
| 4 | Examples + docs + version | Docs |
