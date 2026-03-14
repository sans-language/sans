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
    'err': '**err**(message: String) -> Result\\<_\\>\n\nCreate error Result with message.',

    // String methods
    'ends_with': '**ends_with**(suffix: String) -> Bool\n\nCheck if string ends with suffix. Returns Bool.\n\nUsage: `s.ends_with(".html")`\n\nAlias: `ew`',
    'ew': '**ew**(suffix: String) -> Bool\n\nShort alias for `ends_with`. Check if string ends with suffix.\n\nUsage: `s.ew(".html")`',

    // HTTP methods
    'respond': '**respond**(status: Int, body: String, content_type?: String) -> Int\n\nSend HTTP response. `content_type` defaults to `text/html`.\n\nUsage: `req.respond(200, body)` or `req.respond(200, body, "text/css")`',

    // Type aliases
    'I': '**Int** — 64-bit signed integer',
    'F': '**Float** — 64-bit floating point',
    'B': '**Bool** — Boolean (true/false)',
    'S': '**String** — UTF-8 string',
    'R': '**Result\\<T\\>** — Success or error value',
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
