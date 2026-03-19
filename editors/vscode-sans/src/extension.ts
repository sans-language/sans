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
    'jp': '**json_parse**(s: String) -> JsonValue\n\nParse JSON string. Returns null on error.',
    'jparse': '**json_parse**(s: String) -> JsonValue\n\nParse JSON string. Returns null on error.',
    'json_parse': '**json_parse**(s: String) -> JsonValue\n\nParse JSON string. Returns null on error.',
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
    'serve': '**serve**(port: Int, handler: Fn) -> Int\n\nStart a production HTTP server with auto-threading, HTTP/1.1 keep-alive, and graceful shutdown (SIGINT/SIGTERM). Each connection is handled in a new thread. The handler receives an HttpRequest.\n\nUsage: `serve(8080 fptr("handle"))`',
    'serve_tls': '**serve_tls**(port: Int, cert: String, key: String, handler: Fn) -> Int\n\nStart a production HTTPS server with auto-threading, keep-alive, and graceful shutdown.\n\nUsage: `serve_tls(8443 "cert.pem" "key.pem" fptr("handle"))`',
    'stream_write': '**stream_write**(writer: Int, data: String) -> Int\n\nSend a chunk of data in a chunked HTTP response. The writer is obtained from `req.respond_stream(status)`.\n\nUsage: `stream_write(w "hello\\n")`',
    'stream_end': '**stream_end**(writer: Int) -> Int\n\nFinalize a chunked HTTP response by sending the terminal chunk.\n\nUsage: `stream_end(w)`',
    'cors': '**cors**(req: HttpRequest, origin: String) -> Int\n\nSet CORS response headers: `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`, and `Access-Control-Allow-Headers`. Call before `respond`.\n\nUsage: `cors(req "https://example.com")`',
    'cors_all': '**cors_all**(req: HttpRequest) -> Int\n\nSet CORS response headers with wildcard origin (`*`). Shorthand for `cors(req "*")`.\n\nUsage: `cors_all(req)`',
    'ssl_ctx': '**ssl_ctx**(cert: String, key: String) -> Int\n\nCreate an SSL context from PEM certificate and private key file paths. Returns opaque context pointer.\n\nAdvanced — prefer `https_listen` for most use cases.\n\nUsage: `ctx = ssl_ctx("cert.pem" "key.pem")`',
    'ssl_accept': '**ssl_accept**(ctx: Int, fd: Int) -> Int\n\nPerform TLS handshake on an accepted socket fd using the given SSL context. Returns SSL object pointer.\n\nUsage: `ssl = ssl_accept(ctx fd)`',
    'ssl_read': '**ssl_read**(ssl: Int, buf: Int, len: Int) -> Int\n\nRead up to `len` bytes from a TLS connection into buffer. Returns bytes read.\n\nUsage: `n = ssl_read(ssl buf 4096)`',
    'ssl_write': '**ssl_write**(ssl: Int, buf: Int, len: Int) -> Int\n\nWrite `len` bytes from buffer to a TLS connection. Returns bytes written.\n\nUsage: `ssl_write(ssl ptr(data) data.len)`',
    'ssl_close': '**ssl_close**(ssl: Int) -> Int\n\nShut down TLS connection and free the SSL object.\n\nUsage: `ssl_close(ssl)`',

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

    // String methods
    'substring': '**substring**(start: Int, end: Int) -> String\n\nExtract substring. Slice syntax: `s[0:5]`, `s[6:]`, `s[:5]`\n\nUsage: `"hello world"[0:5]  // "hello"`',
    'ends_with': '**ends_with**(suffix: String) -> Bool\n\nCheck if string ends with suffix. Returns Bool.\n\nUsage: `s.ends_with(".html")`\n\nAlias: `ew`',
    'ew': '**ew**(suffix: String) -> Bool\n\nShort alias for `ends_with`. Check if string ends with suffix.\n\nUsage: `s.ew(".html")`',

    // HTTP methods
    'respond': '**respond**(status: Int, body: String, content_type?: String) -> Int\n\nSend HTTP response. `content_type` defaults to `text/html`.\n\nUsage: `req.respond(200, body)` or `req.respond(200, body, "text/css")`',

    // Memory / low-level
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

    // Arena allocator
    'arena_begin': '**arena_begin**() -> Int\n\nPush a new arena onto the stack. All subsequent `arena_alloc()` calls allocate from this arena until `arena_end()`. Nestable up to 8 deep.\n\nUsage: `arena_begin()`',
    'arena_alloc': '**arena_alloc**(size: Int) -> Int\n\nBump-allocate `size` bytes (8-byte aligned) from the current arena. Falls back to `alloc()` if no arena is active.\n\nUsage: `ptr = arena_alloc(24)`',
    'arena_end': '**arena_end**() -> Int\n\nPop the current arena and free all its memory at once.\n\nUsage: `arena_end()`',

    // Sockets
    'rbind': '**rbind**(port: Int) -> Int\n\nCreate and bind a raw TCP socket to port. Returns socket fd, or -1 on error.\n\nUsage: `fd = rbind(8080)`',
    'rsetsockopt': '**rsetsockopt**(fd: Int, opt: Int, val: Int) -> Int\n\nSet a socket option on fd. Returns 0 on success.\n\nUsage: `rsetsockopt(fd, 1, 1)`',

    // Curl helpers
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
    'find': '**find**(predicate: (T) -> Bool) -> T\n\nReturns first element matching predicate, or 0 if none found.\n\nUsage: `[10 20 30].find(|x:I| B { x > 15 })  // 20`',
    'enumerate': '**enumerate**() -> Array<(Int, T)>\n\nReturns array of (index, value) tuples.\n\nUsage: `[10 20 30].enumerate()  // [(0 10) (1 20) (2 30)]`',
    'zip': '**zip**(other: Array<U>) -> Array<(T, U)>\n\nPairs elements from two arrays into tuples.\n\nUsage: `[1 2].zip([10 20])  // [(1 10) (2 20)]`',

    // Map
    'map': '**M**() or **map**()\n\nCreate an empty hash map with string keys.\n\nUsage: `m = M()`\n`m.set("key" 42)`\n`m.get("key")  // 42`',
    'M': '**M**()\n\nCreate an empty hash map (alias for map()).\n\nUsage: `m = M()`\n`m.set("x" 10)`',

    // Try operator (? is not a word token, but 'unwrap' is shown as method after desugaring)
    'is_err': '**is_err**() -> Bool\n\nCheck if Result is an error.\n\nUsage: `r.is_err()`\n\nSee also: `?` try operator — `r = may_fail()?` unwraps or early-returns error.',
    'is_ok': '**is_ok**() -> Bool\n\nCheck if Result is ok.\n\nUsage: `r.is_ok()`',
    'unwrap': '**unwrap**() -> T\n\nUnwrap a Result, exiting on error. Shorthand: `r!`\n\nSee also: `?` try operator — `r = may_fail()?` unwraps or early-returns error.',
    'unwrap_or': '**unwrap_or**(default: T) -> T\n\nUnwrap a Result, returning default on error.\n\nUsage: `r.unwrap_or(0)`',
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

    // Type aliases
    'I': '**Int** — 64-bit signed integer',
    'F': '**Float** — 64-bit floating point',
    'B': '**Bool** — Boolean (true/false)',
    'S': '**String** — UTF-8 string',
    'R': '**Result\\<T\\>** — Success or error value',

    // Default parameters
    'default': '**Default Parameters**\n\nTrailing function parameters can have default values using `=literal`.\n\nUsage: `f(x:I y:I=0) = x + y`\n`f(5)  // 5`\n`f(5 3)  // 8`',

    // Generic structs
    'struct': '**struct** — Define a struct type\n\nUsage: `struct Point { x I, y I }`\n\nGeneric: `struct Pair<A B> { first A, second B }`\n`Pair<I S>{ first: 1, second: "hi" }`',
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
