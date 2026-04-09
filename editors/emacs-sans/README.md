# Sans Mode for Emacs

Emacs major mode for the [Sans programming language](https://github.com/sans-lang/sans) with syntax highlighting, indentation, and LSP support via eglot.

## Prerequisites

- **Emacs 27.1+** (27.1 minimum; 29+ recommended for built-in eglot)
- **`sans-lsp`** on your `PATH` (for LSP features)

## Installation

### Manual

1. Clone or copy the `editors/emacs-sans/` directory somewhere on your machine.

2. Add the following to your Emacs init file (`~/.emacs.d/init.el` or `~/.emacs`):

```elisp
(add-to-list 'load-path "/path/to/editors/emacs-sans")
(require 'sans-mode)
```

### use-package

```elisp
(use-package sans-mode
  :load-path "/path/to/editors/emacs-sans"
  :mode "\\.sans\\'")
```

### straight.el

```elisp
(straight-use-package
 '(sans-mode :type git :host github :repo "sans-lang/sans"
             :files ("editors/emacs-sans/*.el")))
```

## LSP Support

### Eglot (Emacs 29+, recommended)

`sans-mode` automatically registers `sans-lsp` with eglot. Just enable eglot in Sans buffers:

```elisp
;; Auto-start eglot when opening .sans files
(add-hook 'sans-mode-hook #'eglot-ensure)
```

Make sure `sans-lsp` is on your `PATH`. You can customize the executable path:

```elisp
(setq sans-lsp-executable "/path/to/sans-lsp")
```

### lsp-mode (alternative for older Emacs)

If you prefer `lsp-mode` over eglot:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(sans-mode . "sans"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection "sans-lsp")
    :activation-fn (lsp-activate-on "sans")
    :server-id 'sans-lsp)))

(add-hook 'sans-mode-hook #'lsp-deferred)
```

## Features

- **Syntax highlighting** for keywords, types, built-in functions, constants, operators, strings, comments, and function definitions
- **Indentation** with automatic indent after `{` and dedent on `}`
- **Comment support** (`//` line comments, `M-;` to toggle)
- **Electric pairs** for `{}`, `()`, `[]`, and `""`
- **Eglot integration** with auto-registered `sans-lsp` server
- **Auto-mode** association for `.sans` files

## Customization

| Variable | Default | Description |
|---|---|---|
| `sans-indent-offset` | `4` | Number of spaces per indentation level |
| `sans-lsp-executable` | `"sans-lsp"` | Path to the Sans language server |

```elisp
(setq sans-indent-offset 2)
```

## License

MIT
