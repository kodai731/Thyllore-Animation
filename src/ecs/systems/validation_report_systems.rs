use crate::ecs::resource::ValidationReport;
use thyllore_log_core::validation_stats::{ValidationSeverity, ValidationStats};

pub fn validation_report_sync(report: &mut ValidationReport, snapshot: ValidationStats) {
    let frame = report.frames_seen + 1;
    if report.first_issue_frame.is_none() && (snapshot.errors + snapshot.warnings) > 0 {
        report.first_issue_frame = Some(frame);
    }
    report.stats = snapshot;
    report.frames_seen = frame;
}

pub fn format_validation_summary(
    stats: &ValidationStats,
    first_issue_frame: Option<u64>,
) -> String {
    let base = format!(
        "validation errors: {}, warnings: {}",
        stats.errors, stats.warnings
    );
    if let Some(frame) = first_issue_frame {
        if let Some(ref msg) = stats.first_message {
            format!("{base}, first at frame {frame}: {msg}")
        } else {
            base
        }
    } else {
        base
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
        validation_report_sync(&mut report, ValidationStats::default());
        assert_eq!(report.frames_seen, 1);
        validation_report_sync(&mut report, ValidationStats::default());
        assert_eq!(report.frames_seen, 2);
    }

    #[test]
    fn test_sync_replaces_stats() {
        let mut report = ValidationReport::default();
        validation_report_sync(&mut report, make_snapshot(1, 0));
        assert_eq!(report.stats.errors, 1);
        validation_report_sync(&mut report, make_snapshot(3, 2));
        assert_eq!(report.stats.errors, 3);
        assert_eq!(report.stats.warnings, 2);
    }

    #[test]
    fn test_first_issue_frame_recorded_when_issues_first_appear() {
        let mut report = ValidationReport::default();
        validation_report_sync(&mut report, ValidationStats::default());
        assert!(report.first_issue_frame.is_none());
        validation_report_sync(&mut report, make_snapshot(1, 0));
        assert_eq!(report.first_issue_frame, Some(2));
    }

    #[test]
    fn test_first_issue_frame_not_overwritten() {
        let mut report = ValidationReport::default();
        validation_report_sync(&mut report, make_snapshot(1, 0));
        assert_eq!(report.first_issue_frame, Some(1));
        validation_report_sync(&mut report, make_snapshot(5, 3));
        assert_eq!(report.first_issue_frame, Some(1));
    }

    #[test]
    fn test_first_issue_frame_with_warnings_only() {
        let mut report = ValidationReport::default();
        validation_report_sync(&mut report, ValidationStats::default());
        validation_report_sync(&mut report, make_snapshot(0, 1));
        assert_eq!(report.first_issue_frame, Some(2));
    }

    #[test]
    fn test_never_issues_stays_none() {
        let mut report = ValidationReport::default();
        for _ in 0..5 {
            validation_report_sync(&mut report, ValidationStats::default());
        }
        assert!(report.first_issue_frame.is_none());
        assert_eq!(report.frames_seen, 5);
    }

    #[test]
    fn test_format_validation_summary_no_issues() {
        let stats = ValidationStats::default();
        let summary = format_validation_summary(&stats, None);
        assert_eq!(summary, "validation errors: 0, warnings: 0");
    }

    #[test]
    fn test_format_validation_summary_with_issues() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Error, "bone missing");
        stats.record(ValidationSeverity::Warning, "curve flat");
        let summary = format_validation_summary(&stats, Some(3));
        assert_eq!(
            summary,
            "validation errors: 1, warnings: 1, first at frame 3: bone missing"
        );
    }
}
