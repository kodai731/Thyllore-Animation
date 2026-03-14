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
}
