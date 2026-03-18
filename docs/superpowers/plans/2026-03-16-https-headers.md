# HTTPS Server + Headers/CORS/Cookies Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SSL/TLS support via OpenSSL, request header parsing, response header chaining, CORS helper, and cookie parsing to the Sans HTTP server.

**Architecture:** 5 new OpenSSL built-in functions flow through the standard pipeline (typeck → IR → codegen). A new `runtime/ssl.sans` module wraps them into `https_listen`. The existing `runtime/server.sans` is extended with header parsing, `set_header`, `cors`, and `cookie` support. The request struct grows from 32 to 48 bytes.

**Tech Stack:** OpenSSL 3.x (`-lssl -lcrypto`), existing Sans socket primitives

**Spec:** `docs/superpowers/specs/2026-03-16-https-headers-design.md`

---

## Chunk 1: SSL Built-ins + Linker

### Task 1: Add SSL built-in functions to the Rust compiler pipeline

Add 5 new built-in functions: `ssl_ctx`, `ssl_accept`, `ssl_read`, `ssl_write`, `ssl_close`.

**Files:**
- Modify: `crates/sans-typeck/src/lib.rs`
- Modify: `crates/sans-ir/src/ir.rs`
- Modify: `crates/sans-ir/src/lib.rs`
- Modify: `crates/sans-codegen/src/lib.rs`
- Modify: `crates/sans-driver/src/main.rs`
- Create: `tests/fixtures/ssl_basic.sans`
- Modify: `crates/sans-driver/tests/e2e.rs`

- [ ] **Step 1: Add type checking for SSL functions**

In `crates/sans-typeck/src/lib.rs`, in the `Expr::Call` match, add after the socket functions:

```rust
} else if function == "ssl_ctx" {
    if args.len() != 2 { return Err(TypeError::new("ssl_ctx() takes 2 arguments (cert, key)")); }
    let cert_ty = check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if !is_i64_compat(&cert_ty) { return Err(TypeError::new("ssl_ctx() cert must be String")); }
    let key_ty = check_expr(&args[1], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    if !is_i64_compat(&key_ty) { return Err(TypeError::new("ssl_ctx() key must be String")); }
    return Ok(Type::Int);
} else if function == "ssl_accept" {
    if args.len() != 2 { return Err(TypeError::new("ssl_accept() takes 2 arguments (ctx, fd)")); }
    for arg in args { check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?; }
    return Ok(Type::Int);
} else if function == "ssl_read" {
    if args.len() != 3 { return Err(TypeError::new("ssl_read() takes 3 arguments (ssl, buf, len)")); }
    for arg in args { check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?; }
    return Ok(Type::Int);
} else if function == "ssl_write" {
    if args.len() != 3 { return Err(TypeError::new("ssl_write() takes 3 arguments (ssl, buf, len)")); }
    for arg in args { check_expr(arg, locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?; }
    return Ok(Type::Int);
} else if function == "ssl_close" {
    if args.len() != 1 { return Err(TypeError::new("ssl_close() takes 1 argument (ssl)")); }
    check_expr(&args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, module_exports)?;
    return Ok(Type::Int);
}
```

- [ ] **Step 2: Add IR instructions**

In `crates/sans-ir/src/ir.rs`, add to the Instruction enum:
```rust
SslCtx { dest: Reg, cert: Reg, key: Reg },
SslAccept { dest: Reg, ctx: Reg, fd: Reg },
SslRead { dest: Reg, ssl: Reg, buf: Reg, len: Reg },
SslWrite { dest: Reg, ssl: Reg, buf: Reg, len: Reg },
SslClose { dest: Reg, ssl: Reg },
```

- [ ] **Step 3: Add IR lowering**

In `crates/sans-ir/src/lib.rs`, in the built-in function lowering, add:
```rust
} else if function == "ssl_ctx" {
    let cert_reg = self.lower_expr(&args[0]);
    let key_reg = self.lower_expr(&args[1]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::SslCtx { dest: dest.clone(), cert: cert_reg, key: key_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "ssl_accept" {
    let ctx_reg = self.lower_expr(&args[0]);
    let fd_reg = self.lower_expr(&args[1]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::SslAccept { dest: dest.clone(), ctx: ctx_reg, fd: fd_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "ssl_read" {
    let ssl_reg = self.lower_expr(&args[0]);
    let buf_reg = self.lower_expr(&args[1]);
    let len_reg = self.lower_expr(&args[2]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::SslRead { dest: dest.clone(), ssl: ssl_reg, buf: buf_reg, len: len_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "ssl_write" {
    let ssl_reg = self.lower_expr(&args[0]);
    let buf_reg = self.lower_expr(&args[1]);
    let len_reg = self.lower_expr(&args[2]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::SslWrite { dest: dest.clone(), ssl: ssl_reg, buf: buf_reg, len: len_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
} else if function == "ssl_close" {
    let ssl_reg = self.lower_expr(&args[0]);
    let dest = self.fresh_reg();
    self.instructions.push(Instruction::SslClose { dest: dest.clone(), ssl: ssl_reg });
    self.reg_types.insert(dest.clone(), IrType::Int);
    return dest;
}
```

- [ ] **Step 4: Add codegen**

In `crates/sans-codegen/src/lib.rs`:

1. Declare OpenSSL extern functions:
```rust
// SSL/TLS
let ssl_tls_method = llvm_module.add_function("TLS_server_method", context.ptr_type(Default::default()).fn_type(&[], false), Some(Linkage::External));
let ssl_ctx_new = llvm_module.add_function("SSL_CTX_new", context.ptr_type(Default::default()).fn_type(&[context.ptr_type(Default::default()).into()], false), Some(Linkage::External));
let ssl_ctx_cert = llvm_module.add_function("SSL_CTX_use_certificate_file", i32_type.fn_type(&[context.ptr_type(Default::default()).into(), context.ptr_type(Default::default()).into(), i32_type.into()], false), Some(Linkage::External));
let ssl_ctx_key = llvm_module.add_function("SSL_CTX_use_PrivateKey_file", i32_type.fn_type(&[context.ptr_type(Default::default()).into(), context.ptr_type(Default::default()).into(), i32_type.into()], false), Some(Linkage::External));
let ssl_new_fn = llvm_module.add_function("SSL_new", context.ptr_type(Default::default()).fn_type(&[context.ptr_type(Default::default()).into()], false), Some(Linkage::External));
let ssl_set_fd = llvm_module.add_function("SSL_set_fd", i32_type.fn_type(&[context.ptr_type(Default::default()).into(), i32_type.into()], false), Some(Linkage::External));
let ssl_accept_fn = llvm_module.add_function("SSL_accept", i32_type.fn_type(&[context.ptr_type(Default::default()).into()], false), Some(Linkage::External));
let ssl_read_fn = llvm_module.add_function("SSL_read", i32_type.fn_type(&[context.ptr_type(Default::default()).into(), context.ptr_type(Default::default()).into(), i32_type.into()], false), Some(Linkage::External));
let ssl_write_fn = llvm_module.add_function("SSL_write", i32_type.fn_type(&[context.ptr_type(Default::default()).into(), context.ptr_type(Default::default()).into(), i32_type.into()], false), Some(Linkage::External));
let ssl_shutdown_fn = llvm_module.add_function("SSL_shutdown", i32_type.fn_type(&[context.ptr_type(Default::default()).into()], false), Some(Linkage::External));
let ssl_free_fn = llvm_module.add_function("SSL_free", context.void_type().fn_type(&[context.ptr_type(Default::default()).into()], false), Some(Linkage::External));
```

2. Compile each instruction — `SslCtx`:
   - Call `TLS_server_method()` → method_ptr
   - Call `SSL_CTX_new(method_ptr)` → ctx_ptr
   - Call `SSL_CTX_use_certificate_file(ctx, cert, 1)` (1 = SSL_FILETYPE_PEM)
   - Call `SSL_CTX_use_PrivateKey_file(ctx, key, 1)`
   - ptrtoint ctx → i64, store in dest

   `SslAccept`:
   - inttoptr ctx → ptr, Call `SSL_new(ctx)` → ssl_ptr
   - trunc fd to i32, Call `SSL_set_fd(ssl, fd_i32)`
   - Call `SSL_accept(ssl)`, check result
   - ptrtoint ssl → i64, store in dest (or 0 on failure)

   `SslRead`:
   - inttoptr ssl, inttoptr buf, trunc len to i32
   - Call `SSL_read(ssl, buf, len)` → sext result to i64

   `SslWrite`:
   - Same pattern as SslRead but with `SSL_write`

   `SslClose`:
   - inttoptr ssl, Call `SSL_shutdown(ssl)`, Call `SSL_free(ssl)`

- [ ] **Step 5: Add `-lssl -lcrypto` to linker**

In `crates/sans-driver/src/main.rs`, find the link_args line:
```rust
link_args.extend(["-lcurl".to_string(), ...]);
```
Change to:
```rust
link_args.extend(["-lcurl".to_string(), "-lssl".to_string(), "-lcrypto".to_string(), ...]);
```

- [ ] **Step 6: Add `ssl` to runtime modules list**

In `crates/sans-driver/src/main.rs`, find the runtime modules array and add `"ssl"`:
```rust
"log", "result", "functional", "array_ext", "string_ext", "http", "server", "json", "sock", "curl", "map", "arena", "ssl",
```

- [ ] **Step 7: Run tests**

```bash
LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17) cargo test
```

- [ ] **Step 8: Update version + commit**

Bump version to 0.3.33 in all version files. Commit:
```bash
git commit -m "feat: add SSL built-in functions (ssl_ctx, ssl_accept, ssl_read, ssl_write, ssl_close) — v0.3.33"
```

---

## Chunk 2: SSL Runtime Module + HTTPS Listen

### Task 2: Create `runtime/ssl.sans` and wire `https_listen`

**Files:**
- Create: `runtime/ssl.sans`
- Modify: `runtime/server.sans` (refactor accept/respond to support SSL)
- Modify: `crates/sans-typeck/src/lib.rs` (add `https_listen` / `hl_s`)
- Modify: `crates/sans-ir/src/lib.rs` (lower `https_listen`)
- Create: `examples/https_server.sans`
- Create: `examples/gen_cert.sh`

- [ ] **Step 1: Create `runtime/ssl.sans`**

```sans
// HTTPS server: creates listening socket with SSL context
sans_https_listen(port:I cert:S key:S) I {
  ctx = ssl_ctx(cert, key)
  ctx == 0 ? { p("SSL: failed to create context"); exit(1); 0 } : 0
  fd = sock(2, 1, 0)
  // SO_REUSEADDR
  opt_val = alloc(4)
  store32(0, opt_val, 1)
  rsetsockopt(fd, 1, 2, opt_val, 4)
  sbind(fd, port)
  slisten(fd, 128)
  // Server struct: [fd, port, ssl_ctx] = 24 bytes
  server = alloc(24)
  store64(server, fd)
  store64(server + 8, port)
  store64(server + 16, ctx)
  server
}

// Accept with SSL handshake
sans_https_accept(server:I) I {
  sfd = load64(server)
  ctx = load64(server + 16)
  cfd = saccept(sfd)
  cfd < 0 ? sans_make_empty_request() : sans_https_accept_ssl(ctx, cfd)
}

sans_https_accept_ssl(ctx:I cfd:I) I {
  ssl = ssl_accept(ctx, cfd)
  ssl == 0 ? { sclose(cfd); sans_make_empty_request() } : sans_https_accept_read(ssl, cfd)
}

sans_https_accept_read(ssl:I cfd:I) I {
  buf = alloc(8192)
  n = ssl_read(ssl, buf, 8191)
  n <= 0 ? { ssl_close(ssl); sclose(cfd); sans_make_empty_request() } : sans_https_accept_parse(ssl, cfd, buf, n)
}

sans_https_accept_parse(ssl:I cfd:I buf:I n:I) I {
  store8(buf + n, 0)
  // Reuse the HTTP parser from server.sans
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
  // Extended request struct: 48 bytes
  // [fd, method, path, body, headers_map, resp_headers, ssl_ptr]
  // Store negative fd to signal SSL mode, ssl ptr at offset 48
  req = alloc(56)
  store64(req, cfd)
  store64(req + 8, method)
  store64(req + 16, path)
  store64(req + 24, body)
  store64(req + 32, 0)    // headers (parsed later or on demand)
  store64(req + 40, 0)    // response headers
  store64(req + 48, ssl)  // SSL pointer (0 for plain HTTP)
  // Parse headers
  sans_parse_headers(req, buf, n)
  req
}
```

- [ ] **Step 2: Add `https_listen` type check + IR lowering**

In typeck, add:
```rust
} else if function == "https_listen" || function == "hl_s" {
    if args.len() != 3 { return Err(TypeError::new("https_listen() takes 3 arguments (port, cert, key)")); }
    for arg in args { check_expr(arg, ...)?; }
    return Ok(Type::HttpServer);
}
```

In IR lowering, lower `https_listen` as a `Call` to `sans_https_listen` (it's a runtime function, not a built-in instruction).

Add method on HttpServer: `.accept` for HTTPS server dispatches to `sans_https_accept`.

- [ ] **Step 3: Create examples**

Create `examples/gen_cert.sh`:
```bash
#!/bin/bash
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
echo "Generated cert.pem and key.pem"
```

Create `examples/https_server.sans`:
```sans
main() I {
  p("Generating self-signed cert...")
  sys("openssl req -x509 -newkey rsa:2048 -keyout /tmp/key.pem -out /tmp/cert.pem -days 1 -nodes -subj '/CN=localhost' 2>/dev/null")
  server = https_listen(8443, "/tmp/cert.pem", "/tmp/key.pem")
  p("HTTPS server on https://localhost:8443")
  p("Test: curl -k https://localhost:8443")
  req = server.accept
  req.respond(200, "Hello HTTPS from Sans!")
  0
}
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add HTTPS server (https_listen, ssl.sans runtime) — v0.3.33"
```

---

## Chunk 3: Request Headers + Response Headers + set_header

### Task 3: Extend request struct with header parsing and response headers

**Files:**
- Modify: `runtime/server.sans` (header parsing, set_header, updated respond)
- Modify: `crates/sans-typeck/src/lib.rs` (header, set_header methods)
- Modify: `crates/sans-ir/src/lib.rs` (lower new methods)
- Modify: `crates/sans-codegen/src/lib.rs` (HttpRespond instruction updated)

- [ ] **Step 1: Add header parsing to `runtime/server.sans`**

Add `sans_parse_headers(req, buf, n)` function that walks raw HTTP request lines, parses `Name: Value` pairs, and stores them in a Map at req offset 32.

```sans
sans_parse_headers(req:I buf:I n:I) I {
  headers = M()
  // Skip first line (method path HTTP/1.1)
  i := 0
  // Find end of first line
  while i < n {
    if load8(buf + i) == 10 { i += 1; 0 } else { i += 1; continue }
    break
  }
  // Parse header lines
  while i < n {
    // Check for \r\n (end of headers)
    if load8(buf + i) == 13 { break }
    if load8(buf + i) == 10 { i += 1; continue }
    // Find colon
    colon := i
    while colon < n && load8(buf + colon) != 58 { colon += 1; 0 }
    if colon >= n { break }
    // Extract name (lowercase)
    name = sans_lowercase_slice(buf, i, colon)
    // Skip ": "
    vstart = colon + 1
    if vstart < n && load8(buf + vstart) == 32 { vstart += 1 }
    // Find end of line
    vend := vstart
    while vend < n && load8(buf + vend) != 13 && load8(buf + vend) != 10 { vend += 1; 0 }
    // Extract value
    value = sans_copy_str(buf + vstart, vend - vstart)
    headers.set(name, value)
    // Skip to next line
    i = vend
    if i < n && load8(buf + i) == 13 { i += 1 }
    if i < n && load8(buf + i) == 10 { i += 1 }
    0
  }
  store64(req + 32, headers)
  0
}
```

- [ ] **Step 2: Add `header` method on HttpRequest**

In typeck, add to the HttpRequest method match:
```rust
(Type::HttpRequest, "header") => {
    if args.len() != 1 { return Err(TypeError::new("header() takes 1 argument (name)")); }
    check_expr(&args[0], ...)?;
    return Ok(Type::String);
}
```

In IR lowering, lower `req.header(name)` to a `Call` to `sans_http_request_header`.

In `runtime/server.sans`, add:
```sans
sans_http_request_header(req:I name:I) I {
  headers = load64(req + 32)
  headers == 0 ? sans_empty_str() : headers.has(name) ? headers.get(name) : sans_empty_str()
}
```

- [ ] **Step 3: Add `set_header` method on HttpRequest**

In typeck:
```rust
(Type::HttpRequest, "set_header") => {
    if args.len() != 2 { return Err(TypeError::new("set_header() takes 2 arguments (name, value)")); }
    for arg in args { check_expr(arg, ...)?; }
    return Ok(Type::Int);
}
```

In `runtime/server.sans`, add:
```sans
sans_http_request_set_header(req:I name:I value:I) I {
  rh = load64(req + 40)
  if rh == 0 {
    rh = array<I>()
    store64(req + 40, rh)
  }
  rh.push(name)
  rh.push(value)
  0
}
```

- [ ] **Step 4: Update `sans_http_respond` to include custom headers**

Modify `sans_http_respond_inner` in `runtime/server.sans` to read offset 40 (response headers array) and append each name/value as `\r\nName: Value` to the response.

Also update to use `ssl_write`/`ssl_close` when offset 48 (SSL pointer) is non-zero.

- [ ] **Step 5: Update HTTP accept to use extended request struct (48→56 bytes)**

Update `sans_http_accept_parse` to allocate 56-byte request struct and initialize headers/response_headers/ssl fields to 0.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat: add request header parsing + set_header + SSL-aware respond"
```

---

## Chunk 4: CORS + Cookies + Docs

### Task 4: Add CORS helper and cookie method

**Files:**
- Modify: `runtime/server.sans` (cors, cors_all, cookie functions)
- Modify: `crates/sans-typeck/src/lib.rs` (cookie method)
- Modify: `crates/sans-ir/src/lib.rs`
- Create: `examples/cors_server.sans`

- [ ] **Step 1: Add CORS functions to `runtime/server.sans`**

```sans
sans_cors(req:I origin:I) I {
  sans_http_request_set_header(req, "Access-Control-Allow-Origin", origin)
  sans_http_request_set_header(req, "Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
  sans_http_request_set_header(req, "Access-Control-Allow-Headers", "Content-Type, Authorization")
  0
}

sans_cors_all(req:I) I = sans_cors(req, "*")
```

Add type checks for `cors` and `cors_all` as regular functions (not methods).

- [ ] **Step 2: Add cookie method on HttpRequest**

In typeck:
```rust
(Type::HttpRequest, "cookie") => {
    if args.len() != 1 { return Err(TypeError::new("cookie() takes 1 argument (name)")); }
    check_expr(&args[0], ...)?;
    return Ok(Type::String);
}
```

In `runtime/server.sans`:
```sans
sans_http_request_cookie(req:I name:I) I {
  cookie_hdr = sans_http_request_header(req, "cookie")
  slen(cookie_hdr) == 0 ? sans_empty_str() : sans_parse_cookie(cookie_hdr, name)
}

sans_parse_cookie(hdr:I name:I) I {
  // Parse "key1=val1; key2=val2; ..."
  nlen = slen(name)
  i := 0
  len = slen(hdr)
  while i < len {
    // Skip whitespace
    while i < len && load8(hdr + i) == 32 { i += 1; 0 }
    // Check if this key matches
    match := 1
    j := 0
    while j < nlen && i + j < len {
      if load8(hdr + i + j) != load8(name + j) { match = 0; j = nlen } else { j += 1 }
      0
    }
    if match == 1 && i + nlen < len && load8(hdr + i + nlen) == 61 {
      // Found! Extract value until ; or end
      vstart = i + nlen + 1
      vend := vstart
      while vend < len && load8(hdr + vend) != 59 { vend += 1; 0 }
      return sans_copy_str(hdr + vstart, vend - vstart)
    }
    // Skip to next ;
    while i < len && load8(hdr + i) != 59 { i += 1; 0 }
    if i < len { i += 1 }
    0
  }
  sans_empty_str()
}
```

- [ ] **Step 3: Create CORS example**

`examples/cors_server.sans`:
```sans
main() I {
  server = listen(8080)
  p("CORS server on http://localhost:8080")
  count := 0
  while count < 10 {
    req = server.accept
    method = req.method
    method == "OPTIONS" ? {
      cors(req, "*")
      req.set_header("Access-Control-Max-Age", "86400")
      req.respond(204, "")
    } : {
      cors(req, "*")
      req.respond(200, "{\"message\": \"Hello from Sans!\"}", "application/json")
    }
    count += 1
    0
  }
  0
}
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add CORS helper + cookie parsing"
```

---

### Task 5: Documentation + version bump

**Files:**
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `website/static/docs.html`
- Modify: `editors/vscode-sans/src/extension.ts`
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json`
- Modify: All version files (bump to 0.3.33)
- Modify: `CLAUDE.md` (current version)

- [ ] **Step 1: Update `docs/reference.md`**

Add sections for:
- SSL/TLS: `ssl_ctx`, `ssl_accept`, `ssl_read`, `ssl_write`, `ssl_close`, `https_listen`
- Request headers: `req.header(name)`
- Response headers: `req.set_header(name, value)`
- CORS: `cors(req, origin)`, `cors_all(req)`
- Cookies: `req.cookie(name)`

- [ ] **Step 2: Update `docs/ai-reference.md`**

Add compact reference lines for all new functions.

- [ ] **Step 3: Update `website/static/docs.html`**

Match reference.md sections.

- [ ] **Step 4: Update editor tooling**

Add HOVER_DATA entries for: `ssl_ctx`, `ssl_accept`, `ssl_read`, `ssl_write`, `ssl_close`, `https_listen`, `cors`, `cors_all`, `header`, `set_header`, `cookie`.

Add syntax highlighting for new builtins.

- [ ] **Step 5: Bump version to 0.3.33**

Update all version files.

- [ ] **Step 6: Final commit**

```bash
git commit -m "feat: HTTPS server + headers + CORS + cookies — v0.3.33"
```

---

## Summary

| Task | What | Dependencies |
|------|------|-------------|
| 1 | SSL built-ins in Rust pipeline + linker | — |
| 2 | ssl.sans runtime + https_listen | Task 1 |
| 3 | Header parsing + set_header + SSL-aware respond | Task 2 |
| 4 | CORS + cookies | Task 3 |
| 5 | Documentation + version | Task 4 |
