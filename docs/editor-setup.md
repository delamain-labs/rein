# Editor Setup for Rein

Rein ships a Language Server Protocol (LSP) implementation via `rein lsp`. This guide covers setup for VS Code and Neovim.

---

## Capabilities

The Rein LSP currently provides:

| Capability | Status |
|------------|--------|
| Syntax diagnostics | ✅ Supported |
| Hover (field docs) | ✅ Supported |
| Go-to-definition | ⚠️ Parse-only (same file) |
| Completion | ❌ Not yet implemented |
| Formatting | ✅ Via `rein fmt` (not LSP) |

Diagnostics fire on every save and report parse errors and semantic validation issues directly in your editor.

---

## VS Code

There is no published VS Code extension yet. Use the generic LSP client extension to wire `rein lsp` manually.

### Setup

1. Install [vscode-glspc](https://marketplace.visualstudio.com/items?itemName=dunstontc.vscode-glspc) or the [Generic Language Client](https://marketplace.visualstudio.com/items?itemName=lsp-example.lsp-example) extension.

2. Add to your `settings.json`:

```json
{
  "languageServerExample.trace.server": "verbose",
  "languageServerExample.serverPath": "/path/to/rein",
  "languageServerExample.serverArgs": ["lsp"],
  "languageServerExample.filePattern": "**/*.rein"
}
```

Replace `/path/to/rein` with the output of `which rein`.

3. Associate `.rein` files with a language ID. Add to `settings.json`:

```json
{
  "files.associations": {
    "*.rein": "rein"
  }
}
```

### Known limitations in VS Code

- No syntax highlighting (no TextMate grammar published yet — track [#369](https://github.com/delamain-labs/rein/issues/369))
- Completions not implemented — the LSP will not suggest block types or field names

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

### Optional: basic syntax highlighting via Tree-sitter

A Tree-sitter grammar for Rein does not yet exist. For minimal highlighting, create a filetype detection file and use a generic fallback:

```lua
-- Treat .rein files as similar to TOML for basic highlighting
vim.api.nvim_create_autocmd({ 'BufRead', 'BufNewFile' }, {
  pattern = '*.rein',
  callback = function()
    vim.bo.filetype = 'rein'
    -- Fallback: use a close-enough grammar for color
    vim.treesitter.language.register('toml', 'rein')
  end,
})
```

---

## Verifying the LSP is working

Run `rein lsp` directly in a terminal — it should block waiting for LSP stdio input. If it prints an error and exits, check that your `rein` binary is up to date:

```bash
rein --version
# rein 0.2.1
```

To confirm diagnostics are firing, introduce a syntax error in a `.rein` file and save. The LSP should report the error inline.

---

## Known limitations

- **No completion**: Field name and block type suggestions are not implemented. See the [Language Reference](./language-reference.md) for available block types and fields.
- **Single-file only**: Go-to-definition does not cross file boundaries.
- **No rename**: Symbol renaming is not supported.
- **No formatting via LSP**: Use `rein fmt <file>` from the CLI instead.
