# Sans Language Support for JetBrains IDEs

Syntax highlighting and LSP support for the [Sans programming language](https://sans-lang.com) in JetBrains IDEs.

## Supported IDEs

- IntelliJ IDEA
- WebStorm
- PyCharm
- GoLand
- CLion
- Rider
- RubyMine
- PhpStorm
- Any other JetBrains IDE that supports TextMate bundles

## Prerequisites

- `sans-lsp` must be installed and available on your `PATH`. You can verify this by running:

  ```sh
  sans-lsp --version
  ```

## Installation

### Syntax Highlighting

Sans syntax highlighting is provided via a TextMate bundle.

1. Open your JetBrains IDE
2. Go to **Settings** (Ctrl+Alt+S / Cmd+,)
3. Navigate to **Editor > TextMate Bundles**
4. Click the **+** button
5. Select the `sans.tmbundle` directory from this repository (`editors/jetbrains-sans/sans.tmbundle`)
6. Click **OK** to apply

All `.sans` files will now have syntax highlighting.

### LSP Support

Language Server Protocol support provides intelligent features like hover documentation, completions, diagnostics, and more. This requires the **LSP4IJ** plugin.

#### 1. Install LSP4IJ Plugin

1. Open your JetBrains IDE
2. Go to **Settings** (Ctrl+Alt+S / Cmd+,)
3. Navigate to **Plugins > Marketplace**
4. Search for **LSP4IJ**
5. Click **Install** and restart your IDE

#### 2. Configure the Sans Language Server

1. Go to **Settings** (Ctrl+Alt+S / Cmd+,)
2. Navigate to **Languages & Frameworks > Language Servers**
3. Click **+** to add a new language server
4. Set the **Name** to `Sans`
5. Set the **Command** to `sans-lsp`
6. Set the **File pattern** to `*.sans`
7. Click **OK** to apply

## Features

With the TextMate bundle and LSP configured, you get:

- **Syntax highlighting** -- keywords, types, strings, numbers, operators, comments, and built-in functions
- **Hover documentation** -- hover over any keyword, function, or type to see docs
- **Autocompletion** -- context-aware completions for functions, types, and variables
- **Diagnostics** -- real-time error and warning reporting as you type
- **Go to Definition** -- jump to the definition of any symbol
- **Find References** -- find all usages of a symbol across your project
- **Rename** -- rename symbols across your entire project
- **Code actions** -- quick fixes and refactoring suggestions
- **Document symbols** -- outline view of functions, structs, enums, and traits
