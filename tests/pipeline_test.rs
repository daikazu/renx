use renx::operations::*;
use renx::pipeline::Pipeline;

#[test]
fn empty_pipeline_returns_original() {
    let pipeline = Pipeline::new(vec![]);
    let results = pipeline.apply(&["photo.jpg".to_string()]);
    assert_eq!(results, vec!["photo.jpg"]);
}

#[test]
fn single_operation() {
    let ops = vec![Operation::FindReplace(FindReplace {
        scope: Scope::Both,
        search: "IMG_".to_string(),
        replacement: "beach_".to_string(),
    })];
    let pipeline = Pipeline::new(ops);
    let results = pipeline.apply(&["IMG_001.jpg".to_string(), "IMG_002.jpg".to_string()]);
    assert_eq!(results, vec!["beach_001.jpg", "beach_002.jpg"]);
}

#[test]
fn stacked_operations() {
    let ops = vec![
        Operation::FindReplace(FindReplace {
            scope: Scope::Both,
            search: "IMG_".to_string(),
            replacement: "beach_".to_string(),
        }),
        Operation::CaseChange(CaseChange {
            scope: Scope::Both,
            mode: CaseMode::Lower,
        }),
        Operation::SequentialNumber(SequentialNumber {
            format_width: 3,
            start: 1,
            position: NumberPosition::Prefix,
        }),
    ];
    let pipeline = Pipeline::new(ops);
    let filenames = vec!["IMG_0422.JPG".to_string(), "IMG_0423.JPG".to_string()];
    let results = pipeline.apply(&filenames);
    assert_eq!(results, vec!["001_beach_0422.jpg", "002_beach_0423.jpg"]);
}

#[test]
fn pipeline_preserves_order() {
    let ops = vec![
        Operation::FindReplace(FindReplace {
            scope: Scope::Both,
            search: "old".to_string(),
            replacement: "new".to_string(),
        }),
        Operation::AddPrefix(AddPrefix {
            prefix: "v2_".to_string(),
        }),
    ];
    let pipeline = Pipeline::new(ops);
    let results = pipeline.apply(&["old_file.txt".to_string()]);
    assert_eq!(results, vec!["v2_new_file.txt"]);
}

#[test]
fn insert_date_uses_file_modified_time() {
    use renx::files::FileEntry;
    use renx::operations::{DateSource, InsertDate, NumberPosition};
    use std::fs;
    use std::time::{Duration, UNIX_EPOCH};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    fs::write(&path, "x").unwrap();
    // 2024-03-15 12:00:00 UTC: the same calendar day in every time zone
    let file = fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(UNIX_EPOCH + Duration::from_secs(1_710_504_000))
        .unwrap();
    drop(file);

    let entry = FileEntry {
        file_name: "photo.jpg".to_string(),
        parent_dir: dir.path().to_path_buf(),
    };

    let prefix = Pipeline::new(vec![Operation::InsertDate(InsertDate {
        source: DateSource::Modified,
        format: "%Y-%m-%d".to_string(),
        position: NumberPosition::Prefix,
    })]);
    assert_eq!(
        prefix.apply_entries(std::slice::from_ref(&entry)),
        vec!["2024-03-15_photo.jpg"]
    );

    let suffix = Pipeline::new(vec![Operation::InsertDate(InsertDate {
        source: DateSource::Modified,
        format: "%Y%m%d".to_string(),
        position: NumberPosition::Suffix,
    })]);
    assert_eq!(
        suffix.apply_entries(std::slice::from_ref(&entry)),
        vec!["photo_20240315.jpg"]
    );

    let replace = Pipeline::new(vec![Operation::InsertDate(InsertDate {
        source: DateSource::Modified,
        format: "%Y".to_string(),
        position: NumberPosition::Replace,
    })]);
    assert_eq!(replace.apply_entries(&[entry]), vec!["2024.jpg"]);
}

#[test]
fn insert_date_without_file_leaves_name_unchanged() {
    use renx::operations::{DateSource, InsertDate, NumberPosition};

    let op = Operation::InsertDate(InsertDate {
        source: DateSource::Created,
        format: "%Y-%m-%d".to_string(),
        position: NumberPosition::Prefix,
    });
    // Plain apply has no file to read metadata from
    assert_eq!(op.apply("photo.jpg"), "photo.jpg");
}
