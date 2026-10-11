use std::time::Instant;

use crate::app::frame::FrameInput;
use crate::app::App;
use crate::vulkanr::vulkan::*;

use crate::ecs::resource::{ImGuiInputCapture, MouseInput, UiSettings};
use crate::platform::ui::theme::apply_ui_settings;

pub(crate) fn handle_redraw_requested(
    imgui: &mut imgui::Context,
    platform: &mut imgui_winit_support::WinitPlatform,
    window: &winit::window::Window,
    app: &mut App,
) {
    let dt_ms = if let Some(last) = app.last_frame_instant {
        let elapsed = last.elapsed().as_secs_f32() * 1000.0;
        app.last_frame_instant = Some(Instant::now());
        elapsed
    } else {
        app.last_frame_instant = Some(Instant::now());
        0.0
    };

    apply_changed_ui_settings(imgui, app);
    let ui = imgui.frame();

    let io = ui.io();
    {
        let mut capture = app.data.ecs_world.resource_mut::<ImGuiInputCapture>();
        capture.wants_mouse = io.want_capture_mouse();
    }

    super::input::update_mouse_input(&app.data.ecs_world, ui);

    super::ui_windows::build_ui_windows(ui, app);

    platform.prepare_render(ui, window);

    if let Err(error) = render_ui_frame(imgui, window, app, dt_ms) {
        log_error!("ImGui frame failed: {error:?}");
    }

    app.data.ecs_world.resource_mut::<MouseInput>().end_frame();
}

/// Renders the frame Dear ImGui built: its texture requests are answered before the draw
/// data is recorded.
fn render_ui_frame(
    imgui: &mut imgui::Context,
    window: &winit::window::Window,
    app: &mut App,
    dt_ms: f32,
) -> anyhow::Result<()> {
    let imgui_build_start = Instant::now();
    let consumer = app
        .data
        .imgui
        .renderer_consumer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ImGui renderer consumer is not initialized"))?;
    let pending = imgui.render(consumer);
    let feedback = unsafe { app.apply_imgui_texture_requests(pending.texture_requests())? };
    let frame = pending.reconcile_texture_feedback(feedback)?;
    let imgui_build_ms = imgui_build_start.elapsed().as_secs_f32() * 1000.0;

    unsafe {
        render_frame(app, window, &frame, dt_ms, imgui_build_ms);
    }
    Ok(())
}

fn apply_changed_ui_settings(imgui: &mut imgui::Context, app: &mut App) {
    let settings = *app.data.ecs_world.resource::<UiSettings>();
    if app.applied_ui_settings == Some(settings) {
        return;
    }

    apply_ui_settings(imgui, &settings);
    app.applied_ui_settings = Some(settings);
}

unsafe fn render_frame(
    app: &mut App,
    window: &winit::window::Window,
    draw_data: &imgui::DrawData,
    dt_ms: f32,
    imgui_build_ms: f32,
) {
    let frame_result = app.drive_frame(
        FrameInput {
            draw_data,
            dt_ms,
            imgui_build_ms,
        },
        super::file_dialog::queue_file_dialog_commands,
    );

    if let Err(e) = frame_result {
        let msg = e.to_string();
        if msg.contains("SWAPCHAIN_OUT_OF_DATE") {
            if let Err(e) = app.recreate_swapchain(window) {
                log_error!("Failed to recreate swapchain: {:?}", e);
                return;
            }
            // Write swapchain_recreate marker event to exposure dump sink if it exists
            if let Some(sink) = app
                .data
                .ecs_world
                .get_resource::<crate::ecs::resource::ExposureDumpSink>()
            {
                let frame = app.resource::<crate::ecs::resource::FrameClock>().frame;
                use std::fs::OpenOptions;
                use std::io::Write;
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&sink.path)
                {
                    let _ = writeln!(
                        file,
                        "{{\"event\":\"swapchain_recreate\",\"frame\":{}}}",
                        frame
                    );
                }
            }
        } else {
            log_error!("Frame error: {:?}", e);
        }
    }
}
