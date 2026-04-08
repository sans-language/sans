"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deactivate = exports.activate = void 0;
const vscode = require("vscode");

const HOVER_DATA = {
    // I/O
    'p': '**print**(value: String|Int|Float|Bool) → Int\n\nPrint value to stdout with newline.',
    'print': '**print**(value: String|Int|Float|Bool) → Int\n\nPrint value to stdout with newline.',
    'fr': '**file_read**(path: String) → String\n\nRead entire file contents. Returns "" on error.',
    'fread': '**file_read**(path: String) → String\n\nRead entire file contents. Returns "" on error.',
    'file_read': '**file_read**(path: String) → String\n\nRead entire file contents. Returns "" on error.',
    'fw': '**file_write**(path: String, content: String) → Int\n\nWrite content to file. Returns 1 success, 0 error.',
    'fwrite': '**file_write**(path: String, content: String) → Int\n\nWrite content to file. Returns 1 success, 0 error.',
    'file_write': '**file_write**(path: String, content: String) → Int\n\nWrite content to file. Returns 1 success, 0 error.',
    'fa': '**file_append**(path: String, content: String) → Int\n\nAppend content to file. Returns 1 success, 0 error.',
    'fappend': '**file_append**(path: String, content: String) → Int\n\nAppend content to file. Returns 1 success, 0 error.',
    'file_append': '**file_append**(path: String, content: String) → Int\n\nAppend content to file. Returns 1 success, 0 error.',
    'fe': '**file_exists**(path: String) → Bool\n\nCheck if file exists.',
    'fexists': '**file_exists**(path: String) → Bool\n\nCheck if file exists.',
    'file_exists': '**file_exists**(path: String) → Bool\n\nCheck if file exists.',

    // Conversions
    'str': '**int_to_string**(n: Int) → String\n\nConvert integer to string.',
    'itos': '**int_to_string**(n: Int) → String\n\nConvert integer to string.',
    'int_to_string': '**int_to_string**(n: Int) → String\n\nConvert integer to string.',
    'stoi': '**string_to_int**(s: String) → Int\n\nParse string as integer. Returns 0 on invalid input.',
    'string_to_int': '**string_to_int**(s: String) → Int\n\nParse string as integer. Returns 0 on invalid input.',
    'itof': '**int_to_float**(n: Int) → Float\n\nConvert integer to float.',
    'int_to_float': '**int_to_float**(n: Int) → Float\n\nConvert integer to float.',
    'ftoi': '**float_to_int**(f: Float) → Int\n\nTruncate float to integer.',
    'float_to_int': '**float_to_int**(f: Float) → Int\n\nTruncate float to integer.',
    'ftos': '**float_to_string**(f: Float) → String\n\nConvert float to string.',
    'float_to_string': '**float_to_string**(f: Float) → String\n\nConvert float to string.',

    // JSON
    'jo': '**json_object**() → JsonValue\n\nCreate empty JSON object `{}`.',
    'jobj': '**json_object**() → JsonValue\n\nCreate empty JSON object `{}`.',
    'json_object': '**json_object**() → JsonValue\n\nCreate empty JSON object `{}`.',
    'ja': '**json_array**() → JsonValue\n\nCreate empty JSON array `[]`.',
    'jarr': '**json_array**() → JsonValue\n\nCreate empty JSON array `[]`.',
    'json_array': '**json_array**() → JsonValue\n\nCreate empty JSON array `[]`.',
    'js': '**json_string**(s: String) → JsonValue\n\nWrap string as JSON value.',
    'jstr': '**json_string**(s: String) → JsonValue\n\nWrap string as JSON value.',
    'json_string': '**json_string**(s: String) → JsonValue\n\nWrap string as JSON value.',
    'ji': '**json_int**(n: Int) → JsonValue\n\nWrap integer as JSON value.',
    'json_int': '**json_int**(n: Int) → JsonValue\n\nWrap integer as JSON value.',
    'jb': '**json_bool**(b: Bool) → JsonValue\n\nWrap boolean as JSON value.',
    'json_bool': '**json_bool**(b: Bool) → JsonValue\n\nWrap boolean as JSON value.',
    'jn': '**json_null**() → JsonValue\n\nCreate JSON null value.',
    'json_null': '**json_null**() → JsonValue\n\nCreate JSON null value.',
    'jp': '**json_parse**(s: String) → JsonValue\n\nParse JSON string. Returns null on error.',
    'jparse': '**json_parse**(s: String) → JsonValue\n\nParse JSON string. Returns null on error.',
    'json_parse': '**json_parse**(s: String) → JsonValue\n\nParse JSON string. Returns null on error.',
    'jfy': '**json_stringify**(v: JsonValue) → String\n\nSerialize JSON value to compact string.',
    'jstringify': '**json_stringify**(v: JsonValue) → String\n\nSerialize JSON value to compact string.',
    'json_stringify': '**json_stringify**(v: JsonValue) → String\n\nSerialize JSON value to compact string.',

    // HTTP
    'hg': '**http_get**(url: String) → HttpResponse\n\nPerform HTTP GET request. Status 0 on error.',
    'hget': '**http_get**(url: String) → HttpResponse\n\nPerform HTTP GET request. Status 0 on error.',
    'http_get': '**http_get**(url: String) → HttpResponse\n\nPerform HTTP GET request. Status 0 on error.',
    'hp': '**http_post**(url: String, body: String, content_type: String) → HttpResponse\n\nPerform HTTP POST request.',
    'hpost': '**http_post**(url: String, body: String, content_type: String) → HttpResponse\n\nPerform HTTP POST request.',
    'http_post': '**http_post**(url: String, body: String, content_type: String) → HttpResponse\n\nPerform HTTP POST request.',
    'listen': '**http_listen**(port: Int) → HttpServer\n\nStart HTTP server on port.',
    'hl': '**http_listen**(port: Int) → HttpServer\n\nStart HTTP server on port.',
    'http_listen': '**http_listen**(port: Int) → HttpServer\n\nStart HTTP server on port.',

    // Logging
    'ld': '**log_debug**(msg: String) → Int\n\nLog at DEBUG level to stderr.',
    'log_debug': '**log_debug**(msg: String) → Int\n\nLog at DEBUG level to stderr.',
    'li': '**log_info**(msg: String) → Int\n\nLog at INFO level to stderr.',
    'log_info': '**log_info**(msg: String) → Int\n\nLog at INFO level to stderr.',
    'lw': '**log_warn**(msg: String) → Int\n\nLog at WARN level to stderr.',
    'log_warn': '**log_warn**(msg: String) → Int\n\nLog at WARN level to stderr.',
    'le': '**log_error**(msg: String) → Int\n\nLog at ERROR level to stderr.',
    'log_error': '**log_error**(msg: String) → Int\n\nLog at ERROR level to stderr.',
    'll': '**log_set_level**(level: Int) → Int\n\n0=DEBUG, 1=INFO, 2=WARN, 3=ERROR.',
    'log_set_level': '**log_set_level**(level: Int) → Int\n\n0=DEBUG, 1=INFO, 2=WARN, 3=ERROR.',

    // Result
    'ok': '**ok**(value: T) → Result<T>\n\nWrap value in successful Result.',
    'err': '**err**(message: String) → Result<_>\n\nCreate error Result with message.',

    // Type aliases
    'I': '**Int** — 64-bit signed integer',
    'F': '**Float** — 64-bit floating point',
    'B': '**Bool** — Boolean (true/false)',
    'S': '**String** — UTF-8 string',
    'R': '**Result<T>** — Success or error value',

    // Keywords
    'fn': '**fn** — Function definition keyword (optional in Sans)',
    'let': '**let** — Variable binding (optional — bare assignment also works)',
    'mut': '**mut** — Mutable variable modifier',
    'struct': '**struct** — Define a struct type',
    'enum': '**enum** — Define an enum type',
    'trait': '**trait** — Define a trait interface',
    'impl': '**impl** — Implement methods or traits for a type',
    'match': '**match** — Pattern matching expression',
    'spawn': '**spawn** — Spawn a new thread',
    'channel': '**channel**<T>() — Create sender/receiver pair',
    'mutex': '**mutex**(value) — Create mutex wrapping a value',
    'import': '**import** "path" — Import a module',

    // CLI tools
    'lint': '**sans lint** <file|dir> — Static analysis without building.\n\nRules: unused-imports, unreachable-code, empty-catch, shadowed-vars, unnecessary-mut.\n\n`--error=<rule>` promotes to error. `--quiet` suppresses warnings.\n\nConfig in sans.json: `{"lint":{"rule":"error|warn|off"}}`.',
};

function activate(context) {
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
exports.activate = activate;

function deactivate() {}
exports.deactivate = deactivate;
