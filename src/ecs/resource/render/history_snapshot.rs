/// The previous frame's snapshot of everything an effect's temporal history depends on. A frame
/// whose snapshot equals it may reuse the accumulated history.
pub struct HistorySnapshotState<S> {
    pub previous: Option<S>,
}

impl<S> Default for HistorySnapshotState<S> {
    fn default() -> Self {
        Self { previous: None }
    }
}
