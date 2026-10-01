/// A `--batch-debug-action-at` entry: the action text the CLI validated and the frame it runs on.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledBatchAction {
    pub frame: u64,
    pub action: String,
}

#[derive(Clone, Debug, Default)]
pub struct ScheduledBatchActions {
    pub pending: Vec<ScheduledBatchAction>,
}
