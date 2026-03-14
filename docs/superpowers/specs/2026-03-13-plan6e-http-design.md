# Plan 6e: HTTP Client Design Spec

## Goal

Add HTTP client support via an opaque `HttpResponse` built-in type backed by a C runtime library using libcurl. Users can make GET and POST requests, inspect status codes, read response bodies, and check response headers. No new syntax — everything uses existing function call and method call syntax.

## Scope

**Request functions (free functions):**
- `http_get(url)` — perform GET request, returns HttpResponse
- `http_post(url, body, content_type)` — perform POST request with body and content type, returns HttpResponse

**Response methods (on HttpResponse):**
- `.status()` — HTTP status code as Int (200, 404, etc.)
- `.body()` — response body as String
- `.header(name)` — get response header value by name (case-insensitive)
- `.ok()` — true if status is 2xx

**Out of scope (deferred):** PUT/DELETE/PATCH methods, custom request headers, timeouts, authentication, cookies, redirects control (libcurl follows redirects by default), streaming responses, async/concurrent requests, HTTPS certificate configuration, proxy support, multipart form data, file upload/download.

## Decisions

- **Opaque built-in type.** `HttpResponse` is a new type like `JsonValue`. Users never see the internal representation. All access goes through built-in functions and methods.
- **C runtime library with libcurl.** HTTP logic lives in `runtime/http.c`, compiled separately and linked alongside the user's object file. Uses libcurl's easy API for synchronous requests.
- **Sentinel values on error.** If a request fails (network error, DNS failure, etc.), `http_get`/`http_post` return an HttpResponse with status 0, empty body, and no headers. No panics, no crashes.
- **libcurl dependency.** The runtime links against `-lcurl`. libcurl is available by default on macOS (via CommandLineTools SDK) and most Linux distributions. The `cc` link step adds `-lcurl`.
- **Follow redirects.** libcurl's `CURLOPT_FOLLOWLOCATION` is enabled by default, following up to 10 redirects.
- **No new keywords or syntax.** All functions are built-in function names. All accessors use existing method call syntax.
- **Linking change.** The `cc` invocation in `main.rs` and both e2e test helpers must compile `runtime/http.c` and link with `-lcurl`.

## Runtime Representation

An `HttpResponse` in Sans is a pointer (stored as i64) to a heap-allocated C struct:

```c
typedef struct CyHttpResponse {
    long status_code;           // HTTP status code (0 on error)
    char* body;                 // malloc'd response body (null-terminated)
    long body_len;              // body length in bytes
    char** header_names;        // malloc'd array of lowercase header names
    char** header_values;       // malloc'd array of header values
    long header_count;          // number of headers
    long header_cap;            // capacity of header arrays
} CyHttpResponse;
```

The struct is entirely internal. Sans code only interacts with it via the C functions declared in codegen.

## Syntax

```cyflym
fn main() Int {
    // Simple GET
    let resp = http_get("https://httpbin.org/get")
    if resp.ok() {
        print(resp.body())
    }

    // Check status
    let status = resp.status()

    // Read a header
    let content_type = resp.header("content-type")
    print(content_type)

    // POST with JSON body
    let post_resp = http_post(
        "https://httpbin.org/post",
        "{\"key\":\"value\"}",
        "application/json"
    )
    print(post_resp.body())

    status
}
```

## Type System

### New Type Variant

Add `HttpResponse` to the `Type` enum in `crates/sans-typeck/src/types.rs`. The `Display` impl should format it as `"HttpResponse"`.

### Type Checking Rules

| Expression | Args | Returns |
|---|---|---|
| `http_get(url)` | `url: String` | `HttpResponse` |
| `http_post(url, body, content_type)` | `url: String, body: String, content_type: String` | `HttpResponse` |

### Method Checking Rules (on `Type::HttpResponse`)

| Method | Args | Returns |
|---|---|---|
| `.status()` | none | `Int` |
| `.body()` | none | `String` |
| `.header(name)` | `name: String` | `String` |
| `.ok()` | none | `Bool` |

### Type Errors

- Wrong argument count -> existing "expected N arguments" pattern
- Wrong argument type -> existing "expected String" pattern
- Method call on non-HttpResponse -> existing "no method" pattern

## IR Changes

### New IrType Variant

```rust
IrType::HttpResponse
```

The `ir_type_for_return` helper in `crates/sans-ir/src/lib.rs` must map `Type::HttpResponse` to `IrType::HttpResponse`.

### New Instructions

**Request functions:**
```rust
HttpGet { dest: Reg, url: Reg },                              // http_get(url)
HttpPost { dest: Reg, url: Reg, body: Reg, content_type: Reg }, // http_post(url, body, ct)
```

**Response accessors:**
```rust
HttpStatus { dest: Reg, response: Reg },      // .status()
HttpBody { dest: Reg, response: Reg },         // .body()
HttpHeader { dest: Reg, response: Reg, name: Reg }, // .header(name)
HttpOk { dest: Reg, response: Reg },           // .ok()
```

### IR Lowering

In `lower_expr` for `Expr::Call`, add cases for `"http_get"` and `"http_post"` matching the existing built-in function pattern.

Method calls on `IrType::HttpResponse` lower to the corresponding accessor instructions.

`dest` register types: `HttpGet`/`HttpPost` -> `IrType::HttpResponse`. `HttpBody`/`HttpHeader` -> `IrType::Str`. `HttpStatus` -> `IrType::Int`. `HttpOk` -> `IrType::Bool`.

## Codegen Changes

### External Function Declarations

All functions from `runtime/http.c`:

```
declare i8* @cy_http_get(i8*)                  ; CyHttpResponse* cy_http_get(const char* url)
declare i8* @cy_http_post(i8*, i8*, i8*)       ; CyHttpResponse* cy_http_post(const char* url, const char* body, const char* content_type)
declare i64 @cy_http_status(i8*)               ; long cy_http_status(CyHttpResponse*)
declare i8* @cy_http_body(i8*)                 ; char* cy_http_body(CyHttpResponse*)
declare i8* @cy_http_header(i8*, i8*)          ; char* cy_http_header(CyHttpResponse*, const char* name)
declare i64 @cy_http_ok(i8*)                   ; long cy_http_ok(CyHttpResponse*)
```

### Instruction Compilation

Each HTTP IR instruction compiles to a single C function call. No branching, no phi nodes — the C functions handle all error paths internally.

**Request functions:** Call `cy_http_get` / `cy_http_post`, get back pointer, store as i64 in `regs` and as pointer in `ptrs`.

**Accessors returning String:** Call `cy_http_body` / `cy_http_header`, store pointer in both `regs` and `ptrs`.

**Accessors returning Int:** Call `cy_http_status`, store i64 in `regs`.

**Accessors returning Bool:** Call `cy_http_ok`, store i64 in `regs`.

### Linking Change

**`main.rs`:** Before invoking `cc` to link, compile `runtime/http.c` to a temporary `http.o`:
```
cc -c runtime/http.c -o /tmp/cyflym_http_runtime.o
cc user.o /tmp/cyflym_json_runtime.o /tmp/cyflym_http_runtime.o -lcurl -o binary
```

**E2E test helpers:** Both `compile_and_run` and `compile_and_run_dir` in `crates/sans-driver/tests/e2e.rs` need updating. The `http.c` path resolves via `CARGO_MANIFEST_DIR/../../runtime/http.c`. Compile it to a temp `http.o` and include in the `cc` link step with `-lcurl`.

## C Runtime Library (`runtime/http.c`)

### Response Construction

```c
static CyHttpResponse* make_error_response(void) {
    CyHttpResponse* resp = malloc(sizeof(CyHttpResponse));
    resp->status_code = 0;
    resp->body = malloc(1);
    resp->body[0] = '\0';
    resp->body_len = 0;
    resp->header_names = NULL;
    resp->header_values = NULL;
    resp->header_count = 0;
    resp->header_cap = 0;
    return resp;
}
```

### Write Callback (for response body)

libcurl delivers response data in chunks via a callback. The callback appends to a growing buffer:

```c
typedef struct {
    char* data;
    long size;
    long cap;
} WriteBuffer;

static size_t write_callback(char* ptr, size_t size, size_t nmemb, void* userdata) {
    WriteBuffer* buf = (WriteBuffer*)userdata;
    long bytes = size * nmemb;
    // grow if needed
    while (buf->size + bytes + 1 > buf->cap) {
        buf->cap = buf->cap ? buf->cap * 2 : 1024;
        buf->data = realloc(buf->data, buf->cap);
    }
    memcpy(buf->data + buf->size, ptr, bytes);
    buf->size += bytes;
    buf->data[buf->size] = '\0';
    return bytes;
}
```

### Header Callback (for response headers)

libcurl delivers headers one at a time via a separate callback:

```c
static size_t header_callback(char* buffer, size_t size, size_t nitems, void* userdata) {
    CyHttpResponse* resp = (CyHttpResponse*)userdata;
    long len = size * nitems;
    // Skip status line and empty lines
    // Find ':' separator
    // Extract name (lowercase) and value (trimmed)
    // Append to resp->header_names and resp->header_values
    return len;
}
```

### GET Request

```c
CyHttpResponse* cy_http_get(const char* url) {
    CURL* curl = curl_easy_init();
    if (!curl) return make_error_response();

    CyHttpResponse* resp = malloc(sizeof(CyHttpResponse));
    // init resp fields...
    WriteBuffer buf = {NULL, 0, 0};

    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, header_callback);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, resp);

    CURLcode res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
        curl_easy_cleanup(curl);
        free(buf.data);
        free(resp);
        return make_error_response();
    }

    long status;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    resp->status_code = status;
    resp->body = buf.data ? buf.data : strdup("");
    resp->body_len = buf.size;

    curl_easy_cleanup(curl);
    return resp;
}
```

### POST Request

Same as GET but with:
```c
curl_easy_setopt(curl, CURLOPT_POST, 1L);
curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body);
curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, strlen(body));
// Set Content-Type header
struct curl_slist* headers = NULL;
char header_buf[256];
snprintf(header_buf, sizeof(header_buf), "Content-Type: %s", content_type);
headers = curl_slist_append(headers, header_buf);
curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
// ... perform, cleanup headers list ...
```

### Accessor Functions

```c
long cy_http_status(CyHttpResponse* resp) { return resp->status_code; }

char* cy_http_body(CyHttpResponse* resp) { return resp->body; }

char* cy_http_header(CyHttpResponse* resp, const char* name) {
    // Case-insensitive search through header_names
    // Return matching header_values entry, or "" if not found
}

long cy_http_ok(CyHttpResponse* resp) {
    return (resp->status_code >= 200 && resp->status_code < 300) ? 1 : 0;
}
```

### Memory

All CyHttpResponse structs, body buffers, and header strings are malloc'd. No free — consistent with the rest of Sans (leaked until process exit).

## Testing

### Unit Tests (~8 new)

**Type Checker (~5):**
- `http_get` accepts String, returns HttpResponse
- `http_post` accepts (String, String, String), returns HttpResponse
- Methods: `.status()` -> Int, `.body()` -> String, `.ok()` -> Bool
- `.header(String)` -> String
- Error: wrong argument type to `http_get` (Int instead of String)

**IR (~3):**
- `http_get` lowers to `HttpGet` instruction with `IrType::HttpResponse`
- `http_post` lowers to `HttpPost` instruction with `IrType::HttpResponse`
- `.body()` method lowers to `HttpBody` instruction with `IrType::Str`

### E2E Tests (~2 new)

**`http_get_status.cy`** — GET request to a local/reliable URL, check that status is non-zero. Exit code = 1 if status > 0, else 0. (Note: E2E HTTP tests require network access; use a simple public endpoint or test with error response for offline resilience.)

**`http_error_handling.cy`** — Request to an invalid URL, verify status is 0 and ok() is false. Exit code = 1 (success = error handled correctly).

Note: HTTP E2E tests are inherently dependent on network availability. The error handling test works offline. The GET test uses a reliable public endpoint but may fail without network.

### Estimated Total: ~268 existing + ~11 new = ~279 tests

## Deferred

- PUT/DELETE/PATCH HTTP methods
- Custom request headers (`http_set_header`)
- Request timeouts
- Authentication (Basic, Bearer)
- Cookie handling
- Redirect control (disable, limit count)
- Streaming/chunked responses
- Async/concurrent requests
- HTTPS certificate verification control
- Proxy support
- Multipart form data / file upload
- Response body as bytes (binary)
- Connection pooling / keep-alive
- HTTP/2 support configuration
- Memory cleanup
