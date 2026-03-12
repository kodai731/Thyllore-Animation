use crate::app::GUIData;
use crate::debugview::DebugViewMode;
use crate::ecs::events::UIEventQueue;
use crate::ecs::resource::GridMeshData;
use crate::ecs::World;

pub struct DebugWindowState {
    pub debug_view_mode: DebugViewMode,
}

pub fn build_debug_panel_content(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut DebugWindowState,
    gui_data: &mut GUIData,
    ecs_world: &World,
) {
    build_debug_view_mode_panel(ui, state);
    ui.separator();

    build_debug_panel(ui, ui_events, gui_data, ecs_world);
    ui.separator();

    build_grid_debug_panel(ui, ui_events, ecs_world);
    ui.separator();

    build_fbx_debug_panel(ui);
    ui.separator();

    build_mouse_info(ui, gui_data);
}

fn build_debug_view_mode_panel(ui: &imgui::Ui, state: &mut DebugWindowState) {
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

fn build_debug_panel(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    gui_data: &mut GUIData,
    ecs_world: &World,
) {
    ui.text("Debug Info:");

    ui.checkbox("Show Click Debug", &mut gui_data.show_click_debug);
    ui.checkbox(
        "Show Light Ray to Model",
        &mut gui_data.show_light_ray_to_model,
    );

    if ui.button("Debug Shadow Info") {
        ui_events.send(crate::ecs::events::UIEvent::DebugShadowInfo);
    }

    ui.same_line();

    if ui.button("Debug Billboard Depth") {
        ui_events.send(crate::ecs::events::UIEvent::DebugBillboardDepth);
    }

    if ui.button("Dump Debug Information") {
        ui_events.send(crate::ecs::events::UIEvent::DumpDebugInfo);
    }

    ui.same_line();

    if ui.button("Dump Animation Debug") {
        ui_events.send(crate::ecs::events::UIEvent::DumpAnimationDebug);
    }

    if ui.button("Add Test Constraints") {
        ui_events.send(crate::ecs::events::UIEvent::CreateTestConstraints);
    }
    ui.same_line();
    if ui.button("Clear Constraints") {
        ui_events.send(crate::ecs::events::UIEvent::ClearTestConstraints);
    }

    if ui.button("Add Spring Bones") {
        ui_events.send(crate::ecs::events::UIEvent::AddTestSpringBones);
    }
    ui.same_line();
    if ui.button("Clear Spring Bones") {
        ui_events.send(crate::ecs::events::UIEvent::ClearSpringBones);
    }

    build_spring_bone_bake_panel(ui, ui_events, ecs_world);
}

fn build_spring_bone_bake_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::events::UIEvent;
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    let Some(state) = ecs_world.get_resource::<SpringBoneState>() else {
        return;
    };

    ui.separator();
    match state.mode {
        SpringBoneMode::Realtime => {
            ui.text("Spring Bone: Realtime");
            ui.text_colored([0.5, 0.8, 0.5, 1.0], "  Simulating...");
            if ui.button("Bake Spring Bones") {
                ui_events.send(UIEvent::SpringBoneBake);
            }
        }
        SpringBoneMode::Baked => {
            let clip_id = state.baked_clip_source_id.unwrap_or(0);
            ui.text(format!("Spring Bone: Baked (clip_id={})", clip_id));
            ui.text_colored(
                [0.7, 0.7, 0.5, 1.0],
                "  Editing will switch to BakedOverride",
            );
            if ui.button("Discard Bake") {
                ui_events.send(UIEvent::SpringBoneDiscardBake);
            }
            ui.same_line();
            if ui.button("Save Bake (.ron)") {
                ui_events.send(UIEvent::SpringBoneSaveBake);
            }
        }
        SpringBoneMode::BakedOverride => {
            let clip_id = state.baked_clip_source_id.unwrap_or(0);
            ui.text(format!("Spring Bone: BakedOverride (clip_id={})", clip_id));
            ui.text_colored([1.0, 0.7, 0.3, 1.0], "  Manually edited");
            if ui.button("Re-bake") {
                ui_events.send(UIEvent::SpringBoneRebake);
            }
            ui.same_line();
            if ui.button("Discard Bake") {
                ui_events.send(UIEvent::SpringBoneDiscardBake);
            }
            ui.same_line();
            if ui.button("Save Bake (.ron)") {
                ui_events.send(UIEvent::SpringBoneSaveBake);
            }
        }
    }
}

fn build_grid_debug_panel(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::events::UIEvent;

    ui.text("Grid:");
    if let Some(grid) = ecs_world.get_resource::<GridMeshData>() {
        let mut show_y = grid.show_y_axis_grid;
        if ui.checkbox("Show Y-Axis Grid", &mut show_y) {
            ui_events.send(UIEvent::SetGridShowYAxis(show_y));
        }
    }
}

fn build_fbx_debug_panel(ui: &imgui::Ui) {
    use crate::debugview::FBX_DEBUG;

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

fn build_mouse_info(ui: &imgui::Ui, gui_data: &mut GUIData) {
    ui.text(format!(
        "Mouse Position: ({:.1},{:.1})",
        gui_data.mouse_pos[0], gui_data.mouse_pos[1]
    ));
    ui.text(format!(
        "is left clicked: ({:.1})",
        gui_data.is_left_clicked
    ));
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
