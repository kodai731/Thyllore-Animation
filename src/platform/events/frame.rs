use std::time::Instant;

use crate::app::frame::FrameInput;
use crate::app::App;
#[cfg(debug_assertions)]
use crate::platform::ui::{build_click_debug_overlay, DebugWindowState};
use crate::platform::ui::{SceneOverlayState, StatusBarState};
use crate::vulkanr::vulkan::*;

use crate::ecs::resource::{ImGuiInputCapture, MouseInput};

pub(crate) fn handle_redraw_requested(
    imgui: &mut imgui::Context,
    platform: &mut imgui_winit_support::WinitPlatform,
    window: &winit::window::Window,
    app: &mut App,
    status_bar_state: &mut StatusBarState,
    #[cfg(feature = "auto-rig")]
    text_to_mesh_dialog: &mut crate::platform::ui::TextToMeshDialogState,
    #[cfg(feature = "auto-rig")]
    text_to_animation_dialog: &mut crate::platform::ui::TextToAnimationDialogState,
) {
    let dt_ms = if let Some(last) = app.last_frame_instant {
        let elapsed = last.elapsed().as_secs_f32() * 1000.0;
        app.last_frame_instant = Some(Instant::now());
        elapsed
    } else {
        app.last_frame_instant = Some(Instant::now());
        0.0
    };

    let ui = imgui.frame();

    let io = ui.io();
    {
        let mut capture = app.data.ecs_world.resource_mut::<ImGuiInputCapture>();
        capture.wants_mouse = io.want_capture_mouse;
    }

    super::input::update_mouse_input(&app.data.ecs_world, ui);

    #[cfg(debug_assertions)]
    let mut debug_state = DebugWindowState {
        debug_view_mode: app
            .resource::<crate::ecs::resource::DebugViewState>()
            .debug_view_mode,
    };

    let mut overlay_state = SceneOverlayState {
        model: app.resource::<crate::ecs::ModelState>().clone(),
        viewport: Default::default(),
        #[cfg(feature = "auto-rig")]
        open_text_to_mesh_dialog: false,
        #[cfg(feature = "auto-rig")]
        open_text_to_animation_dialog: false,
    };

    super::ui_windows::build_ui_windows(
        ui,
        app,
        #[cfg(debug_assertions)]
        &mut debug_state,
        &mut overlay_state,
        status_bar_state,
        #[cfg(feature = "auto-rig")]
        text_to_mesh_dialog,
        #[cfg(feature = "auto-rig")]
        text_to_animation_dialog,
    );

    *app.resource_mut::<crate::ecs::ModelState>() = overlay_state.model;

    #[cfg(debug_assertions)]
    {
        app.resource_mut::<crate::ecs::resource::DebugViewState>()
            .debug_view_mode = debug_state.debug_view_mode;
    }

    #[cfg(debug_assertions)]
    build_click_debug_overlay(ui, &app.data.ecs_world);

    platform.prepare_render(ui, window);

    let imgui_build_start = Instant::now();
    let draw_data = imgui.render();
    let imgui_build_ms = imgui_build_start.elapsed().as_secs_f32() * 1000.0;

    unsafe {
        render_frame(app, window, draw_data, dt_ms, imgui_build_ms);
    }

    app.data.ecs_world.resource_mut::<MouseInput>().end_frame();
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
        super::file_dialog::open_file_dialogs,
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
