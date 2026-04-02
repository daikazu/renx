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
    Filtering,
    SelectingOperationType,
    EditingOperationParams,
    ConfirmExecute,
    HelpOverlay,
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
        let selected_files: Vec<String> = self
            .filtered_indices
            .iter()
            .filter(|&&i| self.selected[i])
            .map(|&i| self.files[i].file_name.clone())
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
            self.update_preview();
        }
    }

    /// Select all or deselect all visible (filtered) files.
    pub fn toggle_select_all(&mut self) {
        let all_selected = self
            .filtered_indices
            .iter()
            .all(|&i| self.selected[i]);
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
        for (i, (_, new_a)) in self.preview.iter().enumerate() {
            for (j, (_, new_b)) in self.preview.iter().enumerate() {
                if i != j && new_a == new_b {
                    conflicts.push(i);
                    break;
                }
            }
            if new_a.is_empty() {
                conflicts.push(i);
                continue;
            }
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
