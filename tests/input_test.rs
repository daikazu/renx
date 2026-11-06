use renx::app::{char_len, char_to_byte_index};

#[test]
fn char_to_byte_index_ascii() {
    assert_eq!(char_to_byte_index("hello", 0), 0);
    assert_eq!(char_to_byte_index("hello", 3), 3);
    assert_eq!(char_to_byte_index("hello", 5), 5);
    assert_eq!(char_to_byte_index("hello", 100), 5); // beyond end
}

#[test]
fn char_to_byte_index_multibyte() {
    let s = "café";
    assert_eq!(char_len(s), 4);
    assert_eq!(char_to_byte_index(s, 0), 0); // 'c'
    assert_eq!(char_to_byte_index(s, 1), 1); // 'a'
    assert_eq!(char_to_byte_index(s, 2), 2); // 'f'
    assert_eq!(char_to_byte_index(s, 3), 3); // 'é' starts at byte 3
    assert_eq!(char_to_byte_index(s, 4), 5); // end (é is 2 bytes)
}

#[test]
fn char_to_byte_index_emoji() {
    let s = "a😀b";
    assert_eq!(char_len(s), 3);
    assert_eq!(char_to_byte_index(s, 0), 0); // 'a'
    assert_eq!(char_to_byte_index(s, 1), 1); // '😀'
    assert_eq!(char_to_byte_index(s, 2), 5); // 'b' (😀 is 4 bytes)
    assert_eq!(char_to_byte_index(s, 3), 6); // end
}

#[test]
fn popup_multibyte_insert_and_delete() {
    // Simulate the operations that would happen in handle_param_editor
    let mut field = String::new();
    let mut cursor: usize = 0;

    // Type "café"
    for ch in "café".chars() {
        let byte_pos = char_to_byte_index(&field, cursor);
        field.insert(byte_pos, ch);
        cursor += 1;
    }
    assert_eq!(field, "café");
    assert_eq!(cursor, 4);

    // Backspace (delete 'é')
    let byte_start = char_to_byte_index(&field, cursor - 1);
    let byte_end = char_to_byte_index(&field, cursor);
    field.replace_range(byte_start..byte_end, "");
    cursor -= 1;
    assert_eq!(field, "caf");
    assert_eq!(cursor, 3);

    // Move cursor to position 1 (after 'c'), insert 'ü'
    cursor = 1;
    let byte_pos = char_to_byte_index(&field, cursor);
    field.insert(byte_pos, 'ü');
    assert_eq!(field, "cüaf");
}

#[test]
fn invalid_regex_is_rejected_with_message() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use renx::app::{App, InputMode};
    use renx::input::handle_key;

    let mut app = App::new(vec![]);
    app.input_mode = InputMode::EditingOperationParams;
    app.popup_type_cursor = 1; // Regex Replace
    app.popup_fields = vec!["[0-9".to_string(), "x".to_string()];
    app.popup_field_options = vec![None, None];
    app.popup_field_selections = vec![0, 0];
    app.popup_text_cursors = vec![4, 1];

    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.operations.is_empty(), "invalid regex must not be added");
    assert_eq!(app.input_mode, InputMode::EditingOperationParams);
    assert!(
        app.status_message.contains("Invalid regex"),
        "status: {}",
        app.status_message
    );
}
