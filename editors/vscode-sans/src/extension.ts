import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

const HOVER_DATA: Record<string, string> = {
    // Types — short forms
    "I": "```sans\nI (Int)\n```\n64-bit signed integer.",
    "F": "```sans\nF (Float)\n```\n64-bit floating point.",
    "B": "```sans\nB (Bool)\n```\nBoolean.",
    "S": "```sans\nS (String)\n```\nUTF-8 string.",
    "J": "```sans\nJ (JsonValue)\n```\nOpaque JSON value.",
    "R": "```sans\nR<T> (Result<T>)\n```\nOk/err container.",
    "O": "```sans\nO<T> (Option<T>)\n```\nSome/none container.",
    "M": "```sans\nM<K V> (Map<K,V>)\n```\nHash map. `M()` = `M<S I>()`.",
    "It": "```sans\nIt<T> (Iter<T>)\n```\nLazy pull-based iterator.",
    // Types — long forms
    "Int": "```sans\nI (Int)\n```\n64-bit signed integer.",
    "Float": "```sans\nF (Float)\n```\n64-bit floating point.",
    "Bool": "```sans\nB (Bool)\n```\nBoolean.",
    "String": "```sans\nS (String)\n```\nUTF-8 string.",
    "JsonValue": "```sans\nJ (JsonValue)\n```\nOpaque JSON value.",
    "Result": "```sans\nR<T> (Result<T>)\n```\nOk/err container.",
    "Option": "```sans\nO<T> (Option<T>)\n```\nSome/none container.",
    "Map": "```sans\nM<K V> (Map<K,V>)\n```\nHash map. `M()` = `M<S I>()`.",
    "Iter": "```sans\nIt<T> (Iter<T>)\n```\nLazy pull-based iterator.",

    // Keywords
    "if": "```sans\nif cond { ... }\n```\nConditional branch.",
    "else": "```sans\nelse { ... }\n```\nAlternate branch.",
    "while": "```sans\nwhile cond { ... }\n```\nLoop while condition is true.",
    "for": "```sans\nfor x in collection { ... }\n```\nIterate over collection.",
    "break": "```sans\nbreak\n```\nExit loop.",
    "continue": "```sans\ncontinue\n```\nSkip to next iteration.",
    "return": "```sans\nreturn val\n```\nReturn from function.",
    "match": "```sans\nmatch val { ... }\n```\nPattern matching.",
    "fn": "```sans\nfn name(args) -> T { ... }\n```\nFunction definition.",
    "struct": "```sans\nstruct Name { ... }\n```\nStruct definition.",
    "enum": "```sans\nenum Name { ... }\n```\nEnum definition.",
    "trait": "```sans\ntrait Name { ... }\n```\nTrait definition.",
    "impl": "```sans\nimpl Name { ... }\n```\nImplementation block.",
    "import": "```sans\nimport \"path\"\n```\nImport module.",
    "pub": "```sans\npub fn ...\n```\nPublic visibility modifier.",
    "defer": "```sans\ndefer expr\n```\nExecute expression when scope exits.",
    "spawn": "```sans\nspawn { ... }\n```\nSpawn concurrent task.",
    "select": "```sans\nselect { ... }\n```\nMultiplex channel operations.",
    "dyn": "```sans\ndyn Trait\n```\nDynamic dispatch.",
    "in": "```sans\nfor x in collection\n```\nIteration keyword.",

    // I/O & Print
    "p": "```sans\np(v) -> I\n```\nPrint value to stdout.",
    "print": "```sans\np(v) -> I\n```\nPrint value to stdout.",
    "fr": "```sans\nfr(path) -> S\n```\nRead file contents.",
    "file_read": "```sans\nfr(path) -> S\n```\nRead file contents.",
    "fread": "```sans\nfr(path) -> S\n```\nRead file contents.",
    "fw": "```sans\nfw(path body) -> I\n```\nWrite file.",
    "file_write": "```sans\nfw(path body) -> I\n```\nWrite file.",
    "fwrite": "```sans\nfw(path body) -> I\n```\nWrite file.",
    "fa": "```sans\nfa(path body) -> I\n```\nAppend to file.",
    "file_append": "```sans\nfa(path body) -> I\n```\nAppend to file.",
    "fappend": "```sans\nfa(path body) -> I\n```\nAppend to file.",
    "fe": "```sans\nfe(path) -> B\n```\nCheck file exists.",
    "file_exists": "```sans\nfe(path) -> B\n```\nCheck file exists.",
    "fexists": "```sans\nfe(path) -> B\n```\nCheck file exists.",
    "rl": "```sans\nrl(path) -> [S]\n```\nRead file, split lines.",
    "read_lines": "```sans\nrl(path) -> [S]\n```\nRead file, split lines.",
    "wl": "```sans\nwl(path lines) -> I\n```\nJoin + trailing \\n.",
    "write_lines": "```sans\nwl(path lines) -> I\n```\nJoin + trailing \\n.",
    "al": "```sans\nal(path line) -> I\n```\nAppend line + \\n.",
    "append_line": "```sans\nal(path line) -> I\n```\nAppend line + \\n.",
    "read_line": "```sans\nread_line(prompt) -> S\n```\nPrint prompt, read stdin.",
    "srl": "```sans\nsrl() -> S\n```\nRead line from stdin.",
    "stdin_read_line": "```sans\nsrl() -> S\n```\nRead line from stdin.",
    "srb": "```sans\nsrb(n) -> S\n```\nRead n bytes from stdin.",
    "stdin_read_bytes": "```sans\nsrb(n) -> S\n```\nRead n bytes from stdin.",

    // Conversion
    "str": "```sans\nstr(n) -> S\n```\nInt to string.",
    "itos": "```sans\nstr(n) -> S\n```\nInt to string.",
    "int_to_string": "```sans\nstr(n) -> S\n```\nInt to string.",
    "stoi": "```sans\nstoi(s) -> I\n```\nString to int.",
    "string_to_int": "```sans\nstoi(s) -> I\n```\nString to int.",
    "itof": "```sans\nitof(n) -> F\n```\nInt to float.",
    "int_to_float": "```sans\nitof(n) -> F\n```\nInt to float.",
    "ftoi": "```sans\nftoi(f) -> I\n```\nFloat to int.",
    "float_to_int": "```sans\nftoi(f) -> I\n```\nFloat to int.",
    "ftos": "```sans\nftos(f) -> S\n```\nFloat to string.",
    "float_to_string": "```sans\nftos(f) -> S\n```\nFloat to string.",
    "stof": "```sans\nstof(s) -> F\n```\nString to float.",
    "string_to_float": "```sans\nstof(s) -> F\n```\nString to float.",

    // Math
    "abs": "```sans\nabs(n) -> I\n```\nAbsolute value.",
    "min": "```sans\nmin(a b) -> I\n```\nMinimum.",
    "max": "```sans\nmax(a b) -> I\n```\nMaximum.",
    "floor": "```sans\nfloor(x) -> F\n```\nFloor.",
    "ceil": "```sans\nceil(x) -> F\n```\nCeiling.",
    "round": "```sans\nround(x) -> F\n```\nRound.",
    "sqrt": "```sans\nsqrt(x) -> F\n```\nSquare root.",
    "sin": "```sans\nsin(x) -> F\n```\nSine.",
    "cos": "```sans\ncos(x) -> F\n```\nCosine.",
    "tan": "```sans\ntan(x) -> F\n```\nTangent.",
    "asin": "```sans\nasin(x) -> F\n```\nInverse sine.",
    "acos": "```sans\nacos(x) -> F\n```\nInverse cosine.",
    "atan": "```sans\natan(x) -> F\n```\nInverse tangent.",
    "atan2": "```sans\natan2(y x) -> F\n```\nTwo-argument arctangent.",
    "log": "```sans\nlog(x) -> F\n```\nNatural logarithm.",
    "log10": "```sans\nlog10(x) -> F\n```\nBase-10 logarithm.",
    "exp": "```sans\nexp(x) -> F\n```\nExponential.",
    "pow": "```sans\npow(base exp) -> F\n```\nExponentiation.",
    "fabs": "```sans\nfabs(x) -> F\n```\nFloat absolute value.",
    "fmin": "```sans\nfmin(a b) -> F\n```\nFloat minimum.",
    "fmax": "```sans\nfmax(a b) -> F\n```\nFloat maximum.",
    "PI": "```sans\nPI() -> F\n```\n3.141592653589793.",
    "E_CONST": "```sans\nE_CONST() -> F\n```\n2.718281828459045.",

    // Range & Iterator
    "range": "```sans\nrange(n) -> [I]\nrange(a b) -> [I]\n```\nEager integer range array.",
    "iter": "```sans\niter(n) -> It<I>\niter(a b) -> It<I>\n```\nLazy range iterator (no alloc).",

    // JSON
    "jo": "```sans\njo() -> J\n```\nEmpty JSON object.",
    "json_object": "```sans\njo() -> J\n```\nEmpty JSON object.",
    "ja": "```sans\nja() -> J\n```\nEmpty JSON array.",
    "json_array": "```sans\nja() -> J\n```\nEmpty JSON array.",
    "jp": "```sans\njp(s) -> R<J>\n```\nParse JSON string.",
    "json_parse": "```sans\njp(s) -> R<J>\n```\nParse JSON string.",
    "jparse": "```sans\njp(s) -> R<J>\n```\nParse JSON string.",
    "js": "```sans\njs(s) -> J\n```\nJSON string value.",
    "json_string": "```sans\njs(s) -> J\n```\nJSON string value.",
    "jstr": "```sans\njs(s) -> J\n```\nJSON string value.",
    "ji": "```sans\nji(n) -> J\n```\nJSON int value.",
    "json_int": "```sans\nji(n) -> J\n```\nJSON int value.",
    "jb": "```sans\njb(b) -> J\n```\nJSON bool value.",
    "json_bool": "```sans\njb(b) -> J\n```\nJSON bool value.",
    "jn": "```sans\njn() -> J\n```\nJSON null.",
    "json_null": "```sans\njn() -> J\n```\nJSON null.",
    "jfy": "```sans\njfy(v) -> S\n```\nSerialize to JSON string.",
    "json_stringify": "```sans\njfy(v) -> S\n```\nSerialize to JSON string.",
    "jstringify": "```sans\njfy(v) -> S\n```\nSerialize to JSON string.",

    // HTTP
    "hg": "```sans\nhg(url) -> HttpResponse\n```\nHTTP GET request.",
    "http_get": "```sans\nhg(url) -> HttpResponse\n```\nHTTP GET request.",
    "hp": "```sans\nhp(url body ct) -> HttpResponse\n```\nHTTP POST request.",
    "http_post": "```sans\nhp(url body ct) -> HttpResponse\n```\nHTTP POST request.",
    "listen": "```sans\nlisten(port) -> HttpServer\n```\nStart HTTP listener.",
    "http_listen": "```sans\nlisten(port) -> HttpServer\n```\nStart HTTP listener.",
    "hl": "```sans\nlisten(port) -> HttpServer\n```\nStart HTTP listener.",
    "serve": "```sans\nserve(port handler) -> I\n```\nProduction server with auto-threading.",
    "serve_tls": "```sans\nserve_tls(port cert key handler) -> I\n```\nHTTPS server.",
    "cors": "```sans\ncors(req origin) -> I\n```\nSet CORS headers.",
    "cors_all": "```sans\ncors_all(req) -> I\n```\nCORS wildcard.",
    "ca": "```sans\ncors_all(req) -> I\n```\nCORS wildcard.",

    // Result/Option
    "ok": "```sans\nok(v) -> R<T>\n```\nWrap value in Ok.",
    "err": "```sans\nerr(msg) -> R<_>\nerr(code msg) -> R<_>\n```\nCreate error result.",
    "some": "```sans\nsome(v) -> O<T>\n```\nWrap value in Some.",
    "none": "```sans\nnone() -> O<T>\n```\nNone value.",

    // Env & System
    "getenv": "```sans\ngetenv(name) -> S\n```\nRead environment variable.",
    "genv": "```sans\ngetenv(name) -> S\n```\nRead environment variable.",
    "args": "```sans\nargs() -> [S]\n```\nCommand-line arguments.",
    "sh": "```sans\nsh(cmd) -> S\n```\nExecute command, capture stdout.",
    "shell": "```sans\nsh(cmd) -> S\n```\nExecute command, capture stdout.",
    "sys": "```sans\nsys(cmd) -> I\n```\nRun command, return exit code.",
    "system": "```sans\nsys(cmd) -> I\n```\nRun command, return exit code.",
    "exit": "```sans\nexit(code) -> I\n```\nExit process.",
    "sleep": "```sans\nsleep(ms) -> I\n```\nPause milliseconds.",
    "time": "```sans\ntime() -> I\n```\nUnix timestamp.",
    "now": "```sans\ntime() -> I\n```\nUnix timestamp.",
    "random": "```sans\nrandom(max) -> I\n```\nCrypto-seeded random [0..max).",
    "rand": "```sans\nrandom(max) -> I\n```\nCrypto-seeded random [0..max).",

    // File System
    "mkdir": "```sans\nmkdir(path) -> I\n```\nCreate directory.",
    "rmdir": "```sans\nrmdir(path) -> I\n```\nRemove directory.",
    "remove": "```sans\nremove(path) -> I\n```\nDelete file.",
    "rm": "```sans\nremove(path) -> I\n```\nDelete file.",
    "listdir": "```sans\nlistdir(path) -> [S]\n```\nList directory contents.",
    "ls": "```sans\nlistdir(path) -> [S]\n```\nList directory contents.",
    "is_dir": "```sans\nis_dir(path) -> B\n```\nCheck if path is directory.",

    // Path
    "pjoin": "```sans\npjoin(a b) -> S\n```\nJoin paths with /.",
    "path_join": "```sans\npjoin(a b) -> S\n```\nJoin paths with /.",
    "pdir": "```sans\npdir(p) -> S\n```\nDirectory component.",
    "path_dir": "```sans\npdir(p) -> S\n```\nDirectory component.",
    "pbase": "```sans\npbase(p) -> S\n```\nFilename.",
    "path_base": "```sans\npbase(p) -> S\n```\nFilename.",
    "pext": "```sans\npext(p) -> S\n```\nExtension.",
    "path_ext": "```sans\npext(p) -> S\n```\nExtension.",
    "pstem": "```sans\npstem(p) -> S\n```\nFilename without extension.",
    "path_stem": "```sans\npstem(p) -> S\n```\nFilename without extension.",

    // Encoding
    "b64e": "```sans\nb64e(s) -> S\n```\nBase64 encode.",
    "base64_encode": "```sans\nb64e(s) -> S\n```\nBase64 encode.",
    "b64d": "```sans\nb64d(s) -> S\n```\nBase64 decode.",
    "base64_decode": "```sans\nb64d(s) -> S\n```\nBase64 decode.",
    "urle": "```sans\nurle(s) -> S\n```\nURL encode.",
    "url_encode": "```sans\nurle(s) -> S\n```\nURL encode.",
    "urld": "```sans\nurld(s) -> S\n```\nURL decode.",
    "url_decode": "```sans\nurld(s) -> S\n```\nURL decode.",
    "hexe": "```sans\nhexe(s) -> S\n```\nHex encode.",
    "hex_encode": "```sans\nhexe(s) -> S\n```\nHex encode.",
    "hexd": "```sans\nhexd(s) -> S\n```\nHex decode.",
    "hex_decode": "```sans\nhexd(s) -> S\n```\nHex decode.",

    // Crypto
    "sha256": "```sans\nsha256(s) -> S\n```\nSHA-256 hex digest.",
    "sha512": "```sans\nsha512(s) -> S\n```\nSHA-512 hex digest.",
    "md5": "```sans\nmd5(s) -> S\n```\nMD5 hex digest.",
    "hmac256": "```sans\nhmac256(key msg) -> S\n```\nHMAC-SHA256.",
    "hmac_sha256": "```sans\nhmac256(key msg) -> S\n```\nHMAC-SHA256.",
    "randb": "```sans\nrandb(n) -> S\n```\nN crypto random bytes as hex.",
    "random_bytes": "```sans\nrandb(n) -> S\n```\nN crypto random bytes as hex.",

    // Assertions
    "assert": "```sans\nassert(cond) -> I\n```\nFail if false.",
    "assert_eq": "```sans\nassert_eq(a b) -> I\n```\nFail if a != b.",
    "assert_ne": "```sans\nassert_ne(a b) -> I\n```\nFail if a == b.",
    "assert_ok": "```sans\nassert_ok(r) -> I\n```\nFail if err.",
    "assert_err": "```sans\nassert_err(r) -> I\n```\nFail if ok.",
    "assert_some": "```sans\nassert_some(o) -> I\n```\nFail if none.",
    "assert_none": "```sans\nassert_none(o) -> I\n```\nFail if some.",

    // Time
    "tnow": "```sans\ntnow() -> I\n```\nCurrent unix timestamp.",
    "time_now": "```sans\ntnow() -> I\n```\nCurrent unix timestamp.",
    "tfmt": "```sans\ntfmt(t fmt) -> S\n```\nFormat time with strftime.",
    "time_format": "```sans\ntfmt(t fmt) -> S\n```\nFormat time with strftime.",
    "tyear": "```sans\ntyear(t) -> I\n```\nExtract year.",
    "time_year": "```sans\ntyear(t) -> I\n```\nExtract year.",
    "tmon": "```sans\ntmon(t) -> I\n```\nExtract month.",
    "time_month": "```sans\ntmon(t) -> I\n```\nExtract month.",
    "tday": "```sans\ntday(t) -> I\n```\nExtract day.",
    "time_day": "```sans\ntday(t) -> I\n```\nExtract day.",
    "thour": "```sans\nthour(t) -> I\n```\nExtract hour.",
    "time_hour": "```sans\nthour(t) -> I\n```\nExtract hour.",
    "tmin": "```sans\ntmin(t) -> I\n```\nExtract minute.",
    "time_minute": "```sans\ntmin(t) -> I\n```\nExtract minute.",
    "tsec": "```sans\ntsec(t) -> I\n```\nExtract second.",
    "time_second": "```sans\ntsec(t) -> I\n```\nExtract second.",
    "twday": "```sans\ntwday(t) -> I\n```\nExtract weekday.",
    "time_weekday": "```sans\ntwday(t) -> I\n```\nExtract weekday.",
    "tadd": "```sans\ntadd(t n) -> I\n```\nAdd seconds to timestamp.",
    "time_add": "```sans\ntadd(t n) -> I\n```\nAdd seconds to timestamp.",
    "tdiff": "```sans\ntdiff(a b) -> I\n```\nDifference in seconds.",
    "time_diff": "```sans\ntdiff(a b) -> I\n```\nDifference in seconds.",

    // Unicode
    "char_count": "```sans\nchar_count(s) -> I\n```\nUTF-8 codepoint count.",
    "ccount": "```sans\nchar_count(s) -> I\n```\nUTF-8 codepoint count.",
    "chars": "```sans\nchars(s) -> [S]\n```\nSplit into UTF-8 chars.",
    "is_ascii": "```sans\nis_ascii(s) -> I\n```\n1 if all bytes < 128.",
    "utf8_valid": "```sans\nutf8_valid(s) -> I\n```\n1 if valid UTF-8.",
    "string_reverse": "```sans\nsrev(s) -> S\n```\nUTF-8 aware reverse.",
    "srev": "```sans\nsrev(s) -> S\n```\nUTF-8 aware reverse.",

    // Logging
    "ld": "```sans\nld(s) -> I\n```\nLog debug message.",
    "log_debug": "```sans\nld(s) -> I\n```\nLog debug message.",
    "li": "```sans\nli(s) -> I\n```\nLog info message.",
    "log_info": "```sans\nli(s) -> I\n```\nLog info message.",
    "lw": "```sans\nlw(s) -> I\n```\nLog warning message.",
    "log_warn": "```sans\nlw(s) -> I\n```\nLog warning message.",
    "le": "```sans\nle(s) -> I\n```\nLog error message.",
    "log_error": "```sans\nle(s) -> I\n```\nLog error message.",

    // Regex
    "rmatch": "```sans\nrmatch(pat txt) -> I\n```\n1 if pattern matches.",
    "regex_match": "```sans\nrmatch(pat txt) -> I\n```\n1 if pattern matches.",
    "rfind": "```sans\nrfind(pat txt) -> S\n```\nFirst regex match.",
    "regex_find": "```sans\nrfind(pat txt) -> S\n```\nFirst regex match.",
    "rrepl": "```sans\nrrepl(pat txt repl) -> S\n```\nReplace first regex match.",
    "regex_replace": "```sans\nrrepl(pat txt repl) -> S\n```\nReplace first regex match.",
};

const SHELL_METACHARACTERS = /[|;&$`()"'<>!#*?\[\]{}~\n\r]/;

function validateLspPath(lspPath: string): string | undefined {
    if (SHELL_METACHARACTERS.test(lspPath)) {
        vscode.window.showErrorMessage(
            `sans.lspPath contains invalid characters: "${lspPath}". Path must not contain shell metacharacters.`
        );
        return undefined;
    }
    if (lspPath.includes(' ')) {
        vscode.window.showErrorMessage(
            `sans.lspPath contains spaces (possible command injection): "${lspPath}". Path must point to a single executable.`
        );
        return undefined;
    }
    const resolved = path.resolve(lspPath);
    if (!fs.existsSync(resolved) && !fs.existsSync(lspPath)) {
        vscode.window.showErrorMessage(
            `sans.lspPath does not exist: "${lspPath}". Please set a valid path to the Sans language server.`
        );
        return undefined;
    }
    return lspPath;
}

export function activate(context: vscode.ExtensionContext) {
    const hoverProvider = vscode.languages.registerHoverProvider('sans', {
        provideHover(document, position) {
            const range = document.getWordRangeAtPosition(position);
            if (!range) return undefined;
            const word = document.getText(range);
            const info = HOVER_DATA[word];
            if (!info) return undefined;
            return new vscode.Hover(new vscode.MarkdownString(info));
        }
    });
    context.subscriptions.push(hoverProvider);

    const config = vscode.workspace.getConfiguration('sans');
    const lspPath = config.get<string>('lspPath', 'sans-lsp');

    const validatedPath = validateLspPath(lspPath);
    if (!validatedPath) {
        return;
    }

    const serverOptions: ServerOptions = {
        run: { command: validatedPath, args: [] },
        debug: { command: validatedPath, args: [] }
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

    client.start().catch((err: Error) => {
        const msg = err?.message || String(err);
        if (msg.includes('ENOENT') || msg.includes('not found')) {
            vscode.window.showErrorMessage(
                `Sans Language Server not found at "${lspPath}". ` +
                'Install it or set "sans.lspPath" in settings.'
            );
        } else {
            vscode.window.showErrorMessage(
                `Sans Language Server failed to start: ${msg}`
            );
        }
        client = undefined;
    });
    context.subscriptions.push({
        dispose: () => { if (client) { client.stop(); } }
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) { return undefined; }
    return client.stop();
}
