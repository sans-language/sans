# Form Data Parsing + Graceful Shutdown — Design Spec

## Overview

Add `req.form(name)` for parsing HTML form submissions (both URL-encoded and multipart text fields), and graceful shutdown via SIGINT/SIGTERM signal handling in `serve()`. All implemented purely in Sans.

## 1. Form Data Parsing

### API

```sans
name = req.form("username")     // "john"
email = req.form("email")       // "" if not found
```

### Content Type Detection

`sans_http_request_form(req, name)` checks the `Content-Type` request header:

- `application/x-www-form-urlencoded` → parse body as `key1=value1&key2=value2`
- `multipart/form-data` → parse boundary-separated parts, extract text field values
- Anything else → return empty string

### URL-Encoded Parsing

Body format: `username=john&email=john%40example.com`

1. URL-decode the body
2. Split on `&`
3. For each pair, split on `=`
4. If key matches `name`, return URL-decoded value

Reuses existing `sans_url_decode()`.

### Multipart Parsing (text fields only)

Content-Type: `multipart/form-data; boundary=----WebKitFormBoundary...`

Body format:
```
------WebKitFormBoundary\r\n
Content-Disposition: form-data; name="username"\r\n
\r\n
john\r\n
------WebKitFormBoundary\r\n
Content-Disposition: form-data; name="email"\r\n
\r\n
john@example.com\r\n
------WebKitFormBoundary--\r\n
```

Algorithm:
1. Extract boundary from Content-Type header (after `boundary=`)
2. Walk body, split on `--{boundary}`
3. For each part: parse `Content-Disposition` to get field `name`
4. If name matches, extract value (text between blank line and next boundary)
5. Skip parts with `filename=` (file uploads — not supported in this version)

### Implementation

In `runtime/server.sans`:

```
sans_http_request_form(req, name):
  ct = req.header("content-type")
  if ct starts with "application/x-www-form-urlencoded":
    return sans_parse_urlencoded_form(req.body, name)
  if ct starts with "multipart/form-data":
    boundary = extract boundary from ct
    return sans_parse_multipart_form(req.body, boundary, name)
  return ""
```

## 2. Graceful Shutdown

### Behavior

When `serve()` receives SIGINT (Ctrl+C) or SIGTERM:
1. Stop accepting new connections (exit accept loop)
2. In-flight request handlers continue to completion (threads finish naturally)
3. Close server socket
4. Print "Shutting down..." message
5. Exit cleanly

### New Built-in Functions

| Function | Signature | C Function | Purpose |
|----------|-----------|------------|---------|
| `signal_handler(signum)` | `(I) → I` | Custom: sets global flag | Register signal that sets shutdown flag |
| `signal_check()` | `() → I` | Custom: reads global flag | Returns 1 if signal received |
| `spoll(fd, timeout_ms)` | `(I, I) → I` | `poll()` | Poll fd for readability, returns 1/0 |

### Signal Handler Implementation

The self-hosted codegen emits:

```llvm
@__sans_signal_flag = global i64 0

define void @__sans_signal_handler(i32 %sig) {
  store i64 1, ptr @__sans_signal_flag
  ret void
}
```

`signal_handler(signum)` calls: `signal(signum, @__sans_signal_handler)`
`signal_check()` returns: `load i64, ptr @__sans_signal_flag`

### Poll Implementation

`spoll(fd, timeout_ms)` wraps the POSIX `poll()` syscall:

```llvm
; struct pollfd { int fd; short events; short revents; }
%pfd = alloca i64  ; 8 bytes is enough for pollfd
store i32 %fd, ptr %pfd
%events_ptr = getelementptr i8, ptr %pfd, i64 4
store i16 1, ptr %events_ptr  ; POLLIN = 1
%result = call i32 @poll(ptr %pfd, i32 1, i32 %timeout)
; result > 0 means ready
```

### Updated `serve()` Flow

```
sans_serve(port, handler):
  signal_handler(2)    // SIGINT
  signal_handler(15)   // SIGTERM
  server = listen(port)
  fd = server.fd
  while signal_check() == 0:
    ready = spoll(fd, 1000)   // 1 sec poll timeout
    if ready:
      req = accept(server)
      if valid: spawn worker(handler, req, server)
  p("Shutting down...")
  sclose(fd)
  0
```

Workers in-flight finish naturally — their threads complete the current request, send the response, and exit.

## 3. Files Changed

| File | Change |
|------|--------|
| `runtime/server.sans` | Add `sans_http_request_form`, `sans_parse_urlencoded_form`, `sans_parse_multipart_form`, boundary extraction. Update `sans_serve` with signal handling + poll loop. |
| `compiler/typeck.sans` | Add type checks: `form` method on HttpRequest, `signal_handler`, `signal_check`, `spoll` builtins |
| `compiler/ir.sans` | IR lowering for new functions |
| `compiler/codegen.sans` | Add `signal`, `poll` extern declarations. Add codegen for signal_handler (emit global flag + handler function), signal_check (load global), spoll (poll syscall wrapper) |
| `compiler/constants.sans` | New IR opcodes |
| `docs/reference.md` | Document form(), signal handling, serve() graceful shutdown |
| `docs/ai-reference.md` | Compact reference |
| `website/static/docs.html` | Website docs |
| `editors/vscode-sans/src/extension.ts` | Hover data |
| `editors/vscode-sans/syntaxes/sans.tmLanguage.json` | Syntax highlighting |

## 4. Testing

### Form parsing test
```sans
// Manual test with curl:
// curl -X POST -d "username=john&email=test" http://localhost:8080/form
handle(req:I) I {
  method = req.method
  method == "POST" ? {
    name = req.form("username")
    req.respond(200, "Hello " + name)
  } : req.respond(200, "Send a POST")
}
```

### Graceful shutdown test
```bash
# Start server, send request, Ctrl+C
# Should see "Shutting down..." and clean exit
```

## 5. Dependencies

- `signal()` from libc — for registering signal handlers
- `poll()` from libc — for non-blocking accept with timeout
- Both available on macOS/Linux without additional linking
