use std::time::Instant;

use anyhow::Result;

use crate::app::App;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{AppCommand, AppCommandQueue, CpuFrameTimings};
use crate::ecs::systems::phases::{run_event_dispatch_phase, run_last_phase};

pub struct FrameInput<'a> {
    pub draw_data: &'a imgui::DrawData,
    pub dt_ms: f32,
    pub imgui_build_ms: f32,
}

impl App {
    /// Runs one frame; the phase order is documented in `.claude/rules/ecs-architecture.md` (Phase Pipeline).
    pub unsafe fn drive_frame(
        &mut self,
        input: FrameInput<'_>,
        open_file_dialogs: impl FnOnce(&[UIEvent], &App) -> Vec<AppCommand>,
    ) -> Result<()> {
        self.dispatch_ui_events(open_file_dialogs);

        let gpu_wait_start = Instant::now();
        let image_index = self.begin_frame()?;
        let gpu_wait_ms = gpu_wait_start.elapsed().as_secs_f32() * 1000.0;

        let update_start = Instant::now();
        self.update(image_index)?;
        let update_ms = update_start.elapsed().as_secs_f32() * 1000.0;

        let render_cpu_start = Instant::now();
        self.render(image_index, input.draw_data)?;
        let render_cpu_ms = render_cpu_start.elapsed().as_secs_f32() * 1000.0;

        run_last_phase(&self.data.ecs_world, |slot| {
            self.capture_context(image_index, slot)
        })?;

        self.data.ecs_world.insert_resource(CpuFrameTimings {
            frame: self.frame as u64,
            dt_ms: input.dt_ms,
            stages: vec![
                ("imgui_build".to_string(), input.imgui_build_ms),
                ("gpu_wait".to_string(), gpu_wait_ms),
                ("update".to_string(), update_ms),
                ("render_cpu".to_string(), render_cpu_ms),
            ],
            imgui_vtx: input.draw_data.total_vtx_count as u32,
            imgui_idx: input.draw_data.total_idx_count as u32,
        });
        Ok(())
    }

    unsafe fn dispatch_ui_events(
        &mut self,
        open_file_dialogs: impl FnOnce(&[UIEvent], &App) -> Vec<AppCommand>,
    ) {
        let model_bounds = self.data.graphics_resources.calculate_model_bounds();
        let (file_events, mut commands) = run_event_dispatch_phase(
            &mut self.data.ecs_world,
            &mut self.data.ecs_assets,
            model_bounds,
        );
        commands.extend(open_file_dialogs(&file_events, self));

        self.data
            .ecs_world
            .resource_mut::<AppCommandQueue>()
            .commands
            .append(&mut commands);
        self.apply_app_commands();
    }
}
