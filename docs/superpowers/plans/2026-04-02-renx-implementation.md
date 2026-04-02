# renx Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust TUI tool for bulk file renaming with composable operation stacks and live preview.

**Architecture:** Single-binary Rust app using ratatui for the TUI. Three-panel layout (files, operations, preview) with a synchronous event loop. Operations implement a `Transform` trait and are composed into a pipeline. No async, no config files.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, clap 4, glob 0.3, regex 1

---

## File Structure

```
src/
├── main.rs          — entry point, CLI parsing with clap, launches TUI
├── app.rs           — App struct (state), Panel enum, InputMode enum
├── operations.rs    — Transform trait + 7 operation types
├── pipeline.rs      — runs filenames through ordered operation stack
├── files.rs         — glob expansion, file discovery, rename execution
├── ui.rs            — renders all panels, status bars, popups
└── input.rs         — maps key events to App mutations

tests/
├── operations_test.rs  — unit tests for each operation type
├── pipeline_test.rs    — integration tests for operation stacking
└── files_test.rs       — file discovery and rename tests
```

---

### Task 1: Project Scaffold + Basic TUI Loop

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "renx"
version = "0.1.0"
edition = "2021"
description = "A TUI tool for bulk file renaming"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
glob = "0.3"
regex = "1"
```

- [ ] **Step 2: Create .gitignore**

```
/target
.superpowers/
```

- [ ] **Step 3: Create minimal main.rs that launches a TUI and quits on 'q'**

```rust
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}

fn render(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .title(" renx ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(Text::raw("Press 'q' to quit"))
        .block(block);
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Build and verify it runs**

Run: `cargo build`
Expected: Compiles with no errors.

Run: `cargo run`
Expected: TUI appears with bordered box saying "Press 'q' to quit". Press 'q' to exit cleanly.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml .gitignore src/main.rs
git commit -m "feat: project scaffold with basic TUI loop"
```

---

### Task 2: Transform Trait + Find & Replace + Remove Pattern

**Files:**
- Create: `src/operations.rs`
- Modify: `src/main.rs` (add `mod operations;`)
- Create: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests for FindReplace and RemovePattern**

Create `tests/operations_test.rs`:

```rust
use renx::operations::{FindReplace, Operation, RemovePattern};

#[test]
fn find_replace_basic() {
    let op = Operation::FindReplace(FindReplace {
        search: "IMG_".to_string(),
        replacement: "vacation_".to_string(),
    });
    assert_eq!(op.apply("IMG_0001.jpg"), "vacation_0001.jpg");
}

#[test]
fn find_replace_multiple_occurrences() {
    let op = Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "b".to_string(),
    });
    assert_eq!(op.apply("banana.txt"), "bbnbnb.txt");
}

#[test]
fn find_replace_no_match() {
    let op = Operation::FindReplace(FindReplace {
        search: "xyz".to_string(),
        replacement: "abc".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn remove_pattern_basic() {
    let op = Operation::RemovePattern(RemovePattern {
        pattern: "copy of ".to_string(),
    });
    assert_eq!(op.apply("copy of document.txt"), "document.txt");
}

#[test]
fn remove_pattern_multiple() {
    let op = Operation::RemovePattern(RemovePattern {
        pattern: " ".to_string(),
    });
    assert_eq!(op.apply("my cool file.txt"), "mycoolfile.txt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operations_test`
Expected: FAIL — cannot find `renx::operations`

- [ ] **Step 3: Implement operations.rs with FindReplace and RemovePattern**

Create `src/operations.rs`:

```rust
/// A single rename operation that transforms a filename string.
#[derive(Clone, Debug)]
pub enum Operation {
    FindReplace(FindReplace),
    RemovePattern(RemovePattern),
}

impl Operation {
    /// Apply this operation to a filename, returning the transformed name.
    pub fn apply(&self, filename: &str) -> String {
        match self {
            Operation::FindReplace(op) => op.apply(filename),
            Operation::RemovePattern(op) => op.apply(filename),
        }
    }

    /// Human-readable description for the operation stack UI.
    pub fn description(&self) -> String {
        match self {
            Operation::FindReplace(op) => {
                format!("Replace \"{}\" → \"{}\"", op.search, op.replacement)
            }
            Operation::RemovePattern(op) => format!("Remove \"{}\"", op.pattern),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FindReplace {
    pub search: String,
    pub replacement: String,
}

impl FindReplace {
    fn apply(&self, filename: &str) -> String {
        filename.replace(&self.search, &self.replacement)
    }
}

#[derive(Clone, Debug)]
pub struct RemovePattern {
    pub pattern: String,
}

impl RemovePattern {
    fn apply(&self, filename: &str) -> String {
        filename.replace(&self.pattern, "")
    }
}
```

- [ ] **Step 4: Add module declaration to main.rs and make it a library**

Add to the top of `src/main.rs`:

```rust
pub mod operations;
```

Note: For tests to import `renx::operations`, the modules must be declared as `pub` in either `main.rs` or a `lib.rs`. We'll use `main.rs` with pub modules for simplicity.

Actually, Rust convention: tests import from the library crate, not the binary. Create `src/lib.rs` instead:

Create `src/lib.rs`:

```rust
pub mod operations;
```

And in `src/main.rs`, add at the top (below existing imports):

```rust
// modules are in lib.rs
```

No changes needed to `main.rs` — the library modules are accessed via `renx::` in tests.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test operations_test`
Expected: All 5 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/operations.rs src/lib.rs tests/operations_test.rs
git commit -m "feat: Transform trait with FindReplace and RemovePattern operations"
```

---

### Task 3: Regex Replace Operation

**Files:**
- Modify: `src/operations.rs`
- Modify: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests for RegexReplace**

Add to `tests/operations_test.rs`:

```rust
use renx::operations::RegexReplace;

#[test]
fn regex_replace_capture_groups() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"(\d+)_(\w+)".to_string(),
        replacement: "${2}_${1}".to_string(),
    });
    assert_eq!(op.apply("001_photo.jpg"), "photo_001.jpg");
}

#[test]
fn regex_replace_simple_pattern() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("file_123_v2.txt"), "file_NUM_vNUM.txt");
}

#[test]
fn regex_replace_no_match() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn regex_replace_invalid_pattern_returns_original() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"[invalid".to_string(),
        replacement: "x".to_string(),
    });
    assert_eq!(op.apply("file.txt"), "file.txt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operations_test`
Expected: FAIL — cannot find `RegexReplace`

- [ ] **Step 3: Add RegexReplace to operations.rs**

Add to `src/operations.rs`:

```rust
use regex::Regex;
```

Add variant to the `Operation` enum:

```rust
RegexReplace(RegexReplace),
```

Add match arms in `Operation::apply` and `Operation::description`:

```rust
// In apply:
Operation::RegexReplace(op) => op.apply(filename),

// In description:
Operation::RegexReplace(op) => {
    format!("Regex \"{}\" → \"{}\"", op.pattern, op.replacement)
}
```

Add the struct and impl:

```rust
#[derive(Clone, Debug)]
pub struct RegexReplace {
    pub pattern: String,
    pub replacement: String,
}

impl RegexReplace {
    fn apply(&self, filename: &str) -> String {
        match Regex::new(&self.pattern) {
            Ok(re) => re.replace_all(filename, self.replacement.as_str()).to_string(),
            Err(_) => filename.to_string(),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test operations_test`
Expected: All 9 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/operations.rs tests/operations_test.rs
git commit -m "feat: add RegexReplace operation with capture group support"
```

---

### Task 4: Case Change Operation

**Files:**
- Modify: `src/operations.rs`
- Modify: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests for CaseChange**

Add to `tests/operations_test.rs`:

```rust
use renx::operations::{CaseChange, CaseMode};

#[test]
fn case_change_lowercase() {
    let op = Operation::CaseChange(CaseChange { mode: CaseMode::Lower });
    assert_eq!(op.apply("MY_FILE.TXT"), "my_file.txt");
}

#[test]
fn case_change_uppercase() {
    let op = Operation::CaseChange(CaseChange { mode: CaseMode::Upper });
    assert_eq!(op.apply("my_file.txt"), "MY_FILE.TXT");
}

#[test]
fn case_change_title() {
    let op = Operation::CaseChange(CaseChange { mode: CaseMode::Title });
    assert_eq!(op.apply("hello world.txt"), "Hello World.Txt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operations_test`
Expected: FAIL — cannot find `CaseChange`

- [ ] **Step 3: Add CaseChange to operations.rs**

Add variant to `Operation` enum:

```rust
CaseChange(CaseChange),
```

Add match arms:

```rust
// In apply:
Operation::CaseChange(op) => op.apply(filename),

// In description:
Operation::CaseChange(op) => {
    let mode_str = match op.mode {
        CaseMode::Lower => "lowercase",
        CaseMode::Upper => "UPPERCASE",
        CaseMode::Title => "Title Case",
    };
    format!("{}", mode_str)
}
```

Add structs and impl:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum CaseMode {
    Lower,
    Upper,
    Title,
}

#[derive(Clone, Debug)]
pub struct CaseChange {
    pub mode: CaseMode,
}

impl CaseChange {
    fn apply(&self, filename: &str) -> String {
        match self.mode {
            CaseMode::Lower => filename.to_lowercase(),
            CaseMode::Upper => filename.to_uppercase(),
            CaseMode::Title => filename
                .split_inclusive(|c: char| !c.is_alphanumeric())
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            upper + &chars.as_str().to_lowercase()
                        }
                        None => String::new(),
                    }
                })
                .collect(),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test operations_test`
Expected: All 12 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/operations.rs tests/operations_test.rs
git commit -m "feat: add CaseChange operation (lower, upper, title)"
```

---

### Task 5: Add Prefix + Add Suffix Operations

**Files:**
- Modify: `src/operations.rs`
- Modify: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/operations_test.rs`:

```rust
use renx::operations::{AddPrefix, AddSuffix};

#[test]
fn add_prefix_basic() {
    let op = Operation::AddPrefix(AddPrefix {
        prefix: "2024_".to_string(),
    });
    assert_eq!(op.apply("vacation.jpg"), "2024_vacation.jpg");
}

#[test]
fn add_suffix_before_extension() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_final".to_string(),
    });
    assert_eq!(op.apply("photo.jpg"), "photo_final.jpg");
}

#[test]
fn add_suffix_no_extension() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_backup".to_string(),
    });
    assert_eq!(op.apply("Makefile"), "Makefile_backup");
}

#[test]
fn add_suffix_multiple_dots() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_v2".to_string(),
    });
    assert_eq!(op.apply("archive.tar.gz"), "archive.tar_v2.gz");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operations_test`
Expected: FAIL — cannot find `AddPrefix` / `AddSuffix`

- [ ] **Step 3: Add both operations to operations.rs**

Add variants to `Operation` enum:

```rust
AddPrefix(AddPrefix),
AddSuffix(AddSuffix),
```

Add match arms:

```rust
// In apply:
Operation::AddPrefix(op) => op.apply(filename),
Operation::AddSuffix(op) => op.apply(filename),

// In description:
Operation::AddPrefix(op) => format!("Prefix \"{}\"", op.prefix),
Operation::AddSuffix(op) => format!("Suffix \"{}\"", op.suffix),
```

Add structs and impls:

```rust
#[derive(Clone, Debug)]
pub struct AddPrefix {
    pub prefix: String,
}

impl AddPrefix {
    fn apply(&self, filename: &str) -> String {
        format!("{}{}", self.prefix, filename)
    }
}

#[derive(Clone, Debug)]
pub struct AddSuffix {
    pub suffix: String,
}

impl AddSuffix {
    fn apply(&self, filename: &str) -> String {
        match filename.rfind('.') {
            Some(dot_pos) => {
                let stem = &filename[..dot_pos];
                let ext = &filename[dot_pos..];
                format!("{}{}{}", stem, self.suffix, ext)
            }
            None => format!("{}{}", filename, self.suffix),
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test operations_test`
Expected: All 16 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/operations.rs tests/operations_test.rs
git commit -m "feat: add AddPrefix and AddSuffix operations"
```

---

### Task 6: Sequential Number Operation

**Files:**
- Modify: `src/operations.rs`
- Modify: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests**

Note: Sequential numbering is unique — it needs to know the file's index in the list. The `apply` method needs an index parameter. We'll add `apply_with_index` to `Operation`.

Add to `tests/operations_test.rs`:

```rust
use renx::operations::{SequentialNumber, NumberPosition};

#[test]
fn sequential_prefix() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 3,
        start: 1,
        position: NumberPosition::Prefix,
    });
    assert_eq!(op.apply_with_index("photo.jpg", 0), "001_photo.jpg");
    assert_eq!(op.apply_with_index("photo.jpg", 4), "005_photo.jpg");
}

#[test]
fn sequential_suffix() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 0,
        position: NumberPosition::Suffix,
    });
    assert_eq!(op.apply_with_index("photo.jpg", 0), "photo_00.jpg");
    assert_eq!(op.apply_with_index("photo.jpg", 9), "photo_09.jpg");
}

#[test]
fn sequential_replace() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 4,
        start: 1,
        position: NumberPosition::Replace,
    });
    assert_eq!(op.apply_with_index("anything.jpg", 0), "0001.jpg");
    assert_eq!(op.apply_with_index("anything.jpg", 99), "0100.jpg");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operations_test`
Expected: FAIL — cannot find `SequentialNumber`

- [ ] **Step 3: Add apply_with_index to Operation and implement SequentialNumber**

First, add `apply_with_index` to `Operation`:

```rust
/// Apply with a file index (used by SequentialNumber).
/// Non-sequential operations ignore the index.
pub fn apply_with_index(&self, filename: &str, index: usize) -> String {
    match self {
        Operation::SequentialNumber(op) => op.apply(filename, index),
        other => other.apply(filename),
    }
}
```

Add variant to `Operation` enum:

```rust
SequentialNumber(SequentialNumber),
```

Add match arm to `apply` (non-indexed version uses index 0):

```rust
// In apply:
Operation::SequentialNumber(op) => op.apply(filename, 0),

// In description:
Operation::SequentialNumber(op) => {
    let pos = match op.position {
        NumberPosition::Prefix => "prefix",
        NumberPosition::Suffix => "suffix",
        NumberPosition::Replace => "replace",
    };
    let hashes: String = "#".repeat(op.format_width as usize);
    format!("Sequential {} ({})", hashes, pos)
}
```

Add structs and impl:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum NumberPosition {
    Prefix,
    Suffix,
    Replace,
}

#[derive(Clone, Debug)]
pub struct SequentialNumber {
    pub format_width: u32,
    pub start: u32,
    pub position: NumberPosition,
}

impl SequentialNumber {
    fn apply(&self, filename: &str, index: usize) -> String {
        let number = self.start + index as u32;
        let formatted = format!("{:0>width$}", number, width = self.format_width as usize);

        match self.position {
            NumberPosition::Prefix => format!("{}_{}", formatted, filename),
            NumberPosition::Suffix => match filename.rfind('.') {
                Some(dot_pos) => {
                    let stem = &filename[..dot_pos];
                    let ext = &filename[dot_pos..];
                    format!("{}_{}{}", stem, formatted, ext)
                }
                None => format!("{}_{}", filename, formatted),
            },
            NumberPosition::Replace => match filename.rfind('.') {
                Some(dot_pos) => {
                    let ext = &filename[dot_pos..];
                    format!("{}{}", formatted, ext)
                }
                None => formatted,
            },
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test operations_test`
Expected: All 19 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/operations.rs tests/operations_test.rs
git commit -m "feat: add SequentialNumber operation with prefix/suffix/replace modes"
```

---

### Task 7: Pipeline Module

**Files:**
- Create: `src/pipeline.rs`
- Modify: `src/lib.rs` (add `pub mod pipeline;`)
- Create: `tests/pipeline_test.rs`

- [ ] **Step 1: Write failing tests for pipeline**

Create `tests/pipeline_test.rs`:

```rust
use renx::operations::*;
use renx::pipeline::Pipeline;

#[test]
fn empty_pipeline_returns_original() {
    let pipeline = Pipeline::new(vec![]);
    let results = pipeline.apply(&["photo.jpg".to_string()]);
    assert_eq!(results, vec!["photo.jpg"]);
}

#[test]
fn single_operation() {
    let ops = vec![Operation::FindReplace(FindReplace {
        search: "IMG_".to_string(),
        replacement: "beach_".to_string(),
    })];
    let pipeline = Pipeline::new(ops);
    let results = pipeline.apply(&["IMG_001.jpg".to_string(), "IMG_002.jpg".to_string()]);
    assert_eq!(results, vec!["beach_001.jpg", "beach_002.jpg"]);
}

#[test]
fn stacked_operations() {
    let ops = vec![
        Operation::FindReplace(FindReplace {
            search: "IMG_".to_string(),
            replacement: "beach_".to_string(),
        }),
        Operation::CaseChange(CaseChange {
            mode: CaseMode::Lower,
        }),
        Operation::SequentialNumber(SequentialNumber {
            format_width: 3,
            start: 1,
            position: NumberPosition::Prefix,
        }),
    ];
    let pipeline = Pipeline::new(ops);
    let filenames = vec![
        "IMG_0422.JPG".to_string(),
        "IMG_0423.JPG".to_string(),
    ];
    let results = pipeline.apply(&filenames);
    assert_eq!(results, vec!["001_beach_0422.jpg", "002_beach_0423.jpg"]);
}

#[test]
fn pipeline_preserves_order() {
    // Replace then prefix — order matters
    let ops = vec![
        Operation::FindReplace(FindReplace {
            search: "old".to_string(),
            replacement: "new".to_string(),
        }),
        Operation::AddPrefix(AddPrefix {
            prefix: "v2_".to_string(),
        }),
    ];
    let pipeline = Pipeline::new(ops);
    let results = pipeline.apply(&["old_file.txt".to_string()]);
    assert_eq!(results, vec!["v2_new_file.txt"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test pipeline_test`
Expected: FAIL — cannot find `renx::pipeline`

- [ ] **Step 3: Implement pipeline.rs**

Create `src/pipeline.rs`:

```rust
use crate::operations::Operation;

pub struct Pipeline {
    operations: Vec<Operation>,
}

impl Pipeline {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self { operations }
    }

    /// Apply all operations in order to each filename.
    /// Returns a Vec of transformed filenames in the same order as input.
    pub fn apply(&self, filenames: &[String]) -> Vec<String> {
        filenames
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut result = name.clone();
                for op in &self.operations {
                    result = op.apply_with_index(&result, index);
                }
                result
            })
            .collect()
    }
}
```

- [ ] **Step 4: Add module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod pipeline;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test pipeline_test`
Expected: All 4 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/pipeline.rs src/lib.rs tests/pipeline_test.rs
git commit -m "feat: pipeline module composes operations in order"
```

---

### Task 8: Files Module (Discovery + Glob)

**Files:**
- Create: `src/files.rs`
- Modify: `src/lib.rs` (add `pub mod files;`)
- Create: `tests/files_test.rs`

- [ ] **Step 1: Write failing tests for file discovery**

Create `tests/files_test.rs`:

```rust
use std::fs;
use renx::files;

#[test]
fn discover_files_in_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let files = files::discover_dir(dir.path()).unwrap();
    // Should only get files, not directories
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.file_name == "a.txt"));
    assert!(files.iter().any(|f| f.file_name == "b.txt"));
}

#[test]
fn discover_with_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("photo.jpg"), "").unwrap();
    fs::write(dir.path().join("photo.png"), "").unwrap();
    fs::write(dir.path().join("notes.txt"), "").unwrap();

    let pattern = dir.path().join("*.jpg").to_string_lossy().to_string();
    let files = files::discover_glob(&pattern).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name, "photo.jpg");
}

#[test]
fn rename_file_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("old.txt");
    fs::write(&original, "content").unwrap();

    let entry = files::FileEntry {
        file_name: "old.txt".to_string(),
        parent_dir: dir.path().to_path_buf(),
    };

    let result = files::rename_file(&entry, "new.txt");
    assert!(result.is_ok());
    assert!(!original.exists());
    assert!(dir.path().join("new.txt").exists());
    assert_eq!(fs::read_to_string(dir.path().join("new.txt")).unwrap(), "content");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test files_test`
Expected: FAIL — cannot find `renx::files`

- [ ] **Step 3: Implement files.rs**

Create `src/files.rs`:

```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use glob::glob;

/// Represents a file that can be renamed.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub file_name: String,
    pub parent_dir: PathBuf,
}

impl FileEntry {
    pub fn full_path(&self) -> PathBuf {
        self.parent_dir.join(&self.file_name)
    }
}

/// Discover all files (not directories) in a directory.
pub fn discover_dir(path: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(FileEntry {
                    file_name: name.to_string(),
                    parent_dir: path.to_path_buf(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(entries)
}

/// Discover files matching a glob pattern.
pub fn discover_glob(pattern: &str) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let paths = glob(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    for path_result in paths {
        let path = path_result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if path.is_file() {
            if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
                if let Some(name_str) = name.to_str() {
                    entries.push(FileEntry {
                        file_name: name_str.to_string(),
                        parent_dir: parent.to_path_buf(),
                    });
                }
            }
        }
    }
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(entries)
}

/// Rename a single file on disk.
pub fn rename_file(entry: &FileEntry, new_name: &str) -> io::Result<()> {
    let old_path = entry.full_path();
    let new_path = entry.parent_dir.join(new_name);
    fs::rename(old_path, new_path)
}
```

- [ ] **Step 4: Add module to lib.rs and add tempfile dev dependency**

Add to `src/lib.rs`:

```rust
pub mod files;
```

Add to `Cargo.toml` under a new `[dev-dependencies]` section:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test files_test`
Expected: All 3 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/files.rs src/lib.rs Cargo.toml tests/files_test.rs
git commit -m "feat: file discovery with directory listing and glob support"
```

---

### Task 9: App State

**Files:**
- Create: `src/app.rs`
- Modify: `src/lib.rs` (add `pub mod app;`)

- [ ] **Step 1: Create app.rs with full application state**

Create `src/app.rs`:

```rust
use crate::files::FileEntry;
use crate::operations::Operation;
use crate::pipeline::Pipeline;

/// Which panel currently has focus.
#[derive(Clone, Debug, PartialEq)]
pub enum Panel {
    Files,
    Operations,
    Preview,
}

/// Whether the app is in normal mode or showing a popup.
#[derive(Clone, Debug, PartialEq)]
pub enum InputMode {
    Normal,
    SelectingOperationType,
    EditingOperationParams,
    HelpOverlay,
}

/// The full application state.
pub struct App {
    /// All discovered files.
    pub files: Vec<FileEntry>,
    /// Whether each file is selected for renaming.
    pub selected: Vec<bool>,
    /// Cursor position in the file list.
    pub file_cursor: usize,
    /// The ordered operation stack.
    pub operations: Vec<Operation>,
    /// Cursor position in the operation stack.
    pub op_cursor: usize,
    /// Which panel has focus.
    pub focused_panel: Panel,
    /// Current input mode (normal vs popup).
    pub input_mode: InputMode,
    /// Cached preview results (new filenames for selected files).
    pub preview: Vec<(String, String)>,
    /// Scroll offset for the preview panel.
    pub preview_scroll: usize,
    /// Whether the user has confirmed execution and the app should quit.
    pub should_quit: bool,
    /// Whether renames were executed (for exit messaging).
    pub executed: bool,
    /// Status bar message.
    pub status_message: String,
    /// Index of the operation type being selected in the popup.
    pub popup_type_cursor: usize,
    /// Text fields for the operation parameter popup.
    pub popup_fields: Vec<String>,
    /// Which field is focused in the parameter popup.
    pub popup_field_cursor: usize,
    /// If editing an existing operation, its index. None if adding new.
    pub editing_op_index: Option<usize>,
}

impl App {
    pub fn new(files: Vec<FileEntry>) -> Self {
        let count = files.len();
        let mut app = App {
            files,
            selected: vec![true; count], // all selected by default
            file_cursor: 0,
            operations: Vec::new(),
            op_cursor: 0,
            focused_panel: Panel::Files,
            input_mode: InputMode::Normal,
            preview: Vec::new(),
            preview_scroll: 0,
            should_quit: false,
            executed: false,
            status_message: String::new(),
            popup_type_cursor: 0,
            popup_fields: Vec::new(),
            popup_field_cursor: 0,
            editing_op_index: None,
        };
        app.update_preview();
        app
    }

    /// Recalculate the preview based on current operations and selected files.
    pub fn update_preview(&mut self) {
        let pipeline = Pipeline::new(self.operations.clone());
        let selected_files: Vec<String> = self
            .files
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, &sel)| sel)
            .map(|(f, _)| f.file_name.clone())
            .collect();
        let new_names = pipeline.apply(&selected_files);
        self.preview = selected_files
            .into_iter()
            .zip(new_names.into_iter())
            .collect();
    }

    /// Count of selected files.
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    /// Cycle focus to the next panel.
    pub fn next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            Panel::Files => Panel::Operations,
            Panel::Operations => Panel::Preview,
            Panel::Preview => Panel::Files,
        };
    }

    /// Toggle selection of file at cursor.
    pub fn toggle_file_selection(&mut self) {
        if !self.files.is_empty() {
            self.selected[self.file_cursor] = !self.selected[self.file_cursor];
            self.update_preview();
        }
    }

    /// Select all or deselect all files.
    pub fn toggle_select_all(&mut self) {
        let all_selected = self.selected.iter().all(|&s| s);
        let new_value = !all_selected;
        for sel in &mut self.selected {
            *sel = new_value;
        }
        self.update_preview();
    }

    /// Delete the operation at the current cursor.
    pub fn delete_operation(&mut self) {
        if !self.operations.is_empty() {
            self.operations.remove(self.op_cursor);
            if self.op_cursor >= self.operations.len() && self.op_cursor > 0 {
                self.op_cursor -= 1;
            }
            self.update_preview();
        }
    }

    /// Move the operation at cursor up in the stack.
    pub fn move_operation_up(&mut self) {
        if self.op_cursor > 0 {
            self.operations.swap(self.op_cursor, self.op_cursor - 1);
            self.op_cursor -= 1;
            self.update_preview();
        }
    }

    /// Move the operation at cursor down in the stack.
    pub fn move_operation_down(&mut self) {
        if self.op_cursor + 1 < self.operations.len() {
            self.operations.swap(self.op_cursor, self.op_cursor + 1);
            self.op_cursor += 1;
            self.update_preview();
        }
    }

    /// Check for naming conflicts in the preview.
    /// Returns indices of conflicting entries.
    pub fn find_conflicts(&self) -> Vec<usize> {
        let mut conflicts = Vec::new();
        for (i, (_, new_a)) in self.preview.iter().enumerate() {
            // Check against other renamed files
            for (j, (_, new_b)) in self.preview.iter().enumerate() {
                if i != j && new_a == new_b {
                    conflicts.push(i);
                    break;
                }
            }
            // Check for empty names
            if new_a.is_empty() {
                conflicts.push(i);
                continue;
            }
            // Check for invalid characters
            if new_a.contains('/') || new_a.contains('\0') {
                conflicts.push(i);
            }
        }
        conflicts
    }

    /// Whether execution is blocked (conflicts exist).
    pub fn has_errors(&self) -> bool {
        !self.find_conflicts().is_empty()
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod app;
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/lib.rs
git commit -m "feat: App state with panel focus, file selection, and preview caching"
```

---

### Task 10: UI Layout — All Panels

**Files:**
- Create: `src/ui.rs`
- Modify: `src/lib.rs` (add `pub mod ui;`)
- Modify: `src/main.rs` (integrate App + UI)

- [ ] **Step 1: Create ui.rs with the three-panel layout and status bars**

Create `src/ui.rs`:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};

use crate::app::{App, InputMode, Panel};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Vertical: top bar | main content | preview | bottom bar
    let [top_bar, main_area, preview_area, bottom_bar] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Percentage(30),
            Constraint::Length(1),
        ])
        .areas(area);

    // Horizontal split for main area: files | operations
    let [files_area, ops_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .areas(main_area);

    render_top_bar(frame, app, top_bar);
    render_file_list(frame, app, files_area);
    render_operation_stack(frame, app, ops_area);
    render_preview(frame, app, preview_area);
    render_bottom_bar(frame, app, bottom_bar);

    // Render popups on top
    match app.input_mode {
        InputMode::SelectingOperationType => render_type_picker(frame, app, area),
        InputMode::EditingOperationParams => render_param_editor(frame, app, area),
        InputMode::HelpOverlay => render_help(frame, area),
        InputMode::Normal => {}
    }
}

fn render_top_bar(frame: &mut Frame, app: &App, area: Rect) {
    let dir = app
        .files
        .first()
        .map(|f| f.parent_dir.display().to_string())
        .unwrap_or_else(|| ".".to_string());
    let text = Line::from(vec![
        Span::styled(" \u{1F4C2} ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "{} ({} files, {} selected)",
            dir,
            app.files.len(),
            app.selected_count()
        )),
    ]);
    let bar = Paragraph::new(text).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Files;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let selected = app.selected[i];
            let is_cursor = i == app.file_cursor && focused;

            let check = if selected { "\u{2714} " } else { "  " };
            let cursor = if is_cursor { "\u{25B8} " } else { "  " };
            let text = format!("{}{}{}", cursor, check, file.file_name);

            let style = if is_cursor {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else if selected {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_operation_stack(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Operations;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(" Operations ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.operations.is_empty() {
        let hint = Paragraph::new("\n  Press 'a' to add an operation")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(hint, area);
        return;
    }

    let items: Vec<ListItem> = app
        .operations
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let is_cursor = i == app.op_cursor && focused;
            let text = format!("{}. {}", i + 1, op.description());
            let style = if is_cursor {
                Style::default().fg(Color::Green).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Yellow)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Preview;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let conflicts = app.find_conflicts();

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.preview.is_empty() {
        let hint = Paragraph::new("  No files selected")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, inner);
        return;
    }

    // Header line
    let max_old_len = app
        .preview
        .iter()
        .map(|(old, _)| old.len())
        .max()
        .unwrap_or(0)
        .max(8);

    let mut lines: Vec<Line> = Vec::new();
    let header = Line::from(vec![
        Span::styled(
            format!("{:<width$}", "Original", width = max_old_len + 2),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("  →  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Renamed", Style::default().fg(Color::DarkGray)),
    ]);
    lines.push(header);

    let visible_count = inner.height.saturating_sub(1) as usize; // minus header
    let start = app.preview_scroll;
    let end = (start + visible_count).min(app.preview.len());

    for (display_idx, (old, new)) in app.preview[start..end].iter().enumerate() {
        let actual_idx = start + display_idx;
        let is_conflict = conflicts.contains(&actual_idx);
        let is_noop = old == new;

        let new_style = if is_conflict {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else if is_noop {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Green)
        };

        let line = Line::from(vec![
            Span::styled(
                format!("{:<width$}", old, width = max_old_len + 2),
                Style::default().fg(Color::White),
            ),
            Span::styled("  →  ", Style::default().fg(Color::DarkGray)),
            Span::styled(new.clone(), new_style),
        ]);
        lines.push(line);
    }

    if end < app.preview.len() {
        lines.push(Line::from(Span::styled(
            format!("  \u{22EE} {} more files", app.preview.len() - end),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let preview_text = Paragraph::new(lines);
    frame.render_widget(preview_text, inner);
}

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let hints = match app.focused_panel {
        Panel::Files => "Tab: switch │ j/k: move │ Space: toggle │ v: all │ Enter: execute │ q: quit",
        Panel::Operations => "Tab: switch │ a: add │ e: edit │ d: delete │ J/K: reorder │ Enter: execute │ q: quit",
        Panel::Preview => "Tab: switch │ j/k: scroll │ Enter: execute │ q: quit",
    };

    let right = if app.has_errors() {
        Span::styled(" CONFLICTS ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(
            format!(" {} ops ", app.operations.len()),
            Style::default().fg(Color::DarkGray),
        )
    };

    let status_msg = if !app.status_message.is_empty() {
        app.status_message.clone()
    } else {
        hints.to_string()
    };

    // We'll just show the hints for now
    let line = Line::from(vec![
        Span::styled(format!(" {}", status_msg), Style::default().fg(Color::DarkGray)),
    ]);
    let bar = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn render_type_picker(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(40, 12, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Add Operation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let types = [
        "Find & Replace",
        "Regex Replace",
        "Sequential Number",
        "Case Change",
        "Add Prefix",
        "Add Suffix",
        "Remove Pattern",
    ];

    let items: Vec<ListItem> = types
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == app.popup_type_cursor {
                Style::default().fg(Color::Cyan).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {}", name)).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup);
}

fn render_param_editor(frame: &mut Frame, app: &App, area: Rect) {
    let field_count = app.popup_fields.len() as u16;
    let height = 4 + field_count * 2;
    let popup = centered_rect(50, height, area);
    frame.render_widget(Clear, popup);

    let title = if app.editing_op_index.is_some() {
        " Edit Operation "
    } else {
        " Operation Parameters "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let labels = get_field_labels(app);

    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in labels.iter().zip(app.popup_fields.iter()).enumerate() {
        let is_focused = i == app.popup_field_cursor;
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = if is_focused {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let cursor = if is_focused { "▎" } else { " " };

        lines.push(Line::from(Span::styled(
            format!("  {}", label),
            label_style,
        )));
        lines.push(Line::from(vec![
            Span::styled(format!("  {}", cursor), value_style),
            Span::styled(value.clone(), value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter: confirm │ Esc: cancel │ Tab: next field",
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn get_field_labels(app: &App) -> Vec<&'static str> {
    // The labels depend on which operation type was selected
    // We use popup_type_cursor to determine this
    match app.popup_type_cursor {
        0 => vec!["Search:", "Replace with:"],          // Find & Replace
        1 => vec!["Regex pattern:", "Replace with:"],   // Regex Replace
        2 => vec!["Width (e.g. 3):", "Start number:", "Position (prefix/suffix/replace):"], // Sequential
        3 => vec!["Mode (lower/upper/title):"],         // Case Change
        4 => vec!["Prefix:"],                           // Add Prefix
        5 => vec!["Suffix:"],                           // Add Suffix
        6 => vec!["Pattern to remove:"],                // Remove Pattern
        _ => vec![],
    }
}

fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let help_text = vec![
        "",
        "  Navigation",
        "    Tab          Switch between panels",
        "    j/k  ↑/↓    Move cursor up/down",
        "",
        "  File List",
        "    Space        Toggle file selection",
        "    v            Select/deselect all",
        "",
        "  Operations",
        "    a            Add new operation",
        "    e            Edit selected operation",
        "    d            Delete selected operation",
        "    J/K          Reorder operation up/down",
        "",
        "  Global",
        "    Enter        Execute renames",
        "    ?            Toggle this help",
        "    q / Esc      Quit",
    ];

    let lines: Vec<Line> = help_text
        .iter()
        .map(|&s| Line::from(Span::styled(s, Style::default().fg(Color::White))))
        .collect();

    let content = Paragraph::new(lines).block(block);
    frame.render_widget(content, popup);
}
```

- [ ] **Step 2: Add module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod ui;
```

- [ ] **Step 3: Update main.rs to use App and UI**

Replace `src/main.rs` entirely:

```rust
use std::io;

use crossterm::event::{self, Event, KeyEventKind};

use renx::app::App;
use renx::files;
use renx::ui;

fn main() -> io::Result<()> {
    // For now, discover files in the current directory
    let file_list = files::discover_dir(std::path::Path::new("."))?;
    let mut app = App::new(file_list);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // Minimal input handling for now — just quit
                match key.code {
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 4: Build and verify it runs**

Run: `cargo build`
Expected: Compiles with no errors.

Run: `cargo run`
Expected: TUI appears with three panels showing files in the current directory. 'q' to quit.

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs src/lib.rs src/main.rs
git commit -m "feat: three-panel TUI layout with file list, operations, and preview"
```

---

### Task 11: Input Handling

**Files:**
- Create: `src/input.rs`
- Modify: `src/lib.rs` (add `pub mod input;`)
- Modify: `src/main.rs` (call input handler)

- [ ] **Step 1: Create input.rs**

Create `src/input.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode, Panel};
use crate::operations::*;

/// Handle a key event and mutate app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::SelectingOperationType => handle_type_picker(app, key),
        InputMode::EditingOperationParams => handle_param_editor(app, key),
        InputMode::HelpOverlay => handle_help(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.next_panel();
        }
        KeyCode::Char('?') => {
            app.input_mode = InputMode::HelpOverlay;
        }
        KeyCode::Enter => {
            if !app.has_errors() && !app.preview.is_empty() {
                execute_renames(app);
            } else if app.has_errors() {
                app.status_message = "Cannot execute: conflicts detected".to_string();
            }
        }
        _ => match app.focused_panel {
            Panel::Files => handle_files_input(app, key),
            Panel::Operations => handle_ops_input(app, key),
            Panel::Preview => handle_preview_input(app, key),
        },
    }
}

fn handle_files_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.file_cursor + 1 < app.files.len() {
                app.file_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.file_cursor > 0 {
                app.file_cursor -= 1;
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_file_selection();
        }
        KeyCode::Char('v') => {
            app.toggle_select_all();
        }
        // Allow 'a' from file panel too for convenience
        KeyCode::Char('a') => {
            app.input_mode = InputMode::SelectingOperationType;
            app.popup_type_cursor = 0;
        }
        _ => {}
    }
}

fn handle_ops_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.operations.is_empty() && app.op_cursor + 1 < app.operations.len() {
                app.op_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.op_cursor > 0 {
                app.op_cursor -= 1;
            }
        }
        KeyCode::Char('a') => {
            app.editing_op_index = None;
            app.input_mode = InputMode::SelectingOperationType;
            app.popup_type_cursor = 0;
        }
        KeyCode::Char('d') => {
            app.delete_operation();
        }
        KeyCode::Char('e') => {
            if !app.operations.is_empty() {
                start_editing_operation(app);
            }
        }
        KeyCode::Char('J') => {
            app.move_operation_down();
        }
        KeyCode::Char('K') => {
            app.move_operation_up();
        }
        _ => {}
    }
}

fn handle_preview_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.preview_scroll + 1 < app.preview.len() {
                app.preview_scroll += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.preview_scroll > 0 {
                app.preview_scroll -= 1;
            }
        }
        _ => {}
    }
}

fn handle_type_picker(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.popup_type_cursor < 6 {
                app.popup_type_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.popup_type_cursor > 0 {
                app.popup_type_cursor -= 1;
            }
        }
        KeyCode::Enter => {
            // Initialize fields for the selected type
            let field_count = match app.popup_type_cursor {
                0 => 2, // Find & Replace
                1 => 2, // Regex Replace
                2 => 3, // Sequential Number
                3 => 1, // Case Change
                4 => 1, // Add Prefix
                5 => 1, // Add Suffix
                6 => 1, // Remove Pattern
                _ => 0,
            };
            app.popup_fields = vec![String::new(); field_count];
            app.popup_field_cursor = 0;
            app.input_mode = InputMode::EditingOperationParams;
        }
        _ => {}
    }
}

fn handle_param_editor(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.popup_fields.clear();
        }
        KeyCode::Tab => {
            if !app.popup_fields.is_empty() {
                app.popup_field_cursor = (app.popup_field_cursor + 1) % app.popup_fields.len();
            }
        }
        KeyCode::Enter => {
            if let Some(op) = build_operation_from_fields(app) {
                if let Some(idx) = app.editing_op_index {
                    app.operations[idx] = op;
                } else {
                    app.operations.push(op);
                    app.op_cursor = app.operations.len() - 1;
                }
                app.update_preview();
            }
            app.input_mode = InputMode::Normal;
            app.popup_fields.clear();
            app.editing_op_index = None;
        }
        KeyCode::Backspace => {
            if let Some(field) = app.popup_fields.get_mut(app.popup_field_cursor) {
                field.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(field) = app.popup_fields.get_mut(app.popup_field_cursor) {
                field.push(c);
            }
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

fn build_operation_from_fields(app: &App) -> Option<Operation> {
    let fields = &app.popup_fields;
    match app.popup_type_cursor {
        0 => Some(Operation::FindReplace(FindReplace {
            search: fields.get(0)?.clone(),
            replacement: fields.get(1)?.clone(),
        })),
        1 => Some(Operation::RegexReplace(RegexReplace {
            pattern: fields.get(0)?.clone(),
            replacement: fields.get(1)?.clone(),
        })),
        2 => {
            let width: u32 = fields.get(0)?.parse().unwrap_or(3);
            let start: u32 = fields.get(1)?.parse().unwrap_or(1);
            let position = match fields.get(2)?.as_str() {
                "suffix" => NumberPosition::Suffix,
                "replace" => NumberPosition::Replace,
                _ => NumberPosition::Prefix,
            };
            Some(Operation::SequentialNumber(SequentialNumber {
                format_width: width,
                start,
                position,
            }))
        }
        3 => {
            let mode = match fields.get(0)?.as_str() {
                "upper" => CaseMode::Upper,
                "title" => CaseMode::Title,
                _ => CaseMode::Lower,
            };
            Some(Operation::CaseChange(CaseChange { mode }))
        }
        4 => Some(Operation::AddPrefix(AddPrefix {
            prefix: fields.get(0)?.clone(),
        })),
        5 => Some(Operation::AddSuffix(AddSuffix {
            suffix: fields.get(0)?.clone(),
        })),
        6 => Some(Operation::RemovePattern(RemovePattern {
            pattern: fields.get(0)?.clone(),
        })),
        _ => None,
    }
}

fn start_editing_operation(app: &mut App) {
    let op = &app.operations[app.op_cursor];
    let (type_idx, fields) = match op {
        Operation::FindReplace(o) => (0, vec![o.search.clone(), o.replacement.clone()]),
        Operation::RegexReplace(o) => (1, vec![o.pattern.clone(), o.replacement.clone()]),
        Operation::SequentialNumber(o) => {
            let pos = match o.position {
                NumberPosition::Prefix => "prefix",
                NumberPosition::Suffix => "suffix",
                NumberPosition::Replace => "replace",
            };
            (2, vec![o.format_width.to_string(), o.start.to_string(), pos.to_string()])
        }
        Operation::CaseChange(o) => {
            let mode = match o.mode {
                CaseMode::Lower => "lower",
                CaseMode::Upper => "upper",
                CaseMode::Title => "title",
            };
            (3, vec![mode.to_string()])
        }
        Operation::AddPrefix(o) => (4, vec![o.prefix.clone()]),
        Operation::AddSuffix(o) => (5, vec![o.suffix.clone()]),
        Operation::RemovePattern(o) => (6, vec![o.pattern.clone()]),
    };
    app.popup_type_cursor = type_idx;
    app.popup_fields = fields;
    app.popup_field_cursor = 0;
    app.editing_op_index = Some(app.op_cursor);
    app.input_mode = InputMode::EditingOperationParams;
}

fn execute_renames(app: &mut App) {
    let mut success = 0;
    let mut failures: Vec<String> = Vec::new();

    for (old_name, new_name) in &app.preview {
        if old_name == new_name {
            continue; // skip no-ops
        }
        // Find the file entry
        if let Some(entry) = app.files.iter().find(|f| f.file_name == *old_name) {
            match crate::files::rename_file(entry, new_name) {
                Ok(_) => success += 1,
                Err(e) => failures.push(format!("{}: {}", old_name, e)),
            }
        }
    }

    if failures.is_empty() {
        app.status_message = format!("Renamed {} files successfully", success);
    } else {
        app.status_message = format!(
            "Renamed {} files, {} failed: {}",
            success,
            failures.len(),
            failures.first().unwrap_or(&String::new())
        );
    }
    app.executed = true;
    app.should_quit = true;
}
```

- [ ] **Step 2: Add module to lib.rs**

Add to `src/lib.rs`:

```rust
pub mod input;
```

- [ ] **Step 3: Update main.rs to use the input handler**

Replace the event handling in `src/main.rs`:

```rust
use std::io;

use crossterm::event::{self, Event, KeyEventKind};

use renx::app::App;
use renx::files;
use renx::input;
use renx::ui;

fn main() -> io::Result<()> {
    let file_list = files::discover_dir(std::path::Path::new("."))?;
    let mut app = App::new(file_list);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    if app.executed {
        println!("{}", app.status_message);
    }

    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                input::handle_key(app, key);
                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}
```

- [ ] **Step 4: Build and manually test**

Run: `cargo build`
Expected: Compiles with no errors.

Run: `cargo run`
Expected: Full TUI with working keybindings — Tab switches panels, j/k navigates, Space toggles files, 'a' opens operation picker, typing fills fields, Enter confirms. 'q' quits.

- [ ] **Step 5: Commit**

```bash
git add src/input.rs src/lib.rs src/main.rs
git commit -m "feat: full input handling with panel navigation, popups, and rename execution"
```

---

### Task 12: CLI Argument Parsing with Clap

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update main.rs with clap argument parsing**

Replace `src/main.rs`:

```rust
use std::io;
use std::path::Path;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};

use renx::app::App;
use renx::files;
use renx::input;
use renx::ui;

/// A TUI tool for bulk file renaming
#[derive(Parser, Debug)]
#[command(name = "renx", version, about)]
struct Args {
    /// File patterns or directory to operate on.
    /// Supports glob patterns like *.txt, photos/**/*.jpg
    #[arg(default_value = ".")]
    patterns: Vec<String>,

    /// Recurse into subdirectories
    #[arg(short, long)]
    recursive: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let file_list = discover_files(&args)?;

    if file_list.is_empty() {
        eprintln!("No files found matching the given patterns.");
        return Ok(());
    }

    let mut app = App::new(file_list);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    if app.executed {
        println!("{}", app.status_message);
    }

    result
}

fn discover_files(args: &Args) -> io::Result<Vec<files::FileEntry>> {
    let mut all_files = Vec::new();

    for pattern in &args.patterns {
        let path = Path::new(pattern);

        if path.is_dir() {
            if args.recursive {
                // Recursive glob: dir/**/*
                let glob_pattern = format!("{}/**/*", pattern.trim_end_matches('/'));
                let mut found = files::discover_glob(&glob_pattern)?;
                all_files.append(&mut found);
            } else {
                let mut found = files::discover_dir(path)?;
                all_files.append(&mut found);
            }
        } else {
            // Treat as glob pattern
            let mut found = files::discover_glob(pattern)?;
            all_files.append(&mut found);
        }
    }

    // Deduplicate by full path
    all_files.sort_by(|a, b| a.full_path().cmp(&b.full_path()));
    all_files.dedup_by(|a, b| a.full_path() == b.full_path());

    Ok(all_files)
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                input::handle_key(app, key);
                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}
```

- [ ] **Step 2: Build and test CLI args**

Run: `cargo build`
Expected: Compiles with no errors.

Run: `cargo run -- --help`
Expected: Shows help text with usage, options for patterns and -r flag.

Run: `cargo run`
Expected: Opens TUI with files in current directory.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: CLI argument parsing with glob patterns and recursive flag"
```

---

### Task 13: Integration Test — End-to-End Rename

**Files:**
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Write an integration test that verifies the full rename pipeline**

Create `tests/integration_test.rs`:

```rust
use std::fs;
use renx::app::App;
use renx::files;
use renx::operations::*;

#[test]
fn end_to_end_rename() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("IMG_001.jpg"), "").unwrap();
    fs::write(dir.path().join("IMG_002.jpg"), "").unwrap();
    fs::write(dir.path().join("IMG_003.jpg"), "").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);

    // Add operations: replace IMG_ with beach_, then lowercase
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "IMG_".to_string(),
        replacement: "beach_".to_string(),
    }));
    app.operations.push(Operation::CaseChange(CaseChange {
        mode: CaseMode::Lower,
    }));
    app.update_preview();

    // Verify preview
    assert_eq!(app.preview.len(), 3);
    assert_eq!(app.preview[0], ("IMG_001.jpg".to_string(), "beach_001.jpg".to_string()));
    assert_eq!(app.preview[1], ("IMG_002.jpg".to_string(), "beach_002.jpg".to_string()));
    assert_eq!(app.preview[2], ("IMG_003.jpg".to_string(), "beach_003.jpg".to_string()));

    // Execute renames
    assert!(!app.has_errors());
    for (old_name, new_name) in &app.preview {
        let entry = app.files.iter().find(|f| f.file_name == *old_name).unwrap();
        files::rename_file(entry, new_name).unwrap();
    }

    // Verify files on disk
    assert!(dir.path().join("beach_001.jpg").exists());
    assert!(dir.path().join("beach_002.jpg").exists());
    assert!(dir.path().join("beach_003.jpg").exists());
    assert!(!dir.path().join("IMG_001.jpg").exists());
}

#[test]
fn conflict_detection_blocks_execution() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);

    // Both files would rename to the same thing
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "x".to_string(),
    }));
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "b".to_string(),
        replacement: "x".to_string(),
    }));
    app.update_preview();

    // Both become "x.txt" — should detect conflict
    assert!(app.has_errors());
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test integration_test`
Expected: All 2 tests PASS.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test`
Expected: All tests PASS (operations, pipeline, files, integration).

- [ ] **Step 4: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: end-to-end integration tests for rename pipeline and conflict detection"
```
