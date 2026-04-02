use std::fs;
use renx::app::App;
use renx::files;
use renx::operations::*;

#[test]
fn end_to_end_rename() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("IMG_001.jpg"), "").unwrap();
    fs::write(dir.path().join("IMG_002.jpg"), "").unwrap();
    fs::write(dir.path().join("IMG_003.jpg"), "").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.toggle_select_all();

    // Add operations: replace IMG_ with beach_, then lowercase
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "IMG_".to_string(),
        replacement: "beach_".to_string(),
    }));
    app.operations.push(Operation::CaseChange(CaseChange {
        mode: CaseMode::Lower,
    }));
    app.update_preview();

    // Verify preview
    assert_eq!(app.preview.len(), 3);
    assert_eq!(app.preview[0], ("IMG_001.jpg".to_string(), "beach_001.jpg".to_string()));
    assert_eq!(app.preview[1], ("IMG_002.jpg".to_string(), "beach_002.jpg".to_string()));
    assert_eq!(app.preview[2], ("IMG_003.jpg".to_string(), "beach_003.jpg".to_string()));

    // Execute renames
    assert!(!app.has_errors());
    for (old_name, new_name) in &app.preview {
        let entry = app.files.iter().find(|f| f.file_name == *old_name).unwrap();
        files::rename_file(entry, new_name).unwrap();
    }

    // Verify files on disk
    assert!(dir.path().join("beach_001.jpg").exists());
    assert!(dir.path().join("beach_002.jpg").exists());
    assert!(dir.path().join("beach_003.jpg").exists());
    assert!(!dir.path().join("IMG_001.jpg").exists());
}

#[test]
fn conflict_detection_blocks_execution() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.toggle_select_all();

    // Both files would rename to the same thing
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "x".to_string(),
    }));
    app.operations.push(Operation::FindReplace(FindReplace {
        search: "b".to_string(),
        replacement: "x".to_string(),
    }));
    app.update_preview();

    // Both become "x.txt" — should detect conflict
    assert!(app.has_errors());
}
