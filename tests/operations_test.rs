use renx::operations::{
    AddPrefix, AddSuffix, CaseChange, CaseMode, ChangeExtension, FindReplace, Insert,
    NumberPosition, Operation, RegexReplace, RemovePattern, Scope, SequentialNumber, Side, Slugify,
    StripDiacritics, Trim, Truncate, ZeroPad,
};

#[test]
fn find_replace_basic() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        scope: Scope::Both,
        search: "IMG_".to_string(),
        replacement: "vacation_".to_string(),
    });
    assert_eq!(op.apply("IMG_0001.jpg"), "vacation_0001.jpg");
}

#[test]
fn find_replace_multiple_occurrences() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        scope: Scope::Both,
        search: "a".to_string(),
        replacement: "b".to_string(),
    });
    assert_eq!(op.apply("banana.txt"), "bbnbnb.txt");
}

#[test]
fn find_replace_no_match() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        scope: Scope::Both,
        search: "xyz".to_string(),
        replacement: "abc".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn remove_pattern_basic() {
    let op = Operation::RemovePattern(RemovePattern {
        ignore_case: false,
        scope: Scope::Both,
        pattern: "copy of ".to_string(),
    });
    assert_eq!(op.apply("copy of document.txt"), "document.txt");
}

#[test]
fn remove_pattern_multiple() {
    let op = Operation::RemovePattern(RemovePattern {
        ignore_case: false,
        scope: Scope::Both,
        pattern: " ".to_string(),
    });
    assert_eq!(op.apply("my cool file.txt"), "mycoolfile.txt");
}

#[test]
fn regex_replace_capture_groups() {
    let op = Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: r"(\d+)_(\w+)".to_string(),
        replacement: "${2}_${1}".to_string(),
    });
    assert_eq!(op.apply("001_photo.jpg"), "photo_001.jpg");
}

#[test]
fn regex_replace_simple_pattern() {
    let op = Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("file_123_v2.txt"), "file_NUM_vNUM.txt");
}

#[test]
fn regex_replace_no_match() {
    let op = Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: r"\d+".to_string(),
        replacement: "NUM".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn regex_replace_invalid_pattern_returns_original() {
    let op = Operation::RegexReplace(RegexReplace {
        scope: Scope::Both,
        pattern: r"[invalid".to_string(),
        replacement: "x".to_string(),
    });
    assert_eq!(op.apply("file.txt"), "file.txt");
}

#[test]
fn case_change_lowercase() {
    let op = Operation::CaseChange(CaseChange {
        scope: Scope::Both,
        mode: CaseMode::Lower,
    });
    assert_eq!(op.apply("MY_FILE.TXT"), "my_file.txt");
}

#[test]
fn case_change_uppercase() {
    let op = Operation::CaseChange(CaseChange {
        scope: Scope::Both,
        mode: CaseMode::Upper,
    });
    assert_eq!(op.apply("my_file.txt"), "MY_FILE.TXT");
}

#[test]
fn case_change_title() {
    let op = Operation::CaseChange(CaseChange {
        scope: Scope::Both,
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
        step: 1,
        separator: "_".to_string(),
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
        step: 1,
        separator: "_".to_string(),
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
        step: 1,
        separator: "_".to_string(),
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
        scope: Scope::Both,
        mode: CaseMode::Title,
    });
    assert_eq!(op.apply("hello world.txt"), "Hello World.txt");
    assert_eq!(op.apply("my photo.JPG"), "My Photo.jpg");
}

#[test]
fn slugify_preserves_non_latin_alphanumeric() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("日本 photo.jpg"), "日本-photo.jpg");
}

#[test]
fn find_replace_empty_search_is_noop() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        scope: Scope::Both,
        search: "".to_string(),
        replacement: "x".to_string(),
    });
    assert_eq!(op.apply("hello.txt"), "hello.txt");
}

#[test]
fn sequential_suffix_dotfile() {
    let op = Operation::SequentialNumber(SequentialNumber {
        step: 1,
        separator: "_".to_string(),
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

// --- Scope -------------------------------------------------------------

#[test]
fn find_replace_scope_name_leaves_extension_alone() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        search: "e".to_string(),
        replacement: "x".to_string(),
        scope: Scope::Name,
    });
    assert_eq!(op.apply("file.jpeg"), "filx.jpeg");
}

#[test]
fn find_replace_scope_ext_changes_only_extension() {
    let op = Operation::FindReplace(FindReplace {
        ignore_case: false,
        search: "jpeg".to_string(),
        replacement: "jpg".to_string(),
        scope: Scope::Ext,
    });
    assert_eq!(op.apply("jpeg.jpeg"), "jpeg.jpg");
    assert_eq!(op.apply("noext"), "noext");
}

#[test]
fn regex_replace_scope_name() {
    let op = Operation::RegexReplace(RegexReplace {
        pattern: r"\d+".to_string(),
        replacement: "N".to_string(),
        scope: Scope::Name,
    });
    assert_eq!(op.apply("img123.mp4"), "imgN.mp4");
}

#[test]
fn remove_pattern_scope_ext() {
    let op = Operation::RemovePattern(RemovePattern {
        ignore_case: false,
        pattern: "bak".to_string(),
        scope: Scope::Ext,
    });
    assert_eq!(op.apply("notes.bak.txtbak"), "notes.bak.txt");
}

#[test]
fn case_change_lower_scope_name_keeps_extension_case() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Lower,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("Photo.JPG"), "photo.JPG");
}

#[test]
fn case_change_upper_scope_ext() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Upper,
        scope: Scope::Ext,
    });
    assert_eq!(op.apply("Photo.jpg"), "Photo.JPG");
}

// --- New case modes ----------------------------------------------------

#[test]
fn case_change_snake() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Snake,
        scope: Scope::Both,
    });
    assert_eq!(op.apply("My File-Name v2.TXT"), "my_file_name_v2.txt");
    assert_eq!(op.apply("myFileName.txt"), "my_file_name.txt");
}

#[test]
fn case_change_kebab() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Kebab,
        scope: Scope::Both,
    });
    assert_eq!(op.apply("My File_Name v2.TXT"), "my-file-name-v2.txt");
}

#[test]
fn case_change_camel() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Camel,
        scope: Scope::Both,
    });
    assert_eq!(op.apply("my file-name v2.txt"), "myFileNameV2.txt");
}

#[test]
fn case_change_sentence() {
    let op = Operation::CaseChange(CaseChange {
        mode: CaseMode::Sentence,
        scope: Scope::Both,
    });
    assert_eq!(op.apply("hello_WORLD again.TXT"), "Hello_world again.txt");
}

// --- Strip diacritics / slugify transliteration -------------------------

#[test]
fn strip_diacritics_removes_accents() {
    let op = Operation::StripDiacritics(StripDiacritics { scope: Scope::Both });
    assert_eq!(op.apply("Café Ångström.TXT"), "Cafe Angstrom.TXT");
}

#[test]
fn strip_diacritics_scope_name() {
    let op = Operation::StripDiacritics(StripDiacritics { scope: Scope::Name });
    assert_eq!(op.apply("résumé.tèxt"), "resume.tèxt");
}

#[test]
fn slugify_transliterates_accents() {
    let op = Operation::Slugify(Slugify {
        separator: "-".to_string(),
    });
    assert_eq!(op.apply("Café Résumé.PDF"), "cafe-resume.pdf");
}

// --- Change Extension ---------------------------------------------------

#[test]
fn change_extension_replaces_existing() {
    let op = Operation::ChangeExtension(ChangeExtension {
        extension: "jpg".to_string(),
    });
    assert_eq!(op.apply("photo.jpeg"), "photo.jpg");
}

#[test]
fn change_extension_accepts_leading_dot() {
    let op = Operation::ChangeExtension(ChangeExtension {
        extension: ".png".to_string(),
    });
    assert_eq!(op.apply("photo.jpeg"), "photo.png");
}

#[test]
fn change_extension_adds_when_missing() {
    let op = Operation::ChangeExtension(ChangeExtension {
        extension: "txt".to_string(),
    });
    assert_eq!(op.apply("README"), "README.txt");
    assert_eq!(op.apply(".bashrc"), ".bashrc.txt");
}

#[test]
fn change_extension_empty_strips() {
    let op = Operation::ChangeExtension(ChangeExtension {
        extension: String::new(),
    });
    assert_eq!(op.apply("archive.tar.gz"), "archive.tar");
    assert_eq!(op.apply("noext"), "noext");
}

// --- Trim ---------------------------------------------------------------

#[test]
fn trim_from_start_of_name() {
    let op = Operation::Trim(Trim {
        count: 4,
        from: Side::Start,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("IMG_0001.jpg"), "0001.jpg");
}

#[test]
fn trim_from_end_of_name() {
    let op = Operation::Trim(Trim {
        count: 5,
        from: Side::End,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("report_copy.pdf"), "report.pdf");
}

#[test]
fn trim_more_than_length_gives_empty_stem() {
    let op = Operation::Trim(Trim {
        count: 99,
        from: Side::Start,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("abc.txt"), ".txt");
}

#[test]
fn trim_counts_characters_not_bytes() {
    let op = Operation::Trim(Trim {
        count: 2,
        from: Side::Start,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("éàbc.txt"), "bc.txt");
}

// --- Insert -------------------------------------------------------------

#[test]
fn insert_at_position_from_start() {
    let op = Operation::Insert(Insert {
        text: "-".to_string(),
        position: 4,
        from: Side::Start,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("20240315_photo.jpg"), "2024-0315_photo.jpg");
}

#[test]
fn insert_at_position_from_end() {
    let op = Operation::Insert(Insert {
        text: "_v2".to_string(),
        position: 0,
        from: Side::End,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("report.pdf"), "report_v2.pdf");
}

#[test]
fn insert_beyond_length_appends() {
    let op = Operation::Insert(Insert {
        text: "X".to_string(),
        position: 50,
        from: Side::Start,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("abc.txt"), "abcX.txt");
}

// --- Zero-pad -----------------------------------------------------------

#[test]
fn zero_pad_pads_every_number_run() {
    let op = Operation::ZeroPad(ZeroPad {
        width: 2,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("track 3 of 12.mp3"), "track 03 of 12.mp3");
}

#[test]
fn zero_pad_leaves_longer_numbers_alone() {
    let op = Operation::ZeroPad(ZeroPad {
        width: 3,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("ep7-part1234.mkv"), "ep007-part1234.mkv");
}

#[test]
fn zero_pad_scope_both_touches_extension_digits() {
    let op = Operation::ZeroPad(ZeroPad {
        width: 2,
        scope: Scope::Both,
    });
    assert_eq!(op.apply("a1.r1"), "a01.r01");
}

// --- Truncate -----------------------------------------------------------

#[test]
fn truncate_limits_name_length() {
    let op = Operation::Truncate(Truncate {
        max_len: 6,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("a_very_long_name.txt"), "a_very.txt");
}

#[test]
fn truncate_shorter_name_unchanged() {
    let op = Operation::Truncate(Truncate {
        max_len: 20,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("short.txt"), "short.txt");
}

#[test]
fn truncate_counts_characters_not_bytes() {
    let op = Operation::Truncate(Truncate {
        max_len: 2,
        scope: Scope::Name,
    });
    assert_eq!(op.apply("日本語.txt"), "日本.txt");
}

// --- Ignore case ----------------------------------------------------------

#[test]
fn find_replace_ignore_case_matches_any_casing() {
    let op = Operation::FindReplace(FindReplace {
        search: "img".to_string(),
        replacement: "photo".to_string(),
        scope: Scope::Both,
        ignore_case: true,
    });
    assert_eq!(op.apply("IMG_001.jpg"), "photo_001.jpg");
    assert_eq!(op.apply("Img-Img.txt"), "photo-photo.txt");
}

#[test]
fn find_replace_case_sensitive_by_default_flag() {
    let op = Operation::FindReplace(FindReplace {
        search: "img".to_string(),
        replacement: "photo".to_string(),
        scope: Scope::Both,
        ignore_case: false,
    });
    assert_eq!(op.apply("IMG_001.jpg"), "IMG_001.jpg");
}

#[test]
fn find_replace_ignore_case_treats_replacement_literally() {
    let op = Operation::FindReplace(FindReplace {
        search: "a.b".to_string(),
        replacement: "$1".to_string(),
        scope: Scope::Both,
        ignore_case: true,
    });
    assert_eq!(op.apply("xA.By.txt"), "x$1y.txt");
}

#[test]
fn remove_pattern_ignore_case() {
    let op = Operation::RemovePattern(RemovePattern {
        pattern: "copy of ".to_string(),
        scope: Scope::Both,
        ignore_case: true,
    });
    assert_eq!(op.apply("Copy Of report.pdf"), "report.pdf");
}

// --- Sequential step and separator ----------------------------------------

#[test]
fn sequential_number_with_step() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 3,
        start: 10,
        step: 5,
        separator: "_".to_string(),
        position: NumberPosition::Prefix,
    });
    assert_eq!(op.apply_with_index("a.txt", 0), "010_a.txt");
    assert_eq!(op.apply_with_index("a.txt", 2), "020_a.txt");
}

#[test]
fn sequential_number_custom_separator() {
    let prefix = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 1,
        step: 1,
        separator: "-".to_string(),
        position: NumberPosition::Prefix,
    });
    assert_eq!(prefix.apply_with_index("a.txt", 0), "01-a.txt");

    let suffix = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 1,
        step: 1,
        separator: " ".to_string(),
        position: NumberPosition::Suffix,
    });
    assert_eq!(suffix.apply_with_index("a.txt", 0), "a 01.txt");

    let none = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 1,
        step: 1,
        separator: String::new(),
        position: NumberPosition::Prefix,
    });
    assert_eq!(none.apply_with_index("a.txt", 0), "01a.txt");
}

#[test]
fn sequential_number_replace_ignores_separator() {
    let op = Operation::SequentialNumber(SequentialNumber {
        format_width: 2,
        start: 1,
        step: 1,
        separator: "-".to_string(),
        position: NumberPosition::Replace,
    });
    assert_eq!(op.apply_with_index("a.txt", 0), "01.txt");
}
