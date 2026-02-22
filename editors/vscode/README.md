# Rein Language Support for VSCode

Syntax highlighting, diagnostics, completions, and hover docs for `.rein` files.

## Features

- **Syntax highlighting** — keywords, strings, numbers, currencies, operators, comments
- **Real-time diagnostics** — parse errors and validation warnings as you type
- **Completions** — keyword completions with descriptions
- **Hover documentation** — hover over keywords to see syntax and examples

## Requirements

The `rein` binary must be installed and available in your PATH (or configure `rein.serverPath`).

## Installation

1. Install `rein`: `cargo install --git https://github.com/delamain-labs/rein.git`
2. Install this extension (or copy to `~/.vscode/extensions/rein-lang/`)
3. Open a `.rein` file

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `rein.serverPath` | `rein` | Path to the rein binary |
