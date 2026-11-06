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

mod editor_wiring {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use renx::app::{App, InputMode};
    use renx::input::{handle_key, start_editing_operation};
    use renx::operations::*;

    fn key(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            key(app, KeyCode::Char(c));
        }
    }

    fn every_operation() -> Vec<Operation> {
        vec![
            Operation::FindReplace(FindReplace {
                search: "a".into(),
                replacement: "b".into(),
                scope: Scope::Name,
                ignore_case: true,
            }),
            Operation::RegexReplace(RegexReplace {
                pattern: r"\d+".into(),
                replacement: "N".into(),
                scope: Scope::Ext,
            }),
            Operation::RemovePattern(RemovePattern {
                pattern: "copy".into(),
                scope: Scope::Name,
                ignore_case: true,
            }),
            Operation::CaseChange(CaseChange {
                mode: CaseMode::Kebab,
                scope: Scope::Name,
            }),
            Operation::AddPrefix(AddPrefix {
                prefix: "pre_".into(),
            }),
            Operation::AddSuffix(AddSuffix {
                suffix: "_suf".into(),
            }),
            Operation::Insert(Insert {
                text: "-".into(),
                position: 4,
                from: Side::End,
                scope: Scope::Both,
            }),
            Operation::Trim(Trim {
                count: 3,
                from: Side::End,
                scope: Scope::Ext,
            }),
            Operation::Truncate(Truncate {
                max_len: 12,
                scope: Scope::Both,
            }),
            Operation::SequentialNumber(SequentialNumber {
                format_width: 4,
                start: 7,
                step: 3,
                separator: "-".into(),
                position: NumberPosition::Suffix,
            }),
            Operation::ZeroPad(ZeroPad {
                width: 5,
                scope: Scope::Ext,
            }),
            Operation::InsertDate(InsertDate {
                source: DateSource::Created,
                format: "%Y%m%d".into(),
                position: NumberPosition::Suffix,
            }),
            Operation::ChangeExtension(ChangeExtension {
                extension: "png".into(),
            }),
            Operation::Slugify(Slugify {
                separator: "_".into(),
            }),
            Operation::StripDiacritics(StripDiacritics { scope: Scope::Ext }),
        ]
    }

    #[test]
    fn picker_reaches_every_operation_type() {
        let mut app = App::new(vec![]);
        app.input_mode = InputMode::SelectingOperationType;
        app.popup_type_cursor = 0;
        for _ in 0..100 {
            key(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(app.popup_type_cursor, OP_TYPES.len() - 1);
        assert_eq!(OP_TYPES.len(), every_operation().len());

        for (i, spec) in OP_TYPES.iter().enumerate() {
            app.input_mode = InputMode::SelectingOperationType;
            app.popup_type_cursor = i;
            key(&mut app, KeyCode::Enter);
            assert_eq!(
                app.input_mode,
                InputMode::EditingOperationParams,
                "{}",
                spec.name
            );
            assert_eq!(app.popup_fields.len(), spec.fields.len(), "{}", spec.name);
            key(&mut app, KeyCode::Esc);
        }
    }

    #[test]
    fn every_operation_survives_an_edit_round_trip() {
        for op in every_operation() {
            let expected = op.description();
            let mut app = App::new(vec![]);
            app.operations.push(op);
            app.op_cursor = 0;

            start_editing_operation(&mut app);
            assert_eq!(
                app.input_mode,
                InputMode::EditingOperationParams,
                "{}",
                expected
            );
            key(&mut app, KeyCode::Enter);

            assert_eq!(app.operations.len(), 1, "{}", expected);
            assert_eq!(app.operations[0].description(), expected);
        }
    }

    #[test]
    fn scope_is_selectable_from_the_popup() {
        let mut app = App::new(vec![]);
        app.input_mode = InputMode::SelectingOperationType;
        app.popup_type_cursor = 0; // Find & Replace
        key(&mut app, KeyCode::Enter);

        type_text(&mut app, "a");
        key(&mut app, KeyCode::Tab);
        type_text(&mut app, "b");
        key(&mut app, KeyCode::Tab); // Ignore case
        key(&mut app, KeyCode::Tab); // Scope
                                     // Scope selector: default is "both"; walk left to "name"
        for _ in 0..5 {
            key(&mut app, KeyCode::Left);
        }
        key(&mut app, KeyCode::Enter);

        assert_eq!(app.operations.len(), 1);
        match &app.operations[0] {
            Operation::FindReplace(op) => {
                assert_eq!(op.search, "a");
                assert_eq!(op.replacement, "b");
                assert_eq!(op.scope, Scope::Name);
            }
            other => panic!("unexpected op {:?}", other),
        }
    }

    #[test]
    fn new_operation_popups_prefill_sensible_defaults() {
        let mut app = App::new(vec![]);
        let seq = OP_TYPES
            .iter()
            .position(|t| t.name == "Sequential Number")
            .unwrap();
        app.input_mode = InputMode::SelectingOperationType;
        app.popup_type_cursor = seq;
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.popup_fields[0], "3");
        assert_eq!(app.popup_fields[1], "1");
        key(&mut app, KeyCode::Esc);

        let date = OP_TYPES
            .iter()
            .position(|t| t.name == "Insert Date")
            .unwrap();
        app.input_mode = InputMode::SelectingOperationType;
        app.popup_type_cursor = date;
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.popup_fields[1], "%Y-%m-%d");
    }

    #[test]
    fn invalid_date_format_is_rejected_with_message() {
        let mut app = App::new(vec![]);
        let date = OP_TYPES
            .iter()
            .position(|t| t.name == "Insert Date")
            .unwrap();
        app.input_mode = InputMode::SelectingOperationType;
        app.popup_type_cursor = date;
        key(&mut app, KeyCode::Enter);
        key(&mut app, KeyCode::Tab); // to format field
        for _ in 0..10 {
            key(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "%Q");
        key(&mut app, KeyCode::Enter);

        assert!(app.operations.is_empty());
        assert_eq!(app.input_mode, InputMode::EditingOperationParams);
        assert!(
            app.status_message.contains("Invalid date format"),
            "status: {}",
            app.status_message
        );
    }
}
