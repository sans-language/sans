# Multi-Editor Support Design

**Date:** 2026-04-08
**Status:** Approved
**Goal:** Add Neovim, Emacs, and JetBrains support so all editors get the same Sans LSP features via the shared `sans-lsp` binary.

---

## 1. Architecture

All editors share:
- **`sans-lsp` binary** — already built, supports 13 LSP methods (hover, go-to-definition, completion, diagnostics, semantic tokens, references, rename, folding, signature help, document/workspace symbols, code actions)
- **TextMate grammar** — existing `sans.tmLanguage.json` with 100+ builtins, all keywords, types, operators
- **Language configuration** — comment style (`//`), bracket pairs, indentation rules

Each editor gets a minimal directory under `editors/` with LSP client config, syntax highlighting, and a README.

## 2. Neovim (`editors/neovim-sans/`)

### Files

**`ftdetect/sans.vim`** — Filetype detection:
```vim
autocmd BufRead,BufNewFile *.sans set filetype=sans
```

**`ftplugin/sans.vim`** — Language settings:
```vim
setlocal commentstring=//\ %s
setlocal shiftwidth=2
setlocal tabstop=2
setlocal expandtab
```

**`syntax/sans.vim`** — Vim syntax highlighting derived from the TextMate grammar patterns. Covers:
- Keywords: if, else, while, for, in, match, struct, enum, trait, impl, import, pub, return, break, continue, defer, spawn, select, as, dyn, g
- Types: I, S, B, F, Int, String, Bool, Float, Array, Map, Option, Result, R, O, M, J, Fn, JsonValue
- Builtins: p, print, str, slen, fr, fw, append, exit, args, time, random, json_parse, json_get, json_set, ok, err, some, none, etc.
- Strings, numbers, comments, operators
- Function definitions and calls

**`lua/sans/init.lua`** — LSP setup using Neovim's built-in LSP client:
```lua
-- Standalone setup (no lspconfig dependency)
vim.api.nvim_create_autocmd("FileType", {
  pattern = "sans",
  callback = function()
    vim.lsp.start({
      name = "sans-lsp",
      cmd = { "sans-lsp" },
      root_dir = vim.fs.dirname(vim.fs.find({ "sans.json" }, { upward = true })[1]) or vim.fn.getcwd(),
    })
  end,
})
```

Also document nvim-lspconfig integration for users who prefer it.

**`README.md`** — Installation instructions covering:
1. Copy/symlink to `~/.config/nvim/` or use a plugin manager
2. Ensure `sans-lsp` is on PATH
3. Features available (list all 13 LSP methods)
4. Optional: nvim-lspconfig snippet

### No tree-sitter grammar
The Vim syntax file + LSP semantic tokens provide sufficient highlighting. Tree-sitter grammar is deferred.

## 3. Emacs (`editors/emacs-sans/`)

### Files

**`sans-mode.el`** — Major mode providing:
- Syntax highlighting via `font-lock` keywords (same coverage as the Vim syntax file)
- Comment support (`//`)
- Basic indentation (2-space indent on `{`, dedent on `}`)
- LSP integration via `eglot` (built into Emacs 29+):
  - Register `sans-lsp` as the LSP server for `sans-mode`
  - Auto-start on `.sans` files
- `auto-mode-alist` entry for `.sans` files

**`README.md`** — Installation instructions covering:
1. Add `sans-mode.el` to load-path
2. Ensure `sans-lsp` is on PATH
3. `(require 'sans-mode)` in init.el
4. Features available
5. Optional: `lsp-mode` configuration for users not on Emacs 29+

### Design notes
- Single file (`sans-mode.el`) keeps it simple — no package dependencies beyond built-in `eglot`
- `eglot` is preferred over `lsp-mode` since it's built into Emacs 29+ (no install required)
- Document `lsp-mode` as alternative for older Emacs versions

## 4. JetBrains (`editors/jetbrains-sans/`)

### Files

**`sans.tmbundle/`** — TextMate bundle directory:
- `Syntaxes/sans.tmLanguage.json` — copy of the existing TextMate grammar
- `info.plist` — bundle metadata (name, UUID, file types)

**`README.md`** — Installation instructions covering:
1. Import TextMate bundle: Settings > Editor > TextMate Bundles > add path
2. Install "LSP4IJ" plugin (free, supports generic LSP servers)
3. Configure LSP: Settings > Languages & Frameworks > Language Servers > add `sans-lsp` for `*.sans` files
4. Features available

### Design notes
- JetBrains IDEs natively import TextMate bundles for syntax highlighting
- LSP4IJ is the community-standard LSP plugin for JetBrains (replaces deprecated "LSP Support" plugin)
- No custom plugin code needed — just configuration and documentation

## 5. Shared Assets

The TextMate grammar (`editors/vscode-sans/syntaxes/sans.tmLanguage.json`) is the single source of truth for syntax patterns. The Neovim and Emacs syntax files are derived from it but written in their native formats (Vim script / Elisp) for better integration. JetBrains imports the `.tmLanguage.json` directly.

## 6. What We're NOT Building

- No tree-sitter grammar (deferred — TextMate + LSP semantic tokens is sufficient)
- No custom LSP client code (all editors use built-in LSP clients)
- No package manager publishing (no MELPA, vim-plug registry, JetBrains marketplace)
- No custom plugin UIs or commands beyond what the LSP provides

## 7. Success Criteria

- [ ] Neovim: `.sans` files get syntax highlighting, LSP starts automatically, hover/go-to-def/completion work
- [ ] Emacs: `sans-mode` activates on `.sans` files, font-lock highlighting works, eglot connects to `sans-lsp`
- [ ] JetBrains: TextMate bundle provides highlighting, LSP4IJ connects to `sans-lsp` for hover/completion/diagnostics
- [ ] All three editors documented with step-by-step installation instructions
- [ ] All editors get the same feature set via the shared LSP server
