# Code Brittleness & Gap Analysis for renx

## CRITICAL — Data Loss Risks

### 1. `rename_file` can silently overwrite existing files
**Severity: CRITICAL** — `files.rs:62-65`

`fs::rename` will silently overwrite an existing file at the destination path on Unix. `find_conflicts()` only checks for duplicates *within the preview set* — it never checks whether `new_name` already exists on disk as an unselected file.

Example: You have `a.txt` and `b.txt`. You select only `a.txt` and rename it to `b.txt`. No conflict is detected. `b.txt` is silently destroyed.

**Fix**: Before renaming, check `entry.parent_dir.join(new_name).exists()` and skip/error if the target exists and isn't the source file.

### 2. Rename ordering causes data loss with transitive renames
**Severity: CRITICAL** — `input.rs:574-578`

`execute_renames` iterates the preview in order and renames sequentially. If file A→B and file B→C, renaming A→B first overwrites the original B before B→C can execute. The lookup `app.files.iter().find(|f| f.file_name == *old_name)` then finds the *entry* for B but B on disk is now what was A.

**Fix**: Either detect chains and rename in reverse-dependency order, or rename to temporary names first (A→A.tmp, B→B.tmp), then move temps to final names. Alternatively, check that the source still has the expected content/inode.

### 3. Force rename (`X`) bypasses all safety
**Severity: CRITICAL** — `input.rs:37-42`

`X` calls `execute_renames` directly with zero checks — no conflict detection, no confirmation. A user could accidentally hit Shift+X and destroy files. At minimum, `X` should still check for the disk-overwrite case above.

### 4. No atomicity or rollback on partial failure
**Severity: HIGH** — `input.rs:570-598`

If rename #5 of 20 fails, the first 4 are already done. The app quits immediately (`should_quit = true`), leaving files in a half-renamed state with no way to undo. The status message only shows the *first* failure.

**Fix**: Collect all failures and either show them all, or write a log. Consider a dry-run validation pass before starting (check all targets are writable, all sources exist).

## HIGH — Crash / Panic Paths

### 5. `infer_filter` panics on non-UTF-8 boundary slicing
**Severity: HIGH** — `main.rs:163-168`

```rust
let candidate = &first[start..start + len];
```

`first` is a `String` produced by `to_lowercase()`. The `start..start+len` range indexes by byte, but iterates by char count in the loop bound (`first.len()` is byte length). For ASCII this works, but `to_lowercase()` on certain Unicode characters changes byte length (e.g., "İ" → "i̇" which is 2 bytes → 3 bytes). Slicing at arbitrary byte offsets into a non-ASCII lowercase string will panic.

**Fix**: Use `first.char_indices()` for safe slicing, or operate on `Vec<char>`.

### 6. Byte-level string indexing in operations
**Severity: HIGH** — `operations.rs:168-176`, `operations.rs:200-204`, `operations.rs:243-249`

`AddSuffix`, `Slugify`, and `SequentialNumber` all use `rfind('.')` which returns a byte offset, then slice with `&filename[..dot_pos]`. This is safe for ASCII but will panic on filenames where a multi-byte UTF-8 character appears before the dot *only if the dot lands mid-codepoint* — which can't happen since `.` is a single byte. However, the same pattern in `operations.rs:562-563` with `&value[..text_cursor]` in `ui.rs` is dangerous because `text_cursor` counts *characters* but is used as a byte index.

Wait — actually `text_cursor` is manipulated by character insertion/deletion in `handle_param_editor` and tracks positions within `field.insert(pos, c)` and `field.remove(pos)`. `String::insert` and `String::remove` take byte indices. If a user types a multi-byte character, `pos` increments by 1 but the byte length increases by more. **This will panic on the next cursor movement or character insertion with non-ASCII input.**

**Severity: CRITICAL** for the popup text cursor — `input.rs:369-377`. Typing any non-ASCII character (accented letters, emoji, CJK) into an operation parameter field will cause a panic on subsequent edits.

### 7. `popup_text_cursors` / `popup_fields` can desync
**Severity: MEDIUM** — `input.rs:257-379`

The param editor accesses `app.popup_text_cursors.get(cursor)` and `app.popup_fields.get(cursor)` separately. If these vectors ever have different lengths (e.g., via a bug in `setup_fields_for_type`), the editor silently does nothing instead of crashing — but this masks bugs.

## HIGH — Broken Behavior

### 8. `find_conflicts` misses rename-to-self-on-disk scenario
**Severity: HIGH** — `app.rs:247-265`

`find_conflicts` checks: (1) duplicate new names within preview, (2) empty names, (3) names with `/` or `\0`. It does NOT check:
- New name collides with an *existing file on disk* that isn't being renamed (see #1)
- New name collides with the *original name* of a different selected file (cross-rename like A→B, B→A without a temp step)
- Names containing only whitespace
- Names starting with `.` (creating hidden files unintentionally — maybe just a warning)
- Names exceeding filesystem limits (255 bytes on most filesystems)

### 9. `execute_renames` matches by filename, not identity
**Severity: HIGH** — `input.rs:578`

```rust
if let Some(entry) = app.files.iter().find(|f| f.file_name == *old_name) {
```

If two files in different subdirectories (recursive mode) have the same filename, this finds the *first* match, not the correct one. In recursive mode, renaming `subdir_a/photo.jpg` could instead rename `subdir_b/photo.jpg`.

**Fix**: Match on both `file_name` and `parent_dir`, or store the full `FileEntry` reference in the preview.

### 10. Preview only shows filtered+selected files but renames all selected
**Severity: MEDIUM** — `app.rs:160-166` vs `input.rs:574`

`update_preview` only includes files from `filtered_indices` that are selected. But if you select files, then apply a filter that hides some selected files, those hidden-but-selected files aren't in the preview. When executing, `execute_renames` only operates on `app.preview`, so filtered-out selected files won't be renamed. This is arguably correct but could confuse users — they selected 10 files, preview shows 5, execution renames 5.

## MEDIUM — Edge Cases

### 11. `AddSuffix` and `SequentialNumber::Suffix` use `rfind('.')` — wrong for dotfiles
**Severity: MEDIUM** — `operations.rs:168`

For `.gitignore`, `rfind('.')` returns position 0, so `stem = ""` and `ext = ".gitignore"`. Adding suffix `_backup` produces `_backup.gitignore` — the stem becomes the suffix. This is probably wrong; the user likely wants `.gitignore_backup`.

Same issue with `.bashrc` → suffix `_old` → `_old.bashrc`.

### 12. `Slugify` strips all non-ASCII characters
**Severity: MEDIUM** — `operations.rs:209`

`c.is_ascii_alphanumeric()` means any accented character (é, ñ, ü) or non-Latin script is replaced with the separator. `café.txt` → `caf.txt`. This should probably use `c.is_alphanumeric()` and transliterate, or at least document this behavior.

### 13. `CaseChange::Title` operates on the full filename including extension
**Severity: LOW** — `operations.rs:133-147`

`"photo.jpg"` → `"Photo.Jpg"`. The `.jpg` extension becomes `.Jpg`. This is probably undesirable for most users who want title-cased stems but unchanged extensions.

### 14. Empty search string in `FindReplace` causes infinite expansion
**Severity: MEDIUM** — `operations.rs:85-87`

`"hello".replace("", "X")` in Rust produces `"XhXeXlXlXoX"`. If the user leaves the search field blank and enters a replacement, every character gets surrounded. Not a crash, but surprising and potentially destructive.

### 15. Sequential number overflow
**Severity: LOW** — `operations.rs:238`

`self.start + index as u32` can overflow if start is near `u32::MAX` or there are billions of files. Unlikely in practice.

## MEDIUM — State Consistency

### 16. `file_cursor` can be 0 when `filtered_indices` is empty
**Severity: MEDIUM** — `app.rs:132-136`

After `update_filter`, if no files match, `filtered_indices` is empty and `file_cursor` is set to `saturating_sub(1) = 0`. Then `actual_file_index()` returns `None` which is handled, but `handle_files_input` computes `max = 0.saturating_sub(1) = 0` and allows cursor movement to position 0 which is out of bounds for an empty list. The cursor operations are all guarded by comparisons so no panic occurs, but the UI may show a cursor highlight on nothing.

### 17. `op_cursor` not validated when entering edit mode from files panel
**Severity: LOW** — `input.rs:101-103`

Pressing `a` in the files panel opens the type picker but doesn't set `editing_op_index = None`. It's implicitly `None` from initialization, but if the user had previously edited an operation, `editing_op_index` might still be `Some(idx)`. Actually, looking at the code flow — `editing_op_index` is only set in `start_editing_operation` and cleared on Enter/Esc in the param editor. From the files panel, `a` goes to type picker → param editor → Enter clears it. But if the user does: ops panel → `e` (sets `editing_op_index`) → Esc (clears it) → files panel → `a`, it's fine because Esc clears it. This is actually okay.

## LOW — Code Quality / Test Gaps

### 18. No tests for `execute_renames`
**Severity: MEDIUM** — The integration test manually calls `rename_file` instead of exercising the actual `execute_renames` function. The overwrite, ordering, and partial-failure bugs above are untested.

### 19. No tests for filter + selection interaction
**Severity: LOW** — No test verifies that filtering, selecting, changing filter, and executing produces correct results.

### 20. No tests for Unicode filenames
**Severity: MEDIUM** — All tests use ASCII filenames. The byte-indexing bugs (#5, #6) are completely untested.

### 21. `wildcard_match` operates on `Vec<char>` — correct but O(n*m) space
**Severity: LOW** — For very long filenames or patterns, the DP table could use significant memory. Not a real issue for filenames (max 255 chars), but worth noting.

---

## Summary — Priority Fixes

| # | Severity | Issue | Effort |
|---|----------|-------|--------|
| 1 | CRITICAL | Rename overwrites existing non-selected files | Small |
| 2 | CRITICAL | Rename ordering causes data loss on chains | Medium |
| 6 | CRITICAL | Non-ASCII input in popup fields causes panic | Medium |
| 3 | CRITICAL | Force rename bypasses all safety | Small |
| 4 | HIGH | No rollback on partial rename failure | Medium |
| 9 | HIGH | Recursive mode matches wrong file by name | Small |
| 8 | HIGH | Conflict detection misses disk collisions | Small |
| 5 | HIGH | `infer_filter` panics on Unicode | Small |
| 11 | MEDIUM | Dotfiles treated wrong by suffix operations | Small |
| 14 | MEDIUM | Empty search string causes expansion | Small |
| 12 | MEDIUM | Slugify destroys non-ASCII characters | Small |

The top 3 items are straightforward data-loss bugs that should be fixed before anyone uses this on real files. Items #1 and #8 are essentially the same fix (check disk before renaming). Item #6 is a guaranteed panic reproducible by typing any accented character.
