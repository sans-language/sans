# Sans for Neovim

Neovim plugin providing syntax highlighting and LSP support for the [Sans](https://sans-lang.com) programming language.

## Prerequisites

- **Neovim 0.8+** (required for `vim.lsp.start` and `vim.fs` APIs)
- **`sans-lsp`** on your `PATH` (install from the Sans release or build from source)

## Installation

### Option 1: Copy to your config

Copy or symlink the plugin directory into your Neovim runtime path:

```sh
# Copy
cp -r editors/neovim-sans/* ~/.config/nvim/

# Or symlink
ln -s /path/to/editors/neovim-sans/ftdetect/sans.vim ~/.config/nvim/ftdetect/sans.vim
ln -s /path/to/editors/neovim-sans/ftplugin/sans.vim ~/.config/nvim/ftplugin/sans.vim
ln -s /path/to/editors/neovim-sans/syntax/sans.vim ~/.config/nvim/syntax/sans.vim
ln -s /path/to/editors/neovim-sans/lua/sans ~/.config/nvim/lua/sans
```

### Option 2: Plugin manager

#### lazy.nvim

```lua
{
  dir = "/path/to/editors/neovim-sans",
  ft = "sans",
  config = function()
    require("sans").setup()
  end,
}
```

#### packer.nvim

```lua
use {
  "/path/to/editors/neovim-sans",
  ft = "sans",
  config = function()
    require("sans").setup()
  end,
}
```

#### vim-plug

```vim
Plug '/path/to/editors/neovim-sans'
```

Then add to your `init.lua`:

```lua
require("sans").setup()
```

## Setup

Add this to your `init.lua` to enable the LSP client:

```lua
require("sans").setup()
```

### Custom LSP path

If `sans-lsp` is not on your `PATH`, specify the full path:

```lua
require("sans").setup({
  cmd = { "/path/to/sans-lsp" },
})
```

### Using nvim-lspconfig

If you prefer `nvim-lspconfig`, you can configure the LSP manually instead of using the built-in setup:

```lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.sans_lsp then
  configs.sans_lsp = {
    default_config = {
      cmd = { "sans-lsp" },
      filetypes = { "sans" },
      root_dir = lspconfig.util.root_pattern("sans.json"),
      settings = {},
    },
  }
end

lspconfig.sans_lsp.setup({})
```

## Features

### Syntax Highlighting

- Keywords, declarations, and control flow
- Primitive and built-in types
- All built-in functions and their short aliases
- Strings with interpolation and escape sequences
- Triple-quoted strings
- Comments
- Numbers (integer and float)
- Operators

### LSP Features

The Sans LSP server provides:

- **Hover** -- documentation on hover
- **Go to Definition** -- jump to symbol definitions
- **Completion** -- context-aware autocompletion
- **Diagnostics** -- inline error and warning reporting
- **Semantic Tokens** -- enhanced token-based highlighting
- **References** -- find all references to a symbol
- **Rename** -- rename symbols across files
- **Folding** -- code folding ranges
- **Signature Help** -- function signature information
- **Document Symbols** -- outline of symbols in the current file
- **Workspace Symbols** -- search symbols across the project
- **Code Actions** -- quick fixes and refactoring actions

## File Settings

The plugin automatically configures Sans files with:

- `//` comment style
- 2-space indentation with spaces (no tabs)
- `.sans` suffix for file lookups
