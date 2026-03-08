use crate::ecs::resource::TimelineState;

use super::viewport_window::ViewportInfo;

const FPS_BUFFER_SIZE: usize = 60;
const MEMORY_UPDATE_INTERVAL: u32 = 60;
const OVERLAY_PADDING: f32 = 6.0;
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6];
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];

pub struct StatusBarState {
    fps_buffer: [f32; FPS_BUFFER_SIZE],
    write_index: usize,
    sample_count: usize,
    memory_mb: f32,
    memory_update_counter: u32,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            fps_buffer: [0.0; FPS_BUFFER_SIZE],
            write_index: 0,
            sample_count: 0,
            memory_mb: 0.0,
            memory_update_counter: 0,
        }
    }
}

impl StatusBarState {
    pub fn update_fps(&mut self, delta_time: f32) {
        self.fps_buffer[self.write_index] = delta_time;
        self.write_index = (self.write_index + 1) % FPS_BUFFER_SIZE;
        if self.sample_count < FPS_BUFFER_SIZE {
            self.sample_count += 1;
        }
    }

    pub fn average_fps(&self) -> f32 {
        if self.sample_count == 0 {
            return 0.0;
        }
        let sum: f32 = self.fps_buffer[..self.sample_count].iter().sum();
        let avg_dt = sum / self.sample_count as f32;
        if avg_dt > 0.0 {
            1.0 / avg_dt
        } else {
            0.0
        }
    }

    pub fn update_memory(&mut self) {
        self.memory_update_counter += 1;
        if self.memory_update_counter >= MEMORY_UPDATE_INTERVAL {
            self.memory_update_counter = 0;
            self.memory_mb = read_rss_mb();
        }
    }
}

pub fn build_status_bar_overlay(
    ui: &imgui::Ui,
    state: &mut StatusBarState,
    delta_time: f32,
    viewport_info: &ViewportInfo,
    timeline_state: &TimelineState,
    clip_duration: f32,
) {
    state.update_fps(delta_time);
    state.update_memory();

    let fps = state.average_fps();
    let frame_rate = timeline_state.snap_settings.frame_rate;
    let current_frame = (timeline_state.current_time * frame_rate).round() as i32;
    let total_frames = (clip_duration * frame_rate).round() as i32;
    let current_time = timeline_state.current_time;
    let playback_icon = if timeline_state.playing { ">" } else { "||" };

    let text = format!(
        "FPS:{:.0}  F:{}/{}  {:.3}s  {}  {:.0}MB",
        fps, current_frame, total_frames, current_time, playback_icon, state.memory_mb,
    );

    let text_size = ui.calc_text_size(&text);

    let vp_right = viewport_info.position[0] + viewport_info.size[0];
    let vp_bottom = viewport_info.position[1] + viewport_info.size[1];

    let rect_max = [vp_right, vp_bottom];
    let rect_min = [
        vp_right - text_size[0] - OVERLAY_PADDING * 2.0,
        vp_bottom - text_size[1] - OVERLAY_PADDING * 2.0,
    ];
    let text_pos = [rect_min[0] + OVERLAY_PADDING, rect_min[1] + OVERLAY_PADDING];

    let draw_list = ui.get_foreground_draw_list();
    draw_list
        .add_rect(rect_min, rect_max, BG_COLOR)
        .filled(true)
        .build();
    draw_list.add_text(text_pos, TEXT_COLOR, &text);
}

fn read_rss_mb() -> f32 {
    parse_rss_from_statm(&std::fs::read_to_string("/proc/self/statm").unwrap_or_default())
}

fn parse_rss_from_statm(content: &str) -> f32 {
    // Format: "total_pages rss_pages shared_pages ..."
    content
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .map(|pages| (pages * 4096) as f32 / 1_048_576.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = StatusBarState::default();
        assert_eq!(state.sample_count, 0);
        assert_eq!(state.average_fps(), 0.0);
        assert_eq!(state.memory_mb, 0.0);
    }

    #[test]
    fn test_fps_buffer_average() {
        let mut state = StatusBarState::default();
        // 60 FPS = 1/60 delta_time
        for _ in 0..10 {
            state.update_fps(1.0 / 60.0);
        }
        let fps = state.average_fps();
        assert!((fps - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_fps_buffer_wraps() {
        let mut state = StatusBarState::default();
        // Fill 100 samples (exceeds buffer of 60)
        for _ in 0..100 {
            state.update_fps(1.0 / 30.0);
        }
        assert_eq!(state.sample_count, FPS_BUFFER_SIZE);
        let fps = state.average_fps();
        assert!((fps - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_rss() {
        // 100000 pages * 4096 / 1048576 = 390.625 MB
        let content = "200000 100000 5000 0 0 50000 0";
        let mb = parse_rss_from_statm(content);
        assert!((mb - 390.625).abs() < 0.01);
    }

    #[test]
    fn test_parse_rss_empty() {
        assert_eq!(parse_rss_from_statm(""), 0.0);
    }

    #[test]
    fn test_parse_rss_invalid() {
        assert_eq!(parse_rss_from_statm("abc def"), 0.0);
    }
}
