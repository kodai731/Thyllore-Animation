use std::time::{Duration, Instant};

use anyhow::Result;
use thyllore_vulkan_core::core::RRDevice;

use crate::app::render::BegunFrame;
use crate::app::App;
use crate::ecs::resource::CpuFrameTimings;
use crate::ecs::systems::phases::{run_event_dispatch_phase, run_last_phase};

pub struct FrameInput<'a> {
    pub draw_data: &'a imgui::DrawData,
    pub dt_ms: f32,
    pub imgui_build: Duration,
    pub stopwatch: FrameStopwatch,
}

/// Splits a frame into consecutive stages and separates each stage's device idle waits from its CPU time.
pub struct FrameStopwatch {
    stage_start: Instant,
    device_idle: Duration,
}

impl FrameStopwatch {
    pub fn start(rrdevice: &RRDevice) -> Self {
        Self {
            device_idle: rrdevice.take_idle_wait(),
            stage_start: Instant::now(),
        }
    }

    /// Ends the current stage and returns its CPU time: elapsed minus `measured_wait` and the device idle waits.
    pub fn lap(&mut self, rrdevice: &RRDevice, measured_wait: Duration) -> Duration {
        let stage_device_idle = rrdevice.take_idle_wait();
        let stage_elapsed = self.stage_start.elapsed();

        self.device_idle += stage_device_idle;
        self.stage_start = Instant::now();
        stage_elapsed.saturating_sub(measured_wait + stage_device_idle)
    }

    fn finish(self, rrdevice: &RRDevice) -> Duration {
        self.device_idle + rrdevice.take_idle_wait()
    }
}

fn to_ms(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

impl App {
    /// Runs one frame; the phase order is documented in `.claude/rules/ecs-architecture.md` (Phase Pipeline).
    pub unsafe fn drive_frame(
        &mut self,
        input: FrameInput<'_>,
        queue_file_dialog_commands: impl FnOnce(&App),
    ) -> Result<()> {
        let FrameInput {
            draw_data,
            dt_ms,
            imgui_build,
            mut stopwatch,
        } = input;
        let mut timings = CpuFrameTimings {
            dt_ms,
            imgui_vtx: draw_data.total_vtx_count() as u32,
            imgui_idx: draw_data.total_idx_count() as u32,
            ..Default::default()
        };
        timings.compute.imgui_build_ms = to_ms(imgui_build);

        let frame_result = self.run_frame_stages(
            draw_data,
            &mut stopwatch,
            &mut timings,
            queue_file_dialog_commands,
        );

        timings.frame = self.frame as u64;
        timings.wait.device_idle_ms = to_ms(stopwatch.finish(&self.rrdevice));
        self.data.ecs_world.insert_resource(timings);
        frame_result
    }

    unsafe fn run_frame_stages(
        &mut self,
        draw_data: &imgui::DrawData,
        stopwatch: &mut FrameStopwatch,
        timings: &mut CpuFrameTimings,
        queue_file_dialog_commands: impl FnOnce(&App),
    ) -> Result<()> {
        self.dispatch_ui_events(queue_file_dialog_commands);
        timings.compute.event_dispatch_ms = to_ms(stopwatch.lap(&self.rrdevice, Duration::ZERO));

        let BegunFrame {
            image_index,
            gpu_wait,
        } = self.begin_frame()?;
        timings.wait.gpu_wait_ms = to_ms(gpu_wait);
        timings.compute.frame_begin_ms = to_ms(stopwatch.lap(&self.rrdevice, gpu_wait));

        self.update(image_index)?;
        timings.compute.update_ms = to_ms(stopwatch.lap(&self.rrdevice, Duration::ZERO));

        let render_result = self.render(image_index, draw_data);
        let present_wait = *render_result.as_ref().unwrap_or(&Duration::ZERO);
        timings.wait.present_ms = to_ms(present_wait);
        timings.compute.render_cpu_ms = to_ms(stopwatch.lap(&self.rrdevice, present_wait));
        render_result?;

        run_last_phase(&self.data.ecs_world, |slot| {
            self.capture_context(image_index, slot)
        })?;
        timings.compute.last_phase_ms = to_ms(stopwatch.lap(&self.rrdevice, Duration::ZERO));

        Ok(())
    }

    unsafe fn dispatch_ui_events(&mut self, queue_file_dialog_commands: impl FnOnce(&App)) {
        run_event_dispatch_phase(
            &mut self.data.ecs_world,
            &mut self.data.ecs_assets,
            &self.data.graphics_resources,
        );
        queue_file_dialog_commands(self);

        self.apply_app_commands();
    }
}
