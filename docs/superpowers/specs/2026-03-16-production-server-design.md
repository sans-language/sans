# Production Server (Keep-Alive + Chunked + Concurrency) — Design Spec

## Overview

Upgrade the Sans HTTP server to handle real traffic: auto-threading via `serve()`, HTTP/1.1 keep-alive, and chunked transfer encoding for streaming responses. All implemented purely in Sans (no Rust changes).

## 1. Concurrent Connections via `serve()`

### API

```sans
handle(req:I) I {
  req.respond(200, "hello")
}

main() I {
  serve(8080, fptr("handle"))
}
```

HTTPS variant:
```sans
main() I {
  serve_tls(8443, "cert.pem", "key.pem", fptr("handle"))
}
```

### Implementation

`serve(port, handler)` is a runtime function in `runtime/server.sans`:

```
sans_serve(port, handler):
  server = listen(port)
  while true:
    req = accept(server)
    spawn sans_serve_worker(handler, req, server)
```

Each worker thread handles one connection with keep-alive:

```
sans_serve_worker(handler, first_req, server):
  fd = first_req.fd
  ssl = first_req.ssl
  req = first_req
  loop:
    fcall(handler, req)
    if connection should close: break
    req = read_next_request(fd, ssl)
    if req invalid: break
  close connection
```

`serve_tls(port, cert, key, handler)` is identical but creates an HTTPS server.

### Type Checking

- `serve` — 2 args `(I, I) → I` (port, handler fn ptr)
- `serve_tls` — 4 args `(I, S, S, I) → I` (port, cert, key, handler fn ptr)

### IR Lowering

Both lower as `IR_CALL` to `sans_serve` / `sans_serve_tls`.

## 2. Keep-Alive

### Behavior

HTTP/1.1 connections default to keep-alive. After `respond()` sends the response:

1. Check if client sent `Connection: close` → close socket
2. Check if server set `Connection: close` via `set_header` → close socket
3. Otherwise → read next request on same socket fd

### Changes to `respond()`

Currently `respond()` always closes the connection and sends `Connection: close`. Change to:

- Default: send `Connection: keep-alive` (or omit, since HTTP/1.1 implies it)
- Only send `Connection: close` if the user explicitly set it via `set_header`
- Do NOT close the socket after sending — let the keep-alive loop handle it
- Add `req.close()` method for explicit connection close outside of keep-alive loop

### Keep-Alive Loop (inside serve worker)

```
sans_serve_worker(handler, first_req, server):
  fd = load64(first_req)         // socket fd
  ssl = load64(first_req + 48)   // SSL pointer (0 for HTTP)
  req := first_req
  done := 0
  while done == 0:
    // Call user handler
    fcall(handler, req)
    // Check if connection should close
    should_close = sans_check_close(req)
    if should_close:
      done = 1
    else:
      // Read next request on same fd
      next_req = sans_read_next_request(fd, ssl)
      if next_req invalid (fd < 0):
        done = 1
      else:
        req = next_req
  // Close connection
  if ssl != 0: ssl_close(ssl)
  sclose(fd)
```

### `sans_check_close(req)` logic:
- If client sent `Connection: close` header → return 1
- If server set `Connection: close` response header → return 1
- Otherwise → return 0

### `sans_read_next_request(fd, ssl)`:
- Reuse the existing request parsing logic but on an already-open fd
- Allocate new 56-byte request struct with same fd/ssl
- Parse new headers, body, etc.
- Return new request (or empty request on read error)

## 3. Chunked Streaming via `respond_stream()`

### API

```sans
handle(req:I) I {
  w = req.respond_stream(200)
  w.write("chunk 1\n")
  w.write("chunk 2\n")
  w.end()
}
```

### Response Headers

`respond_stream(status)` sends:
```
HTTP/1.1 200 OK
Transfer-Encoding: chunked
Content-Type: text/plain
{custom headers from set_header}

```

### Chunk Format

Each `w.write(data)` sends:
```
{hex_length}\r\n
{data}\r\n
```

`w.end()` sends the final chunk:
```
0\r\n
\r\n
```

### Stream Writer Struct (24 bytes)

```
offset 0:  fd (I)  — socket fd
offset 8:  ssl (I) — SSL pointer (0 for plain HTTP)
offset 16: req (I) — back-pointer to request
```

### Methods

- `w.write(data:S) I` — send one chunk. Uses `ssend`/`ssl_write`.
- `w.end() I` — send final empty chunk. Does NOT close the connection (keep-alive handles that).

### Hex Length Helper

`sans_int_to_hex(n)` — converts integer to lowercase hex string for chunk size header.

### Type Checking

- `req.respond_stream(status)` — 1 arg, returns Int (stream writer pointer)
- `w.write(data)` — method on stream writer, 1 arg String, returns Int
- `w.end()` — method on stream writer, 0 args, returns Int

Since we don't have a `StreamWriter` type, the writer is just an Int (pointer). The `write` and `end` methods will need to work on Int-typed values. We can use the existing Int method dispatch: check if the method is `write` or `end` and the object was returned from `respond_stream`.

Actually, simpler: make `write` and `end` on the writer be regular function calls:
```sans
w = req.respond_stream(200)
stream_write(w, "chunk 1")
stream_end(w)
```

Or keep method syntax by dispatching `w.write()` and `w.end()` as general Int methods that call the stream functions.

## 4. Files Changed

| File | Change |
|------|--------|
| `runtime/server.sans` | Add `sans_serve`, `sans_serve_tls`, `sans_serve_worker`, keep-alive loop, `sans_read_next_request`, `sans_check_close`, `sans_respond_stream`, `sans_stream_write`, `sans_stream_end`, `sans_int_to_hex`, update `sans_http_respond_inner` to not close connection |
| `compiler/typeck.sans` | Add `serve`, `serve_tls`, `respond_stream`, `stream_write`, `stream_end` type checks |
| `compiler/ir.sans` | Add IR lowering for new functions |
| `compiler/codegen.sans` | Add extern declarations for new runtime functions |
| `compiler/constants.sans` | Add new IR opcodes |
| `examples/concurrent_server.sans` | Concurrent server example |
| `examples/streaming_server.sans` | Chunked streaming example |

## 5. Testing

### Concurrent Server Example
```sans
handle(req:I) I {
  path = req.path_only()
  path == "/" ? req.respond(200, "Hello!") : req.respond(404, "Not Found")
}

main() I {
  p("Server on http://localhost:8080")
  serve(8080, fptr("handle"))
}
```

Test: `curl http://localhost:8080/ & curl http://localhost:8080/ &` — both should get responses concurrently.

### Streaming Example
```sans
handle(req:I) I {
  w = req.respond_stream(200)
  stream_write(w, "line 1\n")
  stream_write(w, "line 2\n")
  stream_write(w, "line 3\n")
  stream_end(w)
}

main() I {
  p("Streaming server on http://localhost:8080")
  serve(8080, fptr("handle"))
}
```

Test: `curl -N http://localhost:8080/` — should see chunks arrive.

### Keep-Alive Test
```bash
curl -v --http1.1 http://localhost:8080/ http://localhost:8080/
# Should reuse connection (no second TCP handshake)
```

## 6. Dependencies

None — uses existing `spawn`, `srecv`/`ssend`/`ssl_read`/`ssl_write`, `fcall`.
