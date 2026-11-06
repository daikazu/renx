use renx::app::App;
use renx::files;
use renx::operations::*;
use std::fs;

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
        scope: Scope::Both,
        search: "IMG_".to_string(),
        replacement: "beach_".to_string(),
    }));
    app.operations.push(Operation::CaseChange(CaseChange {
        scope: Scope::Both,
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
        scope: Scope::Both,
        search: "a".to_string(),
        replacement: "x".to_string(),
    }));
    app.operations.push(Operation::FindReplace(FindReplace {
        scope: Scope::Both,
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
    let a_idx = app
        .files
        .iter()
        .position(|f| f.file_name == "a.txt")
        .unwrap();
    app.selected[a_idx] = true;

    app.operations.push(Operation::FindReplace(FindReplace {
        scope: Scope::Both,
        search: "a".to_string(),
        replacement: "b".to_string(),
    }));
    app.update_preview();

    // Should detect conflict: b.txt already exists on disk
    assert!(app.has_errors());
}

#[test]
fn two_phase_rename_handles_swap() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alpha.txt"), "content_alpha").unwrap();
    fs::write(dir.path().join("beta.txt"), "content_beta").unwrap();

    let mut file_list = files::discover_dir(dir.path()).unwrap();
    file_list.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    let mut app = App::new(file_list);
    app.toggle_select_all();

    // Swap alpha<->beta via operations
    app.operations.push(Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: "^alpha".to_string(),
        replacement: "SWAP".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: "^beta".to_string(),
        replacement: "alpha".to_string(),
    }));
    app.operations.push(Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: "^SWAP".to_string(),
        replacement: "beta".to_string(),
    }));
    app.update_preview();

    assert!(!app.has_errors());

    // Simulate two-phase rename manually (same algorithm as execute_renames)
    let renames: Vec<(usize, String)> = app
        .preview
        .iter()
        .filter(|(idx, new)| app.files[*idx].file_name != *new)
        .cloned()
        .collect();

    // Phase 1: temp names
    let mut temps: Vec<(files::FileEntry, String, String)> = Vec::new();
    for (file_idx, new_name) in &renames {
        let entry = &app.files[*file_idx];
        let temp_name = format!(".renx_tmp_{}", entry.file_name);
        files::rename_file(entry, &temp_name).unwrap();
        temps.push((
            files::FileEntry {
                file_name: temp_name,
                parent_dir: entry.parent_dir.clone(),
            },
            new_name.clone(),
            entry.file_name.clone(),
        ));
    }

    // Phase 2: final names
    for (temp_entry, final_name, _) in &temps {
        files::rename_file(temp_entry, final_name).unwrap();
    }

    // Verify swap happened correctly
    assert_eq!(
        fs::read_to_string(dir.path().join("alpha.txt")).unwrap(),
        "content_beta"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("beta.txt")).unwrap(),
        "content_alpha"
    );
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

#[test]
fn preview_scroll_clamped_when_preview_shrinks() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..20 {
        fs::write(dir.path().join(format!("file_{:02}.txt", i)), "").unwrap();
    }

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.toggle_select_all();
    app.operations.push(Operation::AddPrefix(AddPrefix {
        prefix: "new_".to_string(),
    }));
    app.update_preview();

    // Scroll to near the end
    app.preview_scroll = 19;

    // Now deselect all — preview becomes empty
    app.toggle_select_all();
    app.update_preview();

    // preview_scroll should be clamped to 0
    assert_eq!(app.preview_scroll, 0);
}

#[test]
fn same_new_name_different_directories_is_not_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let sub_a = dir.path().join("a");
    let sub_b = dir.path().join("b");
    fs::create_dir(&sub_a).unwrap();
    fs::create_dir(&sub_b).unwrap();
    fs::write(sub_a.join("old.txt"), "").unwrap();
    fs::write(sub_b.join("old.txt"), "").unwrap();

    let files = vec![
        files::FileEntry {
            file_name: "old.txt".to_string(),
            parent_dir: sub_a.clone(),
        },
        files::FileEntry {
            file_name: "old.txt".to_string(),
            parent_dir: sub_b.clone(),
        },
    ];

    let mut app = App::new(files);
    app.toggle_select_all();

    // Both rename to "new.txt" — should be fine since different directories
    app.operations.push(Operation::FindReplace(FindReplace {
        scope: Scope::Both,
        search: "old".to_string(),
        replacement: "new".to_string(),
    }));
    app.update_preview();

    assert!(!app.has_errors());
}

#[test]
fn case_only_rename_is_not_conflict() {
    // On case-insensitive filesystems (macOS APFS default), "Photo.JPG" and
    // "photo.jpg" resolve to the same file. Renaming a file to a different
    // casing of its own name must not be reported as an on-disk conflict.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Photo.JPG"), "content").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.selected[0] = true;

    app.operations.push(Operation::CaseChange(CaseChange {
        scope: Scope::Both,
        mode: CaseMode::Lower,
    }));
    app.update_preview();

    assert_eq!(app.preview[0].1, "photo.jpg");
    assert!(
        !app.has_errors(),
        "case-only rename should not be a conflict"
    );
}

#[test]
fn case_only_rename_executes() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use renx::input::handle_key;

    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Photo.JPG"), "content").unwrap();

    let file_list = files::discover_dir(dir.path()).unwrap();
    let mut app = App::new(file_list);
    app.selected[0] = true;
    app.operations.push(Operation::CaseChange(CaseChange {
        scope: Scope::Both,
        mode: CaseMode::Lower,
    }));
    app.update_preview();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT),
    );

    assert!(app.executed, "status: {}", app.status_message);
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["photo.jpg".to_string()]);
    assert!(
        app.status_message.contains("Renamed 1"),
        "status: {}",
        app.status_message
    );
}
