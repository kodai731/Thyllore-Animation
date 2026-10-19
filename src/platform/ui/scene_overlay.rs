use std::path::Path;

use imgui::{Condition, StyleVar};

use super::pointer::read_ui_pointer;
#[cfg(feature = "auto-rig")]
use super::text_to_animation_dialog::TextToAnimationDialogState;
#[cfg(feature = "auto-rig")]
use super::text_to_mesh_dialog::TextToMeshDialogState;
use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::ViewportInput;
use crate::ecs::resource::{
    clamp_ui_scale, save_ui_settings, CoordinateSpace, MotionPreference, PanelVisibility,
    TransformGizmoMode, TransformGizmoState, UiDensity, UiSettings, UiWidgetState, MAX_UI_SCALE,
    MIN_UI_SCALE, UI_SETTINGS_PATH,
};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
#[cfg(feature = "auto-rig")]
use crate::ecs::systems::phases::event_dispatch::ml::auto_rig::AutoRigEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::World;
use crate::platform::ui::theme::colors::{srgb_to_linear_rgba, SURFACE3};
use crate::platform::ui::theme::shadow::draw_window_shadow;
use crate::platform::ui::theme::widgets::segmented_control;
use crate::platform::ui::theme::{icon_button, toggle_switch, ButtonState, Icon, ICON_BUTTON_SIZE};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::form_window::{FormPlacement, FormWindow};
use super::scene_settings::{read_scene_settings, send_scene_settings_changes};
use super::scene_settings_rows::scene_settings_rows;

const OVERLAY_MARGIN: f32 = 8.0;
const OVERLAY_WIDTH: f32 = 420.0;
const TOOLBAR_HEIGHT: f32 = 28.0;
const TOOLBAR_PADDING_X: f32 = 6.0;
const TOOLBAR_GROUP_GAP: f32 = 12.0;
const UI_SETTINGS_POPUP: &str = "ui_settings_popup";
const DENSITY_OPTIONS: [(UiDensity, &str); 2] = [
    (UiDensity::Compact, "Compact"),
    (UiDensity::Default, "Default"),
];

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
            draw_window_shadow(ui, ui.clone_style().window_rounding());

            build_file_buttons(ui, ecs_world);
            draw_toolbar_group_separator(ui);
            build_gizmo_buttons(ui, ecs_world);
            draw_toolbar_group_separator(ui);
            build_overlay_buttons(ui, ecs_world);

            #[cfg(feature = "auto-rig")]
            build_auto_rig_buttons(ui, ecs_world);

            build_scene_panel_toggle(ui, ecs_world);
        });

    apply_gizmo_hotkeys(ui, ecs_world);
}

fn scene_panel_form(world: &World, _: &AssetStorage, _: &GraphicsResources) -> Option<FormWindow> {
    if world.resource::<UiWidgetState>().scene_panel == PanelVisibility::Hidden {
        return None;
    }

    let viewport = world.resource::<ViewportInput>();
    let pos_x = viewport.position[0] + viewport.size[0] - OVERLAY_WIDTH - OVERLAY_MARGIN;
    let pos_y = viewport.position[1] + TOOLBAR_HEIGHT + OVERLAY_MARGIN;
    let max_height = viewport.size[1] - TOOLBAR_HEIGHT - 2.0 * OVERLAY_MARGIN;

    let mut sections: Vec<Box<dyn crate::platform::ui::form_window::FormSection>> =
        vec![super::form_window::rows_section(
            scene_settings_rows(&read_scene_settings(world)),
            read_scene_settings(world),
            send_scene_settings_changes,
        )];

    for hook in crate::platform::ui::effect_sections::collect_effect_sections() {
        if let Some(section) = (hook.section)(world) {
            sections.push(section);
        }
    }

    Some(FormWindow {
        title: "Scene Panel",
        placement: FormPlacement::Overlay {
            position: [pos_x, pos_y],
            width: OVERLAY_WIDTH,
            max_height,
        },
        sections,
    })
}

fn build_file_buttons(ui: &imgui::Ui, ecs_world: &World) {
    if icon_button(ui, Icon::FileBox, "Open FBX", ButtonState::Normal) {
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

    if icon_button(ui, Icon::FileBraces, "Open glTF", ButtonState::Normal) {
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
        .add_line(
            [x, y],
            [x, y + ICON_BUTTON_SIZE],
            srgb_to_linear_rgba(SURFACE3),
        )
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

fn build_overlay_buttons(ui: &imgui::Ui, ecs_world: &World) {
    let Some(bone_gizmo) = ecs_world.get_resource::<BoneGizmoData>() else {
        return;
    };
    let visible = bone_gizmo.visible;
    drop(bone_gizmo);

    let (tooltip, state) = if visible {
        ("Bones: shown", ButtonState::Active)
    } else {
        ("Bones: hidden", ButtonState::Normal)
    };
    if icon_button(ui, Icon::Bone, tooltip, state) {
        ecs_world.send_command(OverlayEvent::SetBoneGizmoVisible(!visible));
    }
}

fn apply_gizmo_hotkeys(ui: &imgui::Ui, ecs_world: &World) {
    let scene_view_focused = ecs_world.resource::<ViewportInput>().focused;
    let gizmo_hotkeys_enabled = scene_view_focused
        && !ui.io().key_ctrl()
        && !read_ui_pointer(ui).is_down(imgui::MouseButton::Right);
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

    let style = ui.clone_style();
    let right_edge_x = ui.window_size()[0] - style.window_padding()[0] - ICON_BUTTON_SIZE;
    let settings_x = right_edge_x - ICON_BUTTON_SIZE - style.item_spacing()[0];

    ui.same_line_with_pos(settings_x);
    if icon_button(ui, Icon::Settings, "UI Settings", ButtonState::Normal) {
        ui.open_popup(UI_SETTINGS_POPUP);
    }
    ui.popup(UI_SETTINGS_POPUP, || build_ui_settings_popup(ui, ecs_world));

    ui.same_line_with_pos(right_edge_x);
    if icon_button(ui, Icon::Layers, "Scene Panel", button_state) {
        ecs_world.resource_mut::<UiWidgetState>().scene_panel = toggled_visibility;
    }
}

fn build_ui_settings_popup(ui: &imgui::Ui, ecs_world: &World) {
    let current = *ecs_world.resource::<UiSettings>();
    let mut edited = current;

    ui.text("UI Settings");
    ui.separator();

    ui.slider_config("Scale", MIN_UI_SCALE, MAX_UI_SCALE)
        .build(&mut edited.scale);
    edited.scale = clamp_ui_scale(edited.scale);

    let density_labels = DENSITY_OPTIONS.map(|(_, label)| label);
    let density_selected = DENSITY_OPTIONS
        .iter()
        .position(|(density, _)| *density == edited.density)
        .unwrap_or_default();
    if let Some(index) = segmented_control(ui, "density", &density_labels, density_selected) {
        edited.density = DENSITY_OPTIONS[index].0;
    }

    let mut reduce_motion = edited.reduced_motion == MotionPreference::Reduced;
    if toggle_switch(ui, ecs_world, "Reduce motion", &mut reduce_motion) {
        edited.reduced_motion = if reduce_motion {
            MotionPreference::Reduced
        } else {
            MotionPreference::Full
        };
    }

    if edited != current {
        *ecs_world.resource_mut::<UiSettings>() = edited;
        if let Err(e) = save_ui_settings(Path::new(UI_SETTINGS_PATH), &edited) {
            log_warn!("Failed to save UI settings: {}", e);
        }
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

fn build_scene_overlay(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let viewport = world.resource::<ViewportInput>().clone();
    draw_scene_toolbar(ui, world, &viewport);
}

crate::ui_window!("scene_overlay", Overlay, 0, build_scene_overlay);
crate::ui_window!("scene_panel", Overlay, 1, form = scene_panel_form);
