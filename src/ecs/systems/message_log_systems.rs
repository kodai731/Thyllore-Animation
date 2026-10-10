use crate::ecs::resource::{MessageFilter, MessageLog};
use crate::logger::message_buffer::{
    Message, MessageLevel, MESSAGE_BUFFER, VALIDATION_MESSAGE_PREFIX,
};

pub fn message_log_sync_from_buffer(log: &mut MessageLog) {
    log.sync_from_buffer();
}

pub fn message_log_filtered_messages(log: &MessageLog) -> Vec<&Message> {
    log.messages
        .iter()
        .filter(|m| match log.filter {
            MessageFilter::All => true,
            MessageFilter::WarningAndError => {
                m.level == MessageLevel::Warning || m.level == MessageLevel::Error
            }
            MessageFilter::ErrorOnly => m.level == MessageLevel::Error,
            MessageFilter::Validation => m.text.starts_with(VALIDATION_MESSAGE_PREFIX),
        })
        .collect()
}

pub fn message_log_clear_buffer(log: &mut MessageLog) {
    MESSAGE_BUFFER
        .lock()
        .expect("Failed to lock message buffer")
        .clear();
    log.messages.clear();
    log.info_count = 0;
    log.warning_count = 0;
    log.error_count = 0;
}

pub fn message_log_validation_count(log: &MessageLog) -> usize {
    log.messages
        .iter()
        .filter(|m| m.text.starts_with(VALIDATION_MESSAGE_PREFIX))
        .count()
}

pub fn message_log_show_validation(log: &mut MessageLog) {
    log.filter = MessageFilter::Validation;
    log.select_tab_requested = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(level: MessageLevel, text: &str) -> Message {
        Message {
            level,
            text: text.to_string(),
            timestamp: "00:00:00".to_string(),
        }
    }

    #[test]
    fn test_default() {
        let log = MessageLog::default();
        assert!(log.messages.is_empty());
        assert_eq!(log.filter, MessageFilter::All);
        assert!(log.auto_scroll);
    }

    #[test]
    fn test_filtered_messages_all() {
        let mut log = MessageLog::default();
        log.messages = vec![
            make_message(MessageLevel::Info, "info"),
            make_message(MessageLevel::Warning, "warn"),
            make_message(MessageLevel::Error, "err"),
        ];
        log.filter = MessageFilter::All;
        assert_eq!(message_log_filtered_messages(&log).len(), 3);
    }

    #[test]
    fn test_filtered_messages_warning_and_error() {
        let mut log = MessageLog::default();
        log.messages = vec![
            make_message(MessageLevel::Info, "info"),
            make_message(MessageLevel::Warning, "warn"),
            make_message(MessageLevel::Error, "err"),
        ];
        log.filter = MessageFilter::WarningAndError;
        let filtered = message_log_filtered_messages(&log);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].level, MessageLevel::Warning);
        assert_eq!(filtered[1].level, MessageLevel::Error);
    }

    #[test]
    fn test_filtered_messages_error_only() {
        let mut log = MessageLog::default();
        log.messages = vec![
            make_message(MessageLevel::Info, "info"),
            make_message(MessageLevel::Warning, "warn"),
            make_message(MessageLevel::Error, "err"),
        ];
        log.filter = MessageFilter::ErrorOnly;
        let filtered = message_log_filtered_messages(&log);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, MessageLevel::Error);
    }

    #[test]
    fn test_filtered_messages_validation_only() {
        let mut log = MessageLog::default();
        log.messages = vec![
            make_message(MessageLevel::Info, "info"),
            make_message(MessageLevel::Warning, "[Vulkan] validation warning"),
            make_message(MessageLevel::Error, "err"),
            make_message(MessageLevel::Error, "[Vulkan] validation error"),
        ];
        log.filter = MessageFilter::Validation;
        let filtered = message_log_filtered_messages(&log);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "[Vulkan] validation warning");
        assert_eq!(filtered[1].text, "[Vulkan] validation error");
    }

    #[test]
    fn test_message_log_validation_count() {
        let mut log = MessageLog::default();
        log.messages = vec![
            make_message(MessageLevel::Info, "info"),
            make_message(MessageLevel::Warning, "[Vulkan] validation warning"),
            make_message(MessageLevel::Error, "err"),
            make_message(MessageLevel::Error, "[Vulkan] validation error"),
        ];
        assert_eq!(message_log_validation_count(&log), 2);
    }

    #[test]
    fn test_message_log_show_validation() {
        let mut log = MessageLog::default();
        message_log_show_validation(&mut log);
        assert_eq!(log.filter, MessageFilter::Validation);
        assert!(log.select_tab_requested);
    }
}
