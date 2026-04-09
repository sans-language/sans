;;; sans-mode.el --- Major mode for the Sans programming language -*- lexical-binding: t; -*-

;; Copyright (C) 2024-2026 Sans Language Contributors
;; Author: Sans Language Contributors
;; URL: https://github.com/sans-lang/sans
;; Version: 0.8.6
;; Keywords: languages, sans
;; Package-Requires: ((emacs "27.1"))

;; This file is not part of GNU Emacs.

;; This program is free software: you can redistribute it and/or modify
;; it under the terms of the MIT License.

;;; Commentary:

;; Major mode for editing Sans programming language source files (.sans).
;; Provides syntax highlighting, indentation, comment support, and
;; optional LSP integration via eglot (Emacs 29+) or lsp-mode.

;;; Code:

(require 'prog-mode)

;; ---------------------------------------------------------------------
;; Customization
;; ---------------------------------------------------------------------

(defgroup sans nil
  "Major mode for the Sans programming language."
  :group 'languages
  :prefix "sans-")

(defcustom sans-indent-offset 4
  "Number of spaces for each indentation level in Sans mode."
  :type 'integer
  :group 'sans)

(defcustom sans-lsp-executable "sans-lsp"
  "Path or name of the Sans language server executable."
  :type 'string
  :group 'sans)

;; ---------------------------------------------------------------------
;; Syntax table
;; ---------------------------------------------------------------------

(defvar sans-mode-syntax-table
  (let ((st (make-syntax-table)))
    ;; // line comments
    (modify-syntax-entry ?/ ". 12" st)
    (modify-syntax-entry ?\n ">" st)
    ;; Strings
    (modify-syntax-entry ?\" "\"" st)
    ;; Brackets / parens
    (modify-syntax-entry ?\( "()" st)
    (modify-syntax-entry ?\) ")(" st)
    (modify-syntax-entry ?\[ "(]" st)
    (modify-syntax-entry ?\] ")[" st)
    (modify-syntax-entry ?\{ "(}" st)
    (modify-syntax-entry ?\} "){" st)
    ;; Underscore is a word constituent (for identifiers)
    (modify-syntax-entry ?_ "w" st)
    st)
  "Syntax table for `sans-mode'.")

;; ---------------------------------------------------------------------
;; Font-lock (syntax highlighting)
;; ---------------------------------------------------------------------

(defconst sans-keywords-control
  '("if" "else" "while" "for" "in" "match" "return"
    "spawn" "break" "continue" "defer" "select")
  "Sans control-flow keywords.")

(defconst sans-keywords-declaration
  '("fn" "let" "mut" "struct" "enum" "trait" "impl"
    "import" "pub" "g")
  "Sans declaration keywords.")

(defconst sans-keywords-other
  '("self" "Self" "channel" "mutex" "array" "as" "dyn")
  "Other Sans keywords.")

(defconst sans-types-primitive
  '("Int" "Float" "Bool" "String" "I" "F" "B" "S")
  "Sans primitive type names.")

(defconst sans-types-builtin
  '("Array" "Option" "Result" "JsonValue"
    "HttpResponse" "HttpServer" "HttpRequest"
    "Sender" "Receiver" "Mutex" "JoinHandle"
    "R" "O" "M" "HS" "HR" "Fn" "Map" "J")
  "Sans built-in type names.")

(defconst sans-builtins
  '(;; I/O
    "print" "p" "file_read" "fread" "fr" "file_write" "fwrite" "fw"
    "file_append" "fappend" "fa" "file_exists" "fexists" "fe"
    "read_file" "write_file" "args" "stdin_read_line" "srl"
    "stdin_read_bytes" "srb"
    ;; Conversion
    "int_to_string" "str" "itos" "string_to_int" "stoi"
    "int_to_float" "itof" "float_to_int" "ftoi"
    "float_to_string" "ftos" "string_to_float" "stof"
    ;; Math
    "abs" "min" "max" "range"
    "floor" "ceil" "round" "sqrt" "sin" "cos" "tan"
    "asin" "acos" "atan" "atan2" "log" "log10" "exp"
    "pow" "fabs" "fmin" "fmax" "PI" "E_CONST"
    ;; System
    "sleep" "time" "now" "random" "rand" "print_err" "exit"
    "time_now" "tnow" "time_format" "tfmt" "time_year" "tyear"
    "time_month" "tmon" "time_day" "tday" "time_hour" "thour"
    "time_minute" "tmin" "time_second" "tsec" "time_weekday" "twday"
    "time_add" "tadd" "time_diff" "tdiff"
    ;; JSON
    "json_parse" "jparse" "jp" "json_object" "jobj" "jo"
    "json_array" "jarr" "ja" "json_string" "jstr" "js"
    "json_int" "ji" "json_bool" "jb" "json_null" "jn"
    "json_stringify" "jstringify" "jfy"
    ;; HTTP
    "http_get" "hget" "hg" "http_post" "hpost" "hp"
    "http_listen" "listen" "hl" "https_listen" "hl_s"
    "serve" "serve_tls" "stream_write" "stream_end"
    "cors" "cors_all" "ca" "ssl_ctx" "ssl_accept"
    "ssl_read" "ssl_write" "ssl_close"
    "ws_send" "ws_recv" "ws_close" "is_ws_upgrade" "upgrade_ws"
    "serve_file" "url_decode" "ud" "path_segment" "ps"
    "respond_json" "rj" "respond_stream" "query" "path_only"
    "content_length" "cl" "set_header" "set_max_workers"
    "set_read_timeout" "set_keepalive_timeout" "set_drain_timeout"
    "set_max_body" "set_max_headers" "set_max_header_count" "set_max_url"
    ;; Logging
    "log_debug" "ld" "log_info" "li" "log_warn" "lw"
    "log_error" "le" "log_set_level" "ll" "get_log_level" "set_log_level"
    ;; Result / Option
    "ok" "err" "some" "none"
    ;; Assert
    "assert" "assert_eq" "assert_ne" "assert_ok" "assert_err"
    "assert_some" "assert_none"
    ;; Memory / low-level
    "alloc" "dealloc" "ralloc" "mcpy" "mcmp" "mzero" "slen"
    "load8" "store8" "load16" "store16" "load32" "store32"
    "load64" "store64" "strstr" "bswap16" "bxor" "band" "bor"
    "bshl" "bshr" "system" "sys" "wfd"
    "arena_begin" "arena_alloc" "arena_end" "ab" "aa" "ae"
    "mget" "mset" "mhas"
    "signal_handler" "signal_check" "sigh" "sigc" "spoll"
    "gzip_compress" "gz" "setjmp" "longjmp"
    "panic_enable" "panic_disable" "panic_is_active"
    "panic_get_buf" "panic_fire"
    "pmutex_init" "pmutex_lock" "pmutex_unlock"
    ;; Pointer
    "fptr" "fcall" "fcall2" "fcall3" "ptr" "char_at"
    ;; Path
    "path_join" "pjoin" "path_dir" "pdir" "path_base" "pbase"
    "path_ext" "pext" "path_stem" "pstem" "path_is_abs" "pabs"
    ;; Encoding
    "base64_encode" "b64e" "base64_decode" "b64d"
    "url_encode" "urle" "urld" "hex_encode" "hexe" "hex_decode" "hexd"
    ;; Filesystem
    "getenv" "genv" "mkdir" "rmdir" "remove" "rm"
    "listdir" "ls" "is_dir" "sh" "shell"
    ;; Socket
    "sock" "sbind" "slisten" "saccept" "srecv" "ssend" "sclose"
    "rbind" "rsetsockopt"
    ;; Curl
    "cinit" "csets" "cseti" "cperf" "cclean" "cinfo"
    "curl_slist_append" "curl_slist_free"
    ;; Regex
    "regex_match" "rmatch" "regex_find" "rfind"
    "regex_replace" "rrepl")
  "Sans built-in functions.")

(defconst sans-constants
  '("true" "false")
  "Sans boolean constants.")

(defvar sans-font-lock-keywords
  (let ((kw-control   (regexp-opt sans-keywords-control   'symbols))
        (kw-decl      (regexp-opt sans-keywords-declaration 'symbols))
        (kw-other     (regexp-opt sans-keywords-other     'symbols))
        (tp-prim      (regexp-opt sans-types-primitive     'symbols))
        (tp-builtin   (regexp-opt sans-types-builtin      'symbols))
        (bi           (regexp-opt sans-builtins            'symbols))
        (consts       (regexp-opt sans-constants           'symbols)))
    `(
      ;; Function definitions: fn name(
      ("\\<fn\\s-+\\([a-zA-Z_][a-zA-Z0-9_]*\\)" 1 font-lock-function-name-face)
      ;; Declaration keywords
      (,kw-decl  . font-lock-keyword-face)
      ;; Control keywords
      (,kw-control . font-lock-keyword-face)
      ;; Other keywords
      (,kw-other . font-lock-keyword-face)
      ;; Constants
      (,consts . font-lock-constant-face)
      ;; Primitive types
      (,tp-prim . font-lock-type-face)
      ;; Built-in types
      (,tp-builtin . font-lock-type-face)
      ;; Built-in functions
      (,bi . font-lock-builtin-face)
      ;; := operator
      (":=" . font-lock-keyword-face)))
  "Font-lock keywords for `sans-mode'.")

;; ---------------------------------------------------------------------
;; Indentation
;; ---------------------------------------------------------------------

(defun sans-indent-line ()
  "Indent the current line according to Sans syntax."
  (interactive)
  (let ((indent 0)
        (cur-indent 0))
    (save-excursion
      ;; Calculate indentation from the previous non-blank line
      (beginning-of-line)
      (when (not (bobp))
        (forward-line -1)
        (while (and (looking-at-p "^\\s-*$") (not (bobp)))
          (forward-line -1))
        (setq cur-indent (current-indentation))
        ;; If previous line ends with {, increase indent
        (end-of-line)
        (when (save-excursion
                (skip-chars-backward " \t")
                (eq (char-before) ?\{))
          (setq cur-indent (+ cur-indent sans-indent-offset)))))
    (setq indent cur-indent)
    ;; If current line starts with }, decrease indent
    (save-excursion
      (beginning-of-line)
      (when (looking-at "^\\s-*}")
        (setq indent (max 0 (- indent sans-indent-offset)))))
    (indent-line-to indent)))

;; ---------------------------------------------------------------------
;; Comment support
;; ---------------------------------------------------------------------

(defun sans-comment-dwim (arg)
  "Comment or uncomment current line or region using // style.
With prefix ARG, call `comment-dwim' directly."
  (interactive "*P")
  (let ((comment-start "// ")
        (comment-end ""))
    (comment-dwim arg)))

;; ---------------------------------------------------------------------
;; Major mode definition
;; ---------------------------------------------------------------------

;;;###autoload
(define-derived-mode sans-mode prog-mode "Sans"
  "Major mode for editing Sans programming language source files.

\\{sans-mode-map}"
  :syntax-table sans-mode-syntax-table
  :group 'sans

  ;; Font-lock
  (setq font-lock-defaults '(sans-font-lock-keywords))

  ;; Comments
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+\\s-*")

  ;; Indentation
  (setq-local indent-line-function #'sans-indent-line)
  (setq-local tab-width sans-indent-offset)
  (setq-local indent-tabs-mode nil)

  ;; Electric pairs
  (setq-local electric-pair-pairs
              '((?\{ . ?\})
                (?\( . ?\))
                (?\[ . ?\])
                (?\" . ?\"))))

;; Key bindings
(define-key sans-mode-map (kbd "M-;") #'sans-comment-dwim)

;; ---------------------------------------------------------------------
;; Auto-mode
;; ---------------------------------------------------------------------

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.sans\\'" . sans-mode))

;; ---------------------------------------------------------------------
;; Eglot integration (Emacs 29+)
;; ---------------------------------------------------------------------

(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               `(sans-mode . (,sans-lsp-executable))))

;; ---------------------------------------------------------------------
;; Provide
;; ---------------------------------------------------------------------

(provide 'sans-mode)

;;; sans-mode.el ends here
