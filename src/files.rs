use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use glob::glob;

/// Represents a file that can be renamed.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub file_name: String,
    pub parent_dir: PathBuf,
}

impl FileEntry {
    pub fn full_path(&self) -> PathBuf {
        self.parent_dir.join(&self.file_name)
    }
}

/// Compare two names the way a human would: case-insensitively, with runs of
/// digits compared by numeric value so `file2` sorts before `file10`.
/// Ties are broken by plain byte order so the result is deterministic.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ia = a.chars().peekable();
    let mut ib = b.chars().peekable();
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let na = take_digits(&mut ia);
                let nb = take_digits(&mut ib);
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(ca), Some(cb)) => {
                ia.next();
                ib.next();
                let la = ca.to_lowercase().next().unwrap_or(ca);
                let lb = cb.to_lowercase().next().unwrap_or(cb);
                if la != lb {
                    return la.cmp(&lb);
                }
            }
        }
    }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut out = String::new();
    while let Some(c) = it.peek() {
        if c.is_ascii_digit() {
            out.push(*c);
            it.next();
        } else {
            break;
        }
    }
    out
}

/// Discover all files (not directories) in a directory.
pub fn discover_dir(path: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(FileEntry {
                    file_name: name.to_string(),
                    parent_dir: path.to_path_buf(),
                });
            }
        }
    }
    entries.sort_by(|a, b| natural_cmp(&a.file_name, &b.file_name));
    Ok(entries)
}

/// Discover files matching a glob pattern.
pub fn discover_glob(pattern: &str) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let paths = glob(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    for path_result in paths {
        let path = path_result.map_err(io::Error::other)?;
        if path.is_file() {
            if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
                if let Some(name_str) = name.to_str() {
                    entries.push(FileEntry {
                        file_name: name_str.to_string(),
                        parent_dir: parent.to_path_buf(),
                    });
                }
            }
        }
    }
    entries.sort_by(|a, b| natural_cmp(&a.file_name, &b.file_name));
    Ok(entries)
}

/// Rename a single file on disk.
/// Whether two paths refer to the same file on disk.
///
/// Compares by file identity (device + inode) rather than by path string, so
/// that on case-insensitive filesystems (e.g. macOS APFS) `Photo.JPG` and
/// `photo.jpg` are recognised as the same file. Returns `false` if either
/// path does not exist.
pub fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(ma), Ok(mb)) => ma.dev() == mb.dev() && ma.ino() == mb.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        match (fs::canonicalize(a), fs::canonicalize(b)) {
            (Ok(ca), Ok(cb)) => ca == cb,
            _ => false,
        }
    }
}

pub fn rename_file(entry: &FileEntry, new_name: &str) -> io::Result<()> {
    let old_path = entry.full_path();
    let new_path = entry.parent_dir.join(new_name);
    fs::rename(old_path, new_path)
}
