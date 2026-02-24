# Editor Setup for Rein

Rein provides a first-party VS Code extension and a Tree-sitter grammar for Neovim. Both are included in the repository under `editors/` and `tree-sitter-rein/` respectively.

---

## Capabilities

| Capability | VS Code | Neovim (LSP) |
|------------|---------|--------------|
| Syntax highlighting | ✅ TextMate grammar | ✅ Tree-sitter grammar |
| Diagnostics | ✅ On save | ✅ On save |
| Hover (field docs) | ✅ Supported | ✅ Supported |
| Go-to-definition | ⚠️ Same file only | ⚠️ Same file only |
| Completion | ❌ Not yet implemented | ❌ Not yet implemented |
| Formatting | ✅ Via `rein fmt` (not LSP) | ✅ Via `rein fmt` (not LSP) |

---

## VS Code

The `editors/vscode/` directory contains a first-party extension with syntax highlighting, real-time diagnostics, hover docs, and completion stubs.

### Setup

1. Install `rein`:
   ```bash
   cargo install rein-lang
   ```

2. Install the extension by copying it to your VS Code extensions directory:
   ```bash
   cp -r editors/vscode ~/.vscode/extensions/rein-lang
   ```
   Or build and install from source:
   ```bash
   cd editors/vscode
   npm install
   npm run compile
   # Then reload VS Code — the extension loads from the directory above.
   ```

3. Open a `.rein` file — syntax highlighting and diagnostics activate automatically.

### Configuration

If `rein` is not on your `PATH`, set the binary path in `settings.json`:

```json
{
  "rein.serverPath": "/path/to/rein"
}
```

### Known limitations

- **Completions not implemented** — the LSP will not suggest block types or field names yet.
- **Go-to-definition is single-file only** — cross-file navigation is not supported.

---

## Neovim

Using [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig).

### Setup

Add to your Neovim config (`init.lua`):

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register rein as a custom server
if not configs.rein_lsp then
  configs.rein_lsp = {
    default_config = {
      cmd = { 'rein', 'lsp' },
      filetypes = { 'rein' },
      root_dir = lspconfig.util.root_pattern('rein.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.rein_lsp.setup {}
```

Associate `.rein` files with the `rein` filetype:

```lua
vim.filetype.add({
  extension = {
    rein = 'rein',
  },
})
```

### Tree-sitter syntax highlighting

A Tree-sitter grammar is available at `tree-sitter-rein/`. Build and install it:

```bash
cd tree-sitter-rein
tree-sitter generate
tree-sitter build
```

Then in Neovim, add the parser using `nvim-treesitter`:

```lua
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()
parser_config.rein = {
  install_info = {
    url = "https://github.com/delamain-labs/rein",
    files = { "tree-sitter-rein/src/parser.c" },
    branch = "main",
  },
  filetype = "rein",
}
```

---

## Verifying the LSP is working

Run `rein lsp` directly in a terminal — it should block waiting for LSP stdio input. If it prints an error and exits, check that your `rein` binary is up to date:

```bash
rein --version
# rein <version>
```

To confirm diagnostics are firing, introduce a syntax error in a `.rein` file and save. The LSP should report the error inline.

---

## Known limitations

- **No completion**: Field name and block type suggestions are not implemented. See the [Language Reference](./language-reference.md) for available block types and fields.
- **Single-file only**: Go-to-definition does not cross file boundaries.
- **No rename**: Symbol renaming is not supported.
- **No formatting via LSP**: Use `rein fmt <file>` from the CLI instead.
