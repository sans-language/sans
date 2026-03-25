import * as vscode from 'vscode';

const HOVER_DATA: Record<string, string> = {
    // I/O
    'p': '**print**(value: String|Int|Float|Bool) -> Int\n\nPrint value to stdout with newline.',
    'print': '**print**(value: String|Int|Float|Bool) -> Int\n\nPrint value to stdout with newline.',
    'fr': '**file_read**(path: String) -> String\n\nRead entire file contents. Returns "" on error.',
    'fread': '**file_read**(path: String) -> String\n\nRead entire file contents. Returns "" on error.',
    'file_read': '**file_read**(path: String) -> String\n\nRead entire file contents. Returns "" on error.',
    'fw': '**file_write**(path: String, content: String) -> Int\n\nWrite content to file. Returns 1 on success, 0 on error.',
    'fwrite': '**file_write**(path: String, content: String) -> Int\n\nWrite content to file. Returns 1 on success, 0 on error.',
    'file_write': '**file_write**(path: String, content: String) -> Int\n\nWrite content to file. Returns 1 on success, 0 on error.',
    'fa': '**file_append**(path: String, content: String) -> Int\n\nAppend content to file. Returns 1 on success, 0 on error.',
    'fappend': '**file_append**(path: String, content: String) -> Int\n\nAppend content to file. Returns 1 on success, 0 on error.',
    'file_append': '**file_append**(path: String, content: String) -> Int\n\nAppend content to file. Returns 1 on success, 0 on error.',
    'fe': '**file_exists**(path: String) -> Bool\n\nCheck if file exists.',
    'fexists': '**file_exists**(path: String) -> Bool\n\nCheck if file exists.',
    'file_exists': '**file_exists**(path: String) -> Bool\n\nCheck if file exists.',

    // Conversions
    'str': '**int_to_string**(n: Int) -> String\n\nConvert integer to string.',
    'itos': '**int_to_string**(n: Int) -> String\n\nConvert integer to string.',
    'int_to_string': '**int_to_string**(n: Int) -> String\n\nConvert integer to string.',
    'stoi': '**string_to_int**(s: String) -> Int\n\nParse string as integer. Returns 0 on invalid input.',
    'string_to_int': '**string_to_int**(s: String) -> Int\n\nParse string as integer. Returns 0 on invalid input.',
    'stof': '**string_to_float**(s: String) -> Float\n\nParse string as float.\n\nUsage: `stof("3.14")  // 3.14`',
    'string_to_float': '**string_to_float**(s: String) -> Float\n\nParse string as float.\n\nUsage: `string_to_float("3.14")  // 3.14`',
    'itof': '**int_to_float**(n: Int) -> Float\n\nConvert integer to float.',
    'int_to_float': '**int_to_float**(n: Int) -> Float\n\nConvert integer to float.',
    'ftoi': '**float_to_int**(f: Float) -> Int\n\nTruncate float to integer.',
    'float_to_int': '**float_to_int**(f: Float) -> Int\n\nTruncate float to integer.',
    'ftos': '**float_to_string**(f: Float) -> String\n\nConvert float to string.',
    'float_to_string': '**float_to_string**(f: Float) -> String\n\nConvert float to string.',

    // JSON
    'jo': '**json_object**() -> JsonValue\n\nCreate empty JSON object.',
    'jobj': '**json_object**() -> JsonValue\n\nCreate empty JSON object.',
    'json_object': '**json_object**() -> JsonValue\n\nCreate empty JSON object.',
    'ja': '**json_array**() -> JsonValue\n\nCreate empty JSON array.',
    'jarr': '**json_array**() -> JsonValue\n\nCreate empty JSON array.',
    'json_array': '**json_array**() -> JsonValue\n\nCreate empty JSON array.',
    'js': '**json_string**(s: String) -> JsonValue\n\nWrap string as JSON value.',
    'jstr': '**json_string**(s: String) -> JsonValue\n\nWrap string as JSON value.',
    'json_string': '**json_string**(s: String) -> JsonValue\n\nWrap string as JSON value.',
    'ji': '**json_int**(n: Int) -> JsonValue\n\nWrap integer as JSON value.',
    'json_int': '**json_int**(n: Int) -> JsonValue\n\nWrap integer as JSON value.',
    'jb': '**json_bool**(b: Bool) -> JsonValue\n\nWrap boolean as JSON value.',
    'json_bool': '**json_bool**(b: Bool) -> JsonValue\n\nWrap boolean as JSON value.',
    'jn': '**json_null**() -> JsonValue\n\nCreate JSON null value.',
    'json_null': '**json_null**() -> JsonValue\n\nCreate JSON null value.',
    'jp': '**json_parse**(s: String) -> Result\\<JsonValue\\>\n\nParse JSON string into a Result. Handles objects, arrays, strings, integers, floats, booleans, and null. Returns error Result on invalid JSON or nesting depth > 512. Use `!` to unwrap. **Breaking change (v0.8.1):** previously returned JsonValue.',
    'jparse': '**json_parse**(s: String) -> Result\\<JsonValue\\>\n\nParse JSON string into a Result. Handles objects, arrays, strings, integers, floats, booleans, and null. Returns error Result on invalid JSON or nesting depth > 512. Use `!` to unwrap. **Breaking change (v0.8.1):** previously returned JsonValue.',
    'json_parse': '**json_parse**(s: String) -> Result\\<JsonValue\\>\n\nParse JSON string into a Result. Handles objects, arrays, strings, integers, floats, booleans, and null. Returns error Result on invalid JSON or nesting depth > 512. Use `!` to unwrap. **Breaking change (v0.8.1):** previously returned JsonValue.',
    'jfy': '**json_stringify**(v: JsonValue) -> String\n\nSerialize JSON value to compact string.',
    'jstringify': '**json_stringify**(v: JsonValue) -> String\n\nSerialize JSON value to compact string.',
    'json_stringify': '**json_stringify**(v: JsonValue) -> String\n\nSerialize JSON value to compact string.',

    // HTTP
    'hg': '**http_get**(url: String) -> HttpResponse\n\nPerform HTTP GET request.',
    'hget': '**http_get**(url: String) -> HttpResponse\n\nPerform HTTP GET request.',
    'http_get': '**http_get**(url: String) -> HttpResponse\n\nPerform HTTP GET request.',
    'hp': '**http_post**(url: String, body: String, content_type: String) -> HttpResponse\n\nPerform HTTP POST request.',
    'hpost': '**http_post**(url: String, body: String, content_type: String) -> HttpResponse\n\nPerform HTTP POST request.',
    'http_post': '**http_post**(url: String, body: String, content_type: String) -> HttpResponse\n\nPerform HTTP POST request.',
    'listen': '**http_listen**(port: Int) -> HttpServer\n\nStart HTTP server on port. Returns server handle.',
    'hl': '**http_listen**(port: Int) -> HttpServer\n\nStart HTTP server on port. Returns server handle.',
    'http_listen': '**http_listen**(port: Int) -> HttpServer\n\nStart HTTP server on port. Returns server handle.',
    'hl_s': '**https_listen**(port: Int, cert: String, key: String) -> HttpServer\n\nStart HTTPS server with TLS on port. `cert` and `key` are file paths to the PEM certificate and private key.\n\nAlias: `hl_s`\n\nUsage: `srv = https_listen(8443 "cert.pem" "key.pem")`',
    'https_listen': '**https_listen**(port: Int, cert: String, key: String) -> HttpServer\n\nStart HTTPS server with TLS on port. `cert` and `key` are file paths to the PEM certificate and private key.\n\nUsage: `srv = https_listen(8443 "cert.pem" "key.pem")`',
    'serve': '**serve**(port: Int, handler: Fn) -> Int\n\nStart a production HTTP server with bounded thread pool, HTTP/1.1 keep-alive, auto-gzip, and graceful shutdown (SIGINT/SIGTERM). Configure with `set_max_workers`, `set_read_timeout`, etc.\n\nUsage: `serve(8080 fptr("handle"))`',
    'serve_tls': '**serve_tls**(port: Int, cert: String, key: String, handler: Fn) -> Int\n\nStart a production HTTPS server with auto-threading, keep-alive, and graceful shutdown.\n\nUsage: `serve_tls(8443 "cert.pem" "key.pem" fptr("handle"))`',
    'stream_write': '**stream_write**(writer: Int, data: String) -> Int\n\nSend a chunk of data in a chunked HTTP response. The writer is obtained from `req.respond_stream(status)`.\n\nUsage: `stream_write(w "hello\\n")`',
    'stream_end': '**stream_end**(writer: Int) -> Int\n\nFinalize a chunked HTTP response by sending the terminal chunk.\n\nUsage: `stream_end(w)`',
    'cors': '**cors**(req: HttpRequest, origin: String) -> Int\n\nSet CORS response headers: `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`, and `Access-Control-Allow-Headers`. Call before `respond`.\n\nUsage: `cors(req "https://example.com")`',
    'cors_all': '**cors_all**(req: HttpRequest) -> Int\n\nSet CORS response headers with wildcard origin (`*`). Shorthand for `cors(req "*")`.\n\nUsage: `cors_all(req)`',
    'ca': '**cors_all**(req: HttpRequest) -> Int\n\nAlias for `cors_all()`. Set CORS headers with wildcard origin.\n\nUsage: `ca(req)`',
    'ud': '**url_decode**(s: String) -> String\n\nAlias for `url_decode()`. Decode URL-encoded string.\n\nUsage: `ud(raw)`',
    'ps': '**path_segment**(path: String, idx: Int) -> String\n\nAlias for `path_segment()`. Extract URL path segment.\n\nUsage: `ps("/api/users/42" 2)  // "42"`',
    'sigh': '**signal_handler**(signum: Int) -> Int\n\nAlias for `signal_handler()`. Register signal handler.\n\nUsage: `sigh(2)`',
    'sigc': '**signal_check**() -> Int\n\nAlias for `signal_check()`. Returns 1 if signal received.\n\nUsage: `sigc()`',
    'ssl_ctx': '**ssl_ctx**(cert: String, key: String) -> Int\n\nCreate an SSL context from PEM certificate and private key file paths. Returns opaque context pointer.\n\nAdvanced — prefer `https_listen` for most use cases.\n\nUsage: `ctx = ssl_ctx("cert.pem" "key.pem")`',
    'ssl_accept': '**ssl_accept**(ctx: Int, fd: Int) -> Int\n\nPerform TLS handshake on an accepted socket fd using the given SSL context. Returns SSL object pointer.\n\nUsage: `ssl = ssl_accept(ctx fd)`',
    'ssl_read': '**ssl_read**(ssl: Int, buf: Int, len: Int) -> Int\n\nRead up to `len` bytes from a TLS connection into buffer. Returns bytes read.\n\nUsage: `n = ssl_read(ssl buf 4096)`',
    'ssl_write': '**ssl_write**(ssl: Int, buf: Int, len: Int) -> Int\n\nWrite `len` bytes from buffer to a TLS connection. Returns bytes written.\n\nUsage: `ssl_write(ssl ptr(data) data.len)`',
    'ssl_close': '**ssl_close**(ssl: Int) -> Int\n\nShut down TLS connection and free the SSL object.\n\nUsage: `ssl_close(ssl)`',

    // Server configuration
    'set_max_workers': '**set_max_workers**(n: Int) -> Int\n\nSet max concurrent worker threads (default 256). Connections beyond this limit receive HTTP 503.\n\nUsage: `set_max_workers(128)`',
    'set_read_timeout': '**set_read_timeout**(s: Int) -> Int\n\nSet read timeout in seconds (default 30). Closes connection if no data received within timeout.\n\nUsage: `set_read_timeout(10)`',
    'set_keepalive_timeout': '**set_keepalive_timeout**(s: Int) -> Int\n\nSet keep-alive timeout in seconds (default 60). Time to wait for next request on a persistent connection.\n\nUsage: `set_keepalive_timeout(30)`',
    'set_drain_timeout': '**set_drain_timeout**(s: Int) -> Int\n\nSet shutdown drain timeout in seconds (default 5). Time to wait for in-flight requests during graceful shutdown.\n\nUsage: `set_drain_timeout(10)`',
    'set_max_body': '**set_max_body**(n: Int) -> Int\n\nSet max request body size in bytes (default 1048576 / 1MB). Oversized requests receive HTTP 413.\n\nUsage: `set_max_body(4096)`',
    'set_max_headers': '**set_max_headers**(n: Int) -> Int\n\nSet max total header size in bytes (default 8192 / 8KB). Oversized headers receive HTTP 431.\n\nUsage: `set_max_headers(16384)`',
    'set_max_header_count': '**set_max_header_count**(n: Int) -> Int\n\nSet max number of request headers (default 100). Excess headers receive HTTP 431.\n\nUsage: `set_max_header_count(50)`',
    'set_max_url': '**set_max_url**(n: Int) -> Int\n\nSet max URL length in bytes (default 8192 / 8KB). Oversized URLs receive HTTP 414.\n\nUsage: `set_max_url(2048)`',

    // Low-level threading
    'pmutex_init': '**pmutex_init**(ptr: Int) -> Int\n\nInitialize a raw pthread mutex at the given address.\n\nUsage: `pmutex_init(ptr)`',
    'pmutex_lock': '**pmutex_lock**(ptr: Int) -> Int\n\nLock a raw pthread mutex.\n\nUsage: `pmutex_lock(ptr)`',
    'pmutex_unlock': '**pmutex_unlock**(ptr: Int) -> Int\n\nUnlock a raw pthread mutex.\n\nUsage: `pmutex_unlock(ptr)`',

    // HttpRequest methods
    'header': '**header**(name: String) -> String\n\nGet request header value by name (case-insensitive). Returns "" if not found.\n\nUsage: `ct = req.header("Content-Type")`',
    'set_header': '**set_header**(name: String, value: String) -> Int\n\nAdd a custom response header. Must be called before `respond`.\n\nUsage: `req.set_header("X-Request-Id" "abc123")`',
    'cookie': '**cookie**(name: String) -> String\n\nGet cookie value from the `Cookie` request header. Returns "" if not found.\n\nUsage: `token = req.cookie("session")`',
    'form': '**form**(name: String) -> String\n\nParse form field from POST body. Supports `application/x-www-form-urlencoded` and `multipart/form-data` (text fields only). Returns "" if not found.\n\nUsage: `username = req.form("username")`',
    'signal_handler': '**signal_handler**(signum: Int) -> Int\n\nRegister a signal handler that sets a global shutdown flag. Used by `serve()` internally for graceful shutdown.\n\nUsage: `signal_handler(2)  // SIGINT`',
    'signal_check': '**signal_check**() -> Int\n\nReturns 1 if a registered signal was received, 0 otherwise.\n\nUsage: `while signal_check() == 0 { ... }`',
    'spoll': '**spoll**(fd: Int, timeout_ms: Int) -> Int\n\nPoll a file descriptor for readability with timeout. Returns 1 if ready, 0 on timeout.\n\nUsage: `ready = spoll(fd 1000)`',
    'ws_send': '**ws_send**(ws: Int, msg: String) -> Int\n\nSend a WebSocket text frame.\n\nUsage: `ws_send(ws "hello")`',
    'ws_recv': '**ws_recv**(ws: Int) -> String\n\nReceive next WebSocket text frame. Handles ping/pong automatically. Returns "" on close.\n\nUsage: `msg = ws_recv(ws)`',
    'ws_close': '**ws_close**(ws: Int) -> Int\n\nSend WebSocket close frame and close the socket.\n\nUsage: `ws_close(ws)`',
    'is_ws_upgrade': '**is_ws_upgrade**() -> Int\n\nHttpRequest method. Returns 1 if request is a WebSocket upgrade request, 0 otherwise.\n\nUsage: `req.is_ws_upgrade()`',
    'upgrade_ws': '**upgrade_ws**() -> Int\n\nHttpRequest method. Performs WebSocket handshake (SHA-1 + Base64) and sends 101 response. Returns WebSocket handle.\n\nUsage: `ws = req.upgrade_ws()`',
    'serve_file': '**serve_file**(req: HttpRequest, dir: String) -> Int\n\nServe a static file from `dir` matching the request path. Handles content-type detection, 404 for missing files, and directory traversal protection.\n\nUsage: `serve_file(req "./public")`',
    'url_decode': '**url_decode**(s: String) -> String\n\nDecode a URL-encoded string (`%20` becomes space, `+` becomes space).\n\nUsage: `name = url_decode(raw_name)`',
    'path_segment': '**path_segment**(path: String, idx: Int) -> String\n\nExtract URL path segment at index. Segments are split by `/`.\n\nUsage: `path_segment("/api/users/42" 2)  // "42"`',
    'query': '**query**(name: String) -> String\n\nHttpRequest method. Get query parameter value by name. Returns "" if not found.\n\nUsage: `page = req.query("page")`',
    'path_only': '**path_only**() -> String\n\nHttpRequest method. Returns the path without query string.\n\nUsage: `p = req.path_only()`',
    'content_length': '**content_length**() -> Int\n\nHttpRequest method. Get the Content-Length header value as an integer.\n\nUsage: `len = req.content_length()`',
    'respond_json': '**respond_json**(status: Int, body: String) -> Int\n\nHttpRequest method. Send a JSON response (sets Content-Type: application/json automatically).\n\nUsage: `req.respond_json(200 jfy(data))`',
    'respond_stream': '**respond_stream**(status: Int) -> Int\n\nHttpRequest method. Send HTTP headers with Transfer-Encoding: chunked and return a writer handle. Use `stream_write(w, data)` to send chunks and `stream_end(w)` to finalize.\n\nUsage: `w = req.respond_stream(200)`',
    'sh': '**sh**(cmd: String) -> String\n\nExecute command and capture stdout. Returns "" on failure.\n\nUsage: `output = sh("git status")`',
    'shell': '**sh**(cmd: String) -> String\n\nAlias for `sh()`. Execute command and capture stdout.\n\nUsage: `shell("ls -la")`',
    'cl': '**content_length**() -> Int\n\nAlias for `content_length()`. Get Content-Length.\n\nUsage: `req.cl()`',
    'rj': '**respond_json**(status: Int, body: String) -> Int\n\nAlias for `respond_json()`. Send JSON response.\n\nUsage: `req.rj(200 data)`',

    // Logging
    'ld': '**log_debug**(msg: String) -> Int\n\nLog message at DEBUG level to stderr.',
    'log_debug': '**log_debug**(msg: String) -> Int\n\nLog message at DEBUG level to stderr.',
    'li': '**log_info**(msg: String) -> Int\n\nLog message at INFO level to stderr.',
    'log_info': '**log_info**(msg: String) -> Int\n\nLog message at INFO level to stderr.',
    'lw': '**log_warn**(msg: String) -> Int\n\nLog message at WARN level to stderr.',
    'log_warn': '**log_warn**(msg: String) -> Int\n\nLog message at WARN level to stderr.',
    'le': '**log_error**(msg: String) -> Int\n\nLog message at ERROR level to stderr.',
    'log_error': '**log_error**(msg: String) -> Int\n\nLog message at ERROR level to stderr.',
    'll': '**log_set_level**(level: Int) -> Int\n\nSet minimum log level. 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR.',
    'log_set_level': '**log_set_level**(level: Int) -> Int\n\nSet minimum log level. 0=DEBUG, 1=INFO, 2=WARN, 3=ERROR.',

    // Result
    'ok': '**ok**(value: T) -> Result\\<T\\>\n\nWrap value in successful Result.',
    'err': '**err**(message: String) -> Result\\<_\\>\n**err**(code: Int, message: String) -> Result\\<_\\>\n\nCreate error Result with message. Optionally include an integer error code.\n\nUsage: `err("not found")` or `err(404 "not found")`\n\nRetrieve code with `.code()` method.',
    'code': '**code**() -> Int\n\nResult method. Get the error code from a Result. Returns 0 if no code was set.\n\nUsage: `r.code()  // 404`',
    'and_then': '**and_then**(fn: (T) -> Result\\<U\\>) -> Result\\<U\\>\n\nResult method. Apply `fn` to the ok value where `fn` itself returns a Result. On error, returns the error unchanged. Useful for chaining fallible steps.\n\nUsage: `parse("10").and_then(|n:I| R<I> { n > 0 ? ok(n) : err("negative") })`',
    'map_err': '**map_err**(fn: (String) -> String) -> Result\\<T\\>\n\nResult method. Transform the error message string. On ok, returns the ok unchanged.\n\nUsage: `r.map_err(|e:S| S { "context: {e}" })`',
    'or_else': '**or_else**(fn: (String) -> Result\\<T\\>) -> Result\\<T\\>\n\nResult method. Apply `fn` to the error and return its Result. On ok, returns the ok unchanged. Useful for fallback values.\n\nUsage: `parse("").or_else(|e:S| R<I> { ok(0) })  // ok(0) as fallback`',

    // Option
    'some': '**some**(value: T) -> Option\\<T\\>\n\nCreate a Some option wrapping a value.\n\nUsage: `x = some(42)  // Some(42)`',
    'none': '**none**() -> Option\\<T\\>\n\nCreate a None option (absence of a value).\n\nUsage: `y = none()  // None`',
    'Option': '**Option\\<T\\>** (alias: `O<T>`) — Optional value: Some(v) or None.\n\nRuntime: 16 bytes — tag@0 (0=None, 1=Some), value@8.\n\nCreated by `some(v)` or `none()`. Also returned by `Map.get()` and `Array.find()`.\n\nMethods: `.is_some`, `.is_none`, `.unwrap()`/`!`, `.unwrap_or(default)`\n\nOperators: `opt!` (unwrap), `opt?` (propagate none)',
    'O': '**O\\<T\\>** — Short alias for `Option<T>`. Optional value: Some or None.\n\nUsage: `find_user(id:I) O<S> = id == 1 ? some("alice") : none()`',
    'is_some': '**is_some**() -> Bool\n\nOption method. Returns true if the option is Some.\n\nUsage: `x.is_some  // true`',
    'is_none': '**is_none**() -> Bool\n\nOption method. Returns true if the option is None.\n\nUsage: `x.is_none  // false`',

    // String methods
    'substring': '**substring**(start: Int, end: Int) -> String\n\nExtract substring. Slice syntax: `s[0:5]`, `s[6:]`, `s[:5]`\n\nUsage: `"hello world"[0:5]  // "hello"`',
    'ends_with': '**ends_with**(suffix: String) -> Bool\n\nCheck if string ends with suffix. Returns Bool.\n\nUsage: `s.ends_with(".html")`\n\nAlias: `ew`',
    'ew': '**ew**(suffix: String) -> Bool\n\nShort alias for `ends_with`. Check if string ends with suffix.\n\nUsage: `s.ew(".html")`',

    // HTTP methods
    'respond': '**respond**(status: Int, body: String, content_type?: String) -> Int\n\nSend HTTP response. `content_type` defaults to `text/html`.\n\nUsage: `req.respond(200, body)` or `req.respond(200, body, "text/css")`',

    // Memory / low-level
    'alloc': '**alloc**(n: Int) -> Int\n\nAllocate `n` bytes of memory (malloc). Returns pointer.\n\nUsage: `ptr = alloc(1024)`',
    'dealloc': '**dealloc**(ptr: Int) -> Int\n\nFree memory at pointer (free).\n\nUsage: `dealloc(ptr)`',
    'ralloc': '**ralloc**(ptr: Int, n: Int) -> Int\n\nReallocate memory to `n` bytes (realloc). Returns new pointer.\n\nUsage: `ptr = ralloc(ptr 2048)`',
    'mcpy': '**mcpy**(dest: Int, src: Int, n: Int) -> Int\n\nCopy `n` bytes from src to dest (memcpy).\n\nUsage: `mcpy(dst src 64)`',
    'mcmp': '**mcmp**(a: Int, b: Int, n: Int) -> Int\n\nCompare `n` bytes at two addresses (memcmp). Returns 0 if equal.\n\nUsage: `mcmp(a b 16)  // 0 if equal`',
    'slen': '**slen**(ptr: Int) -> Int\n\nGet string length at pointer (strlen).\n\nUsage: `n = slen(ptr)`',
    'load8': '**load8**(ptr: Int) -> Int\n\nLoad byte (8-bit unsigned) from memory address.\n\nUsage: `v = load8(addr)`',
    'store8': '**store8**(ptr: Int, val: Int) -> Int\n\nStore byte to memory address.\n\nUsage: `store8(addr 0xFF)`',
    'mzero': '**mzero**(ptr: Int, n: Int) -> Int\n\nZero out `n` bytes at pointer (memset 0).\n\nUsage: `mzero(buf 1024)`',
    'wfd': '**wfd**(fd: Int, msg: String) -> Int\n\nWrite string to file descriptor.\n\nUsage: `wfd(2 "error msg")`',
    'load16': '**load16**(ptr: Int) -> Int\n\nLoad 16-bit unsigned integer from memory address.\n\nUsage: `v = load16(addr)`',
    'store16': '**store16**(ptr: Int, val: Int) -> Int\n\nStore 16-bit value to memory address.\n\nUsage: `store16(addr, 0xFF)`',
    'load32': '**load32**(ptr: Int) -> Int\n\nLoad 32-bit unsigned integer from memory address.\n\nUsage: `v = load32(addr)`',
    'store32': '**store32**(ptr: Int, val: Int) -> Int\n\nStore 32-bit value to memory address.\n\nUsage: `store32(addr, 42)`',
    'load64': '**load64**(ptr: Int) -> Int\n\nLoad 64-bit integer from memory address.\n\nUsage: `v = load64(addr)`',
    'store64': '**store64**(ptr: Int, val: Int) -> Int\n\nStore 64-bit value to memory address.\n\nUsage: `store64(addr, val)`',
    'strstr': '**strstr**(haystack: String, needle: String) -> Int\n\nReturn pointer to first occurrence of needle in haystack, or 0 if not found.\n\nUsage: `p = strstr(s, "foo")`',
    'bswap16': '**bswap16**(n: Int) -> Int\n\nByte-swap a 16-bit integer (reverse byte order).\n\nUsage: `be = bswap16(le)`',
    'exit': '**exit**(code: Int) -> Int\n\nTerminate the process with the given exit code.\n\nUsage: `exit(1)`',
    'system': '**system**(cmd: String) -> Int\n\nRun a shell command via libc `system()` and return the exit code.\n\nAlias: `sys`\n\nUsage: `r = system("ls -la")`',
    'sys': '**sys**(cmd: String) -> Int\n\nAlias for `system()`. Run a shell command and return the exit code.\n\nUsage: `r = sys("make")`',

    // Compression
    'gzip_compress': '**gzip_compress**(data: Int, len: Int) -> Int\n\nGzip-compress `len` bytes at `data` pointer. Returns pointer to a 16-byte struct: `[compressed_ptr (i64), compressed_len (i64)]`.\n\nUsage: `result = gzip_compress(buf, buf_len)`\n`ptr = load64(result)`\n`clen = load64(result + 8)`',
    'gz': '**gzip_compress**(data: Int, len: Int) -> Int\n\nAlias for `gzip_compress()`. Returns pointer to [ptr, len] struct.\n\nUsage: `result = gz(buf buf_len)`',

    // Arena allocator
    'arena_begin': '**arena_begin**() -> Int\n\nPush a new arena onto the stack. All subsequent `arena_alloc()` calls allocate from this arena until `arena_end()`. Nestable up to 8 deep.\n\nUsage: `arena_begin()`',
    'arena_alloc': '**arena_alloc**(size: Int) -> Int\n\nBump-allocate `size` bytes (8-byte aligned) from the current arena. Falls back to `alloc()` if no arena is active.\n\nUsage: `ptr = arena_alloc(24)`',
    'arena_end': '**arena_end**() -> Int\n\nPop the current arena and free all its memory at once.\n\nUsage: `arena_end()`',
    'ab': '**arena_begin**() -> Int\n\nAlias for `arena_begin()`. Push new arena.\n\nUsage: `ab()`',
    'aa': '**arena_alloc**(size: Int) -> Int\n\nAlias for `arena_alloc()`. Bump-allocate from arena.\n\nUsage: `ptr = aa(24)`',
    'ae': '**arena_end**() -> Int\n\nAlias for `arena_end()`. Pop arena and free.\n\nUsage: `ae()`',

    // Sockets
    'sock': '**sock**(domain: Int, type: Int, protocol: Int) -> Int\n\nCreate a socket. Returns fd.\n\nUsage: `fd = sock(2 1 0)  // AF_INET, SOCK_STREAM`',
    'sbind': '**sbind**(fd: Int, port: Int) -> Int\n\nBind socket to port.\n\nUsage: `sbind(fd 8080)`',
    'slisten': '**slisten**(fd: Int, backlog: Int) -> Int\n\nListen on socket.\n\nUsage: `slisten(fd 128)`',
    'saccept': '**saccept**(fd: Int) -> Int\n\nAccept connection on socket. Returns client fd.\n\nUsage: `client = saccept(fd)`',
    'srecv': '**srecv**(fd: Int, buf: Int, len: Int) -> Int\n\nReceive data from socket into buffer. Returns bytes read.\n\nUsage: `n = srecv(fd buf 4096)`',
    'ssend': '**ssend**(fd: Int, buf: Int, len: Int) -> Int\n\nSend data from buffer to socket. Returns bytes sent.\n\nUsage: `ssend(fd ptr(msg) msg.len)`',
    'sclose': '**sclose**(fd: Int) -> Int\n\nClose a socket.\n\nUsage: `sclose(fd)`',
    'rbind': '**rbind**(port: Int) -> Int\n\nCreate and bind a raw TCP socket to port. Returns socket fd, or -1 on error.\n\nUsage: `fd = rbind(8080)`',
    'rsetsockopt': '**rsetsockopt**(fd: Int, opt: Int, val: Int) -> Int\n\nSet a socket option on fd. Returns 0 on success.\n\nUsage: `rsetsockopt(fd, 1, 1)`',

    // Curl helpers
    'cinit': '**cinit**() -> Int\n\nInitialize a curl handle.\n\nUsage: `h = cinit()`',
    'csets': '**csets**(handle: Int, opt: Int, val: String) -> Int\n\nSet curl string option.\n\nUsage: `csets(h 10002 "https://example.com")`',
    'cseti': '**cseti**(handle: Int, opt: Int, val: Int) -> Int\n\nSet curl integer option.\n\nUsage: `cseti(h 13 30)  // timeout`',
    'cperf': '**cperf**(handle: Int) -> Int\n\nPerform the curl request.\n\nUsage: `cperf(h)`',
    'cclean': '**cclean**(handle: Int) -> Int\n\nCleanup curl handle.\n\nUsage: `cclean(h)`',
    'cinfo': '**cinfo**(handle: Int, info: Int, buf: Int) -> Int\n\nGet info from completed curl request.\n\nUsage: `cinfo(h 0x200002 buf)  // get response code`',
    'curl_slist_append': '**curl_slist_append**(list: Int, header: String) -> Int\n\nAppend a header string to a curl slist. Pass 0 for list to create a new one. Returns new list pointer.\n\nUsage: `hdrs = curl_slist_append(0, "Content-Type: application/json")`',
    'curl_slist_free': '**curl_slist_free**(list: Int) -> Int\n\nFree a curl slist previously built with curl_slist_append.\n\nUsage: `curl_slist_free(hdrs)`',

    // Globals
    'g': '**g** — global variable declaration keyword\n\nDeclare a mutable global variable.\n\nUsage: `g counter := 0`',

    // Tuples
    'tuple': '**Tuple** — (expr1 expr2 ...)\n\nFixed-size ordered collection of values. Access with `.0`, `.1`, `.2` etc.\n\nUsage: `t = (1 "hi" true)`\n`t.0  // 1`',

    // Lambdas
    'lambda': '**Lambda** — |params| ReturnType { body }\n\nAnonymous function with implicit variable capture from enclosing scope.\n\nUsage: `f = |x:I| I { x + 10 }`\n`f(5)  // 15`',

    // Iterator chain methods
    'any': '**any**(predicate: (T) -> Bool) -> Bool\n\nReturns true if any element satisfies the predicate.\n\nUsage: `[1 2 3].any(|x:I| B { x > 2 })  // true`',
    'find': '**find**(predicate: (T) -> Bool) -> Option\\<T\\>\n\nReturns first element matching predicate as `Some(v)`, or `None` if not found. **Breaking change in v0.7.2**: previously returned `T` (or `0`). Use `!` or `.unwrap_or(default)` to extract.\n\nUsage: `[10 20 30].find(|x:I| B { x > 15 })!  // 20`\n`[10 20 30].find(|x:I| B { x > 100 }).unwrap_or(0)  // 0`',
    'enumerate': '**enumerate**() -> Array<(Int, T)>\n\nReturns array of (index, value) tuples.\n\nUsage: `[10 20 30].enumerate()  // [(0 10) (1 20) (2 30)]`',
    'zip': '**zip**(other: Array<U>) -> Array<(T, U)>\n\nPairs elements from two arrays into tuples.\n\nUsage: `[1 2].zip([10 20])  // [(1 10) (2 20)]`',

    // Map
    'map': '**M\\<K,V\\>**() or **map\\<K,V\\>**()\n\nCreate an empty generic hash map. Bare `M()` defaults to `M<S,I>` (string→int).\n\nSupported key types: `S` (String), `I` (Int). Float keys are not allowed.\n\n`m.get(key)` returns `Option<V>` — use `!` or `.unwrap_or(default)` to extract.\n\nUsage: `m = M()` / `m = M<S S>()` / `m = M<I I>()`\n`m.set("key" 42)`\n`m.get("key")!  // 42`\n`m.get("missing").unwrap_or(0)  // 0`\n\nNote: Also a Result method — `r.map(fn)` transforms the ok value.',
    'M': '**M\\<K,V\\>**()\n\nCreate an empty generic hash map. Bare `M()` defaults to `M<S,I>` (string→int).\n\nSupported variants: `M<S I>()`, `M<I I>()`, `M<I S>()`, `M<S S>()`.\n\n`m.get(key)` returns `Option<V>` — use `!` or `.unwrap_or(default)` to extract.\n\nUsage: `m = M()` `m.set("x" 10)` `m.get("x")!  // 10`',

    // Try operator (? is not a word token, but 'unwrap' is shown as method after desugaring)
    'is_err': '**is_err**() -> Bool\n\nCheck if Result is an error.\n\nUsage: `r.is_err()`\n\nSee also: `?` try operator — `r = may_fail()?` unwraps or early-returns error.',
    'is_ok': '**is_ok**() -> Bool\n\nCheck if Result is ok.\n\nUsage: `r.is_ok()`',
    'unwrap': '**unwrap**() -> T\n\nUnwrap a Result or Option, exiting on error/None. Shorthand: `r!`\n\nSee also: `?` try operator — `r = may_fail()?` unwraps or early-returns error/none.',
    'unwrap_or': '**unwrap_or**(default: T) -> T\n\nUnwrap a Result or Option, returning `default` on error/None.\n\nUsage: `r.unwrap_or(0)` or `opt.unwrap_or(0)`',
    'error': '**error**() -> String\n\nGet error message from a Result.\n\nUsage: `r.error()`',

    // Loop control
    'break': '**break**\n\nExit the nearest enclosing `while` or `for` loop immediately.',
    'continue': '**continue**\n\nSkip the rest of the current iteration and jump to the next iteration of the nearest enclosing loop.',

    // Pointer access
    'ptr': '**ptr**(s: String|Map|Array) -> Int\n\nGet raw i64 pointer of a string, map, or array.\n\nUsage: `p = ptr(s)`\n`load8(ptr(s) + i)  // read byte at index i`',
    'char_at': '**char_at**(s: String, i: Int) -> Int\n\nRead byte at index i in string. Returns 0-255.\nShorthand for `load8(ptr(s) + i)`.\n\nUsage: `char_at("hello" 1)  // 101 = \'e\'`',

    // Multi-arg function pointer calls
    'fcall2': '**fcall2**(ptr: Int, a: Int, b: Int) -> Int\n\nCall a function pointer with 2 arguments.\n\nUsage: `add(a:I b:I) I = a + b`\n`fcall2(fptr("add") 10 20)  // 30`',
    'fcall3': '**fcall3**(ptr: Int, a: Int, b: Int, c: Int) -> Int\n\nCall a function pointer with 3 arguments.\n\nUsage: `fcall3(fptr("f") 1 2 3)`',

    // Map operations (explicit built-ins)
    'mget': '**mget**(map: Int, key: String) -> Int\n\nGet value from Map by string key. Returns 0 if not found.\nUse when a Map is stored as Int (e.g. from `load64`) and `.get()` would dispatch incorrectly.\n\nUsage: `v = mget(m "key")`',
    'mset': '**mset**(map: Int, key: String, val: Int) -> Int\n\nSet key-value pair in Map.\nUse when a Map is stored as Int (e.g. from `load64`) and `.set()` would dispatch incorrectly.\n\nUsage: `mset(m "key" 42)`',
    'mhas': '**mhas**(map: Int, key: String) -> Int\n\nCheck if Map contains key. Returns 1 if found, 0 if not.\nUse when a Map is stored as Int (e.g. from `load64`) and `.has()` would dispatch incorrectly.\n\nUsage: `mhas(m "key")  // 1 or 0`',

    // File I/O (read_file / write_file / args)
    'read_file': '**read_file**(path: String) -> String\n\nRead entire file contents to string.\n\nUsage: `content = read_file("input.txt")`',
    'write_file': '**write_file**(path: String, content: String) -> Int\n\nWrite string to file. Returns 1 on success.\n\nUsage: `write_file("output.txt" "hello")`',
    'args': '**args**() -> Array\\<String\\>\n\nGet command-line arguments as a string array.\n\nUsage: `a = args()`',

    // Filesystem & Process
    'getenv': '**getenv**(name: String) -> String\n\nRead environment variable. Returns "" if not set.\n\nUsage: `home = getenv("HOME")`',
    'genv': '**getenv**(name: String) -> String\n\nAlias for `getenv()`. Read environment variable.\n\nUsage: `genv("PATH")`',
    'mkdir': '**mkdir**(path: String) -> Int\n\nCreate directory and parents (mkdir -p). Returns 1 on success, 0 on error.\n\nUsage: `mkdir("src/lib")`',
    'rmdir': '**rmdir**(path: String) -> Int\n\nRemove empty directory. Returns 1 on success, 0 on error.\n\nUsage: `rmdir("build/tmp")`',
    'rm': '**remove**(path: String) -> Int\n\nAlias for `remove()`. Delete a file. Returns 1 on success, 0 on error.\n\nUsage: `rm("old.txt")`',
    'listdir': '**listdir**(path: String) -> Array\\<String\\>\n\nList directory contents. Returns empty array on error.\n\nUsage: `files = listdir("src/")`',
    'ls': '**listdir**(path: String) -> Array\\<String\\>\n\nAlias for `listdir()`. List directory contents.\n\nUsage: `ls("src/")`',
    'is_dir': '**is_dir**(path: String) -> Bool\n\nCheck if path is a directory.\n\nUsage: `is_dir("/tmp")`',

    // Type aliases
    'I': '**Int** — 64-bit signed integer',
    'F': '**Float** — 64-bit floating point',
    'B': '**Bool** — Boolean (true/false)',
    'S': '**String** — UTF-8 string',
    'R': '**Result\\<T\\>** — Success or error value',
    'J': '**JsonValue** — Opaque JSON value (short alias for JsonValue)',
    'JsonValue': '**JsonValue** — Opaque JSON value. Created via `jo()`, `ja()`, `jp()`. Methods: `.get()`, `.set()`, `.keys()`, `.has()`, `.delete()`, `.type_of()`, `.get_string()`, `.get_int()`',

    // Package manager
    'pkg': '**sans pkg** — Package manager commands\n\n`sans pkg init` — Create sans.json\n`sans pkg add <url> [tag]` — Add dependency\n`sans pkg install` — Install all deps\n`sans pkg remove <url>` — Remove dependency\n`sans pkg list` — List deps\n`sans pkg update <url> [tag]` — Update dependency\n`sans pkg search <query>` — Search index',
    'scope_disable': '**scope_disable**() -> Int\n\nDisable scope-based GC. Used internally by the compiler build pipeline.',
    'scope_enable': '**scope_enable**() -> Int\n\nRe-enable scope-based GC after `scope_disable()`.',

    // Panic recovery (setjmp/longjmp-based error boundaries)
    'setjmp': '**setjmp**(buf: Int) -> Int\n\nSet a jump point for panic recovery. Returns 0 on first call; returns non-zero when jumped to via `longjmp` or `panic_fire`.\n\nUsage:\n`buf := panic_get_buf()`\n`rv := setjmp(buf)`\n`if rv != 0 { // recovered from panic }`',
    'longjmp': '**longjmp**(buf: Int, val: Int) -> Int\n\nJump back to the matching `setjmp` point with value `val`. `setjmp` will return `val`.\n\nUsage: `longjmp(buf 1)`',
    'panic_enable': '**panic_enable**() -> Int\n\nEnable panic recovery. When active, the `!` unwrap operator calls `longjmp` instead of `exit(1)` on `Err`/`None`. Call `setjmp(panic_get_buf())` first.\n\nUsage: `panic_enable()`',
    'panic_disable': '**panic_disable**() -> Int\n\nDisable panic recovery. After calling this, `!` unwrap reverts to `exit(1)` on failure. Call after handling the recovered panic.\n\nUsage: `panic_disable()`',
    'panic_is_active': '**panic_is_active**() -> Int\n\nReturns 1 if panic recovery is currently active (i.e., `panic_enable()` has been called without a matching `panic_disable()`), 0 otherwise.\n\nUsage: `if panic_is_active() == 1 { ... }`',
    'panic_get_buf': '**panic_get_buf**() -> Int\n\nGet the global jmp_buf pointer used for panic recovery. Pass this to `setjmp()` to set the recovery point.\n\nUsage: `buf := panic_get_buf()`\n`rv := setjmp(buf)`',
    'panic_fire': '**panic_fire**() -> Int\n\nManually fire the panic longjmp. Equivalent to calling `longjmp(panic_get_buf() 1)`. Only valid when panic recovery is active.\n\nUsage: `panic_fire()`',

    // Default parameters
    'default': '**Default Parameters**\n\nTrailing function parameters can have default values using `=literal`.\n\nUsage: `f(x:I y:I=0) = x + y`\n`f(5)  // 5`\n`f(5 3)  // 8`',

    // Generic structs
    'struct': '**struct** — Define a struct type\n\nUsage: `struct Point { x I, y I }`\n\nGeneric: `struct Pair<A B> { first A, second B }`\n`Pair<I S>{ first: 1, second: "hi" }`',

    // Match
    'match': '**match** — Pattern match expression\n\nSupports enum variants, integers, strings, wildcards (`_`), bindings, guards, struct destructuring, and tuple destructuring.\n\nUsage:\n`match x { 1 => "one", _ => "other" }`\n`match s { E::A => 0, E::B(x) => x }`\n`match pt { Point { x, y } => x + y }`\n`match pair { (a, b) => a + b }`\n`match n { x if x > 0 => x, _ => 0 }`',

    // Trait objects
    'dyn': '**dyn** TraitName — Trait object type\n\nCreates a dynamically-dispatched fat pointer (data ptr + vtable ptr, 16 bytes heap-allocated). Use `expr as dyn Trait` to coerce a concrete struct. Use `dyn Trait` as a parameter or variable type.\n\nUsage: `v = x as dyn Valued`\n`show(v dyn Valued) I { v.value() }`\n\nLimitations: no trait inheritance, no default implementations, no associated types.',
    'as': '**as** — Trait object coercion\n\nCoerces a concrete struct to a `dyn Trait` fat pointer for dynamic dispatch.\n\nUsage: `v = x as dyn Valued  // coerce Num to dyn Valued`\n`show(x as dyn Valued)      // pass as dyn Trait argument`',

    // Math
    'abs': '**abs**(n: Int) -> Int\n\nReturn absolute value.\n\nUsage: `abs(-5)  // 5`',
    'min': '**min**(a: Int, b: Int) -> Int\n\nReturn the smaller of two integers.\n\nUsage: `min(3 7)  // 3`',
    'max': '**max**(a: Int, b: Int) -> Int\n\nReturn the larger of two integers.\n\nUsage: `max(3 7)  // 7`',

    // Collections
    'range': '**range**(n: Int) -> Array\\<Int\\>\n**range**(a: Int, b: Int) -> Array\\<Int\\>\n\nGenerate array of integers [0..n) or [a..b).\n\nUsage: `range(5)  // [0 1 2 3 4]`\n`range(2 5)  // [2 3 4]`',

    // System
    'sleep': '**sleep**(ms: Int) -> Int\n\nPause execution for `ms` milliseconds.\n\nUsage: `sleep(1000)  // sleep 1 second`',
    'time': '**time**() -> Int\n\nGet current Unix timestamp in seconds.\n\nUsage: `t = time()`',
    'now': '**now**() -> Int\n\nAlias for `time()`. Get current Unix timestamp.\n\nUsage: `t = now()`',
    'random': '**random**(max: Int) -> Int\n\nReturn cryptographically seeded random integer in [0..max).\n\nUsage: `random(100)  // 0-99`',
    'rand': '**rand**(max: Int) -> Int\n\nAlias for `random()`. Return cryptographically seeded random integer in [0..max).\n\nUsage: `rand(6)  // 0-5`',
    'print_err': '**print_err**(value: String) -> Int\n\nPrint to stderr.\n\nUsage: `print_err("error occurred")`',

    // Function pointers
    'fptr': '**fptr**(name: String) -> Int\n\nGet function pointer by name.\n\nUsage: `fp = fptr("handler")`',
    'fcall': '**fcall**(ptr: Int, arg: Int) -> Int\n\nCall function pointer with 1 argument.\n\nUsage: `result = fcall(fp 42)`',

    // Type aliases
    'HS': '**HttpServer** — HTTP server handle type',
    'HR': '**HttpRequest** — HTTP request type',
    'Fn': '**Fn** — Function pointer type',

    // Concurrency
    'spawn': '**spawn** func(args)\n\nStart a new thread running the given function.\n\nUsage: `h = spawn worker(data)`\n`h.join()`',
    'channel': '**channel**\\<T\\>() -> (Sender\\<T\\>, Receiver\\<T\\>)\n\nCreate a typed channel for inter-thread communication.\n\nUsage: `let (tx rx) = channel<I>()`',
    'mutex': '**mutex**(value: T) -> Mutex\\<T\\>\n\nCreate a mutex wrapping the given value.\n\nUsage: `m = mutex(0)`\n`v = m.lock()`\n`m.unlock(v + 1)`',
    'send': '**send**(value: T) -> Int\n\nSend value on channel.\n\nUsage: `tx.send(42)`',
    'recv': '**recv**() -> T\n\nReceive value from channel.\n\nUsage: `v = rx.recv()`',
    'lock': '**lock**() -> T\n\nLock mutex and get inner value.\n\nUsage: `val = mtx.lock()`',
    'unlock': '**unlock**(value: T) -> Int\n\nUnlock mutex with updated value.\n\nUsage: `mtx.unlock(val)`',

    // Array methods
    'push': '**push**(value: T) -> Int\n\nAppend value to array.\n\nUsage: `a.push(42)`',
    'pop': '**pop**() -> T\n\nRemove and return last element.\n\nUsage: `v = a.pop()`',
    'len': '**len**() -> Int\n\nGet length of array, string, map, or JSON value.\n\nUsage: `n = a.len()`',
    'get': '**get**(key/index) -> T\n\nGet element by index (Array, String) or key (Map, JsonValue).\n\nUsage: `a.get(0)` or `m.get("key")`',
    'set': '**set**(key/index, value) -> Int\n\nSet element by index (Array) or key (Map, JsonValue).\n\nUsage: `a.set(0 42)` or `m.set("key" val)`',
    'remove': '**remove**(index: Int) -> T — Array method: remove element at index.\n\n**remove**(path: String) -> Int — Builtin: delete a file. Returns 1 on success. Alias: `rm`.\n\nUsage: `a.remove(2)` or `remove("old.txt")`',
    'contains': '**contains**(value) -> Bool\n\nCheck if array or string contains value.\n\nUsage: `a.contains(42)` or `s.contains("hi")`',
    'filter': '**filter**(f: (T) -> Bool) -> Array\\<T\\>\n\nReturn elements that satisfy predicate.\n\nUsage: `a.filter(|x:I| B { x > 0 })`',
    'sort': '**sort**() -> Array\\<T\\>\n\nSort array in ascending order.\n\nUsage: `a.sort()`',
    'reverse': '**reverse**() -> Array\\<T\\>\n\nReverse array order.\n\nUsage: `a.reverse()`',
    'join': '**join**(sep: String) -> String\n\nJoin array elements with separator.\n\nUsage: `[1 2 3].join(",")  // "1,2,3"`',
    'slice': '**slice**(start: Int, end: Int) -> Array\\<T\\>\n\nReturn sub-array from start to end.\n\nUsage: `a.slice(1 3)`',
    'reduce': '**reduce**(init: T, f: (T, T) -> T) -> T\n\nReduce array to single value.\n\nUsage: `[1 2 3].reduce(0 |a:I b:I| I { a + b })  // 6`',
    'each': '**each**(f: (T) -> Int) -> Int\n\nCall function on each element. Alias: `for_each`.\n\nUsage: `a.each(|x:I| I { p(x) })`',
    'for_each': '**for_each**(f: (T) -> Int) -> Int\n\nCall function on each element. Alias: `each`.\n\nUsage: `a.for_each(|x:I| I { p(x) })`',
    'flat_map': '**flat_map**(f: (T) -> Array\\<U\\>) -> Array\\<U\\>\n\nMap then flatten one level.\n\nUsage: `a.flat_map(|x:I| [I] { [x x*2] })`',
    'fm': '**flat_map**(f: (T) -> Array\\<U\\>) -> Array\\<U\\>\n\nAlias for `flat_map()`.\n\nUsage: `a.fm(|x:I| [I] { [x x*2] })`',
    'sum': '**sum**() -> Int\n\nSum all elements in integer array.\n\nUsage: `[1 2 3].sum()  // 6`',
    'flat': '**flat**() -> Array\\<T\\>\n\nFlatten nested array by one level.\n\nUsage: `[[1 2] [3 4]].flat()  // [1 2 3 4]`',

    // String methods
    'trim': '**trim**() -> String\n\nRemove leading and trailing whitespace.\n\nUsage: `" hi ".trim()  // "hi"`',
    'split': '**split**(delimiter: String) -> Array\\<String\\>\n\nSplit string by delimiter.\n\nUsage: `"a,b,c".split(",")  // ["a" "b" "c"]`',
    'replace': '**replace**(old: String, new: String) -> String\n\nReplace all occurrences of old with new.\n\nUsage: `"hello".replace("l" "r")  // "herro"`',
    'starts_with': '**starts_with**(prefix: String) -> Bool\n\nCheck if string starts with prefix.\n\nUsage: `s.starts_with("/api")`',
    'sw': '**starts_with**(prefix: String) -> Bool\n\nAlias for `starts_with()`.\n\nUsage: `s.sw("/api")`',
    'upper': '**upper**() -> String\n\nConvert to uppercase.\n\nUsage: `"hi".upper()  // "HI"`',
    'lower': '**lower**() -> String\n\nConvert to lowercase.\n\nUsage: `"HI".lower()  // "hi"`',
    'index_of': '**index_of**(sub: String) -> Int\n\nReturn index of first occurrence, or -1 if not found.\n\nUsage: `"hello".index_of("ll")  // 2`',
    'idx': '**index_of**(sub: String) -> Int\n\nAlias for `index_of()`.\n\nUsage: `s.idx("ll")`',
    'repeat': '**repeat**(n: Int) -> String\n\nRepeat string n times.\n\nUsage: `"ab".repeat(3)  // "ababab"`',
    'to_int': '**to_int**() -> Int\n\nParse string as integer. Returns 0 on invalid input.\n\nUsage: `"42".to_int()  // 42`',
    'ti': '**to_int**() -> Int\n\nAlias for `to_int()`.\n\nUsage: `s.ti()`',
    'pad_left': '**pad_left**(width: Int, char: String) -> String\n\nPad string on the left to given width.\n\nUsage: `"42".pad_left(5 "0")  // "00042"`',
    'pl': '**pad_left**(width: Int, char: String) -> String\n\nAlias for `pad_left()`.\n\nUsage: `s.pl(5 "0")`',
    'pad_right': '**pad_right**(width: Int, char: String) -> String\n\nPad string on the right to given width.\n\nUsage: `"hi".pad_right(5 ".")  // "hi..."`',
    'pr': '**pad_right**(width: Int, char: String) -> String\n\nAlias for `pad_right()`.\n\nUsage: `s.pr(5 ".")`',
    'bytes': '**bytes**() -> Int\n\nGet byte length of string.\n\nUsage: `"hello".bytes()  // 5`',
    'to_str': '**to_str**() -> String\n\nConvert Int to string. Alias: `to_string`.\n\nUsage: `42.to_str()  // "42"`',
    'to_string': '**to_string**() -> String\n\nConvert Int to string.\n\nUsage: `42.to_string()  // "42"`',
    'add': '**add**(other: String) -> String\n\nConcatenate strings.\n\nUsage: `s.add(" world")`',

    // Map methods
    'has': '**has**(key: String) -> Bool\n\nCheck if map contains key.\n\nUsage: `m.has("key")`',
    'keys': '**keys**() -> Array\\<String\\>\n\nGet all keys from map.\n\nUsage: `m.keys()`',
    'vals': '**vals**() -> Array\\<Int\\>\n\nGet all values from map.\n\nUsage: `m.vals()`',
    'delete': '**delete**(key: String) -> Int\n\nRemove key from map.\n\nUsage: `m.delete("key")`',
    'entries': '**entries**() -> Array\\<(String, Int)\\>\n\nGet all key-value pairs as tuples.\n\nUsage: `for (k v) in m.entries() { }`',

    // JsonValue methods
    'get_index': '**get_index**(i: Int) -> JsonValue\n\nGet JSON array element by index.\n\nUsage: `arr.get_index(0)`',
    'gidx': '**get_index**(i: Int) -> JsonValue\n\nAlias for `get_index()`.\n\nUsage: `arr.gidx(0)`',
    'get_string': '**get_string**() -> String\n\nExtract string value from JsonValue.\n\nUsage: `v.get_string()`',
    'gs': '**get_string**() -> String\n\nAlias for `get_string()`.\n\nUsage: `v.gs()`',
    'get_int': '**get_int**() -> Int\n\nExtract integer value from JsonValue.\n\nUsage: `v.get_int()`',
    'geti': '**get_int**() -> Int\n\nAlias for `get_int()`.\n\nUsage: `v.geti()`',
    'get_bool': '**get_bool**() -> Bool\n\nExtract boolean value from JsonValue.\n\nUsage: `v.get_bool()`',
    'gb': '**get_bool**() -> Bool\n\nAlias for `get_bool()`.\n\nUsage: `v.gb()`',
    'type_of': '**type_of**() -> String\n\nGet JSON value type: "object", "array", "string", "number", "boolean", or "null".\n\nUsage: `v.type_of()`',
    'typeof': '**type_of**() -> String\n\nAlias for `type_of()`.\n\nUsage: `v.typeof()`',
    'stringify': '**stringify**() -> String\n\nSerialize JsonValue to string. Same as `json_stringify(v)`.\n\nUsage: `v.stringify()`',

    // HttpResponse methods
    'status': '**status**() -> Int\n\nGet HTTP response status code.\n\nUsage: `r.status()  // 200`',
    'body': '**body**() -> String\n\nGet HTTP response/request body.\n\nUsage: `r.body()`',

    // HttpRequest methods (path/method)
    'path': '**path**() -> String\n\nGet request URL path.\n\nUsage: `req.path()`',
    'method': '**method**() -> String\n\nGet request HTTP method.\n\nUsage: `req.method()  // "GET"`',

    // Assertions
    'assert': '**assert**(cond: Bool) -> Int\n\nFail if `cond` is false (zero). Prints line number on failure and exits with code 1.\n\nUsage: `assert(x > 0)`',
    'assert_eq': '**assert_eq**(a: Int, b: Int) -> Int\n\nFail if `a != b`. Prints expected vs got values and line number.\n\nUsage: `assert_eq(result, 42)`',
    'assert_ne': '**assert_ne**(a: Int, b: Int) -> Int\n\nFail if `a == b`. Prints the equal value and line number.\n\nUsage: `assert_ne(x, 0)`',
    'assert_ok': '**assert_ok**(r: Result<T>) -> Int\n\nFail if Result is err. Prints line number.\n\nUsage: `assert_ok(ok(42))`',
    'assert_err': '**assert_err**(r: Result<T>) -> Int\n\nFail if Result is ok. Prints line number.\n\nUsage: `assert_err(err("bad"))`',
    'assert_some': '**assert_some**(o: Option<T>) -> Int\n\nFail if Option is none. Prints line number.\n\nUsage: `assert_some(some(1))`',
    'assert_none': '**assert_none**(o: Option<T>) -> Int\n\nFail if Option is some. Prints line number.\n\nUsage: `assert_none(none())`',

    // Keywords
    'defer': '**defer** statement\n\nDefer execution of a statement until the end of the current scope.\n\nUsage: `defer close(fd)`',
    'select': '**select** { ... }\n\nMultiplex over multiple channel operations. Picks the first ready channel.\n\nUsage:\n```\nselect {\n  v = rx.recv() => handle(v)\n  timeout 1000 => p("timeout")\n}\n```',
    'pub': '**pub** keyword\n\nMark a function or global as public (exported from module).\nAlso used with `import` to re-export a module\'s public symbols.\n\nUsage: `pub f(x:I) = x*2`\nRe-export: `pub import "mod"` — all pub symbols from mod become pub in current module.',
};

export function activate(context: vscode.ExtensionContext) {
    const hoverProvider = vscode.languages.registerHoverProvider('sans', {
        provideHover(document, position) {
            const range = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
            if (!range) return;

            const word = document.getText(range);
            const info = HOVER_DATA[word];
            if (!info) return;

            const markdown = new vscode.MarkdownString(info);
            markdown.isTrusted = true;
            return new vscode.Hover(markdown, range);
        }
    });

    context.subscriptions.push(hoverProvider);
}

export function deactivate() {}
