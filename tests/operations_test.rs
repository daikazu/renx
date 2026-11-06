use renx::operations::{
    AddPrefix, AddSuffix, CaseChange, CaseMode, FindReplace, NumberPosition, Operation,
    RegexReplace, RemovePattern, SequentialNumber, Slugify,
};

#[test]
fn find_replace_basic() {
    let op = Operation::FindReplace(FindReplace {
        search: "IMG_".to_string(),
        replacement: "vacation_".to_string(),
    });
    assert_eq!(op.apply("IMG_0001.jpg"), "vacation_0001.jpg");
}

#[test]
fn find_replace_multiple_occurrences() {
    let op = Operation::FindReplace(FindReplace {
        search: "a".to_string(),
        replacement: "b".to_string(),
    });
    assert_eq!(op.apply("banana.txt"), "bbnbnb.txt");
}

#[test]
fn find_replace_no_match() {
    let op = Operation::FindReplace(FindReplace {
        search: "xyz".to_string(),
        replacement: "abc".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn remove_pattern_basic() {
    let op = Operation::RemovePattern(RemovePattern {
        pattern: "copy of ".to_string(),
    });
    assert_eq!(op.apply("copy of document.txt"), "document.txt");
}

#[test]
fn remove_pattern_multiple() {
    let op = Operation::RemovePattern(RemovePattern {
        pattern: " ".to_string(),
    });
    assert_eq!(op.apply("my cool file.txt"), "mycoolfile.txt");
}

#[test]
fn regex_replace_capture_groups() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"(\d+)_(\w+)".to_string(),
        replacement: "${2}_${1}".to_string(),
    });
    assert_eq!(op.apply("001_photo.jpg"), "photo_001.jpg");
}

#[test]
fn regex_replace_simple_pattern() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("file_123_v2.txt"), "file_NUM_vNUM.txt");
}

#[test]
fn regex_replace_no_match() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn regex_replace_invalid_pattern_returns_original() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"[invalid".to_string(),
        replacement: "x".to_string(),
    });
    assert_eq!(op.apply("file.txt"), "file.txt");
}

#[test]
fn case_change_lowercase() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Lower,
    });
    assert_eq!(op.apply("MY_FILE.TXT"), "my_file.txt");
}

#[test]
fn case_change_uppercase() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Upper,
    });
    assert_eq!(op.apply("my_file.txt"), "MY_FILE.TXT");
}

#[test]
fn case_change_title() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Title,
    });
    assert_eq!(op.apply("hello world.txt"), "Hello World.txt");
}

#[test]
fn add_prefix_basic() {
    let op = Operation::AddPrefix(AddPrefix {
        prefix: "2024_".to_string(),
    });
    assert_eq!(op.apply("vacation.jpg"), "2024_vacation.jpg");
}

#[test]
fn add_suffix_before_extension() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_final".to_string(),
    });
    assert_eq!(op.apply("photo.jpg"), "photo_final.jpg");
}

#[test]
fn add_suffix_no_extension() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_backup".to_string(),
    });
    assert_eq!(op.apply("Makefile"), "Makefile_backup");
}

#[test]
fn add_suffix_multiple_dots() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_v2".to_string(),
    });
    assert_eq!(op.apply("archive.tar.gz"), "archive.tar_v2.gz");
}

#[test]
fn sequential_prefix() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 3,
        start: 1,
        position: NumberPosition::Prefix,
    });
    assert_eq!(op.apply_with_index("photo.jpg", 0), "001_photo.jpg");
    assert_eq!(op.apply_with_index("photo.jpg", 4), "005_photo.jpg");
}

#[test]
fn sequential_suffix() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 0,
        position: NumberPosition::Suffix,
    });
    assert_eq!(op.apply_with_index("photo.jpg", 0), "photo_00.jpg");
    assert_eq!(op.apply_with_index("photo.jpg", 9), "photo_09.jpg");
}

#[test]
fn sequential_replace() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 4,
        start: 1,
        position: NumberPosition::Replace,
    });
    assert_eq!(op.apply_with_index("anything.jpg", 0), "0001.jpg");
    assert_eq!(op.apply_with_index("anything.jpg", 99), "0100.jpg");
}

#[test]
fn slugify_basic() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("My Cool Photo.JPG"), "my-cool-photo.jpg");
}

#[test]
fn slugify_special_chars() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("Hello (World) [2024].txt"), "hello-world-2024.txt");
}

#[test]
fn slugify_collapses_separators() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("too   many---spaces.png"), "too-many-spaces.png");
}

#[test]
fn slugify_custom_separator() {
    let op = Operation::Slugify(Slugify {
        separator: "_".to_string(),
    });
    assert_eq!(op.apply("My Cool Photo.JPG"), "my_cool_photo.jpg");
}

#[test]
fn slugify_no_extension() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("README"), "readme");
}

#[test]
fn add_suffix_dotfile() {
    let op = Operation::AddSuffix(AddSuffix {
        suffix: "_backup".to_string(),
    });
    assert_eq!(op.apply(".gitignore"), ".gitignore_backup");
}

#[test]
fn case_change_title_preserves_extension() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Title,
    });
    assert_eq!(op.apply("hello world.txt"), "Hello World.txt");
    assert_eq!(op.apply("my photo.JPG"), "My Photo.jpg");
}

#[test]
fn slugify_preserves_non_ascii_alphanumeric() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("café photo.jpg"), "café-photo.jpg");
}

#[test]
fn find_replace_empty_search_is_noop() {
    let op = Operation::FindReplace(FindReplace {
        search: "".to_string(),
        replacement: "x".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn sequential_suffix_dotfile() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 1,
        position: NumberPosition::Suffix,
    });
    assert_eq!(op.apply_with_index(".gitignore", 0), ".gitignore_01");
}

#[test]
fn slugify_keeps_leading_dot_of_hidden_files() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply(".bashrc"), ".bashrc");
    assert_eq!(op.apply(".env.local"), ".env.local");
    assert_eq!(op.apply(".My Config.JSON"), ".my-config.json");
}
