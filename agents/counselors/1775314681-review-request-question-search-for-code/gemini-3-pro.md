### CRITICAL: Missing Validation for File Overwrites (Data Loss)
**Finding:** `find_conflicts()` in `src/app.rs` only checks if two files *in the active preview set* resolve to the exact same new name. It completely ignores the actual disk state and unselected files.
**Impact:** If a selected file is renamed to match an existing file on disk (that isn't currently being renamed), `fs::rename` will silently overwrite the existing file on Unix-like systems. This is a severe data loss vector.
**Alternative:** `find_conflicts()` must enforce `!entry.parent_dir.join(new_name).exists()` (or cross-check the target name against the entire `app.files` list) to block overwriting existing data.

### CRITICAL: Cyclic Renames Cause Data Loss
**Finding:** `execute_renames()` loops over `app.preview` and executes `fs::rename` sequentially based on the initial alphabetical order. It lacks cycle detection.
**Impact:** If a user renames `a.txt` -> `b.txt` and `b.txt` -> `c.txt`, `a.txt` overwrites the original `b.txt` on the first iteration. When the loop reaches the original `b.txt`, it actually renames the *new* `b.txt` (formerly `a.txt`) to `c.txt`. The original `b.txt` is permanently lost.
**Alternative:** Perform topological sorting on the rename pairs to resolve dependency chains. For cyclic swaps (`a -> b` and `b -> a`), move files to temporary intermediary names first.

### CRITICAL: UTF-8 Boundary Panics in Input Fields
**Finding:** Text editing in `handle_param_editor` (`src/input.rs`) and cursor slicing in `render_param_editor` (`src/ui.rs`) incorrectly manipulate byte indices (`+ 1` / `- 1`) instead of character boundaries. 
**Impact:** If a user types a multi-byte character (like an emoji, `é`, or non-ASCII punctuation) and moves the cursor with the arrow keys, `*pos` becomes unaligned. The very next keystroke (insert, delete) or UI render slice (`&value[..text_cursor]`) will trigger a hard `panic!` because the index is not on a valid UTF-8 boundary.
**Alternative:** Manage text cursor state as a `char` index, not a byte index. Convert to byte indices via `.chars().take(pos).map(|c| c.len_utf8()).sum()` before performing string insertions, or use a crate like `tui-input` that handles UTF-8 safely.

### CRITICAL: The `--recursive` Flag Architecture is Fundamentally Broken
**Finding:** `App` state and `execute_renames` rely entirely on the basename (`file_name`) string for equality and lookup (`app.files.iter().find(|f| f.file_name == *old_name)`).
**Impact:** If `--recursive` finds `src/main.rs` and `tests/main.rs`, the system cannot distinguish between them. Previewing and executing will find the first `main.rs` in `app.files` multiple times, repeatedly renaming the first file and completely ignoring the subsequent ones. Furthermore, `find_conflicts` will block execution if two files in *different* directories are renamed to the same basename.
**Alternative:** Track items using their absolute path, or assign a unique `usize` ID/UUID to every `FileEntry`. Use this unique identifier in the `preview` tuples instead of relying on basenames.

### HIGH: Argument Resolution Discards User Intent
**Finding:** In `src/main.rs`, `resolve_args` catches shell-expanded globs (e.g., `renx file1.txt file2.txt`). Instead of processing exactly those files, it extracts their parent directory, reads *all* files in that directory, and tries to guess a wildcard filter using `infer_filter()`.
**Impact:** A user explicitly passing two specific files to the CLI will see their entire directory loaded, overriding standard Unix CLI expectations.
**Alternative:** If multiple file arguments are provided, skip directory discovery entirely. Populate `app.files` exactly with the resolved explicit arguments provided by the user.

### HIGH: Input Handling Traps
**Finding:** In `Filtering` mode, the left/right arrow keys are unhandled, preventing users from editing the middle of their filter string. Worse, pressing `Esc` clears the filter completely rather than canceling the edit and reverting to the previous filter state.
**Impact:** Frustrating UX; users are forced to delete and retype their entire filter if they make a typo at the beginning of the string.
**Alternative:** Implement left/right cursor movement in `Filtering` mode. Store a snapshot of the filter when entering the mode, and restore it if `Esc` is pressed.

### MEDIUM: Operation Edge Cases
- **CaseChange (Title Mode):** It splits on all non-alphanumeric characters, meaning `file.txt` becomes `File.Txt`. Capitalizing file extensions is highly destructive and unexpected. Parse the stem and extension separately before applying case changes.
- **Hidden Files:** `AddSuffix` treats `.gitignore` as an extension with no stem, resulting in `_suffix.gitignore` rather than `.gitignore_suffix`. 
- **Windows Separators:** `find_conflicts` rejects `/` in new filenames to prevent directory traversal, but fails to check `\`, leaving Windows users vulnerable to accidental moves or directory traversal attacks.

### LOW: Filter UX Inconsistency
- **Finding:** `wildcard_match` applies a full-string match (`^pattern$`), while an absence of wildcards falls back to `contains` (substring match). 
- **Impact:** Searching for `foo` matches `abc_foo_def`, but searching for `*foo` drops the match unless the user types `*foo*`. This abrupt behavior change is jarring.
