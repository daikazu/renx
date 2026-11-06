# renx

A terminal UI tool for bulk file renaming. Select files, stack operations, preview changes, and execute -- all without leaving the terminal.

![Rust](https://img.shields.io/badge/rust-stable-orange) ![Status](https://img.shields.io/badge/status-early_development-yellow)

## Features

- **Three-panel TUI** -- file list, operation stack, and live preview side by side
- **Stackable operations** -- chain multiple rename transforms in order:
  - Find & Replace (optionally ignoring case)
  - Regex Replace (patterns are validated as you add them)
  - Remove Pattern (optionally ignoring case)
  - Case Change (lower, UPPER, Title, Sentence, snake_case, kebab-case, camelCase)
  - Add Prefix / Suffix
  - Insert Text at a position from the start or end
  - Trim Characters from the start or end
  - Truncate to a maximum length
  - Sequential Numbering (prefix, suffix, or replace, with start, step, and separator)
  - Zero-pad Numbers (`track 3` becomes `track 03`)
  - Insert Date from the file's modified or created time, with a strftime format
  - Change Extension (replace, add, or remove)
  - Slugify (lowercase, accents transliterated, separators collapsed)
  - Strip Accents (`café` becomes `cafe`)
- **Scope** -- text operations can target the whole filename, just the name, or just the extension
- **Natural sorting** -- `file2` lists before `file10`, and ordering ignores case
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
- [chrono](https://github.com/chronotope/chrono) -- date formatting for Insert Date
- [unicode-normalization](https://github.com/unicode-rs/unicode-normalization) -- accent stripping

## License

MIT
