use std::sync::Mutex;

use once_cell::sync::Lazy;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationStats {
    pub errors: usize,
    pub warnings: usize,
    pub first_message: Option<String>,
    pub last_message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

const MAX_MESSAGE_CHARS: usize = 300;

impl ValidationStats {
    pub fn record(&mut self, severity: ValidationSeverity, message: &str) {
        let truncated: String = message.chars().take(MAX_MESSAGE_CHARS).collect::<String>();

        match severity {
            ValidationSeverity::Error => {
                self.errors += 1;
            }
            ValidationSeverity::Warning => {
                self.warnings += 1;
            }
        }

        if self.first_message.is_none() {
            self.first_message = Some(truncated.clone());
        }
        self.last_message = Some(truncated);
    }
}

pub static VALIDATION_STATS: Lazy<Mutex<ValidationStats>> =
    Lazy::new(|| Mutex::new(ValidationStats::default()));

pub fn record_validation(severity: ValidationSeverity, message: &str) {
    let mut stats = VALIDATION_STATS.lock().unwrap();
    stats.record(severity, message);
}

pub fn validation_stats_snapshot() -> ValidationStats {
    let stats = VALIDATION_STATS.lock().unwrap();
    stats.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_error() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Error, "error message");
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.warnings, 0);
        assert_eq!(stats.first_message, Some("error message".to_string()));
        assert_eq!(stats.last_message, Some("error message".to_string()));
    }

    #[test]
    fn test_record_warning() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Warning, "warning message");
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.warnings, 1);
        assert_eq!(stats.first_message, Some("warning message".to_string()));
        assert_eq!(stats.last_message, Some("warning message".to_string()));
    }

    #[test]
    fn test_first_message_only_set_once() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Error, "first");
        stats.record(ValidationSeverity::Warning, "second");
        assert_eq!(stats.first_message, Some("first".to_string()));
        assert_eq!(stats.last_message, Some("second".to_string()));
    }

    #[test]
    fn test_message_truncated_to_300_chars() {
        let mut stats = ValidationStats::default();
        let long_message = "a".repeat(500);
        stats.record(ValidationSeverity::Error, &long_message);
        assert_eq!(
            stats.first_message.as_ref().unwrap().chars().count(),
            MAX_MESSAGE_CHARS
        );
        assert_eq!(
            stats.last_message.as_ref().unwrap().chars().count(),
            MAX_MESSAGE_CHARS
        );
    }

    #[test]
    fn test_short_message_not_truncated() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Error, "short");
        assert_eq!(stats.first_message, Some("short".to_string()));
    }

    #[test]
    fn test_multiple_records_accumulate() {
        let mut stats = ValidationStats::default();
        stats.record(ValidationSeverity::Error, "e1");
        stats.record(ValidationSeverity::Error, "e2");
        stats.record(ValidationSeverity::Warning, "w1");
        assert_eq!(stats.errors, 2);
        assert_eq!(stats.warnings, 1);
    }

    #[test]
    fn record_truncates_multibyte_message_on_char_boundary() {
        let mut stats = ValidationStats::default();
        let long_message = "あ".repeat(301);
        stats.record(ValidationSeverity::Error, &long_message);
        let recorded = stats.first_message.as_ref().unwrap();
        assert_eq!(recorded.chars().count(), MAX_MESSAGE_CHARS);
        assert!(recorded.is_char_boundary(recorded.len()));
    }
}
