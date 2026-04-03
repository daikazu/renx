# renx

A terminal UI tool for bulk file renaming. Select files, stack operations, preview changes, and execute -- all without leaving the terminal.

![Rust](https://img.shields.io/badge/rust-stable-orange) ![Status](https://img.shields.io/badge/status-early_development-yellow)

## Features

- **Three-panel TUI** -- file list, operation stack, and live preview side by side
- **Stackable operations** -- chain multiple rename transforms in order:
  - Find & Replace
  - Regex Replace
  - Sequential Numbering (prefix, suffix, or replace)
  - Case Change (lower, UPPER, Title)
  - Add Prefix / Suffix
  - Remove Pattern
  - Slugify
- **Live preview** with conflict detection before any files are touched
- **Wildcard filtering** (`*`, `?`) to narrow down the file list
- **Glob and directory input** -- pass a directory, glob pattern, or let the shell expand it
- **Recursive mode** (`-r`) to include subdirectories

## Install

### Homebrew

```sh
brew install daikazu/tap/renx
```

### From source

Requires Rust (stable):

```sh
git clone https://github.com/daikazu/renx.git
cd renx
cargo build --release
```

The binary will be at `target/release/renx`.

## Usage

```sh
# Current directory
renx

# Specific directory
renx ~/downloads

# Glob pattern (quote to prevent shell expansion)
renx '~/downloads/*.jpeg'

# Recursive
renx -r ~/projects
```

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle panels |
| `j` / `k` | Move cursor |
| `Space` / `Enter` | Toggle file selection |
| `v` | Select / deselect all |
| `/` | Filter files |
| `a` | Add operation |
| `e` | Edit operation |
| `d` | Delete operation |
| `J` / `K` | Reorder operations |
| `x` | Execute rename |
| `?` | Help overlay |
| `q` | Quit |

## How it works

1. **Select files** -- browse the file list, toggle individual files or select all
2. **Add operations** -- build a pipeline of rename transforms (they apply in order)
3. **Preview** -- see exactly what each file will be renamed to, with conflicts highlighted in red
4. **Execute** -- confirm and rename

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) -- terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) -- terminal input/output
- [clap](https://github.com/clap-rs/clap) -- CLI argument parsing
- [regex](https://github.com/rust-lang/regex) -- regex operations
- [glob](https://github.com/rust-lang/glob) -- file pattern matching

## License

MIT
