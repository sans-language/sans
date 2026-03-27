# Sans LSP Server Design Spec

**Date:** 2026-03-26
**Version target:** 0.9.0
**Branch:** `lsp`
**Status:** Approved design, pending implementation

---

## Overview

A fully Sans-implemented Language Server Protocol (LSP) server (`sans-lsp`) providing Phase 1 features: diagnostics, hover, go-to-definition, and signature help. Written entirely in Sans to dogfood the language for long-running stateful applications.

No fallback. If Sans can't do it, we fix Sans.

## Architecture

```
┌─────────────┐    stdin (JSON-RPC)     ┌──────────────┐
│   Editor     │ ──────────────────────► │              │
│  (VSCode,    │                         │   sans-lsp   │
│   Neovim,    │ ◄────────────────────── │              │
│   etc.)      │    stdout (JSON-RPC)    └──────┬───────┘
└─────────────┘                                 │
                                                │ imports
                                    ┌───────────┼───────────┐
                                    │           │           │
                               ┌────▼───┐ ┌────▼───┐ ┌────▼────┐
                               │ lexer  │ │ parser │ │ typeck  │
                               └────────┘ └────────┘ └─────────┘
                                    (existing compiler modules)
```

**Flow:**
1. Editor launches `sans-lsp` as a child process
2. Editor sends JSON-RPC messages over stdin
3. `sans-lsp` reads messages via buffered stdin reader (new runtime primitive)
4. For analysis, runs compiler pipeline (lexer → parser → typeck) — no IR or codegen
5. Results serialized as JSON-RPC responses, written to stdout
6. Logging/debug goes to stderr only — stdout is the protocol channel

**Constraints:**
- Stdout is sacred — only JSON-RPC messages
- Full recompile per analysis run (Phase 1)
- Document state held in memory as globals (survives scope GC)
- Project root from `initialize` request's `rootUri`

---

## Language Additions

### Buffered Stdin Primitives

Sans currently lacks buffered stdin reading. The LSP protocol requires reading HTTP-style headers line-by-line followed by exact byte counts of JSON. Two new runtime builtins:

| Builtin | Alias | Signature | Description |
|---------|-------|-----------|-------------|
| `stdin_read_line()` | `srl()` | `() S` | Read one line from stdin (up to `\n`), blocking. Returns the line without the trailing `\n`. |
| `stdin_read_bytes(n)` | `srb(n)` | `(I) S` | Read exactly N bytes from stdin, blocking until all received. |

**Implementation:** Built on `srecv(0, buf, len)` with an internal static buffer that accumulates bytes and splits on boundaries. The buffer is a global allocation — scope GC must not free it.

**Pipeline:** typeck → constants → IR → codegen → `runtime/io.sans`

These are general-purpose primitives, not LSP-specific. They go in a new `runtime/io.sans` module.

### Documentation Requirements

Per CLAUDE.md checklist, both builtins get entries in:
- `docs/reference.md`
- `docs/ai-reference.md`
- `website/docs/index.html`
- `editors/vscode-sans/src/extension.ts` (HOVER_DATA)
- `editors/vscode-sans/syntaxes/sans.tmLanguage.json`

---

## Compiler Modifications

### Collect Mode for Diagnostics

The compiler currently prints errors and calls `exit(1)`. The LSP needs errors returned as data.

**Approach:** Add a global flag in the compiler modules that switches behavior:

- **Normal mode (default):** Print errors, exit on fatal — unchanged behavior for `sans build`
- **Collect mode:** Push errors to a global array, continue past errors where possible, never exit

Modules affected:
- `compiler/lexer.sans` — lexer errors (unterminated strings, invalid tokens)
- `compiler/parser.sans` — parse errors (unexpected tokens, malformed expressions)
- `compiler/typeck.sans` — type errors (already collects multiple errors before exiting)

The flag is set by the analyzer before calling into the compiler. After the pass completes, the analyzer reads the collected diagnostics array.

**Diagnostic format:**
```
struct Diagnostic {
    file S,
    line I,
    col I,
    end_line I,
    end_col I,
    message S,
    severity I
}
```

Severity: `1` = error, `2` = warning.

### Symbol Table Access

Typeck already builds internal maps of:
- Functions: name, parameter names/types, return type, source location
- Structs: name, field names/types, source location
- Enums: name, variant names/data, source location
- Traits: name, method signatures, source location
- Variables: name, type, scope, source location
- Imports: module name, resolved path, exported symbols

The analyzer needs read access to these after typeck completes. Expose them as globals that the analyzer module can import and iterate.

**Symbol entry format:**
```
struct Symbol {
    name S,
    kind S,
    type_str S,
    file S,
    line I,
    col I,
    params S,
    doc S
}
```

`kind` is one of: `"function"`, `"struct"`, `"enum"`, `"trait"`, `"variable"`, `"field"`, `"method"`, `"module"`.

---

## LSP Module Structure

All LSP code lives in `lsp/` at the top level of the project:

```
lsp/
  main.sans          # entry point, JSON-RPC main loop, dispatcher
  protocol.sans      # LSP type definitions (Position, Range, Diagnostic, etc.)
  rpc.sans           # JSON-RPC message framing (read/write Content-Length messages)
  analyzer.sans      # wrapper around compiler pipeline (lexer → parser → typeck)
  symbols.sans       # symbol table queries (hover, go-to-def, signature help)
```

### `rpc.sans` — JSON-RPC Message Framing

**Reading inbound messages (stdin):**
1. Call `srl()` to read the `Content-Length: N\r\n` header
2. Parse the integer N from the header string
3. Call `srl()` to consume the blank `\r\n` separator line
4. Call `srb(N)` to read exactly N bytes of JSON body
5. Call `jp()` (json_parse) to get a `JsonValue`
6. Return a struct: `struct RpcMessage { id J, method S, params J }`

**Writing outbound messages (stdout):**
1. Build response `JsonValue` using `jo()`, `js()`, `ji()`, etc.
2. Serialize with `jfy()` (json_stringify)
3. Compute byte length of serialized JSON
4. Write `"Content-Length: {len}\r\n\r\n{json}"` to stdout via `wfd(1, ...)`

**Request vs Notification:**
- Requests have an `id` field — response must include matching `id`
- Notifications have no `id` — no response sent
- Dispatcher checks `id` presence and routes accordingly

### `protocol.sans` — LSP Type Definitions

Struct definitions for LSP protocol types. Manual JSON serialization/deserialization for each (no reflection in Sans).

**Core types (flattened — Sans structs don't nest by reference, so ranges are inlined as fields):**
```
struct Position { line I, character I }
struct Range { start_line I, start_char I, end_line I, end_char I }
struct Location { uri S, range_start_line I, range_start_char I, range_end_line I, range_end_char I }
struct LspDiagnostic { range_start_line I, range_start_char I, range_end_line I, range_end_char I, severity I, message S, source S }
struct TextDocumentIdentifier { uri S }
struct TextDocumentPositionParams { uri S, line I, character I }
struct SignatureHelp { label S, params J, active_param I }
```

**JSON conversion functions:**
- `position_to_json(p:Position) J` — serialize Position to JSON object
- `range_to_json(...) J` — serialize Range
- `diagnostic_to_json(d:LspDiagnostic) J` — serialize Diagnostic
- `location_to_json(l:Location) J` — serialize Location
- `parse_text_document_position(params:J) TextDocumentPositionParams` — deserialize from JSON

Each type has a `to_json` and `from_json` pair. Tedious but straightforward.

### `analyzer.sans` — Compiler Pipeline Wrapper

**Public API:**
```
analyze(path:S root:S) AnalysisResult
```

**`AnalysisResult` contains:**
- `diagnostics` — array of Diagnostic structs
- `symbols` — map of symbol name → Symbol struct
- `positions` — map of `"file:line:col"` → Symbol (for position-based lookup)

**Steps:**
1. Set compiler collect mode flag to `true`
2. Clear any previous diagnostics/symbol arrays
3. Run lexer on the file → get tokens
4. Run parser on tokens → get AST
5. Run typeck on AST with project root for import resolution → get type-checked AST + symbol tables
6. Set collect mode flag back to `false`
7. Build `AnalysisResult` from collected diagnostics and symbol tables
8. Return result

**Scope GC strategy:** The analysis runs in a function scope. Temp data (tokens, AST nodes) are freed when `analyze()` returns. The returned `AnalysisResult` is extracted to a global before the scope exits, so it survives.

**Multi-file handling:** The compiler already resolves `import` statements relative to the source file. The analyzer passes the project root so typeck resolves all imports. The symbol table spans all resolved modules.

### `symbols.sans` — Symbol Table Queries

Functions that query the last analysis result to answer LSP requests:

**`lookup_hover(uri:S line:I col:I) O<S>`**
- Run lexer on the document to find which token spans (line, col)
- Look up that token name in the symbol table
- Return formatted type info string, or `none()` if nothing found
- Format examples: `"f(x:I y:S) S"`, `"struct Point { x I, y I }"`, `"x : I — local variable"`

**`lookup_definition(uri:S line:I col:I) O<Location>`**
- Same token lookup as hover
- Return the Location where that symbol was defined
- Works cross-file — imported symbols point to their source file

**`lookup_signature(uri:S line:I col:I) O<SignatureHelp>`**
- Find the function call context at the cursor position
- Look up function name in symbol table
- Count commas before cursor to determine active parameter index
- Return parameter list and active parameter

### `main.sans` — Entry Point

**Initialization sequence:**
1. Wait for `initialize` request
2. Extract `rootUri` from params (project root)
3. Respond with server capabilities:
   ```json
   {
     "capabilities": {
       "textDocumentSync": 1,
       "hoverProvider": true,
       "definitionProvider": true,
       "signatureHelpProvider": {
         "triggerCharacters": ["(", ","]
       }
     }
   }
   ```
4. Wait for `initialized` notification
5. Enter main loop

**Main loop:**
```
loop {
    msg = rpc_read()
    method = msg.method

    match method {
        "initialize"                    => handle_initialize(msg)
        "initialized"                   => noop
        "textDocument/didOpen"          => handle_did_open(msg)
        "textDocument/didChange"        => handle_did_change(msg)
        "textDocument/didSave"          => handle_did_save(msg)
        "textDocument/didClose"         => handle_did_close(msg)
        "textDocument/hover"            => handle_hover(msg)
        "textDocument/definition"       => handle_definition(msg)
        "textDocument/signatureHelp"    => handle_signature_help(msg)
        "shutdown"                      => handle_shutdown(msg)
        "exit"                          => exit(shutdown_received ? 0 : 1)
        _                               => ignore
    }
}
```

**Error handling:**
- Malformed JSON → log to stderr, continue
- Unknown method → ignore (per LSP spec)
- Analysis failure → send empty diagnostics, log to stderr
- Main loop wrapped in panic recovery (`setjmp`/`longjmp`) — a bad document never crashes the server

**Shutdown:**
- `shutdown` request → respond `null`, set flag
- `exit` notification → `exit(0)` if shutdown received, `exit(1)` otherwise

---

## Document State Management

**Document store:** Global `Map<S, J>` keyed by file URI. Each entry holds document content, version, and last diagnostics as a JSON value.

**Sync mode:** Full document sync (`textDocumentSync: 1`). On `didChange`, the editor sends the entire file content. No incremental diff handling needed for Phase 1.

**Analysis trigger:** `didSave` only for Phase 1. Full recompile on every keystroke would be too slow. Analyze on save, push diagnostics immediately.

**Temp file strategy:** On analysis, write document content to a temp file, run the compiler on it, clean up. The compiler reads files from disk — no API to parse from a string. Temp files use random names to prevent races.

**Scope GC strategy:**
- Document store is a global — scope GC doesn't touch it
- Each message handler runs in function scope — temps freed on return
- Analysis results extracted to globals before scope exit

---

## LSP Feature Details

### 1. Diagnostics (`textDocument/publishDiagnostics`)

- **Trigger:** `didSave` notification
- **Process:** Run `analyze(path, root)` → map diagnostics to LSP format
- **Severity mapping:** Compiler errors → `1` (Error), warnings → `2` (Warning)
- **Output:** Push `textDocument/publishDiagnostics` notification (no request ID)
- **Clear on fix:** Sending an empty diagnostics array clears previous markers

### 2. Hover (`textDocument/hover`)

- **Input:** File URI, line, character (cursor position)
- **Process:** `lookup_hover(uri, line, col)` → find token at position → find symbol in table
- **Output:** Markdown-formatted type signature, or `null` if no symbol found
- **Examples:**
  - Function: `f(x:I y:S) S`
  - Struct: `struct Point { x I, y I }`
  - Variable: `x : I`
  - Enum: `enum Color { Red, Green, Blue(I) }`

### 3. Go-to-Definition (`textDocument/definition`)

- **Input:** File URI, line, character
- **Process:** `lookup_definition(uri, line, col)` → find symbol → return definition location
- **Output:** `Location { uri, range }` pointing to where the symbol was defined
- **Cross-file:** Imported symbols resolve to their source file and position
- **Not found:** Return `null`

### 4. Signature Help (`textDocument/signatureHelp`)

- **Trigger:** Typing `(` or `,` inside a function call
- **Input:** File URI, line, character
- **Process:** `lookup_signature(uri, line, col)` → find enclosing function call → look up params
- **Output:** Function label, parameter list, active parameter index
- **Active param:** Determined by counting commas before cursor position

---

## Editor Integration

### VSCode Extension Update

The existing extension at `editors/vscode-sans/` is updated to:

1. Add a `LanguageClient` that launches `sans-lsp` via stdio
2. Configure the binary path via a VSCode setting (`sans.lspPath`, default: `sans-lsp` on `$PATH`)
3. Remove custom `HOVER_DATA` hover provider — the LSP handles hover now
4. Keep TextMate grammar (`sans.tmLanguage.json`) for syntax highlighting — LSP doesn't replace this
5. Keep any non-hover extension features that don't conflict with LSP

**Dependencies:** Add `vscode-languageclient` npm package to the extension.

### Other Editors

Configuration examples shipped in `editors/`:
- **Neovim:** `lspconfig` entry with `cmd = {"sans-lsp"}`
- **Emacs:** `eglot` or `lsp-mode` config snippet
- **JetBrains:** Generic LSP plugin setup instructions

All editors get identical features via the shared `sans-lsp` binary.

---

## Testing Strategy

### Analyzer Unit Tests (`tests/lsp/analyzer/`)
- Feed `.sans` fixtures to `analyze()`, assert expected diagnostics
- Verify symbol table contains expected entries with correct positions
- Test cross-module analysis (main.sans importing other files)
- Test error recovery — files with multiple errors produce all diagnostics

### JSON-RPC Integration Tests (`tests/lsp/integration/`)
- Test harness spawns `sans-lsp` as subprocess
- Send raw JSON-RPC messages to stdin, read/parse responses from stdout
- Full lifecycle test: initialize → didOpen → didSave → hover → definition → shutdown → exit
- Verify correct `Content-Length` framing on all responses
- Test error cases: malformed JSON, unknown methods, hover on empty position

### Regression Tests (`tests/lsp/regression/`)
- Files with syntax errors → correct diagnostic positions
- Files with type errors → correct error messages
- Files with missing imports → import error diagnostic
- Valid files → zero diagnostics
- Edge cases: empty files, files with only comments, deeply nested expressions

### Stability Tests (`tests/lsp/stability/`)
- Open/close/edit many documents in a loop (1000+ cycles)
- Verify process RSS stays bounded (no memory leak)
- Verify no crash after thousands of requests
- This is the scope GC stress test for long-running Sans processes

---

## Implementation Order

Layered by risk — highest risk first:

### Layer 1: Foundation (highest risk)
1. Implement `stdin_read_line()` / `srl()` and `stdin_read_bytes(n)` / `srb(n)` in runtime
2. Add builtins to compiler pipeline (typeck → constants → IR → codegen)
3. Write `lsp/rpc.sans` — JSON-RPC message framing
4. Write `lsp/protocol.sans` — LSP type definitions and JSON conversion
5. Write `lsp/main.sans` — initialization handshake and main loop skeleton
6. Test: can the LSP start, receive `initialize`, respond, and not crash?

### Layer 2: Analysis Engine (medium risk)
7. Modify compiler modules for collect mode (lexer, parser, typeck)
8. Expose symbol table globals from typeck
9. Write `lsp/analyzer.sans` — compiler pipeline wrapper
10. Write `lsp/symbols.sans` — symbol table query functions
11. Implement diagnostics (didSave → analyze → publishDiagnostics)
12. Implement hover (position lookup → symbol info → formatted response)
13. Test: do diagnostics and hover work end-to-end?

### Layer 3: Complete Features (low risk)
14. Implement go-to-definition
15. Implement signature help
16. Update VSCode extension to use LSP
17. Write editor config examples for Neovim/Emacs/JetBrains
18. Full integration and stability test suite

### Layer 4: Documentation
19. Update `docs/reference.md` with new runtime builtins
20. Update `docs/ai-reference.md`
21. Update website docs
22. Update hover docs and syntax highlighting for new builtins

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Scope GC frees long-lived document state | LSP crashes or loses data | Store all persistent state in globals; test with stability suite |
| Scope GC leaks memory in long-running loop | RSS grows unbounded | Each handler runs in function scope; stability test monitors RSS |
| Compiler exit() kills LSP on bad input | Server dies on first error | Collect mode flag prevents exit; panic recovery wraps handlers |
| Buffered stdin reader doesn't work reliably | LSP can't read messages | Test extensively; `srecv()` on FD 0 is well-tested in HTTP server code |
| Full recompile too slow for large projects | Laggy diagnostics on save | Acceptable for Phase 1; incremental parsing is Phase 2 |
| Temp file writes slow on large documents | Noticeable delay | Temp files are fast for typical Sans file sizes (<1000 LOC) |
| Cross-module symbol resolution incomplete | Go-to-def fails on imports | Compiler already resolves imports; test with multi-file projects |

---

## Out of Scope (Phase 2 / v0.10)

- Incremental/partial parsing (full recompile is Phase 1)
- `didChange` analysis (save-only for Phase 1)
- Autocomplete
- Find all references
- Rename symbol
- Code actions / quick fixes
- Formatting integration (`sans fmt` is a separate tool)
- Semantic tokens (syntax highlighting via LSP)
- Workspace symbols
- Folding ranges

---

## Success Criteria

- `sans-lsp` starts, initializes, and enters main loop without crashing
- Diagnostics appear in editor on save with correct positions and messages
- Hover shows type signatures for functions, structs, enums, variables
- Go-to-definition jumps to the correct source location, including cross-file
- Signature help shows parameter names/types with correct active parameter
- VSCode extension works with `sans-lsp` out of the box
- Stability test: 1000+ request cycles with bounded memory
- Zero regressions in existing compiler tests (`bash tests/run_tests.sh`)
