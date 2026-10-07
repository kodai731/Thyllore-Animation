use thyllore_log_core::validation_stats::ValidationStats;

#[derive(Clone, Debug, Default)]
pub struct ValidationReport {
    pub stats: ValidationStats,
    pub frames_seen: u64,
    pub first_issue_frame: Option<u64>,
}

impl ValidationReport {
    pub fn sync(&mut self, snapshot: ValidationStats) {
        let frame = self.frames_seen + 1;
        if self.first_issue_frame.is_none() && (snapshot.errors + snapshot.warnings) > 0 {
            self.first_issue_frame = Some(frame);
        }
        self.stats = snapshot;
        self.frames_seen = frame;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(errors: usize, warnings: usize) -> ValidationStats {
        let mut stats = ValidationStats::default();
        for i in 0..errors {
            stats.record(ValidationSeverity::Error, &format!("error {}", i));
        }
        for i in 0..warnings {
            stats.record(ValidationSeverity::Warning, &format!("warning {}", i));
        }
        stats
    }

    use thyllore_log_core::validation_stats::ValidationSeverity;

    #[test]
    fn test_default_is_empty() {
        let report = ValidationReport::default();
        assert_eq!(report.frames_seen, 0);
        assert_eq!(report.stats.errors, 0);
        assert_eq!(report.stats.warnings, 0);
        assert!(report.first_issue_frame.is_none());
    }

    #[test]
    fn test_sync_increments_frames_seen() {
        let mut report = ValidationReport::default();
        report.sync(ValidationStats::default());
        assert_eq!(report.frames_seen, 1);
        report.sync(ValidationStats::default());
        assert_eq!(report.frames_seen, 2);
    }

    #[test]
    fn test_sync_replaces_stats() {
        let mut report = ValidationReport::default();
        report.sync(make_snapshot(1, 0));
        assert_eq!(report.stats.errors, 1);
        report.sync(make_snapshot(3, 2));
        assert_eq!(report.stats.errors, 3);
        assert_eq!(report.stats.warnings, 2);
    }

    #[test]
    fn test_first_issue_frame_recorded_when_issues_first_appear() {
        let mut report = ValidationReport::default();
        report.sync(ValidationStats::default());
        assert!(report.first_issue_frame.is_none());
        report.sync(make_snapshot(1, 0));
        assert_eq!(report.first_issue_frame, Some(2));
    }

    #[test]
    fn test_first_issue_frame_not_overwritten() {
        let mut report = ValidationReport::default();
        report.sync(make_snapshot(1, 0));
        assert_eq!(report.first_issue_frame, Some(1));
        report.sync(make_snapshot(5, 3));
        assert_eq!(report.first_issue_frame, Some(1));
    }

    #[test]
    fn test_first_issue_frame_with_warnings_only() {
        let mut report = ValidationReport::default();
        report.sync(ValidationStats::default());
        report.sync(make_snapshot(0, 1));
        assert_eq!(report.first_issue_frame, Some(2));
    }

    #[test]
    fn test_never_issues_stays_none() {
        let mut report = ValidationReport::default();
        for _ in 0..5 {
            report.sync(ValidationStats::default());
        }
        assert!(report.first_issue_frame.is_none());
        assert_eq!(report.frames_seen, 5);
    }
}
