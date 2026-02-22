# tree-sitter-rein

[Tree-sitter](https://tree-sitter.github.io/) grammar for the [Rein](https://github.com/delamain-labs/rein) agent orchestration language.

## Usage

### Generate the parser

```bash
npm install
npx tree-sitter generate
```

### Test parsing

```bash
npx tree-sitter parse ../examples/basic.rein
```

### Syntax highlighting

The `queries/highlights.scm` file provides syntax highlighting queries compatible with any Tree-sitter-powered editor (Neovim, Helix, Zed, etc.).

## Editor Integration

### Neovim (nvim-treesitter)

Add to your nvim-treesitter config:

```lua
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()
parser_config.rein = {
  install_info = {
    url = "path/to/tree-sitter-rein",
    files = { "src/parser.c" },
  },
  filetype = "rein",
}
```

Then copy `queries/highlights.scm` to `~/.config/nvim/queries/rein/highlights.scm`.
