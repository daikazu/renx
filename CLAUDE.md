# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

renx is a Rust TUI tool for bulk file renaming. Users select files, build a pipeline of stackable rename operations, preview results with conflict detection, then execute.

## Commands

```sh
cargo build                  # build
cargo run                    # run against current directory
cargo run -- ~/downloads     # run against a specific directory
cargo run -- -r .            # recursive mode
cargo test                   # run all tests
cargo test operations_test   # run a single test file
cargo test -- --nocapture    # run tests with stdout visible
cargo clippy                 # lint
cargo fmt -- --check         # check formatting
```

## Architecture

**Three-layer design**: input handling -> app state -> UI rendering. The TUI runs a synchronous event loop in `main.rs` (`run()`).

- **`app.rs`** - Central `App` struct holding all state: file list, selection, operation stack, filter, preview cache, popup/input mode. All state mutations happen here. `InputMode` enum drives which popup/overlay is shown. `Panel` enum tracks focus across the three-panel layout.
- **`operations.rs`** - `Operation` enum with 8 variants (FindReplace, RegexReplace, CaseChange, AddPrefix, AddSuffix, SequentialNumber, RemovePattern, Slugify). Each variant wraps a struct with an `apply(filename) -> String` method. `SequentialNumber` uses `apply_with_index` for position-aware numbering.
- **`pipeline.rs`** - `Pipeline` chains operations in order via `apply(&[String]) -> Vec<String>`, threading each filename through every operation sequentially.
- **`files.rs`** - `FileEntry` (file_name + parent_dir), discovery via `discover_dir` and `discover_glob`, and `rename_file` for actual fs rename.
- **`input.rs`** - Key event dispatch, branching on `InputMode`. Normal mode handles panel navigation and file/operation actions; popup modes handle text input and type selection.
- **`ui.rs`** - Ratatui rendering. Three-panel horizontal layout (files, operations, preview) with top status bar and bottom help bar. Popup overlays for operation type picker and parameter editor.
- **`main.rs`** - CLI args via clap, arg resolution (directory vs glob vs shell-expanded), tilde expansion, filter inference from shell-expanded filenames.

**Data flow**: User adds operations -> `App::update_preview()` builds a `Pipeline` and applies it to selected files -> preview panel shows old->new mappings -> `find_conflicts()` checks for duplicates/empty/invalid names -> execute calls `files::rename_file` for each.

## Key Patterns

- Operations work on filename strings only (stem + extension), not full paths. `AddSuffix` and `SequentialNumber::Suffix` insert before the extension dot.
- Filter uses wildcard matching (`*`/`?`) with a DP-based `wildcard_match` function in `app.rs`.
- Preview is recalculated on every operation/selection change via `update_preview()`.
- Tests use `tempfile` crate for filesystem operations in `tests/`.
