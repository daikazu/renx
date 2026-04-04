use renx::app::{char_to_byte_index, char_len};

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
    cursor += 1;
    assert_eq!(field, "cüaf");
}
