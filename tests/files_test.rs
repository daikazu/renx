use std::fs;
use renx::files;

#[test]
fn discover_files_in_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let files = files::discover_dir(dir.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.file_name == "a.txt"));
    assert!(files.iter().any(|f| f.file_name == "b.txt"));
}

#[test]
fn discover_with_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("photo.jpg"), "").unwrap();
    fs::write(dir.path().join("photo.png"), "").unwrap();
    fs::write(dir.path().join("notes.txt"), "").unwrap();

    let pattern = dir.path().join("*.jpg").to_string_lossy().to_string();
    let files = files::discover_glob(&pattern).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name, "photo.jpg");
}

#[test]
fn rename_file_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("old.txt");
    fs::write(&original, "content").unwrap();

    let entry = files::FileEntry {
        file_name: "old.txt".to_string(),
        parent_dir: dir.path().to_path_buf(),
    };

    let result = files::rename_file(&entry, "new.txt");
    assert!(result.is_ok());
    assert!(!original.exists());
    assert!(dir.path().join("new.txt").exists());
    assert_eq!(fs::read_to_string(dir.path().join("new.txt")).unwrap(), "content");
}
