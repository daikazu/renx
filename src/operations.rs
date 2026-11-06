use regex::Regex;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Which part of the filename an operation acts on.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Scope {
    /// The whole filename, extension included.
    #[default]
    Both,
    /// Only the stem (everything before the last dot).
    Name,
    /// Only the extension (without its dot).
    Ext,
}

impl Scope {
    pub fn label(&self) -> &'static str {
        match self {
            Scope::Both => "both",
            Scope::Name => "name",
            Scope::Ext => "ext",
        }
    }

    pub fn from_label(label: &str) -> Scope {
        match label {
            "name" => Scope::Name,
            "ext" => Scope::Ext,
            _ => Scope::Both,
        }
    }

    fn suffix(&self) -> String {
        match self {
            Scope::Both => String::new(),
            other => format!(" [{}]", other.label()),
        }
    }
}

/// Split a filename into `(stem, extension)` where the extension keeps its
/// leading dot. Hidden files such as `.bashrc` have no extension.
pub fn split_ext(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(0) | None => (filename, ""),
        Some(dot_pos) => (&filename[..dot_pos], &filename[dot_pos..]),
    }
}

/// Apply `f` to the part of `filename` selected by `scope` and reassemble.
pub fn apply_scoped(filename: &str, scope: &Scope, f: impl Fn(&str) -> String) -> String {
    match scope {
        Scope::Both => f(filename),
        Scope::Name => {
            let (stem, ext) = split_ext(filename);
            format!("{}{}", f(stem), ext)
        }
        Scope::Ext => {
            let (stem, ext) = split_ext(filename);
            if ext.is_empty() {
                filename.to_string()
            } else {
                format!("{}.{}", stem, f(&ext[1..]))
            }
        }
    }
}

/// Remove accents and other combining marks: `café` becomes `cafe`.
pub fn strip_diacritics(s: &str) -> String {
    s.nfd().filter(|c| !is_combining_mark(*c)).nfc().collect()
}

/// Split into words on non-alphanumeric characters and lower-to-upper
/// transitions: `myFile-name v2` gives `["my", "File", "name", "v2"]`.
fn split_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if c.is_alphanumeric() {
            let camel_boundary = prev
                .map(|p| c.is_uppercase() && (p.is_lowercase() || p.is_ascii_digit()))
                .unwrap_or(false);
            if camel_boundary && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            current.push(c);
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        prev = Some(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Upper-case the first character and lower-case the rest.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// A single rename operation that transforms a filename string.
#[derive(Clone, Debug)]
pub enum Operation {
    FindReplace(FindReplace),
    RemovePattern(RemovePattern),
    RegexReplace(RegexReplace),
    CaseChange(CaseChange),
    AddPrefix(AddPrefix),
    AddSuffix(AddSuffix),
    SequentialNumber(SequentialNumber),
    Slugify(Slugify),
    StripDiacritics(StripDiacritics),
}

impl Operation {
    /// Apply this operation to a filename, returning the transformed name.
    pub fn apply(&self, filename: &str) -> String {
        match self {
            Operation::FindReplace(op) => op.apply(filename),
            Operation::RemovePattern(op) => op.apply(filename),
            Operation::RegexReplace(op) => op.apply(filename),
            Operation::CaseChange(op) => op.apply(filename),
            Operation::AddPrefix(op) => op.apply(filename),
            Operation::AddSuffix(op) => op.apply(filename),
            Operation::SequentialNumber(op) => op.apply(filename, 0),
            Operation::Slugify(op) => op.apply(filename),
            Operation::StripDiacritics(op) => op.apply(filename),
        }
    }

    /// Apply with a file index (used by SequentialNumber).
    /// Non-sequential operations ignore the index.
    pub fn apply_with_index(&self, filename: &str, index: usize) -> String {
        match self {
            Operation::SequentialNumber(op) => op.apply(filename, index),
            Operation::Slugify(op) => op.apply(filename),
            other => other.apply(filename),
        }
    }

    /// Human-readable description for the operation stack UI.
    pub fn description(&self) -> String {
        match self {
            Operation::FindReplace(op) => {
                format!(
                    "Replace \"{}\" → \"{}\"{}",
                    op.search,
                    op.replacement,
                    op.scope.suffix()
                )
            }
            Operation::RemovePattern(op) => {
                format!("Remove \"{}\"{}", op.pattern, op.scope.suffix())
            }
            Operation::RegexReplace(op) => {
                format!(
                    "Regex \"{}\" → \"{}\"{}",
                    op.pattern,
                    op.replacement,
                    op.scope.suffix()
                )
            }
            Operation::CaseChange(op) => {
                format!("{}{}", op.mode.label_pretty(), op.scope.suffix())
            }
            Operation::AddPrefix(op) => format!("Prefix \"{}\"", op.prefix),
            Operation::AddSuffix(op) => format!("Suffix \"{}\"", op.suffix),
            Operation::SequentialNumber(op) => {
                let pos = match op.position {
                    NumberPosition::Prefix => "prefix",
                    NumberPosition::Suffix => "suffix",
                    NumberPosition::Replace => "replace",
                };
                let hashes: String = "#".repeat(op.format_width as usize);
                format!("Sequential {} ({})", hashes, pos)
            }
            Operation::Slugify(op) => {
                let sep = if op.separator == "-" {
                    "hyphens"
                } else {
                    &format!("\"{}\"", op.separator)
                };
                format!("Slugify ({})", sep)
            }
            Operation::StripDiacritics(op) => format!("Strip accents{}", op.scope.suffix()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FindReplace {
    pub search: String,
    pub replacement: String,
    pub scope: Scope,
}

impl FindReplace {
    fn apply(&self, filename: &str) -> String {
        if self.search.is_empty() {
            return filename.to_string();
        }
        apply_scoped(filename, &self.scope, |s| {
            s.replace(&self.search, &self.replacement)
        })
    }
}

#[derive(Clone, Debug)]
pub struct RemovePattern {
    pub pattern: String,
    pub scope: Scope,
}

impl RemovePattern {
    fn apply(&self, filename: &str) -> String {
        if self.pattern.is_empty() {
            return filename.to_string();
        }
        apply_scoped(filename, &self.scope, |s| s.replace(&self.pattern, ""))
    }
}

#[derive(Clone, Debug)]
pub struct RegexReplace {
    pub pattern: String,
    pub replacement: String,
    pub scope: Scope,
}

impl RegexReplace {
    fn apply(&self, filename: &str) -> String {
        match Regex::new(&self.pattern) {
            Ok(re) => apply_scoped(filename, &self.scope, |s| {
                re.replace_all(s, self.replacement.as_str()).to_string()
            }),
            Err(_) => filename.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseMode {
    Lower,
    Upper,
    Title,
    Sentence,
    Snake,
    Kebab,
    Camel,
}

impl CaseMode {
    pub fn label(&self) -> &'static str {
        match self {
            CaseMode::Lower => "lower",
            CaseMode::Upper => "upper",
            CaseMode::Title => "title",
            CaseMode::Sentence => "sentence",
            CaseMode::Snake => "snake",
            CaseMode::Kebab => "kebab",
            CaseMode::Camel => "camel",
        }
    }

    pub fn from_label(label: &str) -> CaseMode {
        match label {
            "upper" => CaseMode::Upper,
            "title" => CaseMode::Title,
            "sentence" => CaseMode::Sentence,
            "snake" => CaseMode::Snake,
            "kebab" => CaseMode::Kebab,
            "camel" => CaseMode::Camel,
            _ => CaseMode::Lower,
        }
    }

    fn label_pretty(&self) -> &'static str {
        match self {
            CaseMode::Lower => "lowercase",
            CaseMode::Upper => "UPPERCASE",
            CaseMode::Title => "Title Case",
            CaseMode::Sentence => "Sentence case",
            CaseMode::Snake => "snake_case",
            CaseMode::Kebab => "kebab-case",
            CaseMode::Camel => "camelCase",
        }
    }

    /// Transform a single piece of text (a stem or an extension).
    fn transform(&self, text: &str) -> String {
        match self {
            CaseMode::Lower => text.to_lowercase(),
            CaseMode::Upper => text.to_uppercase(),
            CaseMode::Title => text
                .split_inclusive(|c: char| !c.is_alphanumeric())
                .map(capitalize)
                .collect(),
            CaseMode::Sentence => {
                let lower = text.to_lowercase();
                let mut done = false;
                lower
                    .chars()
                    .map(|c| {
                        if !done && c.is_alphabetic() {
                            done = true;
                            c.to_uppercase().collect::<String>()
                        } else {
                            c.to_string()
                        }
                    })
                    .collect()
            }
            CaseMode::Snake => split_words(text)
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("_"),
            CaseMode::Kebab => split_words(text)
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("-"),
            CaseMode::Camel => split_words(text)
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    if i == 0 {
                        w.to_lowercase()
                    } else {
                        capitalize(w)
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaseChange {
    pub mode: CaseMode,
    pub scope: Scope,
}

impl CaseChange {
    fn apply(&self, filename: &str) -> String {
        let word_based = !matches!(self.mode, CaseMode::Lower | CaseMode::Upper);
        if word_based && self.scope == Scope::Both {
            // Word-based modes restyle the stem and lower-case the extension.
            let (stem, ext) = split_ext(filename);
            format!("{}{}", self.mode.transform(stem), ext.to_lowercase())
        } else {
            apply_scoped(filename, &self.scope, |s| self.mode.transform(s))
        }
    }
}

#[derive(Clone, Debug)]
pub struct AddPrefix {
    pub prefix: String,
}

impl AddPrefix {
    fn apply(&self, filename: &str) -> String {
        format!("{}{}", self.prefix, filename)
    }
}

#[derive(Clone, Debug)]
pub struct AddSuffix {
    pub suffix: String,
}

impl AddSuffix {
    fn apply(&self, filename: &str) -> String {
        match filename.rfind('.') {
            Some(0) | None => format!("{}{}", filename, self.suffix),
            Some(dot_pos) => {
                let stem = &filename[..dot_pos];
                let ext = &filename[dot_pos..];
                format!("{}{}{}", stem, self.suffix, ext)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumberPosition {
    Prefix,
    Suffix,
    Replace,
}

#[derive(Clone, Debug)]
pub struct SequentialNumber {
    pub format_width: u32,
    pub start: u32,
    pub position: NumberPosition,
}

#[derive(Clone, Debug)]
pub struct Slugify {
    pub separator: String,
}

impl Slugify {
    fn apply(&self, filename: &str) -> String {
        // A leading dot marks a hidden file; keep it and slugify the rest.
        let (hidden, name) = match filename.strip_prefix('.') {
            Some(rest) => (".", rest),
            None => ("", filename),
        };
        // Split stem and extension
        let (stem, ext) = match name.rfind('.') {
            Some(dot_pos) => (&name[..dot_pos], &name[dot_pos..]),
            None => (name, ""),
        };

        // Transliterate, lowercase, replace non-alphanumeric with separator, collapse, trim
        let slug: String = strip_diacritics(stem)
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c
                } else {
                    self.separator.chars().next().unwrap_or('-')
                }
            })
            .collect();

        let sep_char = self.separator.chars().next().unwrap_or('-');
        let mut result = String::new();
        let mut prev_was_sep = true; // trim leading
        for c in slug.chars() {
            if c == sep_char {
                if !prev_was_sep {
                    result.push(c);
                    prev_was_sep = true;
                }
            } else {
                result.push(c);
                prev_was_sep = false;
            }
        }
        // Trim trailing separator
        while result.ends_with(sep_char) {
            result.pop();
        }

        format!("{}{}{}", hidden, result, ext.to_lowercase())
    }
}

impl SequentialNumber {
    fn apply(&self, filename: &str, index: usize) -> String {
        let number = self.start + index as u32;
        let formatted = format!("{:0>width$}", number, width = self.format_width as usize);

        let (stem, ext) = match filename.rfind('.') {
            Some(0) | None => (filename, ""),
            Some(dot_pos) => (&filename[..dot_pos], &filename[dot_pos..]),
        };

        match self.position {
            NumberPosition::Prefix => format!("{}_{}", formatted, filename),
            NumberPosition::Suffix => format!("{}_{}{}", stem, formatted, ext),
            NumberPosition::Replace => format!("{}{}", formatted, ext),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StripDiacritics {
    pub scope: Scope,
}

impl StripDiacritics {
    fn apply(&self, filename: &str) -> String {
        apply_scoped(filename, &self.scope, strip_diacritics)
    }
}
