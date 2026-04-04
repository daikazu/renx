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
    assert_eq!(app.preview[0].1, "beach_001.jpg".to_string());
    assert_eq!(app.preview[1].1, "beach_002.jpg".to_string());
    assert_eq!(app.preview[2].1, "beach_003.jpg".to_string());

    // Execute renames
    assert!(!app.has_errors());
    for (file_idx, new_name) in &app.preview {
        let entry = &app.files[*file_idx];
        files::rename_file(entry, new_name).unwrap();
    }

    // Verify files on disk
    assert!(dir.path().join("beach_001.jpg").exists());
    assert!(dir.path().join("beach_002.jpg").exists());
    assert!(dir.path().join("beach_003.jpg").exists());
    assert!(!dir.path().join("IMG_001.jpg").exists());
}

#[test]
fn recursive_mode_renames_correct_file_by_identity() {
    let dir = tempfile::tempdir().unwrap();
    let sub_a = dir.path().join("a");
    let sub_b = dir.path().join("b");
    fs::create_dir(&sub_a).unwrap();
    fs::create_dir(&sub_b).unwrap();
    fs::write(sub_a.join("report.txt"), "content_a").unwrap();
    fs::write(sub_b.join("report.txt"), "content_b").unwrap();

    let mut file_list = vec![
        files::FileEntry {
            file_name: "report.txt".to_string(),
            parent_dir: sub_a.clone(),
        },
        files::FileEntry {
            file_name: "report.txt".to_string(),
            parent_dir: sub_b.clone(),
        },
    ];
    file_list.sort_by(|a, b| a.full_path().cmp(&b.full_path()));

    let mut app = App::new(file_list);
    app.selected[0] = true;

    app.operations.push(Operation::AddPrefix(AddPrefix {
        prefix: "new_".to_string(),
    }));
    app.update_preview();

    assert_eq!(app.preview.len(), 1);
    assert_eq!(app.preview[0].0, 0);
    assert_eq!(app.preview[0].1, "new_report.txt".to_string());
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

#[test]
fn conflict_detected_when_target_exists_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a_content").unwrap();
    fs::write(dir.path().join("b.txt"), "b_content").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);

    // Select only a.txt, rename it to b.txt (which exists on disk but isn't selected)
    let a_idx = app.files.iter().position(|f| f.file_name == "a.txt").unwrap();
    app.selected[a_idx] = true;

    app.operations.push(Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "b".to_string(),
    }));
    app.update_preview();

    // Should detect conflict: b.txt already exists on disk
    assert!(app.has_errors());
}

#[test]
fn filter_handles_unicode_filenames() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("café_photo.jpg"), "").unwrap();
    fs::write(dir.path().join("café_video.mp4"), "").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.filter_text = "café".to_string();
    app.update_filter();
    assert_eq!(app.filtered_indices.len(), 2);
}
