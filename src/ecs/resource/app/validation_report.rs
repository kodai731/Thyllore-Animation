#[derive(Clone, Debug, Default)]
pub struct ValidationReport {
    pub stats: thyllore_log_core::validation_stats::ValidationStats,
    pub frames_seen: u64,
    pub first_issue_frame: Option<u64>,
}

crate::startup_resource!(ValidationReport, CoreResources);
