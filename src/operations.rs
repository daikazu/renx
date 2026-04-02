use regex::Regex;

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
                format!("Replace \"{}\" → \"{}\"", op.search, op.replacement)
            }
            Operation::RemovePattern(op) => format!("Remove \"{}\"", op.pattern),
            Operation::RegexReplace(op) => {
                format!("Regex \"{}\" → \"{}\"", op.pattern, op.replacement)
            }
            Operation::CaseChange(op) => {
                let mode_str = match op.mode {
                    CaseMode::Lower => "lowercase",
                    CaseMode::Upper => "UPPERCASE",
                    CaseMode::Title => "Title Case",
                };
                format!("{}", mode_str)
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
                let sep = if op.separator == "-" { "hyphens" } else { &format!("\"{}\"", op.separator) };
                format!("Slugify ({})", sep)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct FindReplace {
    pub search: String,
    pub replacement: String,
}

impl FindReplace {
    fn apply(&self, filename: &str) -> String {
        filename.replace(&self.search, &self.replacement)
    }
}

#[derive(Clone, Debug)]
pub struct RemovePattern {
    pub pattern: String,
}

impl RemovePattern {
    fn apply(&self, filename: &str) -> String {
        filename.replace(&self.pattern, "")
    }
}

#[derive(Clone, Debug)]
pub struct RegexReplace {
    pub pattern: String,
    pub replacement: String,
}

impl RegexReplace {
    fn apply(&self, filename: &str) -> String {
        match Regex::new(&self.pattern) {
            Ok(re) => re.replace_all(filename, self.replacement.as_str()).to_string(),
            Err(_) => filename.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseMode {
    Lower,
    Upper,
    Title,
}

#[derive(Clone, Debug)]
pub struct CaseChange {
    pub mode: CaseMode,
}

impl CaseChange {
    fn apply(&self, filename: &str) -> String {
        match self.mode {
            CaseMode::Lower => filename.to_lowercase(),
            CaseMode::Upper => filename.to_uppercase(),
            CaseMode::Title => filename
                .split_inclusive(|c: char| !c.is_alphanumeric())
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            upper + &chars.as_str().to_lowercase()
                        }
                        None => String::new(),
                    }
                })
                .collect(),
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
            Some(dot_pos) => {
                let stem = &filename[..dot_pos];
                let ext = &filename[dot_pos..];
                format!("{}{}{}", stem, self.suffix, ext)
            }
            None => format!("{}{}", filename, self.suffix),
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
        // Split stem and extension
        let (stem, ext) = match filename.rfind('.') {
            Some(dot_pos) => (&filename[..dot_pos], &filename[dot_pos..]),
            None => (filename, ""),
        };

        // Lowercase, replace non-alphanumeric with separator, collapse, trim
        let slug: String = stem
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { self.separator.chars().next().unwrap_or('-') })
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

        format!("{}{}", result, ext.to_lowercase())
    }
}

impl SequentialNumber {
    fn apply(&self, filename: &str, index: usize) -> String {
        let number = self.start + index as u32;
        let formatted = format!("{:0>width$}", number, width = self.format_width as usize);

        match self.position {
            NumberPosition::Prefix => format!("{}_{}", formatted, filename),
            NumberPosition::Suffix => match filename.rfind('.') {
                Some(dot_pos) => {
                    let stem = &filename[..dot_pos];
                    let ext = &filename[dot_pos..];
                    format!("{}_{}{}", stem, formatted, ext)
                }
                None => format!("{}_{}", filename, formatted),
            },
            NumberPosition::Replace => match filename.rfind('.') {
                Some(dot_pos) => {
                    let ext = &filename[dot_pos..];
                    format!("{}{}", formatted, ext)
                }
                None => formatted,
            },
        }
    }
}
