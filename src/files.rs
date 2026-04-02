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
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(entries)
}

/// Discover files matching a glob pattern.
pub fn discover_glob(pattern: &str) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let paths = glob(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    for path_result in paths {
        let path = path_result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
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
    entries.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(entries)
}

/// Rename a single file on disk.
pub fn rename_file(entry: &FileEntry, new_name: &str) -> io::Result<()> {
    let old_path = entry.full_path();
    let new_path = entry.parent_dir.join(new_name);
    fs::rename(old_path, new_path)
}
