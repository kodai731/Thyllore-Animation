use imgui::{Condition, StyleVar};
use thyllore_anim_core::editable::PropertyType;

use super::pointer::read_ui_pointer;
#[cfg(feature = "auto-rig")]
use super::text_to_animation_dialog::TextToAnimationDialogState;
#[cfg(feature = "auto-rig")]
use super::text_to_mesh_dialog::TextToMeshDialogState;
use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::ViewportInput;
use crate::ecs::resource::{
    CoordinateSpace, PanelVisibility, TransformGizmoMode, TransformGizmoState, UiWidgetState,
    WeightHeatmapState,
};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
#[cfg(feature = "auto-rig")]
use crate::ecs::systems::phases::event_dispatch::ml::auto_rig::AutoRigEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::World;
use crate::platform::ui::theme::colors::{srgb_to_linear, SURFACE3};
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::shadow::draw_window_shadow;
use crate::platform::ui::theme::SectionDefault;
use crate::platform::ui::theme::{icon_button, ButtonState, Icon, ICON_BUTTON_SIZE};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::param_widgets::EditedScalars;

const OVERLAY_MARGIN: f32 = 8.0;
const OVERLAY_WIDTH: f32 = 420.0;
const TOOLBAR_HEIGHT: f32 = 28.0;
const TOOLBAR_PADDING_X: f32 = 6.0;
const TOOLBAR_GROUP_GAP: f32 = 12.0;

pub(super) struct GizmoKeyBinding {
    pub(super) key: imgui::Key,
    pub(super) label: &'static str,
    mode: TransformGizmoMode,
}

pub(super) const GIZMO_KEY_BINDINGS: &[GizmoKeyBinding] = &[
    GizmoKeyBinding {
        key: imgui::Key::W,
        label: "Translate",
        mode: TransformGizmoMode::Translate,
    },
    GizmoKeyBinding {
        key: imgui::Key::E,
        label: "Rotate",
        mode: TransformGizmoMode::Rotate,
    },
    GizmoKeyBinding {
        key: imgui::Key::R,
        label: "Scale",
        mode: TransformGizmoMode::Scale,
    },
];

#[cfg(feature = "auto-rig")]
use crate::ecs::resource::{AutoRigState, AutoRigStatus};

fn draw_scene_toolbar(ui: &imgui::Ui, ecs_world: &World, viewport: &ViewportInput) {
    let _window_padding = ui.push_style_var(StyleVar::WindowPadding([
        TOOLBAR_PADDING_X,
        (TOOLBAR_HEIGHT - ICON_BUTTON_SIZE) * 0.5,
    ]));

    ui.window("Scene Toolbar")
        .position(viewport.position, Condition::Always)
        .size([viewport.size[0], TOOLBAR_HEIGHT], Condition::Always)
        .no_decoration()
        .bg_alpha(0.7)
        .no_nav()
        .focus_on_appearing(false)
        .save_settings(false)
        .build(|| {
            draw_window_shadow(ui, ui.clone_style().window_rounding);

            build_file_buttons(ui, ecs_world);
            draw_toolbar_group_separator(ui);
            build_gizmo_buttons(ui, ecs_world);

            #[cfg(feature = "auto-rig")]
            build_auto_rig_buttons(ui, ecs_world);

            build_scene_panel_toggle(ui, ecs_world);
        });

    apply_gizmo_hotkeys(ui, ecs_world);
}

fn draw_scene_panel(ui: &imgui::Ui, ecs_world: &World, viewport: &ViewportInput) {
    if ecs_world.resource::<UiWidgetState>().scene_panel == PanelVisibility::Hidden {
        return;
    }

    let pos_x = viewport.position[0] + viewport.size[0] - OVERLAY_WIDTH - OVERLAY_MARGIN;
    let pos_y = viewport.position[1] + TOOLBAR_HEIGHT + OVERLAY_MARGIN;

    ui.window("Scene Panel")
        .position([pos_x, pos_y], Condition::Always)
        .size_constraints(
            [OVERLAY_WIDTH, 0.0],
            [
                OVERLAY_WIDTH,
                viewport.size[1] - TOOLBAR_HEIGHT - 2.0 * OVERLAY_MARGIN,
            ],
        )
        .always_auto_resize(true)
        .no_decoration()
        .bg_alpha(0.7)
        .no_nav()
        .focus_on_appearing(false)
        .save_settings(false)
        .build(|| {
            draw_window_shadow(ui, ui.clone_style().window_rounding);

            build_overlay_section(ui, ecs_world);

            build_transform_gizmo_section(ui, ecs_world);

            build_dof_section(ui, ecs_world);

            build_auto_exposure_section(ui, ecs_world);

            build_onion_skinning_section(ui, ecs_world);

            for section in crate::platform::ui::effect_sections::collect_effect_sections() {
                (section.draw)(ui, ecs_world);
            }
        });
}

fn build_file_buttons(ui: &imgui::Ui, ecs_world: &World) {
    if icon_button(ui, Icon::FolderOpen, "Open FBX", ButtonState::Normal) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("FBX Files", &["fbx"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log!("Selected FBX file: {}", path_str);
            ecs_world.send_command(CameraEvent::LoadModel { path: path_str });
        }
    }

    ui.same_line();

    if icon_button(ui, Icon::File, "Open glTF", ButtonState::Normal) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF Files", &["gltf", "glb"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log!("Selected glTF file: {}", path_str);
            ecs_world.send_command(CameraEvent::LoadModel { path: path_str });
        }
    }

    ui.same_line();
    if icon_button(ui, Icon::Plus, "Add GLB", ButtonState::Normal) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("GLB Files", &["glb"])
            .pick_files()
        {
            for path in paths {
                let path_str = path.to_string_lossy().to_string();
                log!("Adding GLB file: {}", path_str);
                ecs_world.send_command(CameraEvent::LoadModelAdditive { path: path_str });
            }
        }
    }

    ui.same_line();
    if icon_button(ui, Icon::Camera, "Screenshot", ButtonState::Normal) {
        ecs_world.send_command(CameraEvent::TakeScreenshot);
    }
}

fn draw_toolbar_group_separator(ui: &imgui::Ui) {
    ui.same_line_with_spacing(0.0, TOOLBAR_GROUP_GAP);
    let [x, y] = ui.cursor_screen_pos();
    ui.get_window_draw_list()
        .add_line([x, y], [x, y + ICON_BUTTON_SIZE], srgb_to_linear(SURFACE3))
        .build();
    ui.dummy([1.0, ICON_BUTTON_SIZE]);
    ui.same_line_with_spacing(0.0, TOOLBAR_GROUP_GAP);
}

fn select_mode_button_state(
    current: TransformGizmoMode,
    button_mode: TransformGizmoMode,
) -> ButtonState {
    if current == button_mode {
        ButtonState::Active
    } else {
        ButtonState::Normal
    }
}

fn build_gizmo_buttons(ui: &imgui::Ui, ecs_world: &World) {
    let Some(state) = ecs_world.get_resource::<TransformGizmoState>() else {
        return;
    };
    let mut state_copy = state.clone();
    drop(state);

    let mode_buttons = [
        (TransformGizmoMode::Translate, Icon::Move, "Translate (W)"),
        (TransformGizmoMode::Rotate, Icon::Rotate, "Rotate (E)"),
        (TransformGizmoMode::Scale, Icon::Scale, "Scale (R)"),
    ];
    for (index, (mode, icon, tooltip)) in mode_buttons.into_iter().enumerate() {
        if index > 0 {
            ui.same_line();
        }
        let button_state = select_mode_button_state(state_copy.mode, mode);
        if icon_button(ui, icon, tooltip, button_state) {
            ecs_world.send_command(OverlayEvent::SetTransformGizmoMode(mode));
        }
    }

    let (space_tooltip, space_state, toggled_space) = match state_copy.coordinate_space {
        CoordinateSpace::World => ("Space: World", ButtonState::Normal, CoordinateSpace::Local),
        CoordinateSpace::Local => ("Space: Local", ButtonState::Active, CoordinateSpace::World),
    };
    ui.same_line();
    if icon_button(ui, Icon::Globe, space_tooltip, space_state) {
        ecs_world.send_command(OverlayEvent::SetTransformGizmoSpace(toggled_space));
    }

    let snap_state = if state_copy.snap_enabled {
        ButtonState::Active
    } else {
        ButtonState::Normal
    };
    ui.same_line();
    if icon_button(ui, Icon::Magnet, "Snap", snap_state) {
        state_copy.snap_enabled = !state_copy.snap_enabled;
        ecs_world.send_command(OverlayEvent::UpdateTransformGizmoState(Box::new(
            state_copy,
        )));
    }
}

fn apply_gizmo_hotkeys(ui: &imgui::Ui, ecs_world: &World) {
    let gizmo_hotkeys_enabled =
        !ui.io().key_ctrl && !read_ui_pointer(ui).is_down(imgui::MouseButton::Right);
    if !gizmo_hotkeys_enabled {
        return;
    }

    for binding in GIZMO_KEY_BINDINGS {
        if ui.is_key_pressed(binding.key) {
            ecs_world.send_command(OverlayEvent::SetTransformGizmoMode(binding.mode));
        }
    }
}

fn build_scene_panel_toggle(ui: &imgui::Ui, ecs_world: &World) {
    let visibility = ecs_world.resource::<UiWidgetState>().scene_panel;
    let (button_state, toggled_visibility) = match visibility {
        PanelVisibility::Shown => (ButtonState::Active, PanelVisibility::Hidden),
        PanelVisibility::Hidden => (ButtonState::Normal, PanelVisibility::Shown),
    };

    let right_edge_x = ui.window_size()[0] - ui.clone_style().window_padding[0] - ICON_BUTTON_SIZE;
    ui.same_line_with_pos(right_edge_x);
    if icon_button(ui, Icon::Layers, "Scene Panel", button_state) {
        ecs_world.resource_mut::<UiWidgetState>().scene_panel = toggled_visibility;
    }
}

#[cfg(feature = "auto-rig")]
fn build_auto_rig_buttons(ui: &imgui::Ui, ecs_world: &World) {
    ui.same_line_with_spacing(0.0, TOOLBAR_GROUP_GAP);
    if ui.button("Generate Mesh") {
        ecs_world.resource_mut::<TextToMeshDialogState>().open = true;
    }

    ui.same_line();
    if ui.button("Generate Animation") {
        ecs_world.resource_mut::<TextToAnimationDialogState>().open = true;
    }

    ui.same_line();
    build_auto_rig_section(ui, ecs_world);
}

#[cfg(feature = "auto-rig")]
fn build_auto_rig_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::component::GlbSource;
    use crate::ecs::resource::HierarchyState;
    use crate::ecs::world::Parent;

    let auto_rig_state = ecs_world.resource::<AutoRigState>();
    let status = auto_rig_state.status.clone();
    let joint_count = auto_rig_state.joint_count;
    let bone_count = auto_rig_state.bone_count;
    let gen_time = auto_rig_state.generation_time_ms;
    let error_msg = auto_rig_state.error_message.clone();
    drop(auto_rig_state);

    match status {
        AutoRigStatus::Idle => {
            let hierarchy = ecs_world.resource::<HierarchyState>();
            let selected = hierarchy.selected_entity;
            drop(hierarchy);

            let has_glb_source = selected.map_or(false, |entity| {
                if ecs_world.get_component::<GlbSource>(entity).is_some() {
                    return true;
                }
                if let Some(Parent(parent)) = ecs_world.get_component::<Parent>(entity) {
                    return ecs_world.get_component::<GlbSource>(*parent).is_some();
                }
                false
            });

            if has_glb_source && ui.button("Auto Rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigGenerate {
                    num_sample_points: 65536,
                });
            }
        }

        AutoRigStatus::WaitingForServer => {
            ui.text("Rigging: waiting for server...");
            ui.same_line();
            if ui.button("Cancel##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }

        AutoRigStatus::Rigging => {
            ui.text("Rigging: processing...");
            ui.same_line();
            if ui.button("Cancel##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }

        AutoRigStatus::Previewing => {
            if let Some(gen_time) = gen_time {
                ui.text(format!(
                    "Preview: {} joints, {} bones ({:.1}s)",
                    joint_count.unwrap_or(0),
                    bone_count.unwrap_or(0),
                    gen_time / 1000.0
                ));
                ui.same_line();
            }
            if ui.button("Apply Rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigApply);
            }
            ui.same_line();
            if ui.button("Discard##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }

        AutoRigStatus::Error => {
            if let Some(ref msg) = error_msg {
                ui.text_colored([1.0, 0.3, 0.3, 1.0], format!("Rig error: {}", msg));
                ui.same_line();
            }
            if ui.button("Dismiss##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }
    }
}

pub(super) fn send_key_button(
    ui: &imgui::Ui,
    ecs_world: &World,
    edited: EditedScalars,
    keys: Vec<(PropertyType, f32)>,
) {
    let (Some((first_name, _)), false) = (edited.first(), keys.is_empty()) else {
        return;
    };
    ui.same_line();
    if ui.small_button(format!("K##{first_name}")) {
        for (property_type, value) in keys {
            ecs_world.send_command(ScalarCurveEvent::InsertScalarKey {
                property_type,
                value,
            });
        }
    }
}

fn build_overlay_section(ui: &imgui::Ui, ecs_world: &World) {
    if section_header(ui, ecs_world, "Overlay", SectionDefault::Open) {
        if let Some(bone_gizmo) = ecs_world.get_resource::<BoneGizmoData>() {
            let mut visible = bone_gizmo.visible;
            if crate::platform::ui::theme::toggle_switch(ui, ecs_world, "Show Bones", &mut visible)
            {
                ecs_world.send_command(OverlayEvent::SetBoneGizmoVisible(visible));
            }
        }
        if let Some(heatmap) = ecs_world.get_resource::<WeightHeatmapState>() {
            let mut enabled = heatmap.enabled;
            if crate::platform::ui::theme::toggle_switch(
                ui,
                ecs_world,
                "Show Weight Heatmap (selected bone)",
                &mut enabled,
            ) {
                ecs_world.send_command(OverlayEvent::SetWeightHeatmapEnabled(enabled));
            }
        }
    }
}

fn build_transform_gizmo_section(ui: &imgui::Ui, ecs_world: &World) {
    let Some(state) = ecs_world.get_resource::<TransformGizmoState>() else {
        return;
    };
    let mut state_copy = state.clone();
    drop(state);

    if section_header(ui, ecs_world, "Transform Gizmo", SectionDefault::Open) {
        let snap_edited = state_copy.snap_enabled
            && match state_copy.mode {
                TransformGizmoMode::Translate => ui
                    .slider_config("Snap Value", 0.01, 10.0)
                    .build(&mut state_copy.translate_snap_value),
                TransformGizmoMode::Rotate => ui
                    .slider_config("Snap Degrees", 1.0, 90.0)
                    .build(&mut state_copy.rotate_snap_degrees),
                TransformGizmoMode::Scale => ui
                    .slider_config("Snap Value", 0.01, 1.0)
                    .build(&mut state_copy.scale_snap_value),
            };

        let scale_edited = ui
            .slider_config("Gizmo Scale", 0.01, 0.3)
            .display_format("%.3f")
            .build(&mut state_copy.gizmo_scale);

        if snap_edited || scale_edited {
            ecs_world.send_command(OverlayEvent::UpdateTransformGizmoState(Box::new(
                state_copy,
            )));
        }
    }
}

fn build_dof_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{DepthOfField, PhysicalCameraParameters};

    if section_header(ui, ecs_world, "Depth of Field", SectionDefault::Closed) {
        if let Some(dof) = ecs_world.get_resource::<DepthOfField>() {
            let mut dof_copy = dof.clone();
            drop(dof);

            crate::platform::ui::theme::toggle_switch(
                ui,
                ecs_world,
                "DOF Enabled",
                &mut dof_copy.enabled,
            );

            ui.slider_config("Focus Distance", 0.1, 100.0)
                .build(&mut dof_copy.focus_distance);

            ui.slider_config("Max Blur Radius", 1.0, 32.0)
                .build(&mut dof_copy.max_blur_radius);

            ecs_world.send_command(OverlayEvent::UpdateDepthOfField(dof_copy));
        }

        if let Some(params) = ecs_world.get_resource::<PhysicalCameraParameters>() {
            let mut params_copy = params.clone();
            drop(params);

            ui.slider_config("Aperture (f-stops)", 1.0, 22.0)
                .build(&mut params_copy.aperture_f_stops);

            ui.slider_config("Focal Length (mm)", 10.0, 200.0)
                .build(&mut params_copy.focal_length_mm);

            ecs_world.send_command(OverlayEvent::UpdatePhysicalCamera(params_copy));
        }
    }
}

fn build_auto_exposure_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{AutoExposure, Exposure};

    if section_header(ui, ecs_world, "Auto Exposure", SectionDefault::Closed) {
        if let Some(ae) = ecs_world.get_resource::<AutoExposure>() {
            let mut ae_copy = ae.clone();
            drop(ae);

            crate::platform::ui::theme::toggle_switch(
                ui,
                ecs_world,
                "Auto Exposure Enabled",
                &mut ae_copy.enabled,
            );

            ui.slider_config("Min EV", -10.0, 10.0)
                .build(&mut ae_copy.min_ev);

            ui.slider_config("Max EV", 0.0, 30.0)
                .build(&mut ae_copy.max_ev);

            ui.slider_config("Speed Up", 0.1, 10.0)
                .build(&mut ae_copy.adaptation_speed_up);

            ui.slider_config("Speed Down", 0.1, 10.0)
                .build(&mut ae_copy.adaptation_speed_down);

            ui.slider_config("Low Percent", 0.0, 0.5)
                .build(&mut ae_copy.low_percent);

            ui.slider_config("High Percent", 0.5, 1.0)
                .build(&mut ae_copy.high_percent);

            ecs_world.send_command(OverlayEvent::UpdateAutoExposure(ae_copy));
        }

        if let Some(exposure) = ecs_world.get_resource::<Exposure>() {
            ui.text(format!("Current Exposure: {:.4}", exposure.exposure_value));
            ui.text(format!("Current EV100: {:.2}", exposure.ev100));
        }
    }
}

fn build_onion_skinning_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::OnionSkinningConfig;

    if section_header(ui, ecs_world, "Onion Skinning", SectionDefault::Closed) {
        if let Some(config) = ecs_world.get_resource::<OnionSkinningConfig>() {
            let mut config_copy = config.clone();
            drop(config);

            crate::platform::ui::theme::toggle_switch(
                ui,
                ecs_world,
                "Onion Skin Enabled",
                &mut config_copy.enabled,
            );

            let mut past = config_copy.past_count as i32;
            if ui.slider_config("Past Frames", 0, 4).build(&mut past) {
                config_copy.past_count = past.max(0) as u32;
            }

            let mut future = config_copy.future_count as i32;
            if ui.slider_config("Future Frames", 0, 4).build(&mut future) {
                config_copy.future_count = future.max(0) as u32;
            }

            ui.slider_config("Frame Step", 0.001, 0.2)
                .display_format("%.3f")
                .build(&mut config_copy.frame_step);

            ui.slider_config("Ghost Opacity", 0.0, 1.0)
                .build(&mut config_copy.opacity);

            ui.color_edit3("Past Color", &mut config_copy.past_color);
            ui.color_edit3("Future Color", &mut config_copy.future_color);

            ui.text(format!(
                "Total ghosts: {}",
                crate::ecs::compute_total_ghost_count(&config_copy)
            ));

            ecs_world.send_command(OverlayEvent::UpdateOnionSkinning(config_copy));
        }
    }
}

fn build_scene_overlay(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let viewport = world.resource::<ViewportInput>().clone();
    draw_scene_toolbar(ui, world, &viewport);
    draw_scene_panel(ui, world, &viewport);
}

crate::ui_window!("scene_overlay", Overlay, 0, build_scene_overlay);
