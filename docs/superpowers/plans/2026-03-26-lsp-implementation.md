# Sans LSP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a fully Sans-implemented LSP server (`sans-lsp`) providing diagnostics, hover, go-to-definition, and signature help.

**Architecture:** Standalone stdio binary that reads JSON-RPC from stdin, runs the existing compiler pipeline (lexer → parser → typeck, stopping before IR/codegen) for analysis, and writes JSON-RPC responses to stdout. New runtime builtins for buffered stdin reading. Compiler modified with a collect mode flag so errors are returned as data instead of calling exit().

**Tech Stack:** Sans (self-hosted), LLVM IR codegen, JSON-RPC over stdio, LSP protocol

---

## File Map

### New Files

| File | Responsibility |
|------|---------------|
| `runtime/io.sans` | Buffered stdin reader — `sans_stdin_read_line()` and `sans_stdin_read_bytes(n)` |
| `lsp/main.sans` | LSP entry point — JSON-RPC main loop, request dispatcher, initialization |
| `lsp/rpc.sans` | JSON-RPC message framing — read Content-Length headers, parse JSON, write responses |
| `lsp/protocol.sans` | LSP type structs and JSON serialization/deserialization functions |
| `lsp/analyzer.sans` | Compiler pipeline wrapper — run lexer→parser→typeck, collect diagnostics and symbols |
| `lsp/symbols.sans` | Symbol table queries — hover lookup, go-to-def, signature help |
| `tests/fixtures/stdin_read_line.sans` | Test fixture for `srl()` builtin |
| `tests/fixtures/stdin_read_bytes.sans` | Test fixture for `srb()` builtin |
| `tests/lsp/test_rpc.sh` | Integration test: JSON-RPC round-trip with spawned sans-lsp process |
| `tests/lsp/test_diagnostics.sh` | Integration test: didOpen → didSave → verify diagnostics |
| `tests/lsp/test_hover.sh` | Integration test: hover requests return correct type info |
| `tests/lsp/test_lifecycle.sh` | Integration test: full lifecycle initialize → work → shutdown → exit |

### Modified Files

| File | What Changes |
|------|-------------|
| `compiler/constants.sans:470` | Add `IR_STDIN_READ_LINE = 313` and `IR_STDIN_READ_BYTES = 314` |
| `compiler/typeck.sans:~1240` | Add type checking for `stdin_read_line`/`srl` and `stdin_read_bytes`/`srb` |
| `compiler/ir.sans:~591` | Add IR lowering for the two new builtins |
| `compiler/codegen.sans` | Add LLVM IR emission for the two new builtins |
| `compiler/typeck.sans:138-310` | Add `tc_collect_mode` global flag, modify `tc_error()` to not exit when flag is set |
| `compiler/typeck.sans:4238-4241` | Modify `check_inner()` end to return diagnostics array when in collect mode |
| `compiler/lexer.sans` | Add collect mode error handling (return error token instead of crashing) |
| `compiler/parser.sans:65-74` | Modify `p_expect()` to collect error instead of exit when in collect mode |
| `editors/vscode-sans/src/extension.ts` | Replace hover provider with LSP client, add `vscode-languageclient` |
| `editors/vscode-sans/package.json` | Add `vscode-languageclient` dependency, LSP configuration |
| `docs/reference.md` | Add stdin_read_line and stdin_read_bytes documentation |
| `docs/ai-reference.md` | Add srl/srb to compact reference |

---

## Task 1: Buffered Stdin Runtime Module

**Files:**
- Create: `runtime/io.sans`
- Create: `tests/fixtures/stdin_read_line.sans`
- Create: `tests/fixtures/stdin_read_bytes.sans`

This task creates the runtime functions that the compiler builtins will call. The functions use `srecv(0, buf, len)` to read from stdin (file descriptor 0) with an internal buffer.

- [ ] **Step 1: Create `runtime/io.sans` with buffered reader**

```sans
// ------ Buffered stdin I/O -----------------------------------------------
// Internal buffer for stdin reads. Global so scope GC won't free it.
g sans_stdin_buf = 0
g sans_stdin_buf_len = 0
g sans_stdin_buf_pos = 0
g sans_stdin_buf_cap = 0

sans_stdin_init() I {
  if sans_stdin_buf == 0 {
    sans_stdin_buf_cap = 65536
    sans_stdin_buf = alloc(sans_stdin_buf_cap)
    sans_stdin_buf_len = 0
    sans_stdin_buf_pos = 0
  }
  0
}

// Fill buffer from stdin fd 0. Returns bytes added, 0 on EOF.
sans_stdin_fill() I {
  sans_stdin_init()
  // Compact: move unread bytes to front
  remaining = sans_stdin_buf_len - sans_stdin_buf_pos
  if remaining > 0 && sans_stdin_buf_pos > 0 {
    mcpy(sans_stdin_buf, sans_stdin_buf + sans_stdin_buf_pos, remaining)
  }
  sans_stdin_buf_len = remaining
  sans_stdin_buf_pos = 0
  // Read more
  space = sans_stdin_buf_cap - sans_stdin_buf_len
  if space <= 0 { return 0 }
  n = srecv(0, sans_stdin_buf + sans_stdin_buf_len, space)
  if n > 0 { sans_stdin_buf_len += n }
  n
}

// Read one line from stdin (up to \n). Returns the line without \n.
// Blocks until a full line is available or EOF.
sans_stdin_read_line() S {
  sans_stdin_init()
  // Search for \n in buffered data
  loop_done := 0
  while loop_done == 0 {
    i := sans_stdin_buf_pos
    while i < sans_stdin_buf_len {
      ch = load8(sans_stdin_buf + i)
      if ch == 10 {
        // Found \n — extract line [pos..i), advance past \n
        line_len = i - sans_stdin_buf_pos
        result = alloc(line_len + 1)
        if line_len > 0 { mcpy(result, sans_stdin_buf + sans_stdin_buf_pos, line_len) }
        store8(result + line_len, 0)
        sans_stdin_buf_pos = i + 1
        // Strip trailing \r if present
        if line_len > 0 && load8(result + line_len - 1) == 13 {
          store8(result + line_len - 1, 0)
        }
        return result
      }
      i += 1
    }
    // No \n found — try to read more
    n = sans_stdin_fill()
    if n <= 0 {
      // EOF — return whatever is left
      remaining = sans_stdin_buf_len - sans_stdin_buf_pos
      if remaining <= 0 { return "" }
      result = alloc(remaining + 1)
      mcpy(result, sans_stdin_buf + sans_stdin_buf_pos, remaining)
      store8(result + remaining, 0)
      sans_stdin_buf_pos = sans_stdin_buf_len
      return result
    }
  }
  ""
}

// Read exactly n bytes from stdin. Blocks until all bytes received or EOF.
sans_stdin_read_bytes(count:I) S {
  sans_stdin_init()
  result = alloc(count + 1)
  got := 0
  while got < count {
    // Use buffered data first
    avail = sans_stdin_buf_len - sans_stdin_buf_pos
    if avail > 0 {
      take = if avail < (count - got) { avail } else { count - got }
      mcpy(result + got, sans_stdin_buf + sans_stdin_buf_pos, take)
      sans_stdin_buf_pos += take
      got += take
    } else {
      // Buffer empty — refill
      n = sans_stdin_fill()
      if n <= 0 {
        // EOF before we got all bytes
        store8(result + got, 0)
        return result
      }
    }
  }
  store8(result + count, 0)
  result
}
```

- [ ] **Step 2: Create test fixture `tests/fixtures/stdin_read_line.sans`**

```sans
// expected: hello world
main() {
  // This test pipes "hello world\n" to stdin via: echo "hello world" | sans run tests/fixtures/stdin_read_line.sans
  line = srl()
  p(line)
}
```

- [ ] **Step 3: Create test fixture `tests/fixtures/stdin_read_bytes.sans`**

```sans
// expected: ABCDE
main() {
  // This test pipes "ABCDEFGH" to stdin: echo -n "ABCDEFGH" | sans run tests/fixtures/stdin_read_bytes.sans
  chunk = srb(5)
  p(chunk)
}
```

- [ ] **Step 4: Verify runtime module compiles as part of the compiler build**

The runtime modules in `runtime/` are compiled and linked by the compiler's `do_link()` function in `compiler/main.sans`. Check that `runtime/io.sans` will be included. If not, add it to the runtime list in `do_link()`.

Run: `grep -n "runtime/" compiler/main.sans` to find where runtime files are listed.

Add `runtime/io.sans` to that list.

- [ ] **Step 5: Commit**

```bash
git add runtime/io.sans tests/fixtures/stdin_read_line.sans tests/fixtures/stdin_read_bytes.sans
git commit -m "feat: add buffered stdin runtime module (io.sans)"
```

---

## Task 2: Compiler Builtins for stdin_read_line and stdin_read_bytes

**Files:**
- Modify: `compiler/constants.sans:470`
- Modify: `compiler/typeck.sans:~1240`
- Modify: `compiler/ir.sans:~591`
- Modify: `compiler/codegen.sans`

Add the two new builtins through the full compiler pipeline: typeck → constants → IR → codegen.

- [ ] **Step 1: Add IR opcode constants**

In `compiler/constants.sans`, after line 470 (`g IR_BSHR = 312`), add:

```sans
// Buffered stdin I/O
g IR_STDIN_READ_LINE = 313
g IR_STDIN_READ_BYTES = 314
```

- [ ] **Step 2: Add type checking**

In `compiler/typeck.sans`, in the builtin type-checking section (near line 1240, after the file I/O builtins), add:

```sans
    if name == "stdin_read_line" || name == "srl" {
      if nargs != 0 { tc_error("stdin_read_line() takes no arguments") }
      return make_type(TY_STRING)
    }
    if name == "stdin_read_bytes" || name == "srb" {
      if nargs != 1 { tc_error("stdin_read_bytes() takes exactly 1 argument") }
      at = check_expr(args[0], locals, fn_env, ret_type, structs, enums, methods, generic_fns, traits, mod_exports)
      if types_compatible(at, make_type(TY_INT)) != 1 { tc_error("stdin_read_bytes() requires Int argument, got " + type_to_string(at)) }
      return make_type(TY_STRING)
    }
```

- [ ] **Step 3: Add IR lowering**

In `compiler/ir.sans`, in the I/O builtin section (near line 591, after the file I/O entries), add:

```sans
  else if name == "stdin_read_line" { lower_call_0(ctx, IR_STDIN_READ_LINE, IRTY_STR) }
  else if name == "srl" { lower_call_0(ctx, IR_STDIN_READ_LINE, IRTY_STR) }
  else if name == "stdin_read_bytes" { lower_call_1(ctx, args, IR_STDIN_READ_BYTES, IRTY_STR) }
  else if name == "srb" { lower_call_1(ctx, args, IR_STDIN_READ_BYTES, IRTY_STR) }
```

Note: `lower_call_0` may not exist yet. Check first. If it doesn't exist, check how zero-arg builtins like `time()` are lowered and follow that pattern. The pattern should be:

```sans
lower_call_0(ctx:I opcode:I irtype:I) I {
  dest = ctx_fresh_reg(ctx)
  ctx_emit(ctx, ir_inst0(opcode, dest))
  ctx_set_reg_type(ctx, dest, irtype)
  dest
}
```

If `ir_inst0` doesn't exist either, use `ir_inst1` with a dummy arg of 0, matching how `time()` or `random()` are lowered.

- [ ] **Step 4: Add codegen (LLVM IR emission)**

In `compiler/codegen.sans`, add handlers for the two new opcodes. Find where `IR_FILE_READ` is handled and add nearby:

For `IR_STDIN_READ_LINE` (opcode 313):
```
// ------ IR_STDIN_READ_LINE (313): [op, dest] ------
if op == IR_STDIN_READ_LINE {
  r = cg_fresh_reg(cg)
  emit(cg, "  " + r + " = call i64 @sans_stdin_read_line()")
  cg_set_val(cg, dest, r)
  cg_set_ptr(cg, dest)
  emit_scope_track(cg, r, 5)
  return 0
}
```

For `IR_STDIN_READ_BYTES` (opcode 314):
```
// ------ IR_STDIN_READ_BYTES (314): [op, dest, count_reg] ------
if op == IR_STDIN_READ_BYTES {
  cv = cg_get_val(cg, ir_field(inst, 16))
  r = cg_fresh_reg(cg)
  emit(cg, "  " + r + " = call i64 @sans_stdin_read_bytes(i64 " + cv + ")")
  cg_set_val(cg, dest, r)
  cg_set_ptr(cg, dest)
  emit_scope_track(cg, r, 5)
  return 0
}
```

The `emit_scope_track(cg, r, 5)` tracks the returned string (tag 5 = SCOPE_TAG_STRING) for scope GC cleanup.

- [ ] **Step 5: Add function declarations to codegen preamble**

In `compiler/codegen.sans`, find where external function declarations are emitted (search for `declare.*@sans_`). Add:

```
declare i64 @sans_stdin_read_line()
declare i64 @sans_stdin_read_bytes(i64)
```

- [ ] **Step 6: Build the compiler and run tests**

```bash
cd /home/scott/Development/sans-language/sans
sans build compiler/main.sans -o sans_new
./sans_new build compiler/main.sans -o sans_stage2
echo "hello world" | ./sans_new run tests/fixtures/stdin_read_line.sans
echo -n "ABCDEFGH" | ./sans_new run tests/fixtures/stdin_read_bytes.sans
bash tests/run_tests.sh
```

Expected:
- `stdin_read_line` test prints `hello world`
- `stdin_read_bytes` test prints `ABCDE`
- All existing tests still pass

- [ ] **Step 7: Commit**

```bash
git add compiler/constants.sans compiler/typeck.sans compiler/ir.sans compiler/codegen.sans
git commit -m "feat: add stdin_read_line/srl and stdin_read_bytes/srb builtins"
```

---

## Task 3: Compiler Collect Mode for LSP Analysis

**Files:**
- Modify: `compiler/typeck.sans:138-310`
- Modify: `compiler/typeck.sans:4238-4242`
- Modify: `compiler/parser.sans:65-74`

Add a global flag that switches the compiler from "print errors and exit" to "collect errors and continue." This is what the LSP analyzer calls before running typeck.

- [ ] **Step 1: Add collect mode globals to typeck.sans**

In `compiler/typeck.sans`, after line 147 (`g mono_depth = 0`), add:

```sans
g tc_collect_mode = 0
g tc_collected_symbols = 0
```

- [ ] **Step 2: Modify `tc_error()` to respect collect mode**

In `compiler/typeck.sans`, replace the `tc_error` function (lines 238-251) with:

```sans
tc_error(msg:S) I {
  tc_init_diags()
  d = make_diag(DIAG_ERROR, 0, 0, msg, "")
  tc_diags.push(d)
  if tc_collect_mode == 1 {
    tc_has_error = 1
    return 0
  }
  // Legacy behavior: print all diagnostics then exit
  i := 0
  while i < tc_diags.len() {
    render_diag(tc_diags[i])
    i += 1
    0
  }
  exit(1)
  0
}
```

- [ ] **Step 3: Modify `tc_emit_diags()` to not exit in collect mode**

Find `tc_emit_diags` (near line 295). It currently exits if errors were found. In collect mode, it should return without exiting. Find the function and add a check:

```sans
tc_emit_diags() I {
  if tc_diags == 0 { return 0 }
  if tc_diags.len() == 0 { return 0 }
  if tc_collect_mode == 1 { return 0 }
  // ... existing print-and-exit logic unchanged ...
}
```

- [ ] **Step 4: Add symbol collection at end of check_inner()**

In `compiler/typeck.sans`, at line 4238-4241 (end of `check_inner()`), before `tc_emit_diags()`, add symbol collection:

```sans
  // Collect symbols for LSP if in collect mode
  if tc_collect_mode == 1 {
    tc_collected_symbols = alloc(40)
    store64(tc_collected_symbols, structs_map)
    store64(tc_collected_symbols + 8, enums_map)
    store64(tc_collected_symbols + 16, fn_env)
    store64(tc_collected_symbols + 24, methods_map)
    store64(tc_collected_symbols + 32, trait_registry)
  }
```

Place this right before the `tc_emit_diags()` call on line 4239.

- [ ] **Step 5: Modify parser `p_expect()` for collect mode**

In `compiler/parser.sans` (lines 65-74), `p_expect()` currently prints and calls `exit(1)`. Add a collect mode check. First, the parser needs access to the flag. Add at the top of `parser.sans`:

```sans
import "typeck"
```

Then modify `p_expect()`:

```sans
p_expect(p:I kind:I) I {
  if p_at(p, kind) == 1 { p_advance(p) } else {
    if tc_collect_mode == 1 {
      // In collect mode: record error, return 0 (error token)
      tok = p_peek(p)
      line = load64(tok + 16)
      col = load64(tok + 24)
      tc_error_at_pos(line, col, "parse error: expected token kind " + str(kind) + " got " + str(p_peek_kind(p)))
      return 0
    }
    p("parse error: expected token kind ")
    p(kind)
    p(" got ")
    p(p_peek_kind(p))
    exit(1)
    0
  }
}
```

Note: `tc_error_at_pos` may need to be added — a helper that takes line/col directly instead of a node. Add to typeck.sans:

```sans
tc_error_at_pos(line:I col:I msg:S) I {
  tc_init_diags()
  d = make_diag(DIAG_ERROR, line, col, msg, as_str(tc_current_file))
  tc_diags.push(d)
  tc_has_error = 1
  0
}
```

- [ ] **Step 6: Build compiler and verify collect mode doesn't break normal compilation**

```bash
cd /home/scott/Development/sans-language/sans
sans build compiler/main.sans -o sans_new
./sans_new build compiler/main.sans -o sans_stage2
bash tests/run_tests.sh
```

All tests must pass — collect mode is off by default, so normal compilation is unchanged.

- [ ] **Step 7: Commit**

```bash
git add compiler/typeck.sans compiler/parser.sans
git commit -m "feat: add collect mode to compiler for LSP error collection"
```

---

## Task 4: LSP JSON-RPC Message Framing (`lsp/rpc.sans`)

**Files:**
- Create: `lsp/rpc.sans`

Handles reading and writing LSP protocol messages — Content-Length header parsing, JSON body reading, response framing.

- [ ] **Step 1: Create `lsp/rpc.sans`**

```sans
// ------ JSON-RPC message framing for LSP -----------------------------------
import "io"

// Read one JSON-RPC message from stdin.
// Protocol: "Content-Length: N\r\n\r\n{...json...}"
// Returns parsed JsonValue, or json_null() on EOF/error.
rpc_read() J {
  // Read header line: "Content-Length: 52"
  header = srl()
  if slen(header) == 0 { return jn() }

  // Parse content length from header
  // Header format: "Content-Length: N" (may have other headers, but Content-Length is required)
  content_len := 0
  while slen(header) > 0 {
    if header.starts_with("Content-Length: ") == 1 || header.starts_with("Content-Length:") == 1 {
      // Extract the number after "Content-Length: "
      colon_pos = header.index_of(":")
      num_str = header.substring(colon_pos + 1, slen(header)).trim()
      content_len = stoi(num_str)
    }
    // Read next header line (empty line = end of headers)
    header = srl()
    if slen(header) == 0 { header = "" }
    // Break on empty line (just \r\n which becomes "" after strip)
    if slen(header) == 0 { header = "" }
  }

  if content_len <= 0 { return jn() }

  // Read exactly content_len bytes of JSON body
  body = srb(content_len)
  if slen(body) == 0 { return jn() }

  // Parse JSON
  result = jp(body)
  if result.is_err() {
    wfd(2, "rpc_read: JSON parse error: " + result.unwrap_or(jn()).type_of() + "\n")
    return jn()
  }
  result!
}

// Extract method string from a JSON-RPC message
rpc_method(msg:J) S {
  m = msg.get("method")
  if m.type_of() == "string" { return m.get_string() }
  ""
}

// Extract id from a JSON-RPC message (may be int, string, or null)
rpc_id(msg:J) J {
  msg.get("id")
}

// Extract params from a JSON-RPC message
rpc_params(msg:J) J {
  p = msg.get("params")
  if p.type_of() == "null" { return jo() }
  p
}

// Check if message is a request (has id) vs notification (no id)
rpc_is_request(msg:J) B {
  msg.get("id").type_of() != "null"
}

// Write a JSON-RPC response to stdout.
// id: the request id to echo back
// result: the result JsonValue
rpc_respond(id:J result:J) I {
  resp = jo()
  resp.set("jsonrpc", js("2.0"))
  resp.set("id", id)
  resp.set("result", result)
  body = jfy(resp)
  header = "Content-Length: " + str(slen(body)) + "\r\n\r\n"
  wfd(1, header + body)
  0
}

// Write a JSON-RPC error response to stdout.
rpc_respond_error(id:J code:I message:S) I {
  err_obj = jo()
  err_obj.set("code", ji(code))
  err_obj.set("message", js(message))
  resp = jo()
  resp.set("jsonrpc", js("2.0"))
  resp.set("id", id)
  resp.set("error", err_obj)
  body = jfy(resp)
  header = "Content-Length: " + str(slen(body)) + "\r\n\r\n"
  wfd(1, header + body)
  0
}

// Write a JSON-RPC notification to stdout (no id).
rpc_notify(method:S params:J) I {
  msg = jo()
  msg.set("jsonrpc", js("2.0"))
  msg.set("method", js(method))
  msg.set("params", params)
  body = jfy(msg)
  header = "Content-Length: " + str(slen(body)) + "\r\n\r\n"
  wfd(1, header + body)
  0
}
```

- [ ] **Step 2: Commit**

```bash
git add lsp/rpc.sans
git commit -m "feat: add JSON-RPC message framing module for LSP"
```

---

## Task 5: LSP Protocol Types (`lsp/protocol.sans`)

**Files:**
- Create: `lsp/protocol.sans`

Struct definitions for LSP protocol types and JSON conversion functions.

- [ ] **Step 1: Create `lsp/protocol.sans`**

```sans
// ------ LSP Protocol Type Definitions and JSON Conversion -------

// ------ Serialize helpers ------

// Build a JSON Position object { "line": n, "character": n }
// Note: LSP positions are 0-based
make_lsp_position(line:I character:I) J {
  p = jo()
  p.set("line", ji(line))
  p.set("character", ji(character))
  p
}

// Build a JSON Range object { "start": Position, "end": Position }
make_lsp_range(start_line:I start_char:I end_line:I end_char:I) J {
  r = jo()
  r.set("start", make_lsp_position(start_line, start_char))
  r.set("end", make_lsp_position(end_line, end_char))
  r
}

// Build a JSON Location object { "uri": string, "range": Range }
make_lsp_location(uri:S start_line:I start_char:I end_line:I end_char:I) J {
  loc = jo()
  loc.set("uri", js(uri))
  loc.set("range", make_lsp_range(start_line, start_char, end_line, end_char))
  loc
}

// Build a JSON Diagnostic object
// severity: 1=Error, 2=Warning, 3=Info, 4=Hint
make_lsp_diagnostic(start_line:I start_char:I end_line:I end_char:I severity:I message:S) J {
  d = jo()
  d.set("range", make_lsp_range(start_line, start_char, end_line, end_char))
  d.set("severity", ji(severity))
  d.set("message", js(message))
  d.set("source", js("sans"))
  d
}

// Build publishDiagnostics params { "uri": string, "diagnostics": Diagnostic[] }
make_publish_diagnostics(uri:S diagnostics:J) J {
  p = jo()
  p.set("uri", js(uri))
  p.set("diagnostics", diagnostics)
  p
}

// Build a Hover result { "contents": { "kind": "markdown", "value": string } }
make_lsp_hover(content:S) J {
  mc = jo()
  mc.set("kind", js("markdown"))
  mc.set("value", js(content))
  h = jo()
  h.set("contents", mc)
  h
}

// Build server capabilities for initialize response
make_server_capabilities() J {
  cap = jo()
  // Full document sync (client sends entire document on change)
  cap.set("textDocumentSync", ji(1))
  cap.set("hoverProvider", jb(1))
  cap.set("definitionProvider", jb(1))
  // Signature help with trigger characters
  sh = jo()
  triggers = ja()
  triggers.push(js("("))
  triggers.push(js(","))
  sh.set("triggerCharacters", triggers)
  cap.set("signatureHelpProvider", sh)
  cap
}

// Build initialize result
make_initialize_result() J {
  r = jo()
  r.set("capabilities", make_server_capabilities())
  // Server info
  info = jo()
  info.set("name", js("sans-lsp"))
  info.set("version", js("0.9.0"))
  r.set("serverInfo", info)
  r
}

// ------ Deserialize helpers ------

// Extract textDocument.uri from params
params_uri(params:J) S {
  td = params.get("textDocument")
  if td.type_of() == "null" { return "" }
  u = td.get("uri")
  if u.type_of() == "string" { return u.get_string() }
  ""
}

// Extract position (line, character) from params
// Returns tuple-like: line * 1000000 + character (encoded as single int)
// Caller uses: line = result / 1000000, char = result % 1000000
params_position(params:J) I {
  pos = params.get("position")
  if pos.type_of() == "null" { return 0 }
  line = pos.get("line").get_int()
  ch = pos.get("character").get_int()
  line * 1000000 + ch
}

// Extract text content from didOpen/didChange params
params_text(params:J) S {
  td = params.get("textDocument")
  if td.type_of() != "null" {
    // didOpen: textDocument.text
    t = td.get("text")
    if t.type_of() == "string" { return t.get_string() }
  }
  // didChange: contentChanges[0].text (full sync mode)
  changes = params.get("contentChanges")
  if changes.type_of() != "null" && changes.len() > 0 {
    first = changes.get_index(0)
    t = first.get("text")
    if t.type_of() == "string" { return t.get_string() }
  }
  ""
}

// Convert a file:// URI to a filesystem path
uri_to_path(uri:S) S {
  // "file:///home/user/foo.sans" -> "/home/user/foo.sans"
  if uri.starts_with("file://") == 1 {
    return uri.substring(7, slen(uri))
  }
  uri
}

// Convert a filesystem path to a file:// URI
path_to_uri(path:S) S {
  "file://" + path
}
```

- [ ] **Step 2: Commit**

```bash
git add lsp/protocol.sans
git commit -m "feat: add LSP protocol type definitions and JSON helpers"
```

---

## Task 6: LSP Analyzer — Compiler Integration (`lsp/analyzer.sans`)

**Files:**
- Create: `lsp/analyzer.sans`

Wraps the compiler pipeline to run analysis and extract diagnostics + symbols.

- [ ] **Step 1: Create `lsp/analyzer.sans`**

```sans
// ------ LSP Analyzer: wraps compiler for diagnostics and symbol extraction ------
import "lexer"
import "parser"
import "typeck"
import "constants"
import "protocol"

// Global storage for last analysis results
g lsp_last_diags = 0
g lsp_last_symbols = 0
g lsp_project_root = ""

// Set the project root for import resolution
analyzer_set_root(root:S) I {
  lsp_project_root = root
  0
}

// Run analysis on a file. Writes content to a temp file, runs lexer→parser→typeck.
// Returns number of diagnostics found.
analyze_file(uri:S content:S) I {
  path = uri_to_path(uri)

  // Write content to a temp file for the compiler to read
  tmp = "/tmp/sans_lsp_" + str(time()) + ".sans"
  fw(tmp, content)

  // Enable collect mode
  tc_collect_mode = 1
  tc_init_diags()
  tc_has_error = 0
  tc_current_file = path
  tc_current_source = content
  tc_has_source = 1

  // Run lexer
  lx = make_lexer(content)
  tokens = lex_all(lx)

  // Run parser (may add diagnostics in collect mode)
  pr = make_parser(tokens)
  program = parse_program(pr)

  // Run typeck with empty module exports (single-file for now)
  // TODO: resolve imports for multi-file support
  mod_exports = M()
  check_module(program, mod_exports)

  // Disable collect mode
  tc_collect_mode = 0

  // Store diagnostics
  lsp_last_diags = tc_diags
  lsp_last_symbols = tc_collected_symbols

  // Clean up temp file
  rm(tmp)

  // Return diagnostic count
  if lsp_last_diags == 0 { return 0 }
  lsp_last_diags.len()
}

// Convert collected diagnostics to LSP JSON format.
// Returns a JSON array of Diagnostic objects.
get_diagnostics_json(uri:S) J {
  diags_json = ja()
  if lsp_last_diags == 0 { return diags_json }

  i := 0
  while i < lsp_last_diags.len() {
    d = lsp_last_diags[i]
    sev = diag_severity(d)
    line = diag_line(d)
    col = diag_col(d)
    msg = diag_msg(d)

    // LSP severity: 1=Error, 2=Warning (compiler uses DIAG_ERROR=0, DIAG_WARN=1)
    lsp_sev = if sev == DIAG_ERROR { 1 } else { 2 }

    // LSP lines are 0-based, compiler lines are 1-based
    lsp_line = if line > 0 { line - 1 } else { 0 }
    lsp_col = if col > 0 { col - 1 } else { 0 }

    diag = make_lsp_diagnostic(lsp_line, lsp_col, lsp_line, lsp_col + 1, lsp_sev, msg)
    diags_json.push(diag)
    i += 1
  }
  diags_json
}

// Look up symbol info at a position for hover.
// Returns a formatted string, or "" if nothing found.
lookup_hover_at(content:S line:I col:I) S {
  if lsp_last_symbols == 0 { return "" }

  // Find the word at the given position
  word = word_at_position(content, line, col)
  if slen(word) == 0 { return "" }

  // Check function environment (offset 16 in symbol table)
  fn_env = load64(lsp_last_symbols + 16)
  if fn_env != 0 && mhas(fn_env, word) {
    sig = mget(fn_env, word)
    return format_fn_signature(word, sig)
  }

  // Check structs (offset 0)
  structs = load64(lsp_last_symbols)
  if structs != 0 && mhas(structs, word) {
    fields = mget(structs, word)
    return format_struct_signature(word, fields)
  }

  // Check enums (offset 8)
  enums = load64(lsp_last_symbols + 8)
  if enums != 0 && mhas(enums, word) {
    variants = mget(enums, word)
    return format_enum_signature(word, variants)
  }

  ""
}

// Look up definition location for a symbol at a position.
// Returns "file:line:col" or "" if not found.
lookup_definition_at(content:S line:I col:I) S {
  // For Phase 1: definition lookup uses the symbol table
  // which tracks source locations from AST nodes
  if lsp_last_symbols == 0 { return "" }

  word = word_at_position(content, line, col)
  if slen(word) == 0 { return "" }

  // Check function environment
  fn_env = load64(lsp_last_symbols + 16)
  if fn_env != 0 && mhas(fn_env, word) {
    sig = mget(fn_env, word)
    // Function signature stores source location
    // The AST node for the function def has line/col
    fn_line = load64(sig + 16)  // source line from fn signature
    fn_col = load64(sig + 24)   // source col
    if fn_line > 0 {
      return str(fn_line) + ":" + str(fn_col)
    }
  }

  ""
}

// Extract the word (identifier) at a given line:col in source text.
// Line is 1-based, col is 1-based (compiler convention).
word_at_position(src:S line:I col:I) S {
  // Find the start of the target line
  cur_line := 1
  i := 0
  len = slen(src)
  while i < len && cur_line < line {
    if load8(src + i) == 10 { cur_line += 1 }
    i += 1
  }
  // Now i is at the start of the target line
  // Move to col position (1-based)
  pos = i + col - 1
  if pos >= len { return "" }

  // Check if we're on an identifier character
  ch = load8(src + pos)
  if is_ident_char(ch) != 1 { return "" }

  // Walk back to start of identifier
  start := pos
  while start > i && is_ident_char(load8(src + start - 1)) == 1 {
    start -= 1
  }
  // Walk forward to end of identifier
  end := pos
  while end < len && is_ident_char(load8(src + end)) == 1 {
    end += 1
  }

  src.substring(start, end)
}

is_ident_char(ch:I) I {
  if ch >= 65 && ch <= 90 { return 1 }   // A-Z
  if ch >= 97 && ch <= 122 { return 1 }  // a-z
  if ch >= 48 && ch <= 57 { return 1 }   // 0-9
  if ch == 95 { return 1 }               // _
  0
}

// Format a function signature for hover display
format_fn_signature(name:S sig:I) S {
  ret_type = load64(sig + 8)
  params_arr = load64(sig + 16)
  result := name + "("
  if params_arr != 0 {
    params_p = load64(params_arr)
    num_params = load64(params_arr + 8)
    pi := 0
    while pi < num_params {
      param = load64(params_p + pi * 8)
      pname = load64(param)
      ptype = load64(param + 8)
      if pi > 0 { result = result + " " }
      result = result + pname + ":" + type_to_string(ptype)
      pi += 1
    }
  }
  result = result + ") " + type_to_string(ret_type)
  result
}

// Format a struct for hover display
format_struct_signature(name:S fields:I) S {
  result := "struct " + name + " { "
  if fields != 0 {
    ks = fields.keys()
    ki := 0
    while ki < ks.len() {
      k = ks.get(ki)
      if ki > 0 { result = result + ", " }
      result = result + k + " " + type_to_string(mget(fields, k))
      ki += 1
    }
  }
  result + " }"
}

// Format an enum for hover display
format_enum_signature(name:S variants:I) S {
  result := "enum " + name + " { "
  if variants != 0 {
    ks = variants.keys()
    ki := 0
    while ki < ks.len() {
      k = ks.get(ki)
      if ki > 0 { result = result + ", " }
      result = result + k
      ki += 1
    }
  }
  result + " }"
}
```

Note: This task depends on knowing the exact layout of function signatures, struct maps, and enum maps from typeck. The offsets used (e.g., `load64(sig + 8)` for return type) are based on the typeck.sans analysis in the exploration phase. During implementation, verify these offsets by reading the actual typeck.sans code where `fn_env` entries are created.

Also note: `lex_all()` and `parse_program()` are the entry points we need from the lexer and parser. Verify these function names exist. If the lexer uses a different API (e.g., `lex()` returns tokens one at a time), adapt accordingly. Check `compiler/main.sans` line 182 for how `parse()` is called — it takes the source string directly, not pre-lexed tokens. The actual flow might be `parse(src)` which internally creates a lexer. If so, simplify to just calling `parse(content)`.

- [ ] **Step 2: Commit**

```bash
git add lsp/analyzer.sans
git commit -m "feat: add LSP analyzer wrapping compiler pipeline"
```

---

## Task 7: Symbol Table Queries (`lsp/symbols.sans`)

**Files:**
- Create: `lsp/symbols.sans`

Provides higher-level query functions for signature help, building on the analyzer.

- [ ] **Step 1: Create `lsp/symbols.sans`**

```sans
// ------ Symbol table queries for LSP features ------
import "analyzer"
import "protocol"

// Look up signature help at a position.
// Finds the enclosing function call and returns parameter info.
// Returns JSON SignatureHelp object or json_null() if not in a call.
lookup_signature_at(content:S line:I col:I) J {
  if lsp_last_symbols == 0 { return jn() }

  // Find the function name by scanning backwards from cursor for '('
  // and counting commas to determine active parameter
  cur_line := 1
  i := 0
  len = slen(content)
  // Navigate to target line
  while i < len && cur_line < line {
    if load8(content + i) == 10 { cur_line += 1 }
    i += 1
  }
  // Navigate to target col (1-based)
  pos = i + col - 1
  if pos >= len { return jn() }

  // Scan backwards to find matching '('
  depth := 0
  comma_count := 0
  scan := pos - 1
  found_paren := 0
  paren_pos := 0

  while scan >= i && found_paren == 0 {
    ch = load8(content + scan)
    if ch == 41 { depth += 1 }       // ')'
    else if ch == 40 {               // '('
      if depth == 0 {
        found_paren = 1
        paren_pos = scan
      } else {
        depth -= 1
      }
    }
    else if ch == 44 && depth == 0 { // ','
      comma_count += 1
    }
    if found_paren == 0 { scan -= 1 }
  }

  if found_paren == 0 { return jn() }

  // Extract the function name before '('
  name_end = paren_pos
  name_start := name_end - 1
  while name_start >= i && is_ident_char(load8(content + name_start)) == 1 {
    name_start -= 1
  }
  name_start += 1
  if name_start >= name_end { return jn() }
  fn_name = content.substring(name_start, name_end)

  // Look up function in symbol table
  fn_env = load64(lsp_last_symbols + 16)
  if fn_env == 0 || mhas(fn_env, fn_name) != 1 { return jn() }

  sig = mget(fn_env, fn_name)
  label = format_fn_signature(fn_name, sig)

  // Build parameter info array
  params_json = ja()
  params_arr = load64(sig + 16)
  if params_arr != 0 {
    params_p = load64(params_arr)
    num_params = load64(params_arr + 8)
    pi := 0
    while pi < num_params {
      param = load64(params_p + pi * 8)
      pname = load64(param)
      ptype = load64(param + 8)
      param_label = pname + ":" + type_to_string(ptype)
      pinfo = jo()
      pinfo.set("label", js(param_label))
      params_json.push(pinfo)
      pi += 1
    }
  }

  // Build SignatureInformation
  sig_info = jo()
  sig_info.set("label", js(label))
  sig_info.set("parameters", params_json)

  // Build SignatureHelp
  result = jo()
  sigs = ja()
  sigs.push(sig_info)
  result.set("signatures", sigs)
  result.set("activeSignature", ji(0))
  result.set("activeParameter", ji(comma_count))
  result
}
```

- [ ] **Step 2: Commit**

```bash
git add lsp/symbols.sans
git commit -m "feat: add symbol table query functions for LSP"
```

---

## Task 8: LSP Main Loop (`lsp/main.sans`)

**Files:**
- Create: `lsp/main.sans`

The entry point. JSON-RPC main loop, request dispatcher, document state management.

- [ ] **Step 1: Create `lsp/main.sans`**

```sans
// ------ Sans Language Server Protocol (LSP) Implementation ------
import "rpc"
import "protocol"
import "analyzer"
import "symbols"

// ------ Document store (global, survives scope GC) ------
g lsp_documents = 0
g lsp_shutdown_received = 0
g lsp_root_uri = ""

lsp_init_documents() I {
  if lsp_documents == 0 { lsp_documents = M<S,S>() }
  0
}

// Store document content
doc_store(uri:S content:S) I {
  lsp_init_documents()
  mset(lsp_documents, uri, content)
  0
}

// Get document content
doc_get(uri:S) S {
  lsp_init_documents()
  if mhas(lsp_documents, uri) != 1 { return "" }
  mget(lsp_documents, uri)
}

// Remove document
doc_remove(uri:S) I {
  lsp_init_documents()
  if mhas(lsp_documents, uri) == 1 { lsp_documents.delete(uri) }
  0
}

// ------ Request handlers ------

handle_initialize(msg:J) I {
  params = rpc_params(msg)
  // Extract rootUri
  root = params.get("rootUri")
  if root.type_of() == "string" {
    lsp_root_uri = root.get_string()
    analyzer_set_root(uri_to_path(lsp_root_uri))
  }
  // Respond with capabilities
  rpc_respond(rpc_id(msg), make_initialize_result())
  wfd(2, "sans-lsp: initialized\n")
  0
}

handle_did_open(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  text = params_text(params)
  doc_store(uri, text)
  // Run initial analysis
  run_diagnostics(uri, text)
  0
}

handle_did_change(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  text = params_text(params)
  doc_store(uri, text)
  // Don't analyze on every change in Phase 1 — wait for save
  0
}

handle_did_save(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  content = doc_get(uri)
  if slen(content) > 0 {
    run_diagnostics(uri, content)
  }
  0
}

handle_did_close(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  doc_remove(uri)
  // Clear diagnostics for closed file
  empty_diags = ja()
  rpc_notify("textDocument/publishDiagnostics", make_publish_diagnostics(uri, empty_diags))
  0
}

handle_hover(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  pos_encoded = params_position(params)
  line = pos_encoded / 1000000 + 1    // LSP 0-based → compiler 1-based
  col = pos_encoded % 1000000 + 1

  content = doc_get(uri)
  if slen(content) == 0 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  hover_text = lookup_hover_at(content, line, col)
  if slen(hover_text) == 0 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  rpc_respond(rpc_id(msg), make_lsp_hover("```sans\n" + hover_text + "\n```"))
  0
}

handle_definition(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  pos_encoded = params_position(params)
  line = pos_encoded / 1000000 + 1
  col = pos_encoded % 1000000 + 1

  content = doc_get(uri)
  if slen(content) == 0 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  def_loc = lookup_definition_at(content, line, col)
  if slen(def_loc) == 0 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  // Parse "line:col" from def_loc
  parts = def_loc.split(":")
  if parts.len() < 2 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }
  def_line = stoi(parts[0]) - 1  // compiler 1-based → LSP 0-based
  def_col = stoi(parts[1]) - 1

  loc = make_lsp_location(uri, def_line, def_col, def_line, def_col)
  rpc_respond(rpc_id(msg), loc)
  0
}

handle_signature_help(msg:J) I {
  params = rpc_params(msg)
  uri = params_uri(params)
  pos_encoded = params_position(params)
  line = pos_encoded / 1000000 + 1
  col = pos_encoded % 1000000 + 1

  content = doc_get(uri)
  if slen(content) == 0 {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  sig = lookup_signature_at(content, line, col)
  if sig.type_of() == "null" {
    rpc_respond(rpc_id(msg), jn())
    return 0
  }

  rpc_respond(rpc_id(msg), sig)
  0
}

handle_shutdown(msg:J) I {
  lsp_shutdown_received = 1
  rpc_respond(rpc_id(msg), jn())
  wfd(2, "sans-lsp: shutdown received\n")
  0
}

// ------ Analysis runner ------

run_diagnostics(uri:S content:S) I {
  // Run analysis — this sets lsp_last_diags and lsp_last_symbols
  analyze_file(uri, content)
  // Convert to LSP format and publish
  diags = get_diagnostics_json(uri)
  rpc_notify("textDocument/publishDiagnostics", make_publish_diagnostics(uri, diags))
  0
}

// ------ Main entry point ------

main() {
  // Disable scope GC for the LSP process — it's long-running
  // and manages its own memory via globals
  scope_disable()

  wfd(2, "sans-lsp: starting\n")

  // Main loop — read and dispatch JSON-RPC messages
  running := 1
  while running == 1 {
    msg = rpc_read()

    // Check for EOF (null message)
    if msg.type_of() == "null" {
      running = 0
    } else {
      method = rpc_method(msg)

      if method == "initialize" { handle_initialize(msg) }
      else if method == "initialized" { 0 }
      else if method == "textDocument/didOpen" { handle_did_open(msg) }
      else if method == "textDocument/didChange" { handle_did_change(msg) }
      else if method == "textDocument/didSave" { handle_did_save(msg) }
      else if method == "textDocument/didClose" { handle_did_close(msg) }
      else if method == "textDocument/hover" { handle_hover(msg) }
      else if method == "textDocument/definition" { handle_definition(msg) }
      else if method == "textDocument/signatureHelp" { handle_signature_help(msg) }
      else if method == "shutdown" { handle_shutdown(msg) }
      else if method == "exit" {
        if lsp_shutdown_received == 1 { exit(0) } else { exit(1) }
      }
      else {
        // Unknown method — if it's a request, respond with method not found
        if rpc_is_request(msg) == 1 {
          rpc_respond_error(rpc_id(msg), -32601, "Method not found: " + method)
        }
        0
      }
      0
    }
    0
  }
  0
}
```

- [ ] **Step 2: Commit**

```bash
git add lsp/main.sans
git commit -m "feat: add LSP main loop with full request dispatcher"
```

---

## Task 9: Integration Tests

**Files:**
- Create: `tests/lsp/test_lifecycle.sh`
- Create: `tests/lsp/test_diagnostics.sh`

Shell-based integration tests that spawn `sans-lsp` and send JSON-RPC messages.

- [ ] **Step 1: Create test directory**

```bash
mkdir -p tests/lsp
```

- [ ] **Step 2: Create `tests/lsp/test_lifecycle.sh`**

```bash
#!/bin/bash
# Test LSP lifecycle: initialize → initialized → shutdown → exit
set -euo pipefail

SANS_LSP="${1:-./sans-lsp}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
PASS=0
FAIL=0

send_msg() {
  local json="$1"
  local len=${#json}
  printf "Content-Length: %d\r\n\r\n%s" "$len" "$json"
}

echo "=== LSP Lifecycle Tests ==="

# Test 1: Initialize → shutdown → exit
echo -n "  initialize + shutdown + exit... "
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}'
SHUTDOWN='{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.2; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 5 "$SANS_LSP" 2>/dev/null || true)

if echo "$RESPONSE" | grep -q '"sans-lsp"'; then
  echo -e "${GREEN}✓${NC}"
  ((PASS++))
else
  echo -e "${RED}✗${NC} (no server info in response)"
  ((FAIL++))
fi

# Test 2: Verify capabilities in initialize response
echo -n "  server capabilities... "
if echo "$RESPONSE" | grep -q '"hoverProvider"'; then
  echo -e "${GREEN}✓${NC}"
  ((PASS++))
else
  echo -e "${RED}✗${NC} (missing hoverProvider)"
  ((FAIL++))
fi

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
```

- [ ] **Step 3: Create `tests/lsp/test_diagnostics.sh`**

```bash
#!/bin/bash
# Test LSP diagnostics: open a file with errors, verify diagnostics are published
set -euo pipefail

SANS_LSP="${1:-./sans-lsp}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
PASS=0
FAIL=0

send_msg() {
  local json="$1"
  local len=${#json}
  printf "Content-Length: %d\r\n\r\n%s" "$len" "$json"
}

echo "=== LSP Diagnostics Tests ==="

# Create a test file with an error
TEST_FILE="/tmp/sans_lsp_test_$$.sans"
cat > "$TEST_FILE" << 'SANS'
main() {
  x = undefined_var
  p(x)
}
SANS
TEST_URI="file://${TEST_FILE}"

# Test: Open file with error → expect diagnostics
echo -n "  diagnostics on didOpen... "
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"initialized","params":{}}'
CONTENT=$(cat "$TEST_FILE" | sed 's/"/\\"/g' | tr '\n' '\\' | sed 's/\\/\\n/g')
DID_OPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${TEST_URI}\",\"languageId\":\"sans\",\"version\":1,\"text\":\"$(cat "$TEST_FILE" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read())[1:-1])')\"}}}"
SHUTDOWN='{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
EXIT='{"jsonrpc":"2.0","method":"exit","params":null}'

RESPONSE=$(( send_msg "$INIT"; sleep 0.3; send_msg "$INITIALIZED"; sleep 0.1; send_msg "$DID_OPEN"; sleep 0.5; send_msg "$SHUTDOWN"; sleep 0.1; send_msg "$EXIT" ) | timeout 5 "$SANS_LSP" 2>/dev/null || true)

if echo "$RESPONSE" | grep -q 'publishDiagnostics'; then
  echo -e "${GREEN}✓${NC}"
  ((PASS++))
else
  echo -e "${RED}✗${NC} (no diagnostics published)"
  ((FAIL++))
fi

# Cleanup
rm -f "$TEST_FILE"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -gt 0 ]; then exit 1; fi
```

- [ ] **Step 4: Make test scripts executable**

```bash
chmod +x tests/lsp/test_lifecycle.sh tests/lsp/test_diagnostics.sh
```

- [ ] **Step 5: Commit**

```bash
git add tests/lsp/
git commit -m "feat: add LSP integration test scripts"
```

---

## Task 10: Build and Integration Verification

**Files:**
- Modify: `compiler/main.sans` (add runtime/io.sans to link list if needed)

Build the full LSP binary and verify it works end-to-end.

- [ ] **Step 1: Check runtime linking**

The compiler's `do_link()` function in `compiler/main.sans` lists all runtime `.sans` files to compile and link. Search for the list:

```bash
grep -n "runtime/" compiler/main.sans
```

Add `runtime/io.sans` to that list, following the same pattern as other runtime files.

- [ ] **Step 2: Build the compiler with the new builtins**

```bash
cd /home/scott/Development/sans-language/sans
sans build compiler/main.sans -o sans_new
```

- [ ] **Step 3: Build the LSP server**

```bash
./sans_new build lsp/main.sans -o sans-lsp
```

If this fails with import resolution errors, the LSP modules need the compiler modules on the import path. The LSP imports `lexer`, `parser`, `typeck`, `constants` — these live in `compiler/`. You may need to either:
- Build from the `compiler/` directory: `cd compiler && ../sans_new build ../lsp/main.sans -o ../sans-lsp`
- Or add the compiler directory to the import search path (check if `sans build` supports a `-I` or similar flag)
- Or use relative imports: `import "../compiler/lexer"` etc.

Check how the compiler currently resolves imports and adapt accordingly.

- [ ] **Step 4: Run stdin builtin tests**

```bash
echo "hello world" | ./sans_new run tests/fixtures/stdin_read_line.sans
# Expected output: hello world

echo -n "ABCDEFGH" | ./sans_new run tests/fixtures/stdin_read_bytes.sans
# Expected output: ABCDE
```

- [ ] **Step 5: Run LSP lifecycle test**

```bash
bash tests/lsp/test_lifecycle.sh ./sans-lsp
```

Expected: all tests pass.

- [ ] **Step 6: Run LSP diagnostics test**

```bash
bash tests/lsp/test_diagnostics.sh ./sans-lsp
```

Expected: diagnostics published for the error file.

- [ ] **Step 7: Run full existing test suite to verify no regressions**

```bash
bash tests/run_tests.sh
```

All existing tests must pass.

- [ ] **Step 8: Commit**

```bash
git add compiler/main.sans
git commit -m "feat: wire up LSP build and verify integration"
```

---

## Task 11: VSCode Extension Update

**Files:**
- Modify: `editors/vscode-sans/package.json`
- Modify: `editors/vscode-sans/src/extension.ts`

Replace the custom hover provider with an LSP client.

- [ ] **Step 1: Add `vscode-languageclient` dependency to `package.json`**

In `editors/vscode-sans/package.json`, add to `devDependencies` and add `dependencies`:

```json
{
  "dependencies": {
    "vscode-languageclient": "^9.0.0",
    "vscode-languageserver-protocol": "^3.17.0"
  },
  "devDependencies": {
    "@types/vscode": "^1.75.0",
    "typescript": "^5.0.0"
  }
}
```

Also add a configuration section in `contributes`:

```json
"configuration": {
  "type": "object",
  "title": "Sans",
  "properties": {
    "sans.lspPath": {
      "type": "string",
      "default": "sans-lsp",
      "description": "Path to the sans-lsp binary"
    }
  }
}
```

- [ ] **Step 2: Rewrite `extension.ts` to use LSP client**

Replace the contents of `editors/vscode-sans/src/extension.ts` with:

```typescript
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('sans');
    const lspPath = config.get<string>('lspPath', 'sans-lsp');

    const serverOptions: ServerOptions = {
        run: { command: lspPath, args: [] },
        debug: { command: lspPath, args: [] }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'sans' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.sans')
        }
    };

    client = new LanguageClient(
        'sans-lsp',
        'Sans Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
    context.subscriptions.push({
        dispose: () => {
            if (client) { client.stop(); }
        }
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) { return undefined; }
    return client.stop();
}
```

- [ ] **Step 3: Compile the extension**

```bash
cd editors/vscode-sans
npm install
npx tsc
```

- [ ] **Step 4: Commit**

```bash
git add editors/vscode-sans/package.json editors/vscode-sans/src/extension.ts
git commit -m "feat: update VSCode extension to use LSP client"
```

---

## Task 12: Documentation Updates

**Files:**
- Modify: `docs/reference.md`
- Modify: `docs/ai-reference.md`
- Modify: `editors/vscode-sans/src/extension.ts` (HOVER_DATA entries — if keeping as fallback)
- Modify: `editors/vscode-sans/syntaxes/sans.tmLanguage.json`

Per CLAUDE.md documentation checklist, update all docs for the new builtins.

- [ ] **Step 1: Add to `docs/ai-reference.md`**

In the "Functions (short | long)" section, add:

```
srl()          stdin_read_line()      -> S (read line from stdin)
srb(n)         stdin_read_bytes(n)    I -> S (read n bytes from stdin)
```

- [ ] **Step 2: Add to `docs/reference.md`**

Add a new subsection under I/O:

```markdown
### Stdin I/O

| Function | Alias | Description |
|----------|-------|-------------|
| `stdin_read_line()` | `srl()` | Read one line from stdin (blocking). Returns the line without trailing newline. |
| `stdin_read_bytes(n)` | `srb(n)` | Read exactly `n` bytes from stdin (blocking). |

These are used for building interactive programs and protocol servers (e.g., the LSP server).
```

- [ ] **Step 3: Add syntax highlighting for new builtins**

In `editors/vscode-sans/syntaxes/sans.tmLanguage.json`, find the builtin function pattern and add `stdin_read_line|srl|stdin_read_bytes|srb` to the list.

- [ ] **Step 4: Commit**

```bash
git add docs/reference.md docs/ai-reference.md editors/vscode-sans/syntaxes/sans.tmLanguage.json
git commit -m "docs: add stdin I/O builtins and LSP documentation"
```

---

## Task 13: Final Verification and Cleanup

- [ ] **Step 1: Full build from clean state**

```bash
cd /home/scott/Development/sans-language/sans
sans build compiler/main.sans -o sans_new
./sans_new build lsp/main.sans -o sans-lsp
```

- [ ] **Step 2: Run all test suites**

```bash
# Existing compiler tests
bash tests/run_tests.sh

# LSP integration tests
bash tests/lsp/test_lifecycle.sh ./sans-lsp
bash tests/lsp/test_diagnostics.sh ./sans-lsp

# Stdin builtin tests
echo "hello world" | ./sans_new run tests/fixtures/stdin_read_line.sans
echo -n "ABCDE" | ./sans_new run tests/fixtures/stdin_read_bytes.sans
```

- [ ] **Step 3: Manual smoke test**

Start the LSP manually and send a test message:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}' | \
  ( printf "Content-Length: %d\r\n\r\n" $(echo -n '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}' | wc -c); \
    echo -n '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp","capabilities":{}}}' ) | \
  timeout 3 ./sans-lsp 2>/dev/null
```

Expected: JSON-RPC response with server capabilities.

- [ ] **Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final LSP integration verification"
```

---

## Implementation Notes

**Import resolution for LSP modules:** The LSP modules (`lsp/*.sans`) import compiler modules (`compiler/*.sans`). The Sans compiler resolves imports relative to the source file. When building `lsp/main.sans`, imports like `import "lexer"` will look for `lsp/lexer.sans` first. This won't work — the compiler modules are in `compiler/`. Options:
1. Use relative imports: `import "../compiler/lexer"` (if supported)
2. Build from project root with explicit paths
3. Symlink compiler modules into `lsp/`
4. Copy the needed compiler `.sans` files

Check during Task 10 which approach works. The compiler's import resolution is in `compiler/main.sans` `resolve_import()` (line 98). Read it to understand the exact resolution rules.

**Scope GC in the LSP:** The main loop calls `scope_disable()` at startup. This means ALL allocations are manual — nothing gets automatically freed. This is safe for Phase 1 since the LSP process is short-lived (restarted by the editor). For Phase 2, selective scope management (enable/disable per handler) would prevent memory growth.

**Function signature layout in typeck:** The offsets used in `analyzer.sans` (e.g., `load64(sig + 8)` for return type) assume a specific memory layout. During implementation, verify by reading how `fn_env` entries are created in `typeck.sans` `check_inner()`. Search for where function definitions are added to `fn_env` and check the struct layout.

**Parser API:** The plan assumes `parse(src)` is the entry point that does lexing + parsing. Verify in `compiler/main.sans` line 182: `program = parse(src)`. If the parser has a different API, adapt `analyzer.sans` accordingly.
