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
            search: "IMG_".to_string(),
            replacement: "beach_".to_string(),
        }),
        Operation::CaseChange(CaseChange {
            mode: CaseMode::Lower,
        }),
        Operation::SequentialNumber(SequentialNumber {
            format_width: 3,
            start: 1,
            position: NumberPosition::Prefix,
        }),
    ];
    let pipeline = Pipeline::new(ops);
    let filenames = vec![
        "IMG_0422.JPG".to_string(),
        "IMG_0423.JPG".to_string(),
    ];
    let results = pipeline.apply(&filenames);
    assert_eq!(results, vec!["001_beach_0422.jpg", "002_beach_0423.jpg"]);
}

#[test]
fn pipeline_preserves_order() {
    let ops = vec![
        Operation::FindReplace(FindReplace {
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
