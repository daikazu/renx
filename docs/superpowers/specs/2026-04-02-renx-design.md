# renx — Bulk File Rename TUI

A terminal-based tool for bulk renaming files using composable operation stacks with live preview. Built in Rust with `ratatui`.

## CLI Interface

```
renx                      # TUI with all files in current dir
renx *.jpg                # TUI with only jpg files
renx photos/              # TUI with files in photos/
renx photos/**/*.png      # Recursive glob
renx -r photos/           # Recursive, all files
```

- Glob patterns expanded at invocation to build initial file list
- In-TUI filtering and individual file toggling
- No flags for operations — all built interactively
- `clap` for argument parsing

## TUI Layout

Single-view with three panels and two status bars:

- **Top bar** — current directory, file count, selection count
- **Left panel** — file list with checkmarks for selected files, cursor highlight
- **Right panel** — ordered operation stack with add/edit/delete hints
- **Bottom panel** — live preview showing original → renamed (side-by-side columns)
- **Bottom bar** — context-sensitive keybinding hints

Unselected files are dimmed. Preview updates live as operations or selections change.

## Operation Types

Operations are stored in an ordered stack. Each transforms the filename output of the previous one.

| Operation | Parameters | Example |
|-----------|-----------|---------|
| Find & Replace | search string, replacement | `IMG_` → `vacation_` |
| Regex Replace | pattern, replacement (capture groups `$1`, `$2`) | `(\d+)_(\w+)` → `$2_$1` |
| Sequential Number | format (`###`), start number, position (prefix/suffix/replace entire stem) | `photo_001.jpg` |
| Case Change | mode (lower, upper, title, sentence) | `MY FILE` → `my file` |
| Add Prefix | prefix string | `2024_vacation_photo.jpg` |
| Add Suffix | suffix string (before extension) | `photo_final.jpg` |
| Remove Pattern | string or regex to strip | Remove `copy of ` |

## Interaction Model

### Panel Navigation

`Tab` cycles focus between panels. Focused panel has highlighted border.

### File List Panel

- `j/k` or `↑/↓` — move cursor
- `Space` — toggle select/deselect
- `v` — select all / deselect all
- Files matching invocation glob are pre-selected

### Operation Stack Panel

- `a` — add operation (popup: pick type, then form for parameters)
- `e` — edit selected operation
- `d` — delete selected operation
- `j/k` — move cursor between operations
- `J/K` (shift) — reorder operation up/down

### Preview Panel

- Read-only, auto-updates
- Scrollable with `j/k` when focused

### Global

- `Enter` — execute renames
- `q` or `Esc` — quit without renaming
- `?` — help overlay

### Popups

Modal popup over main view for adding/editing operations. `Tab` moves between fields, `Enter` confirms, `Esc` cancels.

## Architecture

### Modules

- **`main.rs`** — entry point, CLI parsing, launches TUI
- **`app.rs`** — application state (file list, selections, operation stack, focused panel)
- **`operations.rs`** — operation types implementing `Transform` trait (`apply(filename: &str) -> String`)
- **`pipeline.rs`** — runs filenames through operation stack in order
- **`ui.rs`** — renders panels and popups with `ratatui` widgets
- **`input.rs`** — maps key events to actions based on focus/state
- **`files.rs`** — file discovery, glob expansion, rename execution

### Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal backend |
| `clap` | CLI argument parsing |
| `glob` | Glob pattern expansion |
| `regex` | Regex operations |

No async runtime, no database, no config files.

### Data Flow

```
User input → input.rs → mutates App state → pipeline recalculates previews → ui.rs re-renders
```

Synchronous event loop: wait for key event → update state → redraw.

## Error Handling

- **Name conflicts** — if two files would rename to the same name, highlight in red in preview, block execution. Warning in status bar.
- **Invalid characters** — if operation produces OS-invalid characters (`/`, `\0`), flag in red, block execution.
- **Empty result** — if stack produces empty filename, flag it.
- **No-op detection** — if new name matches original, dim in preview, skip renaming.
- **Permission errors** — if rename fails mid-execution, report which succeeded/failed in summary screen before exit.

No undo, no dry-run flag, no backup copies. The preview is the safety mechanism.

## Distribution

- Local Rust project for now (`cargo build`)
- Future: Homebrew formula
