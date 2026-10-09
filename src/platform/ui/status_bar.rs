use crate::asset::AssetStorage;
use crate::ecs::resource::{
    ClipLibrary, CpuFrameTimings, FrameClock, GpuPassTimings, TimelineState, ValidationReport,
    ViewportInput,
};
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::world::World;
use crate::hooks::ui_window::init_window_state;
use crate::platform::ui::message_window::{COLOR_ERROR, COLOR_WARNING};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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
    last_gpu_ms: f32,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            fps_buffer: [0.0; FPS_BUFFER_SIZE],
            write_index: 0,
            sample_count: 0,
            memory_mb: 0.0,
            memory_update_counter: 0,
            last_gpu_ms: 0.0,
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

fn draw_status_bar(
    ui: &imgui::Ui,
    state: &mut StatusBarState,
    delta_time: f32,
    cpu_ms: f32,
    gpu_ms: Option<f32>,
    viewport: &ViewportInput,
    timeline_state: &TimelineState,
    clip_duration: f32,
    errors: usize,
    warnings: usize,
) -> Option<OverlayEvent> {
    state.update_fps(delta_time);
    state.update_memory();

    let gpu_ms = match gpu_ms {
        Some(ms) => {
            state.last_gpu_ms = ms;
            ms
        }
        None => state.last_gpu_ms,
    };

    let fps = state.average_fps();
    let frame_rate = timeline_state.snap_settings.frame_rate;
    let current_frame = (timeline_state.current_time * frame_rate).round() as i32;
    let total_frames = (clip_duration * frame_rate).round() as i32;
    let current_time = timeline_state.current_time;
    let playback_icon = if timeline_state.playing { ">" } else { "||" };

    let text = format!(
        "FPS:{:.0}  CPU {:.1}ms  GPU {:.1}ms  F:{}/{}  {:.3}s  {}  {:.0}MB",
        fps,
        cpu_ms,
        gpu_ms,
        current_frame,
        total_frames,
        current_time,
        playback_icon,
        state.memory_mb,
    );

    let validation_text = format_validation_status(errors, warnings);
    let validation_color = validation_status_color(errors, warnings);

    let text_size = ui.calc_text_size(&text);
    let validation_text_size = ui.calc_text_size(&validation_text);

    let vp_right = viewport.position[0] + viewport.size[0];
    let vp_bottom = viewport.position[1] + viewport.size[1];

    let item_spacing = ui.clone_style().item_spacing[0];
    let window_width =
        text_size[0] + item_spacing + validation_text_size[0] + OVERLAY_PADDING * 2.0;
    let window_height = text_size[1] + OVERLAY_PADDING * 2.0;
    let window_pos = [vp_right - window_width, vp_bottom - window_height];

    let clicked: bool = {
        let result: Option<bool> = ui
            .window("##status_bar")
            .position(window_pos, imgui::Condition::Always)
            .size([window_width, window_height], imgui::Condition::Always)
            .no_decoration()
            .no_nav()
            .bg_alpha(BG_COLOR[3])
            .focus_on_appearing(false)
            .save_settings(false)
            .build(|| {
                ui.text_colored(TEXT_COLOR, &text);
                ui.same_line();
                ui.text_colored(validation_color, &validation_text);
                ui.is_item_clicked()
            });
        result.unwrap_or(false)
    };

    if clicked {
        Some(OverlayEvent::ShowValidationMessages)
    } else {
        None
    }
}

fn validation_status_color(errors: usize, warnings: usize) -> [f32; 4] {
    if errors > 0 {
        COLOR_ERROR
    } else if warnings > 0 {
        COLOR_WARNING
    } else {
        TEXT_COLOR
    }
}

fn format_validation_status(errors: usize, warnings: usize) -> String {
    format!("VK E:{} W:{}", errors, warnings)
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

    #[test]
    fn validation_status_color_errors_red() {
        let color = validation_status_color(1, 0);
        assert_eq!(color, COLOR_ERROR);
        let color = validation_status_color(5, 3);
        assert_eq!(color, COLOR_ERROR);
    }

    #[test]
    fn validation_status_color_warnings_yellow() {
        let color = validation_status_color(0, 1);
        assert_eq!(color, COLOR_WARNING);
        let color = validation_status_color(0, 10);
        assert_eq!(color, COLOR_WARNING);
    }

    #[test]
    fn validation_status_color_zero_white() {
        let color = validation_status_color(0, 0);
        assert_eq!(color, TEXT_COLOR);
    }

    #[test]
    fn format_validation_status_values() {
        assert_eq!(format_validation_status(0, 0), "VK E:0 W:0");
        assert_eq!(format_validation_status(3, 5), "VK E:3 W:5");
        assert_eq!(format_validation_status(1, 0), "VK E:1 W:0");
    }
}

/// Wall-clock values (FPS, memory) are skipped under a fixed step so a reproducible frame stays identical.
fn build_status_bar(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    if world.resource::<FrameClock>().is_fixed() {
        return;
    }

    let frame_ms = world
        .get_resource::<CpuFrameTimings>()
        .map(|timings| timings.dt_ms)
        .unwrap_or(0.0);
    let delta_time = (frame_ms / 1000.0).max(0.001);
    let gpu_ms = world
        .get_resource::<GpuPassTimings>()
        .and_then(|timings| timings.frame_total_ms);
    let viewport = world.resource::<ViewportInput>();
    let timeline_state = world.resource::<TimelineState>();
    let clip_duration = {
        let clip_library = world.resource::<ClipLibrary>();
        crate::ecs::systems::timeline_effective_duration(&timeline_state, &clip_library)
    };

    let mut state = world.resource_mut::<StatusBarState>();
    let (errors, warnings) = world
        .get_resource::<ValidationReport>()
        .map(|report| (report.stats.errors, report.stats.warnings))
        .unwrap_or((0, 0));
    if let Some(event) = draw_status_bar(
        ui,
        &mut state,
        delta_time,
        frame_ms,
        gpu_ms,
        &viewport,
        &timeline_state,
        clip_duration,
        errors,
        warnings,
    ) {
        world.send_command(event);
    }
}

crate::ui_window!(
    "status_bar",
    Bottom,
    1,
    init = init_window_state::<StatusBarState>,
    build = build_status_bar
);
