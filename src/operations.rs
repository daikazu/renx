use chrono::{DateTime, Local};
use regex::Regex;
use std::fmt::Write as _;
use std::path::Path;
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
    ChangeExtension(ChangeExtension),
    Trim(Trim),
    Insert(Insert),
    ZeroPad(ZeroPad),
    Truncate(Truncate),
    InsertDate(InsertDate),
}

/// Per-file context for operations that need more than the name itself.
pub struct OpContext<'a> {
    /// Position of the file among the files being renamed (for numbering).
    pub index: usize,
    /// Path of the file on disk, when known (for metadata such as dates).
    pub path: Option<&'a Path>,
}

impl Operation {
    /// Apply this operation to a filename, returning the transformed name.
    pub fn apply(&self, filename: &str) -> String {
        self.apply_in_context(
            filename,
            &OpContext {
                index: 0,
                path: None,
            },
        )
    }

    /// Apply with a file index (used by SequentialNumber).
    /// Non-sequential operations ignore the index.
    pub fn apply_with_index(&self, filename: &str, index: usize) -> String {
        self.apply_in_context(filename, &OpContext { index, path: None })
    }

    /// Apply with full per-file context.
    pub fn apply_in_context(&self, filename: &str, ctx: &OpContext) -> String {
        match self {
            Operation::FindReplace(op) => op.apply(filename),
            Operation::RemovePattern(op) => op.apply(filename),
            Operation::RegexReplace(op) => op.apply(filename),
            Operation::CaseChange(op) => op.apply(filename),
            Operation::AddPrefix(op) => op.apply(filename),
            Operation::AddSuffix(op) => op.apply(filename),
            Operation::SequentialNumber(op) => op.apply(filename, ctx.index),
            Operation::Slugify(op) => op.apply(filename),
            Operation::StripDiacritics(op) => op.apply(filename),
            Operation::ChangeExtension(op) => op.apply(filename),
            Operation::Trim(op) => op.apply(filename),
            Operation::Insert(op) => op.apply(filename),
            Operation::ZeroPad(op) => op.apply(filename),
            Operation::Truncate(op) => op.apply(filename),
            Operation::InsertDate(op) => op.apply(filename, ctx.path),
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
            Operation::ChangeExtension(op) => {
                let ext = op.extension.trim_start_matches('.');
                if ext.is_empty() {
                    "Remove extension".to_string()
                } else {
                    format!("Extension → .{}", ext)
                }
            }
            Operation::Trim(op) => format!(
                "Trim {} from {}{}",
                op.count,
                op.from.label(),
                op.scope.suffix()
            ),
            Operation::Insert(op) => format!(
                "Insert \"{}\" at {} from {}{}",
                op.text,
                op.position,
                op.from.label(),
                op.scope.suffix()
            ),
            Operation::ZeroPad(op) => {
                format!("Zero-pad numbers to {}{}", op.width, op.scope.suffix())
            }
            Operation::Truncate(op) => {
                format!("Truncate to {} chars{}", op.max_len, op.scope.suffix())
            }
            Operation::InsertDate(op) => {
                let pos = match op.position {
                    NumberPosition::Prefix => "prefix",
                    NumberPosition::Suffix => "suffix",
                    NumberPosition::Replace => "replace",
                };
                format!("Date {} ({}, {})", op.format, op.source.label(), pos)
            }
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

/// Which end of the text an operation counts from.
#[derive(Clone, Debug, PartialEq)]
pub enum Side {
    Start,
    End,
}

impl Side {
    pub fn label(&self) -> &'static str {
        match self {
            Side::Start => "start",
            Side::End => "end",
        }
    }

    pub fn from_label(label: &str) -> Side {
        match label {
            "end" => Side::End,
            _ => Side::Start,
        }
    }
}

/// Replace, add or remove the extension. An empty extension removes it;
/// a leading dot is optional.
#[derive(Clone, Debug)]
pub struct ChangeExtension {
    pub extension: String,
}

impl ChangeExtension {
    fn apply(&self, filename: &str) -> String {
        let (stem, _) = split_ext(filename);
        let ext = self.extension.trim_start_matches('.');
        if ext.is_empty() {
            stem.to_string()
        } else {
            format!("{}.{}", stem, ext)
        }
    }
}

/// Remove a number of characters from the start or end.
#[derive(Clone, Debug)]
pub struct Trim {
    pub count: u32,
    pub from: Side,
    pub scope: Scope,
}

impl Trim {
    fn apply(&self, filename: &str) -> String {
        apply_scoped(filename, &self.scope, |s| {
            let chars: Vec<char> = s.chars().collect();
            let keep = chars.len().saturating_sub(self.count as usize);
            match self.from {
                Side::Start => chars[chars.len() - keep..].iter().collect(),
                Side::End => chars[..keep].iter().collect(),
            }
        })
    }
}

/// Insert text at a character position counted from the start or end.
#[derive(Clone, Debug)]
pub struct Insert {
    pub text: String,
    pub position: u32,
    pub from: Side,
    pub scope: Scope,
}

impl Insert {
    fn apply(&self, filename: &str) -> String {
        apply_scoped(filename, &self.scope, |s| {
            let chars: Vec<char> = s.chars().collect();
            let offset = (self.position as usize).min(chars.len());
            let at = match self.from {
                Side::Start => offset,
                Side::End => chars.len() - offset,
            };
            let mut out: String = chars[..at].iter().collect();
            out.push_str(&self.text);
            out.extend(&chars[at..]);
            out
        })
    }
}

/// Pad every run of digits with leading zeros to at least `width` digits.
#[derive(Clone, Debug)]
pub struct ZeroPad {
    pub width: u32,
    pub scope: Scope,
}

impl ZeroPad {
    fn apply(&self, filename: &str) -> String {
        apply_scoped(filename, &self.scope, |s| {
            let mut out = String::new();
            let mut run = String::new();
            let flush = |run: &mut String, out: &mut String| {
                if !run.is_empty() {
                    for _ in run.len()..self.width as usize {
                        out.push('0');
                    }
                    out.push_str(run);
                    run.clear();
                }
            };
            for c in s.chars() {
                if c.is_ascii_digit() {
                    run.push(c);
                } else {
                    flush(&mut run, &mut out);
                    out.push(c);
                }
            }
            flush(&mut run, &mut out);
            out
        })
    }
}

/// Cut the text down to at most `max_len` characters.
#[derive(Clone, Debug)]
pub struct Truncate {
    pub max_len: u32,
    pub scope: Scope,
}

impl Truncate {
    fn apply(&self, filename: &str) -> String {
        apply_scoped(filename, &self.scope, |s| {
            s.chars().take(self.max_len as usize).collect()
        })
    }
}

/// Which file timestamp to use for InsertDate.
#[derive(Clone, Debug, PartialEq)]
pub enum DateSource {
    Modified,
    Created,
}

impl DateSource {
    pub fn label(&self) -> &'static str {
        match self {
            DateSource::Modified => "modified",
            DateSource::Created => "created",
        }
    }

    pub fn from_label(label: &str) -> DateSource {
        match label {
            "created" => DateSource::Created,
            _ => DateSource::Modified,
        }
    }
}

/// Insert a date taken from the file's metadata, formatted with a strftime
/// pattern such as `%Y-%m-%d`. Leaves the name unchanged when the file's
/// timestamp cannot be read.
#[derive(Clone, Debug)]
pub struct InsertDate {
    pub source: DateSource,
    pub format: String,
    pub position: NumberPosition,
}

impl InsertDate {
    fn apply(&self, filename: &str, path: Option<&Path>) -> String {
        let Some(formatted) = path.and_then(|p| self.formatted_date(p)) else {
            return filename.to_string();
        };
        let (stem, ext) = split_ext(filename);
        match self.position {
            NumberPosition::Prefix => format!("{}_{}", formatted, filename),
            NumberPosition::Suffix => format!("{}_{}{}", stem, formatted, ext),
            NumberPosition::Replace => format!("{}{}", formatted, ext),
        }
    }

    fn formatted_date(&self, path: &Path) -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        let time = match self.source {
            DateSource::Modified => meta.modified().ok()?,
            DateSource::Created => meta.created().or_else(|_| meta.modified()).ok()?,
        };
        let date: DateTime<Local> = time.into();
        let mut out = String::new();
        // An invalid strftime pattern makes the formatter fail; treat it as unreadable.
        write!(out, "{}", date.format(&self.format)).ok()?;
        Some(out)
    }
}

/// Check a strftime format string by formatting the current time with it.
pub fn validate_date_format(format: &str) -> Result<(), String> {
    let mut out = String::new();
    write!(out, "{}", Local::now().format(format))
        .map(|_| ())
        .map_err(|_| format!("Invalid date format: {}", format))
}

// --- Operation type registry (drives the picker and parameter editor) ----

/// One parameter field in the editor popup.
pub struct FieldSpec {
    pub label: &'static str,
    /// `Some` makes this a left/right selector over fixed options.
    pub options: Option<&'static [&'static str]>,
    /// Initial text for a text field, or the initially selected option label.
    pub default: &'static str,
}

/// One entry in the "Add Operation" picker.
pub struct OpTypeSpec {
    pub name: &'static str,
    pub fields: &'static [FieldSpec],
}

const fn text(label: &'static str, default: &'static str) -> FieldSpec {
    FieldSpec {
        label,
        options: None,
        default,
    }
}

const fn choice(
    label: &'static str,
    options: &'static [&'static str],
    default: &'static str,
) -> FieldSpec {
    FieldSpec {
        label,
        options: Some(options),
        default,
    }
}

const SCOPES: &[&str] = &["name", "ext", "both"];
const SIDES: &[&str] = &["start", "end"];
const POSITIONS: &[&str] = &["prefix", "suffix", "replace"];
const CASE_MODES: &[&str] = &[
    "lower", "upper", "title", "sentence", "snake", "kebab", "camel",
];
const DATE_SOURCES: &[&str] = &["modified", "created"];

pub const T_FIND_REPLACE: usize = 0;
pub const T_REGEX_REPLACE: usize = 1;
pub const T_REMOVE_PATTERN: usize = 2;
pub const T_CASE_CHANGE: usize = 3;
pub const T_ADD_PREFIX: usize = 4;
pub const T_ADD_SUFFIX: usize = 5;
pub const T_INSERT: usize = 6;
pub const T_TRIM: usize = 7;
pub const T_TRUNCATE: usize = 8;
pub const T_SEQUENTIAL: usize = 9;
pub const T_ZERO_PAD: usize = 10;
pub const T_INSERT_DATE: usize = 11;
pub const T_CHANGE_EXT: usize = 12;
pub const T_SLUGIFY: usize = 13;
pub const T_STRIP_DIACRITICS: usize = 14;

/// Every operation type in picker order. Indices match the `T_*` constants.
pub const OP_TYPES: &[OpTypeSpec] = &[
    OpTypeSpec {
        name: "Find & Replace",
        fields: &[
            text("Search:", ""),
            text("Replace with:", ""),
            choice("Scope:", SCOPES, "both"),
        ],
    },
    OpTypeSpec {
        name: "Regex Replace",
        fields: &[
            text("Regex pattern:", ""),
            text("Replace with:", ""),
            choice("Scope:", SCOPES, "both"),
        ],
    },
    OpTypeSpec {
        name: "Remove Pattern",
        fields: &[
            text("Pattern to remove:", ""),
            choice("Scope:", SCOPES, "both"),
        ],
    },
    OpTypeSpec {
        name: "Case Change",
        fields: &[
            choice("Mode:", CASE_MODES, "lower"),
            choice("Scope:", SCOPES, "both"),
        ],
    },
    OpTypeSpec {
        name: "Add Prefix",
        fields: &[text("Prefix:", "")],
    },
    OpTypeSpec {
        name: "Add Suffix",
        fields: &[text("Suffix:", "")],
    },
    OpTypeSpec {
        name: "Insert Text",
        fields: &[
            text("Text:", ""),
            text("Position:", "0"),
            choice("From:", SIDES, "start"),
            choice("Scope:", SCOPES, "name"),
        ],
    },
    OpTypeSpec {
        name: "Trim Characters",
        fields: &[
            text("Count:", "1"),
            choice("From:", SIDES, "start"),
            choice("Scope:", SCOPES, "name"),
        ],
    },
    OpTypeSpec {
        name: "Truncate",
        fields: &[text("Max length:", "32"), choice("Scope:", SCOPES, "name")],
    },
    OpTypeSpec {
        name: "Sequential Number",
        fields: &[
            text("Width (e.g. 3):", "3"),
            text("Start number:", "1"),
            choice("Position:", POSITIONS, "prefix"),
        ],
    },
    OpTypeSpec {
        name: "Zero-pad Numbers",
        fields: &[text("Width:", "2"), choice("Scope:", SCOPES, "name")],
    },
    OpTypeSpec {
        name: "Insert Date",
        fields: &[
            choice("Source:", DATE_SOURCES, "modified"),
            text("Format (strftime):", "%Y-%m-%d"),
            choice("Position:", POSITIONS, "prefix"),
        ],
    },
    OpTypeSpec {
        name: "Change Extension",
        fields: &[text("New extension (blank removes):", "")],
    },
    OpTypeSpec {
        name: "Slugify",
        fields: &[choice("Separator:", &["-", "_", "."], "-")],
    },
    OpTypeSpec {
        name: "Strip Accents",
        fields: &[choice("Scope:", SCOPES, "both")],
    },
];

impl Operation {
    /// Index of this operation's type in [`OP_TYPES`].
    pub fn type_index(&self) -> usize {
        match self {
            Operation::FindReplace(_) => T_FIND_REPLACE,
            Operation::RegexReplace(_) => T_REGEX_REPLACE,
            Operation::RemovePattern(_) => T_REMOVE_PATTERN,
            Operation::CaseChange(_) => T_CASE_CHANGE,
            Operation::AddPrefix(_) => T_ADD_PREFIX,
            Operation::AddSuffix(_) => T_ADD_SUFFIX,
            Operation::Insert(_) => T_INSERT,
            Operation::Trim(_) => T_TRIM,
            Operation::Truncate(_) => T_TRUNCATE,
            Operation::SequentialNumber(_) => T_SEQUENTIAL,
            Operation::ZeroPad(_) => T_ZERO_PAD,
            Operation::InsertDate(_) => T_INSERT_DATE,
            Operation::ChangeExtension(_) => T_CHANGE_EXT,
            Operation::Slugify(_) => T_SLUGIFY,
            Operation::StripDiacritics(_) => T_STRIP_DIACRITICS,
        }
    }
}
