# Website Redesign Spec 2: Content + Pages

**Goal:** Update all page content, restructure documentation, add new pages (404), add AI discoverability (llms.txt), and update benchmarks/token comparisons with the correct language ordering.

**Depends on:** Spec 1 (infrastructure + design system) must be implemented first.

---

## Homepage (index.html)

### Hero Section

- **Headline:** "Fewer tokens. Lower costs. Faster code."
- **Subheadline:** "The programming language built for teams that run AI at scale."
- **CTA buttons:** "Download Sans" (primary/Mauve) + "Documentation" (secondary)
- **Version line:** "Alpha — macOS — [all releases](link)"

### Feature Cards

4-card grid (2x2 on desktop, 1-column on mobile). Same four topics, updated copy:

1. **Token-Efficient** — Designed for AI code generation. Short type aliases (`I/S/B/F`), compact function syntax, no unnecessary keywords. Every token counts.
2. **Native Speed** — Compiles to machine code via LLVM. Performance on par with Rust and Go — up to 30x faster than Python on compute-heavy tasks.
3. **Batteries Included** — Built-in HTTP server/client, JSON, file I/O, concurrency with channels and mutexes, error handling with Result types. No package manager needed.
4. **Self-Hosted** — Sans compiles itself. The compiler (~11,600 LOC) and entire runtime are written in Sans — zero C. A fully bootstrapped stage 0→1→2→3 pipeline with fixed-point output.

### Token Efficiency Section

**Heading:** "Why Token-Efficiency Matters"
**Subheading:** "AI models pay per token. Sans uses 40-60% fewer tokens than Go or Rust for the same logic — meaning faster generation, lower cost, and fewer errors."

5-language tabbed code examples (tabs in order: Sans, Go, Rust, Node, Python). Three examples:

1. **Fibonacci** — recursive fib function. Show token count badges on each tab.
2. **JSON Roundtrip** — build/stringify/parse JSON object with 1000 keys.
3. **HTTP Server** — basic GET endpoint returning JSON.

Token counts for Node and Python need to be calculated and added. **Token counting methodology:** count discrete language tokens — identifiers, keywords, operators, literals, punctuation, delimiters. `n-1` = 3 tokens (`n`, `-`, `1`). `fib(n)` = 4 tokens (`fib`, `(`, `n`, `)`). This matches the existing counts on the site. Recount all languages for consistency.

### Performance Section

**Heading:** "Same Speed"

Benchmark table with columns in order: **Sans, Go, Rust, Node, Python**.

Benchmarks to show (subset of full benchmark suite for homepage):
- fib(35)
- loop_sum (1M)
- array_ops (100k)
- string_concat (100k)
- json_roundtrip

Use the latest benchmark data from `benchmarks/results.csv`.

### Quick Start Section

Keep the existing git clone + build + run code block, updated for the new install path (curl one-liner from download page).

---

## Documentation (docs/index.html)

### Restructured Section Order

Reorganize the existing 24 sections into a logical progression following standard language documentation patterns:

1. **Getting Started** — hello world, `sans build`, `sans run`, basic program structure
2. **Types** — `I`, `F`, `B`, `S`, short aliases, type annotations
3. **Variables** — immutable (`x = 42`), mutable (`x := 0`), globals (`g x = 0`)
4. **Functions** — compact syntax, expression bodies, multiple params, return types
5. **Control Flow** — if/else, ternary, while, for-in, match, break/continue
6. **Operators** — arithmetic, comparison, logical, compound assignment, bitwise
7. **Arrays** — literals, indexing, `push`, `map`, `filter`, `any`, `find`, `enumerate`, `zip`
8. **Maps** — `M()`, `set`, `get`, `has`, `keys`, `vals`
9. **Strings** — methods, interpolation, slicing, built-in aliases
10. **Tuples** — `(1 "hello" true)`, `.0` / `.1` access
11. **Structs** — definition, construction, field access, methods via `impl`
12. **Enums** — definition, variants with data, pattern matching
13. **Traits** — definition, implementation, trait bounds
14. **Generics** — generic functions, generic types
15. **Lambdas** — syntax, capture, passing as arguments
16. **Error Handling** — `Result<T>`, `ok`, `err`, `?` propagation, `!` unwrap
17. **Modules** — `import`, module resolution, exports
18. **Concurrency** — `spawn`, channels (`channel<I>()`, `send`, `recv`), `mutex`, join handles
19. **File I/O** — `file_read`/`fr`, `file_write`/`fw`, `file_append`/`fa`, `file_exists`/`fe`
20. **JSON** — `json_parse`/`jp`, `json_stringify`/`jfy`, `json_object`/`jo`, `json_array`/`ja`
21. **HTTP & Networking** — HTTP client (`hg`, `hp`), server (`serve`), TLS, WebSocket, CORS, streaming, static files
22. **Logging** — `log_debug`/`ld`, `log_info`/`li`, `log_warn`/`lw`, `log_error`/`le`
23. **Low-Level** — `alloc`, `load8`/`store8`/`load64`/`store64`, `mcpy`, sockets, curl, SSL, arena allocator
24. **Built-in Names** — full list of builtin function names and aliases
25. **Known Limitations** — no GC, relaxed typeck, closure limitations

Note: Iterators (for-in) are covered within Control Flow (item 5), not as a separate section — Sans does not have a dedicated iterator protocol beyond for-in loops over arrays.

### Sidebar TOC

Sticky sidebar (240px) listing all sections. Matches the restructured order above.

### Content

All existing documentation content is preserved — only the ordering changes. No content additions or removals in this spec (content improvements are ongoing work).

---

## Benchmarks (benchmarks/index.html)

### Column Order

All benchmark tables use the order: **Sans, Go, Rust, Node, Python**.

### Data

Use `benchmarks/results.csv` as the source of truth at implementation time. The values below are current as of spec writing — rerun benchmarks if they are stale:

| Benchmark | Sans | Go | Rust | Node | Python |
|-----------|------|-----|------|------|--------|
| fib | 49.4 | 80.1 | 42.7 | 109.8 | 1566.3 |
| loop_sum | 18.8 | 23.6 | 14.1 | 53.4 | 100.2 |
| array_ops | 17.6 | 25.6 | 14.4 | 58.7 | 54.0 |
| string_concat | 24.9 | 22.1 | 22.9 | 51.9 | 53.9 |
| json_roundtrip | 48.5 | 88.0 | 42.4 | 58.3 | 81.9 |
| concurrent | 19.0 | 24.1 | 14.0 | 83.2 | 81.5 |
| file_io | 34.4 | 23.4 | 13.9 | 52.1 | 41.5 |
| mixed | 17.2 | 23.9 | 15.0 | 51.8 | 50.8 |

HTTP throughput (req/sec): Sans 45,044 | Go 41,107 | Rust 45,447 | Node 44,493 | Python 44,577

### Speedup Table

Recompute speedup vs Python with the new column order.

### Sections

Keep all existing sections (results summary, speedup table, detailed per-benchmark tables, HTTP throughput, methodology, workloads) — just reorder columns.

---

## Download (download/index.html)

Existing content is fine. Minor updates:
- Ensure it uses the new design system components (`<sans-nav>`, `<sans-footer>`).
- Update install command if base path changes.

---

## Examples Audit

Review all files in `examples/` directory:
- Verify each example compiles and runs against the current compiler version.
- Remove stale artifacts (compiled binaries, `.o` files).
- Ensure examples reflect current Sans syntax and best practices.
- Each example on the website should have a corresponding file in `examples/`.

---

## Feature Card Styling

The 4-card grid uses a 2x2 layout on desktop, 1-column on mobile. Visual styling:
- Background: `var(--mantle)`.
- Border: `1px solid var(--surface0)`.
- Border-radius: 12px.
- Padding: 24px 28px.
- Subtle hover: lift with `box-shadow: 0 4px 12px rgba(0,0,0,0.08)` and `transform: translateY(-2px)`, transition 0.2s.
- Heading: 18px, font-weight 600, `var(--text)`.
- Body: 15px, `var(--subtext0)`, line-height 1.6.
- No icons — keep it clean and text-focused.

---

## 404 Page (404.html)

Simple, branded error page:
- Uses `<sans-nav>` and `<sans-footer>` components.
- Centered content: "404 — Page not found"
- Subtext: "The page you're looking for doesn't exist."
- Link: "Go to homepage" button (primary/Mauve).

Note: GitHub Pages serves `404.html` automatically for missing pages.

---

## llms.txt

Embed the full content of `docs/ai-reference.md` directly in `website/llms.txt`. This gives AI agents everything they need in one fetch when someone adds the site URL to Claude or another AI tool.

The file should be served as `text/plain` (GitHub Pages does this by default for `.txt` files).

Update `llms.txt` whenever `ai-reference.md` changes. This can be manual for now — automate later if needed.

---

## Token Count Additions

Add Node.js and Python to the three code comparison examples. For each example, write the equivalent code and count tokens using the same methodology as existing Go/Rust counts.

**Fibonacci:**
- Node: `function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2) }` — count tokens
- Python: `def fib(n): return n if n <= 1 else fib(n-1) + fib(n-2)` — count tokens

**JSON Roundtrip:**
- Node: equivalent using `JSON.stringify`/`JSON.parse` with object construction
- Python: equivalent using `json.dumps`/`json.loads` with dict construction

**HTTP Server:**
- Node: equivalent using `http.createServer`
- Python: equivalent using a minimal HTTP server (Flask or http.server)

Token counts should be calculated consistently — count language tokens (identifiers, operators, literals, keywords, punctuation), not LLM tokens.

---

## Release Workflow Updates

The release workflow's version-bump job must update `sed` paths to target:
- `website/index.html` (not `website/static/index.html`)
- `website/docs/index.html` (not `website/static/docs.html`)
- `website/benchmarks/index.html` (not `website/static/benchmarks.html`)
- `website/download/index.html` (not `website/static/download.html`)

The `sed` pattern changes from matching footer text to matching `<meta name="sans-version" content="...">`.

---

## Out of Scope

- Design system changes (covered in Spec 1)
- Deployment infrastructure (covered in Spec 1)
- Web components implementation (covered in Spec 1)
- New documentation content (only restructuring)
