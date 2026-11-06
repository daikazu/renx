use crate::anim::{DoneState, Effect, EffectKind, FILE_POP_TICKS, INTRO_TICKS};
use crate::files::{self, FileEntry};
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
    Filtering,
    SelectingOperationType,
    EditingOperationParams,
    ConfirmExecute,
    HelpOverlay,
    /// Results screen after renaming; any key skips the cascade, then exits.
    Done,
}

/// The full application state.
pub struct App {
    /// All discovered files.
    pub files: Vec<FileEntry>,
    /// Whether each file is selected for renaming.
    pub selected: Vec<bool>,
    /// Cursor position in the file list (index into filtered_indices).
    pub file_cursor: usize,
    /// Scroll offset for the file list panel.
    pub file_scroll: usize,
    /// The ordered operation stack.
    pub operations: Vec<Operation>,
    /// Cursor position in the operation stack.
    pub op_cursor: usize,
    /// Which panel has focus.
    pub focused_panel: Panel,
    /// Current input mode (normal vs popup).
    pub input_mode: InputMode,
    /// Cached preview results (new filenames for selected files).
    pub preview: Vec<(usize, String)>,
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
    /// Field types: None = text input, Some(options) = selector with current index.
    pub popup_field_options: Vec<Option<Vec<String>>>,
    /// Current selection index for selector fields.
    pub popup_field_selections: Vec<usize>,
    /// Text cursor position within each text field.
    pub popup_text_cursors: Vec<usize>,
    /// Which field is focused in the parameter popup.
    pub popup_field_cursor: usize,
    /// If editing an existing operation, its index. None if adding new.
    pub editing_op_index: Option<usize>,
    /// Current filter text for the file list.
    pub filter_text: String,
    /// Indices into `files` that match the current filter.
    pub filtered_indices: Vec<usize>,
    /// Visible height of the file list panel (set by UI each frame).
    pub file_list_height: usize,
    /// Visible height of the preview panel (set by UI each frame).
    pub preview_height: usize,
    /// Animation frame counter, advanced by `tick()`.
    pub tick: u64,
    /// Whether animations run at all (`--no-animation` turns them off).
    pub animate: bool,
    /// Short-lived visual effects currently playing.
    pub effects: Vec<Effect>,
    /// Results screen state once renames have run.
    pub done: Option<DoneState>,
}

impl App {
    pub fn new(files: Vec<FileEntry>) -> Self {
        let count = files.len();
        let filtered_indices: Vec<usize> = (0..count).collect();
        let mut app = App {
            files,
            selected: vec![false; count],
            file_cursor: 0,
            file_scroll: 0,
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
            popup_field_options: Vec::new(),
            popup_field_selections: Vec::new(),
            popup_text_cursors: Vec::new(),
            popup_field_cursor: 0,
            editing_op_index: None,
            filter_text: String::new(),
            filtered_indices,
            file_list_height: 20,
            preview_height: 10,
            tick: 0,
            animate: false,
            effects: Vec::new(),
            done: None,
        };
        app.update_preview();
        app
    }

    /// Rebuild filtered_indices based on current filter_text.
    /// Supports wildcards: `*` matches any chars, `?` matches one char.
    /// If no wildcards are present, falls back to substring match.
    pub fn update_filter(&mut self) {
        let query = self.filter_text.to_lowercase();
        self.filtered_indices = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                if query.is_empty() {
                    return true;
                }
                let name = f.file_name.to_lowercase();
                if query.contains('*') || query.contains('?') {
                    wildcard_match(&query, &name)
                } else {
                    name.contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();
        // Keep cursor in bounds
        if self.file_cursor >= self.filtered_indices.len() {
            self.file_cursor = self.filtered_indices.len().saturating_sub(1);
        }
    }

    /// Keep file_scroll in sync so the cursor is always visible.
    pub fn ensure_file_cursor_visible(&mut self) {
        if self.file_cursor < self.file_scroll {
            self.file_scroll = self.file_cursor;
        } else if self.file_cursor >= self.file_scroll + self.file_list_height {
            self.file_scroll = self.file_cursor - self.file_list_height + 1;
        }
    }

    /// Get the actual file index for the current cursor position in the filtered list.
    pub fn actual_file_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.file_cursor).copied()
    }

    /// Recalculate the preview based on current operations and selected files.
    /// Only generates preview when there are operations to apply.
    pub fn update_preview(&mut self) {
        if self.operations.is_empty() {
            self.preview.clear();
            return;
        }
        let pipeline = Pipeline::new(self.operations.clone());
        let selected: Vec<(usize, String)> = self
            .filtered_indices
            .iter()
            .filter(|&&i| self.selected[i])
            .map(|&i| (i, self.files[i].file_name.clone()))
            .collect();
        let entries: Vec<FileEntry> = selected
            .iter()
            .map(|(idx, _)| self.files[*idx].clone())
            .collect();
        let new_names = pipeline.apply_entries(&entries);
        self.preview = selected
            .iter()
            .map(|(idx, _)| *idx)
            .zip(new_names)
            .collect();
        // Clamp preview_scroll when preview shrinks
        if self.preview.is_empty() {
            self.preview_scroll = 0;
        } else if self.preview_scroll >= self.preview.len() {
            self.preview_scroll = self.preview.len() - 1;
        }
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

    pub fn prev_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            Panel::Files => Panel::Preview,
            Panel::Operations => Panel::Files,
            Panel::Preview => Panel::Operations,
        };
    }

    /// Toggle selection of file at cursor (respects filter).
    pub fn toggle_file_selection(&mut self) {
        if let Some(actual_idx) = self.actual_file_index() {
            self.selected[actual_idx] = !self.selected[actual_idx];
            self.add_effect(EffectKind::FilePop(actual_idx), FILE_POP_TICKS);
            self.update_preview();
        }
    }

    /// Select all or deselect all visible (filtered) files.
    pub fn toggle_select_all(&mut self) {
        let all_selected = self.filtered_indices.iter().all(|&i| self.selected[i]);
        let new_value = !all_selected;
        for &i in &self.filtered_indices {
            self.selected[i] = new_value;
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
        for (i, (idx_a, new_a)) in self.preview.iter().enumerate() {
            // Check for duplicates within the same parent directory
            for (j, (idx_b, new_b)) in self.preview.iter().enumerate() {
                if i != j
                    && new_a == new_b
                    && self.files[*idx_a].parent_dir == self.files[*idx_b].parent_dir
                {
                    conflicts.push(i);
                    break;
                }
            }
            if new_a.is_empty() || new_a.trim().is_empty() {
                if !conflicts.contains(&i) {
                    conflicts.push(i);
                }
                continue;
            }
            if (new_a.contains('/') || new_a.contains('\0') || new_a.contains('\\'))
                && !conflicts.contains(&i)
            {
                conflicts.push(i);
            }
            // Check if new name collides with an existing file on disk that isn't being renamed
            let entry = &self.files[*idx_a];
            let target_path = entry.parent_dir.join(new_a);
            // `same_file` compares by identity, so a case-only rename on a
            // case-insensitive filesystem is not treated as a collision.
            if target_path.exists() && !files::same_file(&target_path, &entry.full_path()) {
                // Check it's not another file being renamed away in this preview set
                let is_in_preview = self
                    .preview
                    .iter()
                    .any(|(idx, _)| files::same_file(&self.files[*idx].full_path(), &target_path));
                if !is_in_preview && !conflicts.contains(&i) {
                    conflicts.push(i);
                }
            }
        }
        conflicts
    }

    // --- Animation -------------------------------------------------------

    /// Advance one animation frame and drop finished effects.
    pub fn tick(&mut self) {
        self.tick += 1;
        let now = self.tick;
        self.effects.retain(|e| e.progress(now).is_some());
    }

    /// Start an effect (replacing any running effect of the same kind).
    /// Does nothing when animation is off.
    pub fn add_effect(&mut self, kind: EffectKind, duration: u64) {
        if !self.animate {
            return;
        }
        self.effects.retain(|e| e.kind != kind);
        self.effects.push(Effect {
            kind,
            started: self.tick,
            duration,
        });
    }

    /// Progress of a running effect in `0.0..1.0`, if it is running.
    pub fn effect_progress(&self, kind: &EffectKind) -> Option<f32> {
        self.effects
            .iter()
            .find(|e| &e.kind == kind)
            .and_then(|e| e.progress(self.tick))
    }

    /// Startup draw-in progress, `1.0` once finished or when animation is off.
    pub fn intro_progress(&self) -> f32 {
        if !self.animate {
            1.0
        } else {
            (self.tick as f32 / INTRO_TICKS as f32).min(1.0)
        }
    }

    /// Whether any preview row would actually change its name.
    pub fn preview_has_changes(&self) -> bool {
        self.preview
            .iter()
            .any(|(idx, new)| self.files[*idx].file_name != *new)
    }

    /// Rows of the results screen currently showing their new name.
    pub fn done_revealed(&self) -> usize {
        self.done
            .as_ref()
            .map(|d| d.revealed(self.tick, self.animate))
            .unwrap_or(0)
    }

    /// Brightness of a results row, `None` until it is revealed.
    pub fn done_row_glow(&self, i: usize) -> Option<f32> {
        self.done
            .as_ref()
            .and_then(|d| d.row_glow(i, self.tick, self.animate))
    }

    /// Whether the event loop should wake up on a timer to redraw.
    pub fn needs_tick(&self) -> bool {
        if !self.animate {
            return false;
        }
        if self.intro_progress() < 1.0 || !self.effects.is_empty() {
            return true;
        }
        match &self.done {
            Some(d) => !d.complete(self.tick, self.animate),
            None => self.preview_has_changes(),
        }
    }

    /// Whether execution is blocked (conflicts exist).
    pub fn has_errors(&self) -> bool {
        !self.find_conflicts().is_empty()
    }
}

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

/// Simple glob-style wildcard matching.
/// `*` matches zero or more characters, `?` matches exactly one character.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (plen, tlen) = (p.len(), t.len());

    // dp[i][j] = pattern[0..i] matches text[0..j]
    let mut dp = vec![vec![false; tlen + 1]; plen + 1];
    dp[0][0] = true;

    // Leading *s can match empty
    for i in 1..=plen {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=plen {
        for j in 1..=tlen {
            if p[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p[i - 1] == '?' || p[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }

    dp[plen][tlen]
}
