# Multi-Editor Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Neovim, Emacs, and JetBrains editor support via the shared `sans-lsp` binary and TextMate grammar.

**Architecture:** Each editor gets a minimal directory under `editors/` with LSP client config, syntax highlighting (derived from the existing TextMate grammar), and a README. All editors connect to the same `sans-lsp` binary.

**Tech Stack:** Vim script, Lua (Neovim), Emacs Lisp, TextMate bundles (JetBrains)

**Spec:** `docs/superpowers/specs/2026-04-08-multi-editor-support-design.md`

---

### Task 1: Neovim support

**Files:**
- Create: `editors/neovim-sans/ftdetect/sans.vim`
- Create: `editors/neovim-sans/ftplugin/sans.vim`
- Create: `editors/neovim-sans/syntax/sans.vim`
- Create: `editors/neovim-sans/lua/sans/init.lua`
- Create: `editors/neovim-sans/README.md`

- [ ] **Step 1: Create filetype detection**

Create `editors/neovim-sans/ftdetect/sans.vim`:
```vim
autocmd BufRead,BufNewFile *.sans set filetype=sans
```

- [ ] **Step 2: Create filetype plugin**

Create `editors/neovim-sans/ftplugin/sans.vim`:
```vim
setlocal commentstring=//\ %s
setlocal shiftwidth=2
setlocal tabstop=2
setlocal expandtab
setlocal suffixesadd=.sans
```

- [ ] **Step 3: Create syntax highlighting**

Create `editors/neovim-sans/syntax/sans.vim` with keywords, types, builtins, strings, comments, numbers, and operators derived from `editors/vscode-sans/syntaxes/sans.tmLanguage.json`.

Groups:
- `sansKeyword` → control flow: if, else, while, for, in, match, return, spawn, break, continue, defer, select
- `sansDeclaration` → declarations: fn, let, mut, struct, enum, trait, impl, import, pub, g
- `sansBoolean` → true, false
- `sansOther` → self, Self, channel, mutex, array
- `sansType` → I, F, B, S, Int, Float, Bool, String
- `sansBuiltinType` → Array, Option, Result, JsonValue, HttpResponse, HttpServer, HttpRequest, Sender, Receiver, Mutex, JoinHandle, R, O, M, HS, HR, Fn, Map, J
- `sansBuiltinFn` → all builtin functions from the TextMate grammar (p, print, str, slen, fr, fw, ok, err, some, none, json_parse, http_get, alloc, etc.)
- `sansString` → double-quoted strings with escape sequences
- `sansTripleString` → triple-quoted strings
- `sansComment` → // line comments
- `sansNumber` → integers and floats
- `sansOperator` → :=, +=, ==, !=, &&, ||, ?, !, =>

Link groups to standard Vim highlight groups (Keyword, Type, Function, String, Comment, Number, Operator, Boolean).

- [ ] **Step 4: Create LSP configuration**

Create `editors/neovim-sans/lua/sans/init.lua`:
```lua
local M = {}

function M.setup(opts)
  opts = opts or {}
  local cmd = opts.cmd or { "sans-lsp" }

  vim.api.nvim_create_autocmd("FileType", {
    pattern = "sans",
    callback = function(args)
      vim.lsp.start({
        name = "sans-lsp",
        cmd = cmd,
        root_dir = vim.fs.dirname(
          vim.fs.find({ "sans.json" }, { upward = true, path = vim.api.nvim_buf_get_name(args.buf) })[1]
        ) or vim.fn.getcwd(),
      })
    end,
  })
end

return M
```

- [ ] **Step 5: Create README**

Create `editors/neovim-sans/README.md` with:
- Prerequisites (Neovim 0.8+, `sans-lsp` on PATH)
- Installation (copy to `~/.config/nvim/` or use plugin manager)
- Setup: `require("sans").setup()` in init.lua
- Optional: custom LSP path via `require("sans").setup({ cmd = { "/path/to/sans-lsp" } })`
- Optional: nvim-lspconfig snippet
- Features list (all 13 LSP methods)

- [ ] **Step 6: Commit**

```bash
git add editors/neovim-sans/
git commit -m "feat: add Neovim support with syntax highlighting and LSP"
```

---

### Task 2: Emacs support

**Files:**
- Create: `editors/emacs-sans/sans-mode.el`
- Create: `editors/emacs-sans/README.md`

- [ ] **Step 1: Create major mode**

Create `editors/emacs-sans/sans-mode.el` with:

```elisp
;;; sans-mode.el --- Major mode for the Sans programming language -*- lexical-binding: t; -*-

;; Author: Sans Language Team
;; URL: https://github.com/sans-language/sans
;; Version: 0.8.6
;; Keywords: languages

;;; Commentary:
;; Major mode for editing Sans source files (.sans).
;; Provides syntax highlighting and LSP integration via eglot.

;;; Code:

(defvar sans-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?/ ". 12" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\{ "(}" table)
    (modify-syntax-entry ?\} "){" table)
    (modify-syntax-entry ?\[ "(]" table)
    (modify-syntax-entry ?\] ")[" table)
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    (modify-syntax-entry ?_ "w" table)
    table))

(defconst sans-keywords
  '("if" "else" "while" "for" "in" "match" "return" "spawn"
    "break" "continue" "defer" "select" "fn" "let" "mut"
    "struct" "enum" "trait" "impl" "import" "pub" "g"
    "self" "Self" "channel" "mutex" "array" "as" "dyn"))

(defconst sans-types
  '("I" "F" "B" "S" "Int" "Float" "Bool" "String"
    "Array" "Option" "Result" "JsonValue" "HttpResponse"
    "HttpServer" "HttpRequest" "Sender" "Receiver" "Mutex"
    "JoinHandle" "R" "O" "M" "HS" "HR" "Fn" "Map" "J"))

(defconst sans-builtins
  '("print" "p" "str" "slen" "fr" "fw" "fa" "fe" "ok" "err"
    "some" "none" "exit" "args" "time" "random" "sleep"
    "json_parse" "jp" "json_object" "jo" "json_stringify" "jfy"
    "http_get" "hg" "http_post" "hp" "listen" "serve"
    "log_info" "li" "log_warn" "lw" "log_error" "le"
    "alloc" "load64" "store64" "assert" "assert_eq"
    "getenv" "genv" "ls" "is_dir" "sh" "mkdir"))

(defconst sans-constants '("true" "false"))

(defvar sans-font-lock-keywords
  `((,(regexp-opt sans-constants 'words) . font-lock-constant-face)
    (,(regexp-opt sans-keywords 'words) . font-lock-keyword-face)
    (,(regexp-opt sans-types 'words) . font-lock-type-face)
    (,(regexp-opt sans-builtins 'words) . font-lock-builtin-face)
    ("\\b\\([a-zA-Z_][a-zA-Z0-9_]*\\)\\s-*(" 1 font-lock-function-name-face)
    (":=" . font-lock-keyword-face)))

(defun sans-indent-line ()
  "Indent current line of Sans code."
  (interactive)
  (let ((indent 0)
        (prev-indent 0))
    (save-excursion
      (forward-line -1)
      (setq prev-indent (current-indentation))
      (end-of-line)
      (skip-chars-backward " \t")
      (when (and (> (point) (line-beginning-position))
                 (eq (char-before) ?\{))
        (setq prev-indent (+ prev-indent 2))))
    (setq indent prev-indent)
    (save-excursion
      (beginning-of-line)
      (skip-chars-forward " \t")
      (when (eq (char-after) ?\})
        (setq indent (max 0 (- indent 2)))))
    (indent-line-to indent)))

;;;###autoload
(define-derived-mode sans-mode prog-mode "Sans"
  "Major mode for editing Sans source files."
  :syntax-table sans-mode-syntax-table
  (setq-local font-lock-defaults '(sans-font-lock-keywords))
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local indent-line-function #'sans-indent-line)
  (setq-local tab-width 2)
  (setq-local indent-tabs-mode nil))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.sans\\'" . sans-mode))

;; Eglot integration (Emacs 29+)
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs '(sans-mode . ("sans-lsp"))))

(provide 'sans-mode)
;;; sans-mode.el ends here
```

- [ ] **Step 2: Create README**

Create `editors/emacs-sans/README.md` with:
- Prerequisites (Emacs 29+ for eglot, `sans-lsp` on PATH)
- Installation (add to load-path, `(require 'sans-mode)`)
- Eglot auto-starts on `.sans` files
- Alternative: lsp-mode config for older Emacs
- Features list

- [ ] **Step 3: Commit**

```bash
git add editors/emacs-sans/
git commit -m "feat: add Emacs support with sans-mode and eglot LSP"
```

---

### Task 3: JetBrains support

**Files:**
- Create: `editors/jetbrains-sans/sans.tmbundle/Syntaxes/sans.tmLanguage.json`
- Create: `editors/jetbrains-sans/sans.tmbundle/info.plist`
- Create: `editors/jetbrains-sans/README.md`

- [ ] **Step 1: Create TextMate bundle**

Copy `editors/vscode-sans/syntaxes/sans.tmLanguage.json` to `editors/jetbrains-sans/sans.tmbundle/Syntaxes/sans.tmLanguage.json`.

Create `editors/jetbrains-sans/sans.tmbundle/info.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>name</key>
    <string>Sans</string>
    <key>uuid</key>
    <string>d4e3c2b1-a0f9-4e8d-b7c6-5a4b3c2d1e0f</string>
    <key>fileTypes</key>
    <array>
        <string>sans</string>
    </array>
</dict>
</plist>
```

- [ ] **Step 2: Create README**

Create `editors/jetbrains-sans/README.md` with:
- Import TextMate bundle: Settings > Editor > TextMate Bundles > + > select `sans.tmbundle` directory
- Install LSP4IJ plugin from JetBrains marketplace
- Configure LSP: Settings > Languages & Frameworks > Language Servers > add server with command `sans-lsp` for `*.sans` files
- Features list
- Supported IDEs (IntelliJ IDEA, WebStorm, PyCharm, GoLand, etc.)

- [ ] **Step 3: Commit**

```bash
git add editors/jetbrains-sans/
git commit -m "feat: add JetBrains support with TextMate bundle and LSP4IJ"
```

---

### Task 4: Documentation updates

**Files:**
- Modify: `README.md`
- Modify: `docs/reference.md`

- [ ] **Step 1: Update README**

Add an "Editor Support" section listing all four editors (VSCode, Neovim, Emacs, JetBrains) with links to their respective `editors/` directories.

- [ ] **Step 2: Update reference.md**

Add an "Editor Support" section under Tools with brief setup instructions for each editor and a note that all editors share the same LSP features.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/reference.md
git commit -m "docs: add multi-editor support documentation"
```
