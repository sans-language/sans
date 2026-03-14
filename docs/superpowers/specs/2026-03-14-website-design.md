# Sans Documentation Website — Design Spec

**Goal:** Build a documentation website served by Sans itself, showcasing the language's HTTP capabilities while providing docs and benchmark results.

## Scope

Three deliverables in dependency order:
1. **Prerequisite: `ends_with` string method** — needed for MIME type detection
2. **HTTP server extension** — add custom content-type support to `respond()`
3. **Website** — static HTML site served by a Sans program

## Prerequisite: `ends_with` String Method

Same pipeline as existing `starts_with`:
- `crates/sans-typeck/src/lib.rs` — add `ends_with` to `MethodCall` on String type, takes String arg, returns Bool
- `crates/sans-ir/src/ir.rs` — add `StringEndsWith` instruction
- `crates/sans-ir/src/lib.rs` — lower method call to instruction
- `crates/sans-codegen/src/lib.rs` — declare `cy_string_ends_with` extern, compile instruction
- `runtime/string_ext.c` — implement `cy_string_ends_with(const char*, const char*)`

Short alias: `ew` (matching `sw` pattern for `starts_with`)

## HTTP Server Extension

Add optional content-type parameter to `cy_http_respond()`.

**Sans API:**
- `req.respond(200, body)` — defaults to `text/html; charset=utf-8` (backwards compatible)
- `req.respond(200, body, "text/css")` — custom content type

**Implementation approach:** Single C function `cy_http_respond` with 4 parameters (request, status, body, content_type). When content_type is NULL, defaults to `text/html; charset=utf-8`.

**Pipeline changes:**
- `runtime/server.c` — `cy_http_respond()` gains `const char* content_type` param (NULL = default). Increase header buffer from 512 to 1024 bytes to accommodate custom content-type strings.
- `crates/sans-typeck/src/lib.rs` — allow 2 or 3 args on `respond` method (3rd arg must be String)
- `crates/sans-ir/src/ir.rs` — add `HttpRespondWithContentType` instruction variant
- `crates/sans-ir/src/lib.rs` — lower 3-arg respond to `HttpRespondWithContentType`, 2-arg to existing `HttpRespond`
- `crates/sans-codegen/src/lib.rs` — both IR instructions call the same C function; 2-arg passes NULL for content_type

**Tests:**
- Typeck unit test: 2-arg and 3-arg respond both pass; wrong-type 3rd arg fails
- E2E test fixture: `.sans` file exercising 3-arg respond

## Website Structure

```
website/
├── main.sans          # server, routing, MIME detection, file serving
├── static/
│   ├── index.html     # home page
│   ├── docs.html      # language reference
│   ├── benchmarks.html # benchmark results
│   └── style.css      # shared dark-theme stylesheet
```

**Working directory:** The server must be run from the `website/` directory so relative paths resolve correctly (e.g., `sans run main.sans` from within `website/`).

## Router (in main.sans)

Simple path-based routing using `starts_with` and exact `==` matching:
- `path == "/"` → serve `static/index.html`
- `path == "/docs"` → serve `static/docs.html`
- `path == "/benchmarks"` → serve `static/benchmarks.html`
- `path.starts_with("/static/")` → extract sub-path, serve from `static/` directory
- `path == "/favicon.ico"` → 404 (graceful, avoids log noise)
- anything else → 404 page

**Static file serving pattern:**
1. Construct file path: `"static" + path` (where path starts with `/static/...`, strip prefix to get `"static/" + subpath`)
2. Check `file_exists(path)` — if false, respond 404
3. Detect MIME type using `ends_with` on the path
4. `file_read(path)` and `respond(200, content, mime_type)`

**Path traversal mitigation:** Reject any path containing `".."` via `path.contains("..")` check before file serving.

**MIME type detection** by file extension using `ends_with`:
- `.html` → `text/html; charset=utf-8`
- `.css` → `text/css`
- `.js` → `application/javascript`
- `.png` → `image/png`
- `.ico` → `image/x-icon`
- fallback → `text/plain`

## Pages

**Home (`index.html`):**
- Tagline: "A general purpose language created by AI, for AI"
- 3 selling points: token-efficient syntax, native LLVM speed, built-in HTTP/JSON/concurrency
- Side-by-side syntax comparison (Sans vs verbose equivalent)
- Nav links to /docs and /benchmarks

**Docs (`docs.html`):**
- Language reference content pre-converted from `docs/reference.md` to HTML
- Code examples in styled `<pre>` blocks

**Benchmarks (`benchmarks.html`):**
- Results and speedup tables from `benchmarks/README.md` converted to HTML
- Methodology section

**Style (`style.css`):**
- Dark theme, developer-friendly
- Monospace code blocks
- Responsive layout
- Single file, no frameworks

## Constraints

- No JS frameworks or build tools
- Static HTML files read from disk at request time via `file_read()`
- Content is pre-converted to HTML (no markdown rendering in Sans)
- Server runs until stopped (no request count limit like the example)
- Backwards compatible — existing 2-arg `respond()` still works
- No file caching — every request reads from disk (future optimization area)

## Known Limitations

- `cy_http_accept` uses 8192-byte read buffer — large request headers may truncate
- No keep-alive — each response closes the connection
- No GC — long-running server will leak memory from string allocations
