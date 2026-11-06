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
            if app.popup_type_cursor < 7 {
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
            // Regex Replace: refuse an invalid pattern and keep the editor open
            if app.popup_type_cursor == 1 {
                if let Some(pattern) = app.popup_fields.first() {
                    if let Err(e) = regex::Regex::new(pattern) {
                        app.status_message = format!("Invalid regex: {}", e);
                        return;
                    }
                }
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

fn build_operation_from_fields(app: &App) -> Option<Operation> {
    let fields = &app.popup_fields;
    match app.popup_type_cursor {
        0 => Some(Operation::FindReplace(FindReplace {
            scope: Scope::Both,
            search: fields.first()?.clone(),
            replacement: fields.get(1)?.clone(),
        })),
        1 => Some(Operation::RegexReplace(RegexReplace {
            scope: Scope::Both,
            pattern: fields.first()?.clone(),
            replacement: fields.get(1)?.clone(),
        })),
        2 => {
            let width: u32 = fields.first()?.parse().unwrap_or(3);
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
            let mode = match fields.first()?.as_str() {
                "upper" => CaseMode::Upper,
                "title" => CaseMode::Title,
                _ => CaseMode::Lower,
            };
            Some(Operation::CaseChange(CaseChange {
                scope: Scope::Both,
                mode,
            }))
        }
        4 => Some(Operation::AddPrefix(AddPrefix {
            prefix: fields.first()?.clone(),
        })),
        5 => Some(Operation::AddSuffix(AddSuffix {
            suffix: fields.first()?.clone(),
        })),
        6 => Some(Operation::RemovePattern(RemovePattern {
            scope: Scope::Both,
            pattern: fields.first()?.clone(),
        })),
        7 => {
            let sep = fields.first()?.clone();
            Some(Operation::Slugify(Slugify {
                separator: if sep.is_empty() { "-".to_string() } else { sep },
            }))
        }
        _ => None,
    }
}

/// Set up popup_fields, popup_field_options, and popup_field_selections
/// for the current popup_type_cursor. Used when adding a new operation.
fn setup_fields_for_type(app: &mut App) {
    match app.popup_type_cursor {
        0 => {
            // Find & Replace
            app.popup_fields = vec![String::new(); 2];
            app.popup_field_options = vec![None, None];
            app.popup_field_selections = vec![0, 0];
        }
        1 => {
            // Regex Replace
            app.popup_fields = vec![String::new(); 2];
            app.popup_field_options = vec![None, None];
            app.popup_field_selections = vec![0, 0];
        }
        2 => {
            // Sequential Number
            app.popup_fields = vec![String::new(), String::new(), String::new()];
            app.popup_field_options = vec![
                None,
                None,
                Some(vec!["prefix".into(), "suffix".into(), "replace".into()]),
            ];
            app.popup_field_selections = vec![0, 0, 0];
        }
        3 => {
            // Case Change
            app.popup_fields = vec![String::new()];
            app.popup_field_options =
                vec![Some(vec!["lower".into(), "upper".into(), "title".into()])];
            app.popup_field_selections = vec![0];
        }
        4 => {
            // Add Prefix
            app.popup_fields = vec![String::new()];
            app.popup_field_options = vec![None];
            app.popup_field_selections = vec![0];
        }
        5 => {
            // Add Suffix
            app.popup_fields = vec![String::new()];
            app.popup_field_options = vec![None];
            app.popup_field_selections = vec![0];
        }
        6 => {
            // Remove Pattern
            app.popup_fields = vec![String::new()];
            app.popup_field_options = vec![None];
            app.popup_field_selections = vec![0];
        }
        7 => {
            // Slugify
            app.popup_fields = vec![String::new()];
            app.popup_field_options = vec![Some(vec!["-".into(), "_".into(), ".".into()])];
            app.popup_field_selections = vec![0];
        }
        _ => {
            app.popup_fields = Vec::new();
            app.popup_field_options = Vec::new();
            app.popup_field_selections = Vec::new();
        }
    }
    app.popup_text_cursors = vec![0; app.popup_fields.len()];
}

fn start_editing_operation(app: &mut App) {
    let op = app.operations[app.op_cursor].clone();
    let type_idx = match &op {
        Operation::FindReplace(_) => 0,
        Operation::RegexReplace(_) => 1,
        Operation::SequentialNumber(_) => 2,
        Operation::CaseChange(_) => 3,
        Operation::AddPrefix(_) => 4,
        Operation::AddSuffix(_) => 5,
        Operation::RemovePattern(_) => 6,
        Operation::Slugify(_) => 7,
        Operation::StripDiacritics(_) => 8,
        Operation::ChangeExtension(_) => 9,
        Operation::Trim(_) => 10,
        Operation::Insert(_) => 11,
        Operation::ZeroPad(_) => 12,
        Operation::Truncate(_) => 13,
        Operation::InsertDate(_) => 14,
    };
    app.popup_type_cursor = type_idx;
    setup_fields_for_type(app);

    // Pre-fill values from the existing operation
    match &op {
        Operation::FindReplace(o) => {
            app.popup_fields[0] = o.search.clone();
            app.popup_fields[1] = o.replacement.clone();
        }
        Operation::RegexReplace(o) => {
            app.popup_fields[0] = o.pattern.clone();
            app.popup_fields[1] = o.replacement.clone();
        }
        Operation::SequentialNumber(o) => {
            app.popup_fields[0] = o.format_width.to_string();
            app.popup_fields[1] = o.start.to_string();
            app.popup_field_selections[2] = match o.position {
                NumberPosition::Prefix => 0,
                NumberPosition::Suffix => 1,
                NumberPosition::Replace => 2,
            };
        }
        Operation::CaseChange(o) => {
            app.popup_field_selections[0] = match o.mode {
                CaseMode::Lower => 0,
                CaseMode::Upper => 1,
                CaseMode::Title => 2,
                CaseMode::Sentence => 3,
                CaseMode::Snake => 4,
                CaseMode::Kebab => 5,
                CaseMode::Camel => 6,
            };
        }
        Operation::AddPrefix(o) => {
            app.popup_fields[0] = o.prefix.clone();
        }
        Operation::AddSuffix(o) => {
            app.popup_fields[0] = o.suffix.clone();
        }
        Operation::RemovePattern(o) => {
            app.popup_fields[0] = o.pattern.clone();
        }
        Operation::Slugify(o) => {
            app.popup_field_selections[0] = match o.separator.as_str() {
                "_" => 1,
                "." => 2,
                _ => 0,
            };
        }
        Operation::StripDiacritics(_)
        | Operation::ChangeExtension(_)
        | Operation::Trim(_)
        | Operation::Insert(_)
        | Operation::ZeroPad(_)
        | Operation::Truncate(_)
        | Operation::InsertDate(_) => {}
    }

    // Set text cursors to end of each pre-filled value
    app.popup_text_cursors = app
        .popup_fields
        .iter()
        .map(|f| crate::app::char_len(f))
        .collect();
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
