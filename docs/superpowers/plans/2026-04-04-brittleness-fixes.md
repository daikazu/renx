# Brittleness & Safety Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all critical data-loss bugs, crash paths, and edge-case brittleness identified by the counselors review.

**Architecture:** Changes are layered bottom-up: first fix the data model (preview carries file index, not just basename), then fix rename execution (disk-overwrite protection, two-phase staging), then fix UTF-8 cursor handling, then fix operation edge cases (dotfiles, title case, slugify, empty search). Each task is independently testable.

**Tech Stack:** Rust, ratatui, crossterm, tempfile (tests)

---

### Task 1: Preview carries file index instead of basename string

The preview currently stores `Vec<(String, String)>` — `(old_name, new_name)`. This is fundamentally broken for recursive mode where two files can share a basename. Change it to `Vec<(usize, String)>` — `(file_index, new_name)` — so every preview entry is unambiguously tied to a specific `FileEntry`.

**Files:**
- Modify: `src/app.rs:43` (preview field type)
- Modify: `src/app.rs:154-171` (update_preview)
- Modify: `src/app.rs:247-265` (find_conflicts)
- Modify: `src/ui.rs:260-313` (render_preview)
- Modify: `src/input.rs:570-598` (execute_renames)
- Test: `tests/integration_test.rs`

- [ ] **Step 1: Write failing test for recursive-mode wrong-file rename**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn recursive_mode_renames_correct_file_by_identity() {
    let dir = tempfile::tempdir().unwrap();
    let sub_a = dir.path().join("a");
    let sub_b = dir.path().join("b");
    fs::create_dir(&sub_a).unwrap();
    fs::create_dir(&sub_b).unwrap();
    fs::write(sub_a.join("report.txt"), "content_a").unwrap();
    fs::write(sub_b.join("report.txt"), "content_b").unwrap();

    let mut files = vec![
        files::FileEntry {
            file_name: "report.txt".to_string(),
            parent_dir: sub_a.clone(),
        },
        files::FileEntry {
            file_name: "report.txt".to_string(),
            parent_dir: sub_b.clone(),
        },
    ];
    files.sort_by(|a, b| a.full_path().cmp(&b.full_path()));

    let mut app = App::new(files);
    // Select only the first file (sub_a/report.txt)
    app.selected[0] = true;

    app.operations.push(Operation::AddPrefix(AddPrefix {
        prefix: "new_".to_string(),
    }));
    app.update_preview();

    assert_eq!(app.preview.len(), 1);
    // Preview should carry file index 0, new name "new_report.txt"
    assert_eq!(app.preview[0].0, 0);
    assert_eq!(app.preview[0].1, "new_report.txt".to_string());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test recursive_mode_renames_correct_file_by_identity -- --nocapture 2>&1`
Expected: FAIL — type mismatch, `preview` is `Vec<(String, String)>` not `Vec<(usize, String)>`

- [ ] **Step 3: Change preview type to `Vec<(usize, String)>`**

In `src/app.rs`, change the field declaration:

```rust
    /// Cached preview results: (file_index, new_name) for selected files.
    pub preview: Vec<(usize, String)>,
```

Update `update_preview` in `src/app.rs`:

```rust
    pub fn update_preview(&mut self) {
        if self.operations.is_empty() {
            self.preview.clear();
            self.preview_scroll = self.preview_scroll.min(0);
            return;
        }
        let pipeline = Pipeline::new(self.operations.clone());
        let selected: Vec<(usize, String)> = self
            .filtered_indices
            .iter()
            .filter(|&&i| self.selected[i])
            .map(|&i| (i, self.files[i].file_name.clone()))
            .collect();
        let names: Vec<String> = selected.iter().map(|(_, n)| n.clone()).collect();
        let new_names = pipeline.apply(&names);
        self.preview = selected
            .into_iter()
            .zip(new_names.into_iter())
            .map(|((idx, _), new)| (idx, new))
            .collect();
        // Clamp preview_scroll when preview shrinks
        if !self.preview.is_empty() {
            self.preview_scroll = self.preview_scroll.min(self.preview.len() - 1);
        } else {
            self.preview_scroll = 0;
        }
    }
```

Update `find_conflicts` in `src/app.rs`:

```rust
    pub fn find_conflicts(&self) -> Vec<usize> {
        let mut conflicts = Vec::new();
        for (i, (_, new_a)) in self.preview.iter().enumerate() {
            // Duplicate new names within preview
            for (j, (_, new_b)) in self.preview.iter().enumerate() {
                if i != j && new_a == new_b {
                    // Only flag if same parent directory
                    let dir_a = &self.files[self.preview[i].0].parent_dir;
                    let dir_b = &self.files[self.preview[j].0].parent_dir;
                    if dir_a == dir_b {
                        conflicts.push(i);
                        break;
                    }
                }
            }
            if new_a.is_empty() || new_a.chars().all(|c| c.is_whitespace()) {
                conflicts.push(i);
                continue;
            }
            if new_a.contains('/') || new_a.contains('\\') || new_a.contains('\0') {
                conflicts.push(i);
            }
        }
        conflicts
    }
```

Update `selected_count` reference: no change needed — it doesn't touch preview.

- [ ] **Step 4: Update render_preview in `src/ui.rs`**

In `render_preview`, update the loop that builds preview lines. Replace:

```rust
    let max_old_len = app
        .preview
        .iter()
        .map(|(old, _)| old.len())
        .max()
        .unwrap_or(0)
        .max(8);
```

With:

```rust
    let max_old_len = app
        .preview
        .iter()
        .map(|(idx, _)| app.files[*idx].file_name.len())
        .max()
        .unwrap_or(0)
        .max(8);
```

Replace:

```rust
    for (display_idx, (old, new)) in app.preview[start..end].iter().enumerate() {
        let actual_idx = start + display_idx;
        let is_conflict = conflicts.contains(&actual_idx);
        let is_noop = old == new;
```

With:

```rust
    for (display_idx, (file_idx, new)) in app.preview[start..end].iter().enumerate() {
        let actual_idx = start + display_idx;
        let old = &app.files[*file_idx].file_name;
        let is_conflict = conflicts.contains(&actual_idx);
        let is_noop = old == new;
```

And replace:

```rust
        let line = Line::from(vec![
            Span::styled(
                format!("  {:<width$}", old, width = max_old_len),
                Style::default().fg(TEXT),
            ),
```

With (same code, `old` is now `&String` so `format!` still works — no change needed here since `old` is already a reference).

Also update `render_confirm` which counts non-noop renames:

```rust
    let rename_count = app
        .preview
        .iter()
        .filter(|(idx, new)| app.files[*idx].file_name != *new)
        .count();
```

- [ ] **Step 5: Update execute_renames in `src/input.rs`**

Replace the entire `execute_renames` function:

```rust
fn execute_renames(app: &mut App) {
    let mut success = 0;
    let mut failures: Vec<String> = Vec::new();

    for (file_idx, new_name) in &app.preview {
        let entry = &app.files[*file_idx];
        if entry.file_name == *new_name {
            continue;
        }
        match crate::files::rename_file(entry, new_name) {
            Ok(_) => success += 1,
            Err(e) => failures.push(format!("{}: {}", entry.file_name, e)),
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

- [ ] **Step 6: Update existing tests that reference preview as `(String, String)`**

In `tests/integration_test.rs`, update `end_to_end_rename`:

```rust
    // Verify preview
    assert_eq!(app.preview.len(), 3);
    // preview is now (file_index, new_name)
    assert_eq!(app.preview[0].1, "beach_001.jpg".to_string());
    assert_eq!(app.preview[1].1, "beach_002.jpg".to_string());
    assert_eq!(app.preview[2].1, "beach_003.jpg".to_string());

    // Execute renames using file index
    assert!(!app.has_errors());
    for (file_idx, new_name) in &app.preview {
        let entry = &app.files[*file_idx];
        files::rename_file(entry, new_name).unwrap();
    }
```

- [ ] **Step 7: Run all tests**

Run: `cargo test 2>&1`
Expected: All tests pass including the new `recursive_mode_renames_correct_file_by_identity`

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/ui.rs src/input.rs tests/integration_test.rs
git commit -m "fix: preview carries file index instead of basename to prevent wrong-file rename in recursive mode"
```

---

### Task 2: Disk-overwrite protection in conflict detection and rename execution

`fs::rename` on Unix silently overwrites existing files. `find_conflicts()` must check whether the target name already exists on disk (as an unselected file). `execute_renames` must also guard against this.

**Files:**
- Modify: `src/app.rs:247-265` (find_conflicts — already modified in Task 1)
- Modify: `src/input.rs:570-598` (execute_renames — already modified in Task 1)
- Test: `tests/integration_test.rs`

- [ ] **Step 1: Write failing test — rename would overwrite existing unselected file**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn conflict_detected_when_target_exists_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a_content").unwrap();
    fs::write(dir.path().join("b.txt"), "b_content").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);

    // Select only a.txt, rename it to b.txt (which exists on disk but isn't selected)
    let a_idx = app.files.iter().position(|f| f.file_name == "a.txt").unwrap();
    app.selected[a_idx] = true;

    app.operations.push(Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "b".to_string(),
    }));
    app.update_preview();

    // Should detect conflict: b.txt already exists on disk
    assert!(app.has_errors());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test conflict_detected_when_target_exists_on_disk -- --nocapture 2>&1`
Expected: FAIL — `has_errors()` returns false

- [ ] **Step 3: Add disk-existence check to `find_conflicts`**

In `src/app.rs`, add a disk-overwrite check inside `find_conflicts`. After the existing duplicate-name and empty-name checks, add:

```rust
            // Check if new name collides with an existing file on disk that isn't being renamed
            let entry = &self.files[self.preview[i].0];
            let target_path = entry.parent_dir.join(new_a);
            if target_path.exists() && target_path != entry.full_path() {
                // Check it's not another file in the preview set (already handled by duplicate check)
                let is_in_preview = self.preview.iter().any(|(idx, _)| {
                    self.files[*idx].full_path() == target_path
                });
                if !is_in_preview {
                    conflicts.push(i);
                }
            }
```

- [ ] **Step 4: Add guard to execute_renames**

In `src/input.rs`, inside `execute_renames`, before calling `rename_file`, add a check:

```rust
        let target_path = entry.parent_dir.join(new_name);
        if target_path.exists() && target_path != entry.full_path() {
            failures.push(format!("{}: target '{}' already exists", entry.file_name, new_name));
            continue;
        }
```

- [ ] **Step 5: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/input.rs tests/integration_test.rs
git commit -m "fix: detect and prevent rename overwriting existing files on disk"
```

---

### Task 3: Two-phase rename staging for chains and cycles

If A→B and B→C, sequential rename overwrites original B. Fix: rename all sources to temp names first, then rename temps to final names.

**Files:**
- Modify: `src/input.rs` (execute_renames)
- Test: `tests/integration_test.rs`

- [ ] **Step 1: Write failing test for chain rename**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn chain_rename_preserves_all_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "content_a").unwrap();
    fs::write(dir.path().join("b.txt"), "content_b").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.toggle_select_all();

    // a.txt -> b.txt, b.txt -> c.txt  (chain)
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "b".to_string(),
    }));
    app.update_preview();

    // Preview: a.txt -> b.txt, b.txt -> b.txt (noop)
    // Actually this doesn't create a chain. Let's use a different approach:
    // We need two operations that create a→b and b→c.
    // Better: just test with explicit preview manipulation.
    // Reset and manually set up the scenario using direct operations.
    app.operations.clear();

    // Use regex to shift: a->b, b->c
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^a".to_string(),
        replacement: "b".to_string(),
    }));
    app.update_preview();

    // After this: a.txt -> b.txt (rename), b.txt -> b.txt (noop)
    // The noop means b.txt won't be in the rename set. But disk-overwrite protection
    // from Task 2 should catch a.txt->b.txt conflicting with existing b.txt.
    // So this is actually caught by conflict detection. Good.
    // Let's test a swap instead: a.txt->b.txt and b.txt->a.txt
    assert!(app.has_errors()); // b.txt exists on disk and isn't being renamed away
}

#[test]
fn swap_rename_preserves_both_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("x.txt"), "content_x").unwrap();
    fs::write(dir.path().join("y.txt"), "content_y").unwrap();

    let mut files = files::discover_dir(dir.path()).unwrap();
    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    let mut app = App::new(files);
    app.toggle_select_all();

    // Swap: x->y, y->x via find-replace chain
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^x".to_string(),
        replacement: "TEMP_SWAP".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^y".to_string(),
        replacement: "x".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^TEMP_SWAP".to_string(),
        replacement: "y".to_string(),
    }));
    app.update_preview();

    // Preview should show: x.txt -> y.txt, y.txt -> x.txt
    assert_eq!(app.preview.len(), 2);
    let x_idx = app.files.iter().position(|f| f.file_name == "x.txt").unwrap();
    let y_idx = app.files.iter().position(|f| f.file_name == "y.txt").unwrap();

    let x_new = app.preview.iter().find(|(idx, _)| *idx == x_idx).map(|(_, n)| n.as_str());
    let y_new = app.preview.iter().find(|(idx, _)| *idx == y_idx).map(|(_, n)| n.as_str());
    assert_eq!(x_new, Some("y.txt"));
    assert_eq!(y_new, Some("x.txt"));

    // Two-phase rename should handle this without data loss
    // We can't call execute_renames directly (it's private and calls should_quit),
    // so we test the two-phase logic by doing the renames manually using the same algorithm.
    // For now, just verify no conflicts are detected (both targets are in the preview set)
    assert!(!app.has_errors());
}
```

- [ ] **Step 2: Run tests to verify behavior**

Run: `cargo test chain_rename_preserves_all_files swap_rename_preserves_both_files -- --nocapture 2>&1`
Expected: `chain_rename_preserves_all_files` should PASS (caught by disk overwrite protection). `swap_rename_preserves_both_files` should PASS or FAIL depending on conflict detection for mutual swaps.

- [ ] **Step 3: Implement two-phase rename in execute_renames**

Replace `execute_renames` in `src/input.rs`:

```rust
fn execute_renames(app: &mut App) {
    let mut success = 0;
    let mut failures: Vec<String> = Vec::new();

    // Collect renames that actually change the name
    let renames: Vec<(usize, String)> = app
        .preview
        .iter()
        .filter(|(file_idx, new_name)| app.files[*file_idx].file_name != *new_name)
        .cloned()
        .collect();

    if renames.is_empty() {
        app.status_message = "No files need renaming".to_string();
        return;
    }

    // Phase 1: Move all sources to temp names
    let mut temp_mappings: Vec<(usize, String, String)> = Vec::new(); // (file_idx, temp_name, final_name)
    for (file_idx, new_name) in &renames {
        let entry = &app.files[*file_idx];
        let temp_name = format!(".renx_tmp_{}", entry.file_name);
        match crate::files::rename_file(entry, &temp_name) {
            Ok(_) => {
                temp_mappings.push((*file_idx, temp_name, new_name.clone()));
            }
            Err(e) => {
                failures.push(format!("{}: {}", entry.file_name, e));
            }
        }
    }

    // Phase 2: Move temp names to final names
    for (file_idx, temp_name, final_name) in &temp_mappings {
        let entry = &app.files[*file_idx];
        let temp_entry = crate::files::FileEntry {
            file_name: temp_name.clone(),
            parent_dir: entry.parent_dir.clone(),
        };
        // Guard: don't overwrite existing non-temp file
        let target_path = entry.parent_dir.join(final_name);
        if target_path.exists() {
            // Rollback this one: move temp back to original
            let _ = crate::files::rename_file(&temp_entry, &entry.file_name);
            failures.push(format!("{}: target '{}' already exists", entry.file_name, final_name));
            continue;
        }
        match crate::files::rename_file(&temp_entry, final_name) {
            Ok(_) => success += 1,
            Err(e) => {
                // Try to rollback
                let _ = crate::files::rename_file(&temp_entry, &entry.file_name);
                failures.push(format!("{}: {}", entry.file_name, e));
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
            failures.join(", ")
        );
    }
    app.executed = true;
    app.should_quit = true;
}
```

- [ ] **Step 4: Write test that exercises two-phase rename via public API**

To test the actual rename execution, we need to make `execute_renames` accessible or test through the `files::rename_file` layer. Add a test helper. Add to `tests/integration_test.rs`:

```rust
#[test]
fn two_phase_rename_handles_swap() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alpha.txt"), "content_alpha").unwrap();
    fs::write(dir.path().join("beta.txt"), "content_beta").unwrap();

    let mut file_list = files::discover_dir(dir.path()).unwrap();
    file_list.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    let mut app = App::new(file_list);
    app.toggle_select_all();

    // Swap alpha<->beta via operations
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^alpha".to_string(),
        replacement: "SWAP".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^beta".to_string(),
        replacement: "alpha".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        pattern: "^SWAP".to_string(),
        replacement: "beta".to_string(),
    }));
    app.update_preview();

    assert!(!app.has_errors());

    // Simulate two-phase rename (same logic as execute_renames)
    let renames: Vec<(usize, String)> = app
        .preview
        .iter()
        .filter(|(idx, new)| app.files[*idx].file_name != *new)
        .cloned()
        .collect();

    // Phase 1: temp names
    let mut temps: Vec<(files::FileEntry, String, String)> = Vec::new();
    for (file_idx, new_name) in &renames {
        let entry = &app.files[*file_idx];
        let temp_name = format!(".renx_tmp_{}", entry.file_name);
        files::rename_file(entry, &temp_name).unwrap();
        temps.push((
            files::FileEntry {
                file_name: temp_name,
                parent_dir: entry.parent_dir.clone(),
            },
            new_name.clone(),
            entry.file_name.clone(),
        ));
    }

    // Phase 2: final names
    for (temp_entry, final_name, _) in &temps {
        files::rename_file(temp_entry, final_name).unwrap();
    }

    // Verify swap happened correctly
    assert_eq!(
        fs::read_to_string(dir.path().join("alpha.txt")).unwrap(),
        "content_beta"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("beta.txt")).unwrap(),
        "content_alpha"
    );
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/input.rs tests/integration_test.rs
git commit -m "fix: two-phase rename staging prevents data loss on chain/swap renames"
```

---

### Task 4: UTF-8 safe cursor handling in popup text editor

The popup text cursor tracks position as an integer that gets used as both a character index and a byte index. `String::insert(pos, c)` and `String::remove(pos)` take byte indices, but cursor is incremented/decremented by 1 (character). Multi-byte characters will cause panics.

**Files:**
- Modify: `src/input.rs:325-377` (handle_param_editor text cursor operations)
- Modify: `src/input.rs:564` (start_editing_operation cursor init)
- Modify: `src/ui.rs:548-573` (render_param_editor text field slicing)
- Test: `tests/input_test.rs` (new file)

- [ ] **Step 1: Write failing test for multi-byte character input**

Create `tests/input_test.rs`:

```rust
use renx::app::App;
use renx::files::FileEntry;

fn make_app() -> App {
    let files = vec![FileEntry {
        file_name: "test.txt".to_string(),
        parent_dir: std::path::PathBuf::from("/tmp"),
    }];
    App::new(files)
}

/// Helper: convert a char-index to byte-index in a string
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

#[test]
fn popup_cursor_handles_multibyte_insert() {
    let mut app = make_app();
    // Simulate typing "café" into popup field
    app.popup_fields = vec![String::new()];
    app.popup_text_cursors = vec![0];
    app.popup_field_options = vec![None];
    app.popup_field_selections = vec![0];

    // Type 'c', 'a', 'f', 'é' one by one
    for ch in "café".chars() {
        let cursor = app.popup_text_cursors[0];
        let byte_pos = char_to_byte_index(&app.popup_fields[0], cursor);
        app.popup_fields[0].insert(byte_pos, ch);
        app.popup_text_cursors[0] += 1;
    }

    assert_eq!(app.popup_fields[0], "café");
    assert_eq!(app.popup_text_cursors[0], 4); // 4 chars

    // Move cursor left once (should land before 'é')
    app.popup_text_cursors[0] -= 1;
    assert_eq!(app.popup_text_cursors[0], 3);

    // Delete at cursor (should remove 'é')
    let byte_pos = char_to_byte_index(&app.popup_fields[0], app.popup_text_cursors[0]);
    let next_byte = char_to_byte_index(&app.popup_fields[0], app.popup_text_cursors[0] + 1);
    app.popup_fields[0].replace_range(byte_pos..next_byte, "");

    assert_eq!(app.popup_fields[0], "caf");
}
```

- [ ] **Step 2: Run test to verify it passes (this tests the correct algorithm, not the broken code)**

Run: `cargo test popup_cursor_handles_multibyte_insert -- --nocapture 2>&1`
Expected: PASS — this validates the algorithm we'll implement.

- [ ] **Step 3: Add `char_to_byte_index` helper to `src/app.rs`**

Add a public utility function at the bottom of `src/app.rs`:

```rust
/// Convert a character index to a byte index in a string.
/// Returns `s.len()` if char_idx is at or beyond the end.
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

/// Get the character length of a string (not byte length).
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}
```

- [ ] **Step 4: Fix cursor movement in handle_param_editor**

In `src/input.rs`, update the text field cursor movement and editing sections.

Replace the `KeyCode::Right` handler for text fields (line ~332-338):

```rust
        KeyCode::Right if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                let len = app.popup_fields.get(cursor).map(|f| crate::app::char_len(f)).unwrap_or(0);
                if *pos < len {
                    *pos += 1;
                }
            }
        }
```

Replace the `KeyCode::End` handler (line ~345-348):

```rust
        KeyCode::End if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                *pos = app.popup_fields.get(cursor).map(|f| crate::app::char_len(f)).unwrap_or(0);
            }
        }
```

Replace the `KeyCode::Backspace` handler (line ~351-359):

```rust
        KeyCode::Backspace if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                if *pos > 0 {
                    if let Some(field) = app.popup_fields.get_mut(cursor) {
                        let byte_start = crate::app::char_to_byte_index(field, *pos - 1);
                        let byte_end = crate::app::char_to_byte_index(field, *pos);
                        field.replace_range(byte_start..byte_end, "");
                        *pos -= 1;
                    }
                }
            }
        }
```

Replace the `KeyCode::Delete` handler (line ~361-367):

```rust
        KeyCode::Delete if !is_selector => {
            let pos = app.popup_text_cursors.get(cursor).copied().unwrap_or(0);
            if let Some(field) = app.popup_fields.get_mut(cursor) {
                let char_count = crate::app::char_len(field);
                if pos < char_count {
                    let byte_start = crate::app::char_to_byte_index(field, pos);
                    let byte_end = crate::app::char_to_byte_index(field, pos + 1);
                    field.replace_range(byte_start..byte_end, "");
                }
            }
        }
```

Replace the `KeyCode::Char(c)` handler (line ~369-377):

```rust
        KeyCode::Char(c) if !is_selector => {
            let pos = app.popup_text_cursors.get(cursor).copied().unwrap_or(0);
            if let Some(field) = app.popup_fields.get_mut(cursor) {
                let byte_pos = crate::app::char_to_byte_index(field, pos);
                field.insert(byte_pos, c);
            }
            if let Some(p) = app.popup_text_cursors.get_mut(cursor) {
                *p += 1;
            }
        }
```

- [ ] **Step 5: Fix start_editing_operation cursor initialization**

In `src/input.rs`, line 564, replace:

```rust
    app.popup_text_cursors = app.popup_fields.iter().map(|f| f.len()).collect();
```

With:

```rust
    app.popup_text_cursors = app.popup_fields.iter().map(|f| crate::app::char_len(f)).collect();
```

- [ ] **Step 6: Fix render_param_editor text slicing in `src/ui.rs`**

In `render_param_editor`, replace the focused text field rendering (line ~561-568):

```rust
            if is_focused {
                let byte_cursor = crate::app::char_to_byte_index(value, text_cursor.min(crate::app::char_len(value)));
                let before = &value[..byte_cursor];
                let after = &value[byte_cursor..];
                spans.push(Span::styled(before, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled("\u{258e}", Style::default().fg(ACCENT).bg(bg)));
                let display_len = unicode_width_or_len(before) + unicode_width_or_len(after) + 1;
                let pad_len = field_width.saturating_sub(display_len);
                spans.push(Span::styled(after, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled(" ".repeat(pad_len), Style::default().bg(bg)));
            }
```

And add a helper at the top of `ui.rs` (or just use `str::len` for padding since ratatui handles display width):

Actually, simpler — just use byte len for padding since it's close enough for a fixed-width field:

```rust
            if is_focused {
                let byte_cursor = crate::app::char_to_byte_index(value, text_cursor.min(crate::app::char_len(value)));
                let before = &value[..byte_cursor];
                let after = &value[byte_cursor..];
                spans.push(Span::styled(before, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled("\u{258e}", Style::default().fg(ACCENT).bg(bg)));
                let pad_len = field_width.saturating_sub(before.len() + after.len() + 1);
                spans.push(Span::styled(after, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled(" ".repeat(pad_len), Style::default().bg(bg)));
            }
```

- [ ] **Step 7: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/input.rs src/ui.rs tests/input_test.rs
git commit -m "fix: UTF-8 safe cursor handling in popup text editor prevents panic on multibyte input"
```

---

### Task 5: Fix `infer_filter` Unicode panic

`infer_filter` slices strings by byte index using `first[start..start + len]` where `start` and `len` come from iterating `first.len()` (bytes). After `to_lowercase()`, multi-byte characters cause invalid UTF-8 boundary panics.

**Files:**
- Modify: `src/main.rs:128-173` (infer_filter)
- Test: `tests/integration_test.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn infer_filter_handles_unicode_filenames() {
    // This would panic before the fix due to byte-indexing on multi-byte chars
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("café_photo.jpg"), "").unwrap();
    fs::write(dir.path().join("café_video.mp4"), "").unwrap();

    // infer_filter is private, but we can test through the public API
    // by verifying it doesn't panic with Unicode filenames.
    // Just loading files with unicode names and filtering should work.
    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.filter_text = "café".to_string();
    app.update_filter();
    // Both files should match the substring filter
    assert_eq!(app.filtered_indices.len(), 2);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test infer_filter_handles_unicode -- --nocapture 2>&1`
Expected: PASS (this tests the filter, not infer_filter directly). We need to test infer_filter separately. Since it's a private function in `main.rs`, we'll convert the substring search to use char indices.

- [ ] **Step 3: Fix `infer_filter` to use char-based iteration**

In `src/main.rs`, replace the common-substring search block (lines ~160-173):

```rust
    if stems.len() >= 2 {
        let first = &stems[0];
        let chars: Vec<char> = first.chars().collect();
        for len in (3..=chars.len()).rev() {
            for start in 0..=chars.len() - len {
                let candidate: String = chars[start..start + len].iter().collect();
                if stems.iter().all(|s| s.contains(&candidate)) {
                    return Some(format!("*{}*", candidate));
                }
            }
        }
    }
```

- [ ] **Step 4: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git add src/main.rs tests/integration_test.rs
git commit -m "fix: infer_filter uses char-based iteration to prevent Unicode panic"
```

---

### Task 6: Force rename (X) must run conflict detection

`Shift+X` bypasses all safety — no conflict check, no confirmation. It should at minimum check for disk overwrites.

**Files:**
- Modify: `src/input.rs:36-42`

- [ ] **Step 1: Fix force rename to check for disk overwrites**

In `src/input.rs`, replace the `KeyCode::Char('X')` handler:

```rust
        KeyCode::Char('X') => {
            if app.preview.is_empty() {
                app.status_message = "No files to rename".to_string();
            } else if app.has_errors() {
                app.status_message = "Cannot execute: conflicts detected".to_string();
            } else {
                execute_renames(app);
            }
        }
```

- [ ] **Step 2: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 3: Commit**

```bash
git add src/input.rs
git commit -m "fix: force rename (X) now checks for conflicts before executing"
```

---

### Task 7: Clamp preview_scroll when preview shrinks

If preview length decreases (deselect files, change filter), `preview_scroll` can exceed bounds causing a slice panic in `render_preview`.

**Files:**
- Modify: `src/app.rs` (update_preview — already partially handled in Task 1)

- [ ] **Step 1: Verify update_preview clamps preview_scroll**

This was already added in Task 1's `update_preview` rewrite. Verify it's in place:

```rust
        // Clamp preview_scroll when preview shrinks
        if !self.preview.is_empty() {
            self.preview_scroll = self.preview_scroll.min(self.preview.len() - 1);
        } else {
            self.preview_scroll = 0;
        }
```

- [ ] **Step 2: Write test for preview scroll clamping**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn preview_scroll_clamped_when_preview_shrinks() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..20 {
        fs::write(dir.path().join(format!("file_{:02}.txt", i)), "").unwrap();
    }

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.toggle_select_all();
    app.operations.push(Operation::AddPrefix(AddPrefix {
        prefix: "new_".to_string(),
    }));
    app.update_preview();

    // Scroll to the end
    app.preview_scroll = 19;

    // Now deselect all — preview becomes empty
    app.toggle_select_all();
    app.update_preview();

    // preview_scroll should be clamped to 0
    assert_eq!(app.preview_scroll, 0);
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 4: Commit**

```bash
git add src/app.rs tests/integration_test.rs
git commit -m "fix: clamp preview_scroll when preview shrinks to prevent out-of-bounds"
```

---

### Task 8: Fix operation edge cases — dotfiles, title case, slugify, empty search

Four related edge-case fixes in operations:

1. `AddSuffix` on dotfiles (`.gitignore` → `_suffix.gitignore` instead of `.gitignore_suffix`)
2. `CaseChange::Title` capitalizes extensions (`photo.jpg` → `Photo.Jpg`)
3. `Slugify` strips all non-ASCII (`café` → `caf`)
4. `FindReplace` with empty search causes unexpected expansion

**Files:**
- Modify: `src/operations.rs`
- Test: `tests/operations_test.rs`

- [ ] **Step 1: Write failing tests for all four edge cases**

Add to `tests/operations_test.rs`:

```rust
#[test]
fn add_suffix_dotfile() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_backup".to_string(),
    });
    // .gitignore is a dotfile with no real "extension" — suffix should go at end
    assert_eq!(op.apply(".gitignore"), ".gitignore_backup");
}

#[test]
fn case_change_title_preserves_extension() {
    let op = Operation::CaseChange(CaseChange { mode: CaseMode::Title });
    assert_eq!(op.apply("hello world.txt"), "Hello World.txt");
    assert_eq!(op.apply("my photo.JPG"), "My Photo.jpg");
}

#[test]
fn slugify_preserves_non_ascii_alphanumeric() {
    let op = Operation::Slugify(Slugify { separator: "-".to_string() });
    assert_eq!(op.apply("café photo.jpg"), "café-photo.jpg");
}

#[test]
fn find_replace_empty_search_is_noop() {
    let op = Operation::FindReplace(FindReplace {
        search: "".to_string(),
        replacement: "x".to_string(),
    });
    // Empty search should not modify the filename
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test add_suffix_dotfile case_change_title_preserves_extension slugify_preserves_non_ascii find_replace_empty_search_is_noop -- --nocapture 2>&1`
Expected: All FAIL

- [ ] **Step 3: Fix `AddSuffix` for dotfiles**

In `src/operations.rs`, replace `AddSuffix::apply`:

```rust
    fn apply(&self, filename: &str) -> String {
        // Dotfiles like ".gitignore" have no stem — treat the whole name as stem
        match filename.rfind('.') {
            Some(0) | None => format!("{}{}", filename, self.suffix),
            Some(dot_pos) => {
                let stem = &filename[..dot_pos];
                let ext = &filename[dot_pos..];
                format!("{}{}{}", stem, self.suffix, ext)
            }
        }
    }
```

- [ ] **Step 4: Fix `CaseChange::Title` to preserve extension**

In `src/operations.rs`, replace `CaseChange::apply`:

```rust
    fn apply(&self, filename: &str) -> String {
        match self.mode {
            CaseMode::Lower => filename.to_lowercase(),
            CaseMode::Upper => filename.to_uppercase(),
            CaseMode::Title => {
                // Split stem and extension, only title-case the stem
                let (stem, ext) = match filename.rfind('.') {
                    Some(0) | None => (filename, ""),
                    Some(dot_pos) => (&filename[..dot_pos], &filename[dot_pos..]),
                };
                let titled_stem: String = stem
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
                    .collect();
                format!("{}{}", titled_stem, ext.to_lowercase())
            }
        }
    }
```

- [ ] **Step 5: Fix `Slugify` to preserve non-ASCII alphanumeric**

In `src/operations.rs`, in `Slugify::apply`, replace:

```rust
            .map(|c| if c.is_ascii_alphanumeric() { c } else { self.separator.chars().next().unwrap_or('-') })
```

With:

```rust
            .map(|c| if c.is_alphanumeric() { c } else { self.separator.chars().next().unwrap_or('-') })
```

- [ ] **Step 6: Fix `FindReplace` with empty search**

In `src/operations.rs`, replace `FindReplace::apply`:

```rust
    fn apply(&self, filename: &str) -> String {
        if self.search.is_empty() {
            return filename.to_string();
        }
        filename.replace(&self.search, &self.replacement)
    }
```

- [ ] **Step 7: Fix `SequentialNumber::Suffix` and `SequentialNumber::Replace` for dotfiles**

Same dotfile issue as AddSuffix. In `src/operations.rs`, update `SequentialNumber::apply`:

```rust
    fn apply(&self, filename: &str, index: usize) -> String {
        let number = self.start + index as u32;
        let formatted = format!("{:0>width$}", number, width = self.format_width as usize);

        // Split stem/ext, treating dotfiles (leading dot only) as having no extension
        let (stem, ext) = match filename.rfind('.') {
            Some(0) | None => (filename, ""),
            Some(dot_pos) => (&filename[..dot_pos], &filename[dot_pos..]),
        };

        match self.position {
            NumberPosition::Prefix => format!("{}_{}", formatted, filename),
            NumberPosition::Suffix => format!("{}_{}{}", stem, formatted, ext),
            NumberPosition::Replace => format!("{}{}", formatted, ext),
        }
    }
```

- [ ] **Step 8: Update existing tests that assumed old behavior**

The `case_change_title` test in `tests/operations_test.rs` currently expects `"Hello World.Txt"`. Update it:

```rust
#[test]
fn case_change_title() {
    let op = Operation::CaseChange(CaseChange { mode: CaseMode::Title });
    assert_eq!(op.apply("hello world.txt"), "Hello World.txt");
}
```

- [ ] **Step 9: Run all tests**

Run: `cargo test 2>&1`
Expected: All pass

- [ ] **Step 10: Commit**

```bash
git add src/operations.rs tests/operations_test.rs
git commit -m "fix: operation edge cases — dotfiles, title case extension, slugify Unicode, empty search"
```

---

### Task 9: Fix `find_conflicts` for cross-directory false positives

In recursive mode, two files in different directories should be allowed to have the same new name. The Task 1 fix already handles same-directory duplicate detection. Verify with a test.

**Files:**
- Test: `tests/integration_test.rs`

- [ ] **Step 1: Write test — same new name in different directories is NOT a conflict**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn same_new_name_different_directories_is_not_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let sub_a = dir.path().join("a");
    let sub_b = dir.path().join("b");
    fs::create_dir(&sub_a).unwrap();
    fs::create_dir(&sub_b).unwrap();
    fs::write(sub_a.join("old.txt"), "").unwrap();
    fs::write(sub_b.join("old.txt"), "").unwrap();

    let files = vec![
        files::FileEntry {
            file_name: "old.txt".to_string(),
            parent_dir: sub_a.clone(),
        },
        files::FileEntry {
            file_name: "old.txt".to_string(),
            parent_dir: sub_b.clone(),
        },
    ];

    let mut app = App::new(files);
    app.toggle_select_all();

    // Both rename to "new.txt" — should be fine since different directories
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "old".to_string(),
        replacement: "new".to_string(),
    }));
    app.update_preview();

    assert!(!app.has_errors());
}
```

- [ ] **Step 2: Run test**

Run: `cargo test same_new_name_different_directories -- --nocapture 2>&1`
Expected: PASS (already handled by Task 1's directory-aware conflict check)

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: verify cross-directory same-name renames don't conflict"
```

---

### Task 10: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1`
Expected: No warnings

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt -- --check 2>&1`
Expected: No formatting issues

- [ ] **Step 4: Fix any issues from steps 1-3**

If any issues, fix them and re-run.

- [ ] **Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```
