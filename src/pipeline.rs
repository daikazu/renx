use crate::files::FileEntry;
use crate::operations::{OpContext, Operation};

pub struct Pipeline {
    operations: Vec<Operation>,
}

impl Pipeline {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self { operations }
    }

    /// Apply all operations in order to each filename.
    /// Returns a Vec of transformed filenames in the same order as input.
    /// Operations that need file metadata see no path here; use
    /// [`Pipeline::apply_entries`] for that.
    pub fn apply(&self, filenames: &[String]) -> Vec<String> {
        filenames
            .iter()
            .enumerate()
            .map(|(index, name)| self.apply_one(name, &OpContext { index, path: None }))
            .collect()
    }

    /// Apply all operations to each file entry, giving operations access to
    /// the file's path on disk.
    pub fn apply_entries(&self, entries: &[FileEntry]) -> Vec<String> {
        entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let path = entry.full_path();
                self.apply_one(
                    &entry.file_name,
                    &OpContext {
                        index,
                        path: Some(&path),
                    },
                )
            })
            .collect()
    }

    fn apply_one(&self, name: &str, ctx: &OpContext) -> String {
        let mut result = name.to_string();
        for op in &self.operations {
            result = op.apply_in_context(&result, ctx);
        }
        result
    }
}
