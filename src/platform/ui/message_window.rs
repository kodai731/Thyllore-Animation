use crate::ecs::resource::{MessageFilter, MessageLog};
use crate::logger::message_buffer::MessageLevel;

const COLOR_INFO: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
const COLOR_WARNING: [f32; 4] = [1.0, 0.9, 0.3, 1.0];
const COLOR_ERROR: [f32; 4] = [1.0, 0.3, 0.3, 1.0];
const BUTTON_ACTIVE_COLOR: [f32; 4] = [0.3, 0.5, 0.7, 1.0];

pub fn build_message_window_content(ui: &imgui::Ui, message_log: &mut MessageLog) {
    let total = message_log.messages.len();
    let warn_count = message_log.warning_count;
    let err_count = message_log.error_count;

    build_filter_buttons(ui, message_log, total, warn_count, err_count);

    ui.separator();

    build_message_list(ui, message_log);
}

fn build_filter_buttons(
    ui: &imgui::Ui,
    message_log: &mut MessageLog,
    total: usize,
    warn_count: usize,
    err_count: usize,
) {
    let filters = [
        (format!("All ({})", total), MessageFilter::All),
        (
            format!("Warn ({})##warn_filter", warn_count),
            MessageFilter::WarningAndError,
        ),
        (
            format!("Error ({})##err_filter", err_count),
            MessageFilter::ErrorOnly,
        ),
    ];

    for (i, (label, filter)) in filters.iter().enumerate() {
        if i > 0 {
            ui.same_line();
        }
        let is_active = message_log.filter == *filter;
        let token = if is_active {
            Some(ui.push_style_color(imgui::StyleColor::Button, BUTTON_ACTIVE_COLOR))
        } else {
            None
        };
        if ui.button(label) {
            message_log.filter = *filter;
        }
        drop(token);
    }

    ui.same_line();
    if ui.button("Clear") {
        message_log.clear_buffer();
    }

    ui.same_line();
    ui.checkbox("Auto-scroll", &mut message_log.auto_scroll);
}

fn build_message_list(ui: &imgui::Ui, message_log: &MessageLog) {
    let filtered = message_log.filtered_messages();

    ui.child_window("message_list").build(|| {
        for msg in &filtered {
            let color = match msg.level {
                MessageLevel::Info => COLOR_INFO,
                MessageLevel::Warning => COLOR_WARNING,
                MessageLevel::Error => COLOR_ERROR,
            };
            let prefix = match msg.level {
                MessageLevel::Info => "[I]",
                MessageLevel::Warning => "[W]",
                MessageLevel::Error => "[E]",
            };
            ui.text_colored(color, format!("{} {} {}", msg.timestamp, prefix, msg.text));
        }

        if message_log.auto_scroll && !filtered.is_empty() {
            ui.set_scroll_here_y();
        }
    });
}
