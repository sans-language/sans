# HTTPS Server + Headers/CORS/Cookies — Design Spec

## Overview

Add SSL/TLS support via OpenSSL, request header parsing, response header chaining, CORS helper, and cookie parsing to the Sans HTTP server.

## 1. SSL/TLS via OpenSSL

### New Built-in Functions

| Function | Signature | Purpose |
|----------|-----------|---------|
| `ssl_ctx(cert:S, key:S)` | `(S, S) → I` | Create `SSL_CTX` with `TLS_server_method()`, load cert+key files, return ctx pointer |
| `ssl_accept(ctx:I, fd:I)` | `(I, I) → I` | Create `SSL` from ctx, `SSL_set_fd`, `SSL_accept` handshake, return SSL pointer (0 on failure) |
| `ssl_read(ssl:I, buf:I, len:I)` | `(I, I, I) → I` | `SSL_read`, return bytes read (-1 on error) |
| `ssl_write(ssl:I, buf:I, len:I)` | `(I, I, I) → I` | `SSL_write`, return bytes written (-1 on error) |
| `ssl_close(ssl:I)` | `(I) → I` | `SSL_shutdown` + `SSL_free`, return 0 |

### Pipeline

1. **Typeck:** Add type checking for `ssl_ctx`, `ssl_accept`, `ssl_read`, `ssl_write`, `ssl_close`
2. **IR:** Add `SslCtx`, `SslAccept`, `SslRead`, `SslWrite`, `SslClose` instruction variants
3. **IR lowering:** Map built-in names to IR instructions
4. **Codegen:** Declare OpenSSL extern functions, compile instructions to calls
5. **Driver:** Add `-lssl -lcrypto` to linker flags

### Codegen External Declarations

```llvm
declare ptr @TLS_server_method()
declare ptr @SSL_CTX_new(ptr)
declare i32 @SSL_CTX_use_certificate_file(ptr, ptr, i32)
declare i32 @SSL_CTX_use_PrivateKey_file(ptr, ptr, i32)
declare ptr @SSL_new(ptr)
declare i32 @SSL_set_fd(ptr, i32)
declare i32 @SSL_accept(ptr)
declare i32 @SSL_read(ptr, ptr, i32)
declare i32 @SSL_write(ptr, ptr, i32)
declare i32 @SSL_shutdown(ptr)
declare void @SSL_free(ptr)
declare void @SSL_CTX_free(ptr)
```

### New Runtime Module: `runtime/ssl.sans`

```sans
sans_https_listen(port:I cert:S key:S) I {
  ctx = ssl_ctx(cert, key)
  ctx == 0 ? { p("SSL context creation failed"); exit(1); 0 } : 0
  fd = sock(2, 1, 0)              // AF_INET, SOCK_STREAM
  rsetsockopt(fd, ...)             // SO_REUSEADDR
  sbind(fd, port)
  slisten(fd, 128)
  // Return server struct: [fd, port, ssl_ctx]
  server = alloc(24)
  store64(server, fd)
  store64(server + 8, port)
  store64(server + 16, ctx)
  server
}
```

The `accept` method on an HTTPS server:
1. Call `saccept(fd)` to get client fd
2. Call `ssl_accept(ctx, client_fd)` to do TLS handshake
3. Use `ssl_read` instead of `srecv` to read HTTP request
4. Parse HTTP request (reuse existing parser)
5. Store SSL pointer in request struct for use by `respond`

The `respond` method:
- Use `ssl_write` instead of `ssend`
- Use `ssl_close` instead of `sclose`

### User API

```sans
server = https_listen(8443, "cert.pem", "key.pem")
while true {
  req = server.accept
  req.respond(200, "Hello HTTPS!")
}
```

### Linker Change

Add `-lssl -lcrypto` to link flags in `crates/sans-driver/src/main.rs`.

### Self-Signed Cert Example

Add `examples/gen_cert.sh`:
```bash
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
```

## 2. Request Header Parsing

### Extended Request Struct (48 bytes)

```
offset 0:  fd (I)               — client socket fd (or SSL pointer for HTTPS)
offset 8:  method (I)           — method string pointer
offset 16: path (I)             — path string pointer
offset 24: body (I)             — body string pointer
offset 32: headers (I)          — Map: lowercase_header_name → value string
offset 40: response_headers (I) — Array of [name, value] pairs for custom response headers
```

### Header Parsing

In `sans_http_accept_parse`, after extracting method/path/body, walk the raw request buffer line by line:

1. Skip the first line (already parsed as method + path)
2. For each subsequent line until `\r\n\r\n`:
   a. Find `: ` separator
   b. Extract name (before `:`) — lowercase it
   c. Extract value (after `: `, trim trailing `\r\n`)
   d. `mset(headers_map, name, value)`
3. Store headers_map at offset 32

### API

```sans
ct = req.header("content-type")    // returns header value or ""
auth = req.header("authorization") // case-insensitive lookup
```

Method: `HttpRequest.header(name:S) S`

Implementation: lowercase the input name, lookup in headers map.

## 3. Response Headers (Chained)

### `set_header` Method

```sans
req.set_header("X-Custom", "value")
```

Method: `HttpRequest.set_header(name:S, value:S) I`

Implementation:
1. Load response_headers array from offset 40
2. If null, create new array
3. Push name and value as consecutive elements (flat array: [name1, val1, name2, val2, ...])

### Updated `respond`

When building the HTTP response string, after the standard headers (Content-Length, Content-Type, Connection), iterate the response_headers array and append each `name: value\r\n` pair.

## 4. CORS Helper

Plain Sans function in `runtime/server.sans`:

```sans
sans_cors(req:I origin:S) I {
  req.set_header("Access-Control-Allow-Origin", origin)
  req.set_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
  req.set_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
  0
}

sans_cors_all(req:I) I = sans_cors(req, "*")
```

User API:
```sans
req = server.accept
cors(req, "*")                    // or cors_all(req)
req.respond(200, "data")
```

For OPTIONS preflight:
```sans
req.method == "OPTIONS" ? {
  cors(req, "*")
  req.set_header("Access-Control-Max-Age", "86400")
  req.respond(204, "")
} : {
  cors(req, "*")
  req.respond(200, body)
}
```

## 5. Cookie Helper

Method on HttpRequest:

```sans
req.cookie("session_id")   // returns cookie value or ""
```

Implementation in `runtime/server.sans`:

```sans
sans_http_request_cookie(req:I name:S) S {
  cookie_header = req.header("cookie")
  // Parse "key1=val1; key2=val2; ..."
  // Split on "; ", then split each on "="
  // Find matching key, return value
}
```

For setting cookies, use `set_header`:
```sans
req.set_header("Set-Cookie", "session_id=abc123; Path=/; HttpOnly; Secure")
```

## 6. Type System Changes

### New Types (or method extensions)

- `https_listen` / `hl_s` — alias for HTTPS listen, returns HttpServer (same type, but with SSL ctx)
- `req.header(name)` — new method on HttpRequest, returns String
- `req.set_header(name, value)` — new method on HttpRequest, returns Int
- `req.cookie(name)` — new method on HttpRequest, returns String
- `cors(req, origin)` / `cors_all(req)` — plain functions, not methods

### Built-in Function Aliases

| Full name | Short alias |
|-----------|-------------|
| `ssl_ctx` | — |
| `ssl_accept` | — |
| `ssl_read` | — |
| `ssl_write` | — |
| `ssl_close` | — |
| `https_listen` | `hl_s` |
| `cors` | — |
| `cors_all` | — |

## 7. Testing

### SSL Test

```sans
// examples/https_server.sans
main() I {
  server = https_listen(8443, "cert.pem", "key.pem")
  p("HTTPS server on https://localhost:8443")
  req = server.accept
  req.respond(200, "Hello HTTPS!")
  0
}
```

Test with: `curl -k https://localhost:8443`

### Header Test

```sans
// tests/fixtures/http_headers.sans (if testable without network)
// Or manual test via examples/
```

### CORS Test

```sans
req = server.accept
cors(req, "http://example.com")
req.respond(200, "{\"ok\":true}")
// Response includes Access-Control-Allow-Origin: http://example.com
```

## 8. Files Changed

| File | Change |
|------|--------|
| `crates/sans-typeck/src/lib.rs` | Add ssl_ctx, ssl_accept, ssl_read, ssl_write, ssl_close, https_listen, header, set_header, cookie type checks |
| `crates/sans-ir/src/ir.rs` | Add SslCtx, SslAccept, SslRead, SslWrite, SslClose instructions |
| `crates/sans-ir/src/lib.rs` | Add IR lowering for SSL builtins + new methods |
| `crates/sans-codegen/src/lib.rs` | Declare OpenSSL externs, compile SSL instructions |
| `crates/sans-driver/src/main.rs` | Add `-lssl -lcrypto` to linker |
| `runtime/ssl.sans` | New: HTTPS server wrapper |
| `runtime/server.sans` | Add header parsing, set_header, cors, cookie, updated respond |
| `runtime/sock.sans` | Minor: export helpers needed by ssl.sans |
| `compiler/codegen.sans` | Add SSL extern declarations for self-hosted compiler |
| `compiler/ir.sans` | Add SSL instruction lowering for self-hosted compiler |
| `docs/reference.md` | Document all new functions |
| `docs/ai-reference.md` | Compact reference |
| `website/static/docs.html` | Website docs |
| `editors/vscode-sans/src/extension.ts` | Hover data |
| `editors/vscode-sans/syntaxes/sans.tmLanguage.json` | Syntax highlighting |
| `examples/https_server.sans` | HTTPS example |
| `examples/gen_cert.sh` | Self-signed cert generation |
| `examples/cors_server.sans` | CORS example |

## 9. Dependencies

- OpenSSL 3.x (available via Homebrew on macOS)
- `-lssl -lcrypto` linker flags
