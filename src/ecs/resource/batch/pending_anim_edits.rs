use super::BatchAnimEdit;

#[derive(Clone, Debug)]
pub struct PendingBatchAnimEdits {
    pub edits: Vec<BatchAnimEdit>,
}
