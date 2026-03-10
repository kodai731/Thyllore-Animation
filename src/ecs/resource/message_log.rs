use crate::logger::message_buffer::{Message, MessageLevel, MESSAGE_BUFFER};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MessageFilter {
    All,
    WarningAndError,
    ErrorOnly,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self::All
    }
}

pub struct MessageLog {
    pub messages: Vec<Message>,
    pub filter: MessageFilter,
    pub auto_scroll: bool,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
}

impl Default for MessageLog {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            filter: MessageFilter::All,
            auto_scroll: true,
            info_count: 0,
            warning_count: 0,
            error_count: 0,
        }
    }
}

impl MessageLog {
    pub fn sync_from_buffer(&mut self) {
        let buf = MESSAGE_BUFFER
            .lock()
            .expect("Failed to lock message buffer");
        self.messages = buf.snapshot();
        self.info_count = buf.count_by_level(MessageLevel::Info);
        self.warning_count = buf.count_by_level(MessageLevel::Warning);
        self.error_count = buf.count_by_level(MessageLevel::Error);
    }

    pub fn filtered_messages(&self) -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|m| match self.filter {
                MessageFilter::All => true,
                MessageFilter::WarningAndError => {
                    m.level == MessageLevel::Warning || m.level == MessageLevel::Error
                }
                MessageFilter::ErrorOnly => m.level == MessageLevel::Error,
            })
            .collect()
    }

    pub fn clear_buffer(&mut self) {
        MESSAGE_BUFFER
            .lock()
            .expect("Failed to lock message buffer")
            .clear();
        self.messages.clear();
        self.info_count = 0;
        self.warning_count = 0;
        self.error_count = 0;
    }
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
        assert_eq!(log.filtered_messages().len(), 3);
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
        let filtered = log.filtered_messages();
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
        let filtered = log.filtered_messages();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, MessageLevel::Error);
    }
}
