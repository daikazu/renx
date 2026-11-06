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
- **`operations.rs`** - `Operation` enum with 15 variants (FindReplace, RegexReplace, RemovePattern, CaseChange, AddPrefix, AddSuffix, Insert, Trim, Truncate, SequentialNumber, ZeroPad, InsertDate, ChangeExtension, Slugify, StripDiacritics). Each variant wraps a struct with an `apply` method; `Operation::apply_in_context` passes an `OpContext` (file index + optional path) so SequentialNumber and InsertDate can use it. Text operations carry a `Scope` (Both/Name/Ext) applied through `apply_scoped`. The `OP_TYPES` registry (with `T_*` index constants) defines picker names and editor fields; the picker, editor, and `Operation::type_index` all key off it, so a new operation needs an enum variant, an `OP_TYPES` entry, and arms in `build_operation_from_fields` / `start_editing_operation` in `input.rs`.
- **`pipeline.rs`** - `Pipeline` chains operations in order. `apply(&[String])` works on names only; `apply_entries(&[FileEntry])` also supplies each file's path (used by the preview).
- **`files.rs`** - `FileEntry` (file_name + parent_dir), discovery via `discover_dir` and `discover_glob` (sorted with `natural_cmp`), `same_file` identity check for case-insensitive filesystems, and `rename_file` for actual fs rename.
- **`input.rs`** - Key event dispatch, branching on `InputMode`. Normal mode handles panel navigation and file/operation actions; popup modes handle text input and type selection.
- **`ui.rs`** - Ratatui rendering. Three-panel horizontal layout (files, operations, preview) with top status bar and bottom help bar. Popup overlays for operation type picker and parameter editor.
- **`anim.rs`** - Animation timing: tick constants, `Effect`/`EffectKind` (op flash, file pop), `DoneState` for the post-rename cascade, `diff_segments` for change highlighting, `pulse`/`lerp`. Pure logic, no rendering.
- **`main.rs`** - CLI args via clap (`--no-animation` sets `app.animate = false`), arg resolution (directory vs glob vs shell-expanded), tilde expansion, filter inference from shell-expanded filenames. The event loop polls with a `TICK_MS` timeout only while `App::needs_tick()` is true, otherwise blocks on input.

**Data flow**: User adds operations -> `App::update_preview()` builds a `Pipeline` and applies it to selected files -> preview panel shows old->new mappings -> `find_conflicts()` checks for duplicates/empty/invalid names -> execute calls `files::rename_file` for each -> `InputMode::Done` shows the results cascade; the first key skips the cascade, the next sets `should_quit`.

## Key Patterns

- Operations work on filename strings only (stem + extension), not full paths. `AddSuffix` and `SequentialNumber::Suffix` insert before the extension dot. `split_ext` treats a leading dot (`.bashrc`) as part of the name, not an extension.
- Conflict detection compares files by identity (`files::same_file`), not path text, so case-only renames on macOS are not conflicts.
- Filter uses wildcard matching (`*`/`?`) with a DP-based `wildcard_match` function in `app.rs`.
- Preview is recalculated on every operation/selection change via `update_preview()`.
- Animations: `App::tick()` advances `app.tick`; effects and the cascade are pure functions of `tick`, so tests drive them by calling `tick()` directly. `App::new` starts with `animate = false`; tests opt in.
- Tests use `tempfile` crate for filesystem operations in `tests/`.
