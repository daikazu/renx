use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode, Panel};
use crate::operations::*;

/// Handle a key event and mutate app state accordingly.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Filtering => handle_filter_mode(app, key),
        InputMode::SelectingOperationType => handle_type_picker(app, key),
        InputMode::EditingOperationParams => handle_param_editor(app, key),
        InputMode::ConfirmExecute => handle_confirm(app, key),
        InputMode::HelpOverlay => handle_help(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.next_panel();
        }
        KeyCode::BackTab => {
            app.prev_panel();
        }
        KeyCode::Char('?') => {
            app.input_mode = InputMode::HelpOverlay;
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Filtering;
            // Don't clear existing filter — user can continue editing it
        }
        KeyCode::Char('X') => {
            if app.preview.is_empty() {
                app.status_message = "No files to rename".to_string();
            } else if app.has_errors() {
                app.status_message = "Cannot execute: conflicts detected".to_string();
            } else {
                execute_renames(app);
            }
        }
        KeyCode::Char('x') => {
            if app.preview.is_empty() {
                app.status_message = "No files to rename".to_string();
            } else if app.has_errors() {
                app.status_message = "Cannot execute: conflicts detected".to_string();
            } else {
                app.input_mode = InputMode::ConfirmExecute;
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
    let max = app.filtered_indices.len().saturating_sub(1);
    let page = app.file_list_height.max(1);
    let half = (page / 2).max(1);

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.file_cursor < max {
                app.file_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.file_cursor > 0 {
                app.file_cursor -= 1;
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.file_cursor = (app.file_cursor + half).min(max);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.file_cursor = app.file_cursor.saturating_sub(half);
        }
        KeyCode::PageDown => {
            app.file_cursor = (app.file_cursor + page).min(max);
        }
        KeyCode::PageUp => {
            app.file_cursor = app.file_cursor.saturating_sub(page);
        }
        KeyCode::Char('G') => {
            app.file_cursor = max;
        }
        KeyCode::Char('g') => {
            app.file_cursor = 0;
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            app.toggle_file_selection();
        }
        KeyCode::Char('v') => {
            app.toggle_select_all();
        }
        KeyCode::Char('a') => {
            app.input_mode = InputMode::SelectingOperationType;
            app.popup_type_cursor = 0;
        }
        KeyCode::Esc => {
            if !app.filter_text.is_empty() {
                app.filter_text.clear();
                app.update_filter();
            }
        }
        _ => {}
    }
}

fn handle_ops_input(app: &mut App, key: KeyEvent) {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Down if shift => {
            app.move_operation_down();
        }
        KeyCode::Up if shift => {
            app.move_operation_up();
        }
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
        KeyCode::Char('e') | KeyCode::Enter => {
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
    let max = app.preview.len().saturating_sub(1);
    let page = app.preview_height.max(1);
    let half = (page / 2).max(1);

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.preview_scroll < max {
                app.preview_scroll += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.preview_scroll > 0 {
                app.preview_scroll -= 1;
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.preview_scroll = (app.preview_scroll + half).min(max);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.preview_scroll = app.preview_scroll.saturating_sub(half);
        }
        KeyCode::PageDown => {
            app.preview_scroll = (app.preview_scroll + page).min(max);
        }
        KeyCode::PageUp => {
            app.preview_scroll = app.preview_scroll.saturating_sub(page);
        }
        KeyCode::Char('G') => {
            app.preview_scroll = max;
        }
        KeyCode::Char('g') => {
            app.preview_scroll = 0;
        }
        _ => {}
    }
}

fn handle_filter_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            // Clear filter and go back to normal
            app.filter_text.clear();
            app.update_filter();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            // Confirm filter and go back to normal
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            app.update_filter();
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            app.update_filter();
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.input_mode = InputMode::Normal;
            execute_renames(app);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.status_message.clear();
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
            if app.popup_type_cursor + 1 < OP_TYPES.len() {
                app.popup_type_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.popup_type_cursor > 0 {
                app.popup_type_cursor -= 1;
            }
        }
        KeyCode::Enter => {
            setup_fields_for_type(app);
            app.popup_field_cursor = 0;
            app.input_mode = InputMode::EditingOperationParams;
        }
        _ => {}
    }
}

fn handle_param_editor(app: &mut App, key: KeyEvent) {
    let cursor = app.popup_field_cursor;
    let is_selector = app
        .popup_field_options
        .get(cursor)
        .map(|o| o.is_some())
        .unwrap_or(false);

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
        KeyCode::BackTab => {
            if !app.popup_fields.is_empty() {
                app.popup_field_cursor = if app.popup_field_cursor == 0 {
                    app.popup_fields.len() - 1
                } else {
                    app.popup_field_cursor - 1
                };
            }
        }
        KeyCode::Enter => {
            // Sync selector values into popup_fields before building
            for (i, opts) in app.popup_field_options.iter().enumerate() {
                if let Some(options) = opts {
                    let sel = app.popup_field_selections.get(i).copied().unwrap_or(0);
                    if let Some(val) = options.get(sel) {
                        app.popup_fields[i] = val.clone();
                    }
                }
            }
            // Refuse unusable parameters and keep the editor open
            if let Err(msg) = validate_fields(app) {
                app.status_message = msg;
                return;
            }
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
        // Selector fields: left/right to cycle options
        KeyCode::Left if is_selector => {
            if let Some(sel) = app.popup_field_selections.get_mut(cursor) {
                if *sel > 0 {
                    *sel -= 1;
                }
            }
        }
        KeyCode::Right if is_selector => {
            if let Some(options) = &app.popup_field_options[cursor] {
                if let Some(sel) = app.popup_field_selections.get_mut(cursor) {
                    if *sel + 1 < options.len() {
                        *sel += 1;
                    }
                }
            }
        }
        // Text fields: cursor movement
        KeyCode::Left if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                if *pos > 0 {
                    *pos -= 1;
                }
            }
        }
        KeyCode::Right if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                let len = app
                    .popup_fields
                    .get(cursor)
                    .map(|f| crate::app::char_len(f))
                    .unwrap_or(0);
                if *pos < len {
                    *pos += 1;
                }
            }
        }
        KeyCode::Home if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                *pos = 0;
            }
        }
        KeyCode::End if !is_selector => {
            if let Some(pos) = app.popup_text_cursors.get_mut(cursor) {
                *pos = app
                    .popup_fields
                    .get(cursor)
                    .map(|f| crate::app::char_len(f))
                    .unwrap_or(0);
            }
        }
        // Text fields: editing
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

fn field(app: &App, i: usize) -> String {
    app.popup_fields.get(i).cloned().unwrap_or_default()
}

fn number(app: &App, i: usize, default: u32) -> u32 {
    field(app, i).trim().parse().unwrap_or(default)
}

/// Check the editor's fields before building an operation from them.
fn validate_fields(app: &App) -> Result<(), String> {
    match app.popup_type_cursor {
        T_REGEX_REPLACE => regex::Regex::new(&field(app, 0))
            .map(|_| ())
            .map_err(|e| format!("Invalid regex: {}", e)),
        T_INSERT_DATE => validate_date_format(&field(app, 1)),
        _ => Ok(()),
    }
}

fn build_operation_from_fields(app: &App) -> Option<Operation> {
    let op = match app.popup_type_cursor {
        T_FIND_REPLACE => Operation::FindReplace(FindReplace {
            search: field(app, 0),
            replacement: field(app, 1),
            scope: Scope::from_label(&field(app, 2)),
        }),
        T_REGEX_REPLACE => Operation::RegexReplace(RegexReplace {
            pattern: field(app, 0),
            replacement: field(app, 1),
            scope: Scope::from_label(&field(app, 2)),
        }),
        T_REMOVE_PATTERN => Operation::RemovePattern(RemovePattern {
            pattern: field(app, 0),
            scope: Scope::from_label(&field(app, 1)),
        }),
        T_CASE_CHANGE => Operation::CaseChange(CaseChange {
            mode: CaseMode::from_label(&field(app, 0)),
            scope: Scope::from_label(&field(app, 1)),
        }),
        T_ADD_PREFIX => Operation::AddPrefix(AddPrefix {
            prefix: field(app, 0),
        }),
        T_ADD_SUFFIX => Operation::AddSuffix(AddSuffix {
            suffix: field(app, 0),
        }),
        T_INSERT => Operation::Insert(Insert {
            text: field(app, 0),
            position: number(app, 1, 0),
            from: Side::from_label(&field(app, 2)),
            scope: Scope::from_label(&field(app, 3)),
        }),
        T_TRIM => Operation::Trim(Trim {
            count: number(app, 0, 1),
            from: Side::from_label(&field(app, 1)),
            scope: Scope::from_label(&field(app, 2)),
        }),
        T_TRUNCATE => Operation::Truncate(Truncate {
            max_len: number(app, 0, 32),
            scope: Scope::from_label(&field(app, 1)),
        }),
        T_SEQUENTIAL => Operation::SequentialNumber(SequentialNumber {
            format_width: number(app, 0, 3),
            start: number(app, 1, 1),
            position: match field(app, 2).as_str() {
                "suffix" => NumberPosition::Suffix,
                "replace" => NumberPosition::Replace,
                _ => NumberPosition::Prefix,
            },
        }),
        T_ZERO_PAD => Operation::ZeroPad(ZeroPad {
            width: number(app, 0, 2),
            scope: Scope::from_label(&field(app, 1)),
        }),
        T_INSERT_DATE => Operation::InsertDate(InsertDate {
            source: DateSource::from_label(&field(app, 0)),
            format: field(app, 1),
            position: match field(app, 2).as_str() {
                "suffix" => NumberPosition::Suffix,
                "replace" => NumberPosition::Replace,
                _ => NumberPosition::Prefix,
            },
        }),
        T_CHANGE_EXT => Operation::ChangeExtension(ChangeExtension {
            extension: field(app, 0),
        }),
        T_SLUGIFY => {
            let sep = field(app, 0);
            Operation::Slugify(Slugify {
                separator: if sep.is_empty() { "-".to_string() } else { sep },
            })
        }
        T_STRIP_DIACRITICS => Operation::StripDiacritics(StripDiacritics {
            scope: Scope::from_label(&field(app, 0)),
        }),
        _ => return None,
    };
    Some(op)
}

/// Set up popup_fields, popup_field_options, and popup_field_selections
/// for the current popup_type_cursor from the OP_TYPES registry.
fn setup_fields_for_type(app: &mut App) {
    let Some(spec) = OP_TYPES.get(app.popup_type_cursor) else {
        app.popup_fields = Vec::new();
        app.popup_field_options = Vec::new();
        app.popup_field_selections = Vec::new();
        app.popup_text_cursors = Vec::new();
        return;
    };
    app.popup_fields = spec
        .fields
        .iter()
        .map(|f| {
            if f.options.is_some() {
                String::new()
            } else {
                f.default.to_string()
            }
        })
        .collect();
    app.popup_field_options = spec
        .fields
        .iter()
        .map(|f| {
            f.options
                .map(|opts| opts.iter().map(|o| o.to_string()).collect())
        })
        .collect();
    app.popup_field_selections = spec
        .fields
        .iter()
        .map(|f| {
            f.options
                .and_then(|opts| opts.iter().position(|o| *o == f.default))
                .unwrap_or(0)
        })
        .collect();
    app.popup_text_cursors = app
        .popup_fields
        .iter()
        .map(|f| crate::app::char_len(f))
        .collect();
}

fn set_text(app: &mut App, i: usize, value: String) {
    app.popup_text_cursors[i] = crate::app::char_len(&value);
    app.popup_fields[i] = value;
}

fn set_choice(app: &mut App, i: usize, label: &str) {
    if let Some(Some(options)) = app.popup_field_options.get(i) {
        if let Some(pos) = options.iter().position(|o| o == label) {
            app.popup_field_selections[i] = pos;
        }
    }
}

fn position_label(position: &NumberPosition) -> &'static str {
    match position {
        NumberPosition::Prefix => "prefix",
        NumberPosition::Suffix => "suffix",
        NumberPosition::Replace => "replace",
    }
}

/// Open the parameter editor pre-filled with the operation under the cursor.
pub fn start_editing_operation(app: &mut App) {
    let op = app.operations[app.op_cursor].clone();
    app.popup_type_cursor = op.type_index();
    setup_fields_for_type(app);

    match &op {
        Operation::FindReplace(o) => {
            set_text(app, 0, o.search.clone());
            set_text(app, 1, o.replacement.clone());
            set_choice(app, 2, o.scope.label());
        }
        Operation::RegexReplace(o) => {
            set_text(app, 0, o.pattern.clone());
            set_text(app, 1, o.replacement.clone());
            set_choice(app, 2, o.scope.label());
        }
        Operation::RemovePattern(o) => {
            set_text(app, 0, o.pattern.clone());
            set_choice(app, 1, o.scope.label());
        }
        Operation::CaseChange(o) => {
            set_choice(app, 0, o.mode.label());
            set_choice(app, 1, o.scope.label());
        }
        Operation::AddPrefix(o) => set_text(app, 0, o.prefix.clone()),
        Operation::AddSuffix(o) => set_text(app, 0, o.suffix.clone()),
        Operation::Insert(o) => {
            set_text(app, 0, o.text.clone());
            set_text(app, 1, o.position.to_string());
            set_choice(app, 2, o.from.label());
            set_choice(app, 3, o.scope.label());
        }
        Operation::Trim(o) => {
            set_text(app, 0, o.count.to_string());
            set_choice(app, 1, o.from.label());
            set_choice(app, 2, o.scope.label());
        }
        Operation::Truncate(o) => {
            set_text(app, 0, o.max_len.to_string());
            set_choice(app, 1, o.scope.label());
        }
        Operation::SequentialNumber(o) => {
            set_text(app, 0, o.format_width.to_string());
            set_text(app, 1, o.start.to_string());
            set_choice(app, 2, position_label(&o.position));
        }
        Operation::ZeroPad(o) => {
            set_text(app, 0, o.width.to_string());
            set_choice(app, 1, o.scope.label());
        }
        Operation::InsertDate(o) => {
            set_choice(app, 0, o.source.label());
            set_text(app, 1, o.format.clone());
            set_choice(app, 2, position_label(&o.position));
        }
        Operation::ChangeExtension(o) => set_text(app, 0, o.extension.clone()),
        Operation::Slugify(o) => set_choice(app, 0, &o.separator),
        Operation::StripDiacritics(o) => set_choice(app, 0, o.scope.label()),
    }

    app.popup_field_cursor = 0;
    app.editing_op_index = Some(app.op_cursor);
    app.input_mode = InputMode::EditingOperationParams;
}

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
            // Rollback: move temp back to original
            let _ = crate::files::rename_file(&temp_entry, &entry.file_name);
            failures.push(format!(
                "{}: target '{}' already exists",
                entry.file_name, final_name
            ));
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
