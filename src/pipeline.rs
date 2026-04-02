use crate::operations::Operation;

pub struct Pipeline {
    operations: Vec<Operation>,
}

impl Pipeline {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self { operations }
    }

    /// Apply all operations in order to each filename.
    /// Returns a Vec of transformed filenames in the same order as input.
    pub fn apply(&self, filenames: &[String]) -> Vec<String> {
        filenames
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut result = name.clone();
                for op in &self.operations {
                    result = op.apply_with_index(&result, index);
                }
                result
            })
            .collect()
    }
}
