use cgmath::Vector3;
use imgui::Condition;

use crate::app::data::LightMoveTarget;
use crate::app::GUIData;
use crate::debugview::{DebugViewMode, FBX_DEBUG};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::World;

pub struct DebugWindowState {
    pub model_path: String,
    pub load_status: String,
    pub light_position: Vector3<f32>,
    pub shadow_strength: f32,
    pub enable_distance_attenuation: bool,
    pub debug_view_mode: DebugViewMode,
}

pub fn build_debug_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut DebugWindowState,
    gui_data: &mut GUIData,
    ecs_world: &World,
) {
    let display_size = ui.io().display_size;
    let debug_height = 250.0;
    let debug_y = display_size[1] - debug_height;

    ui.window("debug window")
        .position([0.0, debug_y], Condition::Always)
        .size([display_size[0], debug_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            build_model_panel(ui, ui_events, state, gui_data);
            ui.separator();

            build_camera_panel(ui, ui_events);
            ui.separator();

            build_screenshot_panel(ui, ui_events);
            ui.separator();

            build_raytracing_panel(ui, ui_events, state);
            ui.separator();

            build_debug_panel(ui, ui_events, gui_data);
            ui.separator();

            build_fbx_debug_panel(ui);
            ui.separator();

            build_light_bounds_panel(ui, ui_events);
            ui.separator();

            build_dof_panel(ui, ecs_world);
            ui.separator();

            build_auto_exposure_panel(ui, ecs_world);

            build_mouse_info(ui, gui_data);
        });
}

fn build_model_panel(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &DebugWindowState,
    _gui_data: &mut GUIData,
) {
    ui.text("Model Loading:");

    if ui.button("Open FBX Model") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("FBX Files", &["fbx"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            crate::log!("Selected FBX file: {}", path_str);
            ui_events.send(UIEvent::LoadModel { path: path_str });
        }
    }

    ui.same_line();

    if ui.button("Open glTF Model") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF Files", &["gltf", "glb"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            crate::log!("Selected glTF file: {}", path_str);
            ui_events.send(UIEvent::LoadModel { path: path_str });
        }
    }

    ui.text(format!(
        "Current Model: {}",
        if state.model_path.is_empty() {
            "None"
        } else {
            &state.model_path
        }
    ));

    ui.text(format!("Status: {}", state.load_status));
}

fn build_camera_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue) {
    ui.text("Camera Controls:");

    if ui.button("reset camera") {
        ui_events.send(UIEvent::ResetCamera);
    }

    ui.same_line();

    if ui.button("reset camera up") {
        ui_events.send(UIEvent::ResetCameraUp);
    }

    if ui.button("move to light gizmo") {
        ui_events.send(UIEvent::MoveCameraToLightGizmo);
    }

    if ui.button("move to model") {
        ui_events.send(UIEvent::MoveCameraToModel);
    }
}

fn build_screenshot_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue) {
    ui.text("Screenshot:");

    if ui.button("Take Screenshot") {
        ui_events.send(UIEvent::TakeScreenshot);
    }
}

fn build_raytracing_panel(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut DebugWindowState,
) {
    ui.text("Ray Tracing Controls:");

    let mut light_pos = [
        state.light_position.x,
        state.light_position.y,
        state.light_position.z,
    ];

    let mut light_changed = false;

    if ui
        .slider_config("Light X", -50.0, 50.0)
        .build(&mut light_pos[0])
    {
        light_changed = true;
    }

    if ui
        .slider_config("Light Y", -50.0, 50.0)
        .build(&mut light_pos[1])
    {
        light_changed = true;
    }

    if ui
        .slider_config("Light Z", -50.0, 50.0)
        .build(&mut light_pos[2])
    {
        light_changed = true;
    }

    if light_changed {
        let new_pos = Vector3::new(light_pos[0], light_pos[1], light_pos[2]);
        state.light_position = new_pos;
        ui_events.send(UIEvent::SetLightPosition(new_pos));
    }

    ui.slider_config("Shadow Strength", 0.0, 1.0)
        .build(&mut state.shadow_strength);

    ui.checkbox(
        "Distance Attenuation",
        &mut state.enable_distance_attenuation,
    );

    ui.text("Debug View Mode:");
    let mut current_mode = state.debug_view_mode.as_int();

    if ui.radio_button("Final (Lit + Shadow)", &mut current_mode, 0) {
        state.debug_view_mode = DebugViewMode::Final;
    }
    if ui.radio_button("Position (World Space)", &mut current_mode, 1) {
        state.debug_view_mode = DebugViewMode::Position;
    }
    if ui.radio_button("Normal (World Space)", &mut current_mode, 2) {
        state.debug_view_mode = DebugViewMode::Normal;
    }
    if ui.radio_button("Shadow Mask", &mut current_mode, 3) {
        state.debug_view_mode = DebugViewMode::ShadowMask;
    }
    if ui.radio_button("N dot L (Green=Lit, Red=Back)", &mut current_mode, 4) {
        state.debug_view_mode = DebugViewMode::NdotL;
    }
    if ui.radio_button("Light Direction", &mut current_mode, 5) {
        state.debug_view_mode = DebugViewMode::LightDirection;
    }
    if ui.radio_button("View Depth (Green=GBuffer depth)", &mut current_mode, 6) {
        state.debug_view_mode = DebugViewMode::ViewDepth;
    }
    if ui.radio_button("ObjectID (Color per ID)", &mut current_mode, 7) {
        state.debug_view_mode = DebugViewMode::ObjectID;
    }
    if ui.radio_button("Selection View (Orange=Selected)", &mut current_mode, 8) {
        state.debug_view_mode = DebugViewMode::SelectionView;
    }
    if ui.radio_button("SelectionUBO (R=count, G=id0)", &mut current_mode, 9) {
        state.debug_view_mode = DebugViewMode::SelectionUBO;
    }
}

fn build_debug_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue, gui_data: &mut GUIData) {
    ui.text("Debug Info:");

    ui.checkbox("Show Click Debug", &mut gui_data.show_click_debug);
    ui.checkbox(
        "Show Light Ray to Model",
        &mut gui_data.show_light_ray_to_model,
    );

    if ui.button("Debug Shadow Info") {
        ui_events.send(UIEvent::DebugShadowInfo);
    }

    ui.same_line();

    if ui.button("Debug Billboard Depth") {
        ui_events.send(UIEvent::DebugBillboardDepth);
    }

    if ui.button("Dump Debug Information") {
        ui_events.send(UIEvent::DumpDebugInfo);
    }

    if ui.button("Add Test Constraints") {
        ui_events.send(UIEvent::CreateTestConstraints);
    }
    ui.same_line();
    if ui.button("Clear Constraints") {
        ui_events.send(UIEvent::ClearTestConstraints);
    }

    if ui.button("Add Spring Bones") {
        ui_events.send(UIEvent::AddTestSpringBones);
    }
    ui.same_line();
    if ui.button("Clear Spring Bones") {
        ui_events.send(UIEvent::ClearSpringBones);
    }
}

fn build_fbx_debug_panel(ui: &imgui::Ui) {
    ui.text("FBX Debug Logs:");

    let mut fbx_anim = FBX_DEBUG.animation_enabled();
    let mut fbx_hier = FBX_DEBUG.hierarchy_enabled();
    let mut fbx_skin = FBX_DEBUG.skinning_enabled();
    let mut fbx_trans = FBX_DEBUG.transform_enabled();

    if ui.checkbox("Animation", &mut fbx_anim) {
        FBX_DEBUG.set_animation(fbx_anim);
    }
    if ui.checkbox("Hierarchy", &mut fbx_hier) {
        FBX_DEBUG.set_hierarchy(fbx_hier);
    }
    if ui.checkbox("Skinning", &mut fbx_skin) {
        FBX_DEBUG.set_skinning(fbx_skin);
    }
    if ui.checkbox("Transform", &mut fbx_trans) {
        FBX_DEBUG.set_transform(fbx_trans);
    }
}

fn build_light_bounds_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue) {
    ui.text("Move Light to Model Bounds:");

    if ui.button("X Min") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::XMin));
    }
    ui.same_line();
    if ui.button("X Max") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::XMax));
    }

    if ui.button("Y Min") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::YMin));
    }
    ui.same_line();
    if ui.button("Y Max") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::YMax));
    }

    if ui.button("Z Min") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::ZMin));
    }
    ui.same_line();
    if ui.button("Z Max") {
        ui_events.send(UIEvent::MoveLightToBounds(LightMoveTarget::ZMax));
    }
}

fn build_dof_panel(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{DepthOfField, PhysicalCameraParameters};

    ui.text("Depth of Field:");

    if let Some(mut dof) = ecs_world.get_resource_mut::<DepthOfField>() {
        ui.checkbox("DOF Enabled", &mut dof.enabled);

        ui.slider_config("Focus Distance", 0.1, 100.0)
            .build(&mut dof.focus_distance);

        ui.slider_config("Max Blur Radius", 1.0, 32.0)
            .build(&mut dof.max_blur_radius);
    }

    if let Some(mut params) = ecs_world.get_resource_mut::<PhysicalCameraParameters>() {
        ui.slider_config("Aperture (f-stops)", 1.0, 22.0)
            .build(&mut params.aperture_f_stops);

        ui.slider_config("Focal Length (mm)", 10.0, 200.0)
            .build(&mut params.focal_length_mm);
    }
}

fn build_auto_exposure_panel(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{AutoExposure, Exposure};

    ui.text("Auto Exposure:");

    if let Some(mut ae) =
        ecs_world.get_resource_mut::<AutoExposure>()
    {
        ui.checkbox("Auto Exposure Enabled", &mut ae.enabled);

        ui.slider_config("Min EV", -10.0, 10.0)
            .build(&mut ae.min_ev);

        ui.slider_config("Max EV", 0.0, 30.0)
            .build(&mut ae.max_ev);

        ui.slider_config("Speed Up", 0.1, 10.0)
            .build(&mut ae.adaptation_speed_up);

        ui.slider_config("Speed Down", 0.1, 10.0)
            .build(&mut ae.adaptation_speed_down);

        ui.slider_config("Low Percent", 0.0, 0.5)
            .build(&mut ae.low_percent);

        ui.slider_config("High Percent", 0.5, 1.0)
            .build(&mut ae.high_percent);
    }

    if let Some(exposure) = ecs_world.get_resource::<Exposure>() {
        ui.text(format!(
            "Current Exposure: {:.4}",
            exposure.exposure_value
        ));
        ui.text(format!("Current EV100: {:.2}", exposure.ev100));
    }
}

fn build_mouse_info(ui: &imgui::Ui, gui_data: &mut GUIData) {
    ui.text(format!(
        "Mouse Position: ({:.1},{:.1})",
        gui_data.mouse_pos[0], gui_data.mouse_pos[1]
    ));
    ui.text(format!("is left clicked: ({:.1})", gui_data.is_left_clicked));
    ui.text(format!(
        "is wheel clicked: ({:.1})",
        gui_data.is_wheel_clicked
    ));
    ui.input_text("file path", &mut gui_data.file_path)
        .read_only(true)
        .build();
}

pub fn build_click_debug_overlay(ui: &imgui::Ui, gui_data: &GUIData) {
    if !gui_data.show_click_debug {
        return;
    }

    static mut IMGUI_SIZE_LOGGED: bool = false;
    unsafe {
        if !IMGUI_SIZE_LOGGED {
            let display_size = ui.io().display_size;
            crate::log!(
                "ImGui display size: {:.1} x {:.1}",
                display_size[0],
                display_size[1]
            );
            IMGUI_SIZE_LOGGED = true;
        }
    }

    if let Some(rect) = gui_data.billboard_click_rect {
        let draw_list = ui.get_foreground_draw_list();
        draw_list
            .add_rect([rect[0], rect[1]], [rect[2], rect[3]], [1.0, 0.0, 0.0, 0.8])
            .filled(true)
            .build();
        draw_list
            .add_rect([rect[0], rect[1]], [rect[2], rect[3]], [1.0, 1.0, 0.0, 1.0])
            .thickness(2.0)
            .build();
    }
}
