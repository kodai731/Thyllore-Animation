use std::rc::Rc;

use imgui::Condition;
use thyllore_anim_core::editable::PropertyType;

#[cfg(feature = "auto-rig")]
use super::text_to_animation_dialog::TextToAnimationDialogState;
#[cfg(feature = "auto-rig")]
use super::text_to_mesh_dialog::TextToMeshDialogState;
use crate::asset::AssetStorage;
use crate::ecs::component::{FlameParam, LightningParam, WaterParam, WindParam};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::ViewportInput;
use crate::ecs::resource::{
    CoordinateSpace, LightningDebugCapture, ModelState, TransformGizmoMode, TransformGizmoState,
    WaterDebugCapture, WeightHeatmapState, WindDebugCapture,
};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
#[cfg(feature = "auto-rig")]
use crate::ecs::systems::phases::event_dispatch::ml::auto_rig::AutoRigEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::{
    FLAME_SPAWN_HOOK, LIGHTNING_SPAWN_HOOK, WATER_SPAWN_HOOK, WIND_SPAWN_HOOK,
};
use crate::ecs::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::param_widgets::{draw_params, draw_tiered_params, EditedScalars};

const OVERLAY_MARGIN: f32 = 8.0;
const OVERLAY_WIDTH: f32 = 420.0;

#[cfg(feature = "auto-rig")]
use crate::ecs::resource::{AutoRigState, AutoRigStatus};

fn draw_scene_overlay(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    model: &mut ModelState,
    ecs_world: &World,
    viewport: &ViewportInput,
) {
    let pos_x = viewport.position[0] + OVERLAY_MARGIN;
    let pos_y = viewport.position[1] + OVERLAY_MARGIN;

    ui.window("Scene Overlay")
        .position([pos_x, pos_y], Condition::Always)
        .size_constraints(
            [OVERLAY_WIDTH, 0.0],
            [OVERLAY_WIDTH, viewport.size[1] - 2.0 * OVERLAY_MARGIN],
        )
        .always_auto_resize(true)
        .no_decoration()
        .bg_alpha(0.7)
        .no_nav()
        .focus_on_appearing(false)
        .save_settings(false)
        .build(|| {
            build_model_section(ui, model, ecs_world);
            ui.separator();

            build_screenshot_section(ui, ecs_world);
            ui.separator();

            build_overlay_section(ui, ecs_world);

            build_transform_gizmo_section(ui, ecs_world);

            build_dof_section(ui, ecs_world);

            build_auto_exposure_section(ui, ecs_world);

            build_onion_skinning_section(ui, ecs_world);

            build_water_section(ui, ui_events, ecs_world);

            build_wind_section(ui, ui_events, ecs_world);

            build_lightning_section(ui, ui_events, ecs_world);

            build_flame_section(ui, ui_events, model, ecs_world, viewport);
        });
}

fn build_model_section(ui: &imgui::Ui, model: &mut ModelState, ecs_world: &World) {
    if ui.button("Open FBX") {
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

    if ui.button("Open glTF") {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF Files", &["gltf", "glb"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log!("Selected glTF file: {}", path_str);
            ecs_world.send_command(CameraEvent::LoadModel { path: path_str });
        }
    }

    if ui.button("Add GLB") {
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

    #[cfg(feature = "auto-rig")]
    if ui.button("Generate Mesh") {
        ecs_world.resource_mut::<TextToMeshDialogState>().open = true;
    }

    #[cfg(feature = "auto-rig")]
    {
        ui.same_line();
        if ui.button("Generate Animation") {
            ecs_world.resource_mut::<TextToAnimationDialogState>().open = true;
        }
    }

    #[cfg(feature = "auto-rig")]
    build_auto_rig_section(ui, ecs_world);

    let model_name = if model.model_path.is_empty() {
        "None"
    } else {
        &model.model_path
    };
    ui.text_wrapped(format!("Model: {}", model_name));
    ui.text(format!("Status: {}", model.load_status));
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
            if ui.button("Cancel##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }

        AutoRigStatus::Rigging => {
            ui.text("Rigging: processing...");
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
            }
            if ui.button("Dismiss##rig") {
                ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
            }
        }
    }
}

fn build_screenshot_section(ui: &imgui::Ui, ecs_world: &World) {
    if ui.button("Screenshot") {
        ecs_world.send_command(CameraEvent::TakeScreenshot);
    }
}

fn flame_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            FlameParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

fn water_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WaterParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

fn wind_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WindParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

fn lightning_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            LightningParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

fn send_key_button(
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
    if ui.collapsing_header("Overlay", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        if let Some(bone_gizmo) = ecs_world.get_resource::<BoneGizmoData>() {
            let mut visible = bone_gizmo.visible;
            if ui.checkbox("Show Bones", &mut visible) {
                ecs_world.send_command(OverlayEvent::SetBoneGizmoVisible(visible));
            }
        }
        if let Some(heatmap) = ecs_world.get_resource::<WeightHeatmapState>() {
            let mut enabled = heatmap.enabled;
            if ui.checkbox("Show Weight Heatmap (selected bone)", &mut enabled) {
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

    if ui.collapsing_header("Transform Gizmo", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        let translate_label = if state_copy.mode == TransformGizmoMode::Translate {
            "[W] Translate *"
        } else {
            "[W] Translate"
        };
        let rotate_label = if state_copy.mode == TransformGizmoMode::Rotate {
            "[E] Rotate *"
        } else {
            "[E] Rotate"
        };
        let scale_label = if state_copy.mode == TransformGizmoMode::Scale {
            "[R] Scale *"
        } else {
            "[R] Scale"
        };

        if ui.button(translate_label) {
            state_copy.mode = TransformGizmoMode::Translate;
        }
        ui.same_line();
        if ui.button(rotate_label) {
            state_copy.mode = TransformGizmoMode::Rotate;
        }
        ui.same_line();
        if ui.button(scale_label) {
            state_copy.mode = TransformGizmoMode::Scale;
        }

        let gizmo_hotkeys_enabled =
            !ui.io().key_ctrl && !ui.is_mouse_down(imgui::MouseButton::Right);
        if ui.is_key_pressed(imgui::Key::W) && gizmo_hotkeys_enabled {
            state_copy.mode = TransformGizmoMode::Translate;
        }
        if ui.is_key_pressed(imgui::Key::E) && gizmo_hotkeys_enabled {
            state_copy.mode = TransformGizmoMode::Rotate;
        }
        if ui.is_key_pressed(imgui::Key::R) && gizmo_hotkeys_enabled {
            state_copy.mode = TransformGizmoMode::Scale;
        }

        let space_label = match state_copy.coordinate_space {
            CoordinateSpace::World => "World",
            CoordinateSpace::Local => "Local",
        };
        if ui.button(format!("Space: {}", space_label)) {
            state_copy.coordinate_space = match state_copy.coordinate_space {
                CoordinateSpace::World => CoordinateSpace::Local,
                CoordinateSpace::Local => CoordinateSpace::World,
            };
        }

        ui.same_line();
        ui.checkbox("Snap", &mut state_copy.snap_enabled);

        if state_copy.snap_enabled {
            match state_copy.mode {
                TransformGizmoMode::Translate => {
                    ui.slider_config("Snap Value", 0.01, 10.0)
                        .build(&mut state_copy.translate_snap_value);
                }
                TransformGizmoMode::Rotate => {
                    ui.slider_config("Snap Degrees", 1.0, 90.0)
                        .build(&mut state_copy.rotate_snap_degrees);
                }
                TransformGizmoMode::Scale => {
                    ui.slider_config("Snap Value", 0.01, 1.0)
                        .build(&mut state_copy.scale_snap_value);
                }
            }
        }

        ui.slider_config("Gizmo Scale", 0.01, 0.3)
            .display_format("%.3f")
            .build(&mut state_copy.gizmo_scale);

        ecs_world.send_command(OverlayEvent::UpdateTransformGizmoState(Box::new(
            state_copy,
        )));
    }
}

fn build_dof_section(ui: &imgui::Ui, ecs_world: &World) {
    use crate::ecs::resource::{DepthOfField, PhysicalCameraParameters};

    if ui.collapsing_header("Depth of Field", imgui::TreeNodeFlags::empty()) {
        if let Some(dof) = ecs_world.get_resource::<DepthOfField>() {
            let mut dof_copy = dof.clone();
            drop(dof);

            ui.checkbox("DOF Enabled", &mut dof_copy.enabled);

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

    if ui.collapsing_header("Auto Exposure", imgui::TreeNodeFlags::empty()) {
        if let Some(ae) = ecs_world.get_resource::<AutoExposure>() {
            let mut ae_copy = ae.clone();
            drop(ae);

            ui.checkbox("Auto Exposure Enabled", &mut ae_copy.enabled);

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

    if ui.collapsing_header("Onion Skinning", imgui::TreeNodeFlags::empty()) {
        if let Some(config) = ecs_world.get_resource::<OnionSkinningConfig>() {
            let mut config_copy = config.clone();
            drop(config);

            ui.checkbox("Onion Skin Enabled", &mut config_copy.enabled);

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

/// Preset combo whose preview is the preset recorded on the selected entity; returns the
/// preset the user just picked, if any.
fn draw_preset_combo(
    ui: &imgui::Ui,
    label: &str,
    names: &[&str],
    applied: Option<&str>,
) -> Option<String> {
    let preview = applied.unwrap_or("(none)");
    let combo = ui.begin_combo(label, preview)?;
    let mut chosen = None;
    for &name in names {
        if ui
            .selectable_config(name)
            .selected(Some(name) == applied)
            .build()
        {
            chosen = Some(name.to_string());
        }
    }
    combo.end();
    chosen
}

fn build_wind_section(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::component::WindTornadoEffect;

    if !ui.collapsing_header("Wind", imgui::TreeNodeFlags::empty()) {
        return;
    }
    let _section_id = ui.push_id("wind");

    if ui.button("Add Wind") {
        ecs_world.send_command(ScalarCurveEvent::AddEffect(WIND_SPAWN_HOOK.key));
    }

    let winds = ecs_world.entities_with::<WindTornadoEffect>();
    let selected_wind_entity = crate::ecs::systems::resolve_selected_wind(ecs_world);
    if winds.len() > 1 {
        let mut current = selected_wind_entity
            .and_then(|entity| winds.iter().position(|&e| e == entity))
            .unwrap_or(0);
        let items: Vec<String> = winds
            .iter()
            .enumerate()
            .map(|(i, &entity)| {
                ecs_world
                    .get_component::<crate::ecs::world::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("Wind {}", i + 1))
            })
            .collect();
        if ui.combo_simple_string("Instance", &mut current, &items) {
            ecs_world.send_command(OverlayEvent::SelectEffectInstance {
                key: WIND_SPAWN_HOOK.key,
                index: current,
            });
        }
    }

    let applied_preset = selected_wind_entity.and_then(|entity| {
        ecs_world
            .get_component::<crate::ecs::component::AppliedWindPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let mut effect_applied_this_frame = false;
    if let Some(chosen) = draw_preset_combo(
        ui,
        "Wind Preset",
        thyllore_effect_core::WIND_PRESET_NAMES,
        applied_preset.as_deref(),
    ) {
        if selected_wind_entity.is_some() {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ui_events.send(UIEvent::ApplyWindPreset(chosen));
            effect_applied_this_frame = true;
        }
    }

    let Some(selected_wind) = selected_wind_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<WindTornadoEffect>(selected_wind) else {
        return;
    };
    let mut effect_copy = effect.clone();
    draw_tiered_params(
        ui,
        &thyllore_effect_core::WIND_UI_PARAMS,
        &thyllore_effect_core::WIND_SCALAR_PARAMS,
        &mut effect_copy,
        &[],
        |ui, edited| wind_key_button(ui, ecs_world, edited),
    );
    if !effect_applied_this_frame {
        ui_events.send(UIEvent::UpdateWindEffect {
            entity: selected_wind,
            effect: Box::new(effect_copy),
        });
    }
    if ui.button("Curves") {
        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
    }
    if ui.collapsing_header("Wind Debug", imgui::TreeNodeFlags::empty()) {
        draw_wind_render_settings(ui, ui_events, ecs_world);
        if ui.button("Dump Debug") {
            ecs_world.send_command(CameraEvent::CaptureNow(Rc::new(WindDebugCapture)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Write wind parameters, UBO, render settings, camera and a screenshot to log/wind/",
            );
        }
    }
}

fn draw_wind_render_settings(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::resource::{WindDebugView, WindRenderSettings, WindShadingMode};

    let Some(settings) = ecs_world.get_resource::<WindRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Shading Mode", settings_copy.shading_mode.label()) {
        for mode in WindShadingMode::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.shading_mode)
                .build()
            {
                settings_copy.shading_mode = mode;
            }
        }
    }
    if let Some(_token) = ui.begin_combo("Debug View", settings_copy.debug_view.label()) {
        for view in WindDebugView::ALL {
            if ui
                .selectable_config(view.label())
                .selected(view == settings_copy.debug_view)
                .build()
            {
                settings_copy.debug_view = view;
            }
        }
    }
    let mut step_count = settings_copy.reference_step_count as i32;
    if ui
        .slider_config("Reference Steps", 16, 2048)
        .build(&mut step_count)
    {
        settings_copy.reference_step_count = step_count.max(1) as u32;
    }
    ui.checkbox(
        "Animate when paused",
        &mut settings_copy.free_run_when_paused,
    );
    ui_events.send(UIEvent::UpdateWindRenderSettings(settings_copy));
}

fn build_lightning_section(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::component::LightningEffect;

    if !ui.collapsing_header("Lightning", imgui::TreeNodeFlags::empty()) {
        return;
    }
    let _section_id = ui.push_id("lightning");

    if ui.button("Add Lightning") {
        ecs_world.send_command(ScalarCurveEvent::AddEffect(LIGHTNING_SPAWN_HOOK.key));
    }

    let lightnings = ecs_world.entities_with::<LightningEffect>();
    let selected_entity = crate::ecs::systems::resolve_selected_lightning(ecs_world);
    if lightnings.len() > 1 {
        let mut current = selected_entity
            .and_then(|entity| lightnings.iter().position(|&e| e == entity))
            .unwrap_or(0);
        let items: Vec<String> = lightnings
            .iter()
            .enumerate()
            .map(|(i, &entity)| {
                ecs_world
                    .get_component::<crate::ecs::world::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("Lightning {}", i + 1))
            })
            .collect();
        if ui.combo_simple_string("Instance", &mut current, &items) {
            ecs_world.send_command(OverlayEvent::SelectEffectInstance {
                key: LIGHTNING_SPAWN_HOOK.key,
                index: current,
            });
        }
    }

    let applied_preset = selected_entity.and_then(|entity| {
        ecs_world
            .get_component::<crate::ecs::component::AppliedLightningPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let mut effect_applied_this_frame = false;
    if let Some(chosen) = draw_preset_combo(
        ui,
        "Lightning Preset",
        thyllore_effect_core::LIGHTNING_PRESET_NAMES,
        applied_preset.as_deref(),
    ) {
        if selected_entity.is_some() {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ui_events.send(UIEvent::ApplyLightningPreset(chosen));
            effect_applied_this_frame = true;
        }
    }

    let Some(selected) = selected_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<LightningEffect>(selected) else {
        return;
    };
    let mut effect_copy = effect.clone();
    let target_name = draw_lightning_target_row(ui, ui_events, ecs_world, selected);
    draw_lightning_path_rows(ui, ui_events, ecs_world, selected);

    if target_name.is_none() {
        draw_params(
            ui,
            &["shape_end_offset"],
            &thyllore_effect_core::LIGHTNING_UI_PARAMS,
            &thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
            &mut effect_copy,
            |ui, edited| lightning_key_button(ui, ecs_world, edited),
        );
    }

    draw_tiered_params(
        ui,
        &thyllore_effect_core::LIGHTNING_UI_PARAMS,
        &thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
        &mut effect_copy,
        &["shape_end_offset"],
        |ui, edited| lightning_key_button(ui, ecs_world, edited),
    );

    if !effect_applied_this_frame {
        ui_events.send(UIEvent::UpdateLightningEffect {
            entity: selected,
            effect: Box::new(effect_copy),
        });
    }
    if ui.button("Curves") {
        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
    }
    if !ui.collapsing_header("Lightning Debug", imgui::TreeNodeFlags::empty()) {
        return;
    }
    draw_lightning_render_settings(ui, ui_events, ecs_world);
    if ui.button("Dump Debug") {
        ecs_world.send_command(CameraEvent::CaptureNow(Rc::new(LightningDebugCapture)));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Write lightning parameters, UBO, render settings, camera and a screenshot to log/lightning/",
        );
    }
}

/// The bolt's end point row: the linked locator's name, or a button that creates one. While a
/// target is linked the end offset follows it, so its slider is hidden.
fn draw_lightning_target_row(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    ecs_world: &World,
    lightning: crate::ecs::world::Entity,
) -> Option<String> {
    use crate::ecs::component::LightningTarget;

    let target_name = ecs_world
        .get_component::<LightningTarget>(lightning)
        .map(|target| target.entity_name.clone());
    match &target_name {
        Some(name) => {
            ui.text(format!("Target: {name}"));
            ui.same_line();
            if ui.small_button("Clear##lightning_target") {
                ui_events.send(UIEvent::ClearLightningTarget);
            }
        }
        None => {
            if ui.button("Add Target") {
                ui_events.send(UIEvent::AddLightningTarget);
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Spawn a locator at the end point; move it to aim the bolt");
            }
        }
    }
    target_name
}

fn draw_lightning_path_rows(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    ecs_world: &World,
    lightning: crate::ecs::world::Entity,
) {
    use crate::ecs::component::LightningPath;

    let waypoints = ecs_world
        .get_component::<LightningPath>(lightning)
        .map(|path| path.waypoints.clone())
        .unwrap_or_default();

    for (i, name) in waypoints.iter().enumerate() {
        ui.text(name);
        ui.same_line();
        if ui.small_button(format!("x##waypoint{i}")) {
            ui_events.send(UIEvent::RemoveLightningWaypoint(i));
        }
    }

    if ui.button("Add Waypoint") {
        ui_events.send(UIEvent::AddLightningWaypoint);
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Spawn a locator halfway to the end point; the bolt passes through waypoints in order",
        );
    }
}

fn draw_lightning_render_settings(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::resource::{LightningDebugView, LightningRenderSettings, LightningShadingMode};

    let Some(settings) = ecs_world.get_resource::<LightningRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Shading Mode", settings_copy.shading_mode.label()) {
        for mode in LightningShadingMode::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.shading_mode)
                .build()
            {
                settings_copy.shading_mode = mode;
            }
        }
    }
    if let Some(_token) = ui.begin_combo("Debug View", settings_copy.debug_view.label()) {
        for view in LightningDebugView::ALL {
            if ui
                .selectable_config(view.label())
                .selected(view == settings_copy.debug_view)
                .build()
            {
                settings_copy.debug_view = view;
            }
        }
    }
    let mut step_count = settings_copy.reference_step_count as i32;
    if ui
        .slider_config("Reference Steps", 16, 2048)
        .build(&mut step_count)
    {
        settings_copy.reference_step_count = step_count.max(1) as u32;
    }
    ui_events.send(UIEvent::UpdateLightningRenderSettings(settings_copy));
}

fn build_water_section(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::component::WaterTorusEffect;

    if ui.collapsing_header("Water", imgui::TreeNodeFlags::empty()) {
        let _section_id = ui.push_id("water");
        // Add Water button (before instance selector, accessible even when no water exists)
        if ui.button("Add Water") {
            ecs_world.send_command(ScalarCurveEvent::AddEffect(WATER_SPAWN_HOOK.key));
        }

        // Instance selector
        let waters = ecs_world.entities_with::<WaterTorusEffect>();
        let selected_water_entity = crate::ecs::systems::resolve_selected_water(ecs_world);
        let clamped_index = selected_water_entity
            .and_then(|entity| waters.iter().position(|&e| e == entity))
            .unwrap_or(0);

        if waters.len() > 1 {
            let mut current = clamped_index;
            let items: Vec<String> = waters
                .iter()
                .enumerate()
                .map(|(i, &entity)| {
                    ecs_world
                        .get_component::<crate::ecs::world::Name>(entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| format!("Water {}", i + 1))
                })
                .collect();
            if ui.combo_simple_string("Instance", &mut current, &items) {
                ecs_world.send_command(OverlayEvent::SelectEffectInstance {
                    key: WATER_SPAWN_HOOK.key,
                    index: current,
                });
            }
        }

        let applied_preset = selected_water_entity.and_then(|entity| {
            ecs_world
                .get_component::<crate::ecs::component::AppliedWaterPreset>(entity)
                .map(|preset| preset.name.clone())
        });
        let mut effect_applied_this_frame = false;
        {
            if let Some(chosen) = draw_preset_combo(
                ui,
                "Water Preset",
                thyllore_effect_core::WATER_PRESET_NAMES,
                applied_preset.as_deref(),
            ) {
                if selected_water_entity.is_some() {
                    ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
                    ui_events.send(UIEvent::ApplyWaterPreset(chosen));
                    effect_applied_this_frame = true;
                }
            }

            // Scalar parameters
            if let Some(selected_water) = selected_water_entity {
                if let Some(effect) = ecs_world.get_component::<WaterTorusEffect>(selected_water) {
                    let mut effect_copy = effect.clone();

                    draw_tiered_params(
                        ui,
                        &thyllore_effect_core::WATER_UI_PARAMS,
                        &thyllore_effect_core::WATER_SCALAR_PARAMS,
                        &mut effect_copy,
                        &[],
                        |ui, edited| water_key_button(ui, ecs_world, edited),
                    );

                    if !effect_applied_this_frame {
                        ui_events.send(UIEvent::UpdateWaterEffect {
                            entity: selected_water,
                            effect: Box::new(effect_copy),
                        });
                    }

                    if ui.button("Curves") {
                        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
                    }
                }
            }
        }
        if ui.collapsing_header("Water Debug", imgui::TreeNodeFlags::empty()) {
            draw_water_render_settings(ui, ui_events, ecs_world);
            if ui.button("Dump Debug") {
                ecs_world.send_command(CameraEvent::CaptureNow(Rc::new(WaterDebugCapture)));
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(
                    "Write water parameters, UBO, camera, render settings and a screenshot to log/water/",
                );
            }
        }
    }
}

fn draw_water_render_settings(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::resource::WaterRenderSettings;

    let Some(settings) = ecs_world.get_resource::<WaterRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Secondary Rays", settings_copy.secondary_rays.label()) {
        for mode in thyllore_effect_core::WaterSecondaryRays::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.secondary_rays)
                .build()
            {
                settings_copy.secondary_rays = mode;
            }
        }
    }

    let mut debug_view = settings_copy.debug_view as f32;
    if ui
        .slider_config("Debug View", 0.0f32, 1.0f32)
        .build(&mut debug_view)
    {
        settings_copy.debug_view = debug_view as i32;
    }

    ui.checkbox(
        "Animate when paused",
        &mut settings_copy.free_run_when_paused,
    );

    ui_events.send(UIEvent::UpdateWaterRenderSettings(settings_copy));
}

fn draw_flame_manual_params(ui: &imgui::Ui, effect: &mut crate::ecs::component::FlameEffect) {
    let mut noise_sharpness =
        thyllore_effect_core::shaping_scale_to_noise_sharpness(effect.noise.shaping_scale);
    if ui
        .slider_config("Noise Sharpness", 0.0, 1.0)
        .display_format("%.2f")
        .build(&mut noise_sharpness)
    {
        effect.noise.shaping_scale =
            thyllore_effect_core::noise_sharpness_to_shaping_scale(noise_sharpness);
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Crispness of the noise pattern: log remap of the tanh shaping \
             scale, harder edges to the right (small scale saturates the \
             tanh into near-binary blobs; ~0.78 = scale 0.25, the measured \
             perceptual sweet spot). Stateless — noise_shaping_scale stays \
             the source of truth (0 = built-in 0.6)",
        );
    }

    let mut wave_segments = effect.wave_segments as i32;
    if ui
        .slider_config(
            "Noise Segments",
            thyllore_effect_core::WAVE_SEGMENTS_MIN as i32,
            thyllore_effect_core::WAVE_SEGMENTS_MAX as i32,
        )
        .build(&mut wave_segments)
    {
        effect.wave_segments = wave_segments as u32;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Closed-form segments per ray: the noise grid aliases into a \
             pixel hatch above Noise Frequency ~2 at 64; 128 resolves \
             frequency ~4 at twice the cost",
        );
    }

    let mut vortex =
        (effect.twist.gain / thyllore_effect_core::VORTEX_MACRO_MAX_GAIN).clamp(0.0, 1.0);
    if ui
        .slider_config("Vortex", 0.0, 1.0)
        .display_format("%.2f")
        .build(&mut vortex)
    {
        let (gain, speed) = thyllore_effect_core::vortex_macro_parameters(vortex);
        effect.twist.gain = gain;
        effect.twist.speed = speed;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Vortex macro: one knob writing both twist parameters along a \
             faster-and-deeper curve (stateless; the fine sliders \
             stay the source of truth)",
        );
    }

    let mut branch_seed = effect.branch.seed as i32;
    if ui.input_int("Branch Seed", &mut branch_seed).build() {
        effect.branch.seed = branch_seed.max(0) as u32;
    }
}

fn build_flame_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    model: &mut ModelState,
    ecs_world: &World,
    viewport: &ViewportInput,
) {
    use crate::ecs::component::FlameEffect;

    if ui.collapsing_header("Flame", imgui::TreeNodeFlags::empty()) {
        let _section_id = ui.push_id("flame");
        let flames = ecs_world.entities_with::<FlameEffect>();
        let selected_flame_entity = crate::ecs::systems::resolve_selected_flame(ecs_world);
        let clamped_index = selected_flame_entity
            .and_then(|entity| flames.iter().position(|&e| e == entity))
            .unwrap_or(0);

        // Instance selector combo (when >1 flame)
        if flames.len() > 1 {
            let mut current = clamped_index;
            let items: Vec<String> = flames
                .iter()
                .enumerate()
                .map(|(i, &entity)| {
                    ecs_world
                        .get_component::<crate::ecs::world::Name>(entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| format!("Flame {}", i + 1))
                })
                .collect();
            if ui.combo_simple_string("Instance", &mut current, &items) {
                ecs_world.send_command(OverlayEvent::SelectEffectInstance {
                    key: FLAME_SPAWN_HOOK.key,
                    index: current,
                });
            }
        }

        let applied_preset = selected_flame_entity.and_then(|entity| {
            ecs_world
                .get_component::<crate::ecs::component::AppliedFlamePreset>(entity)
                .map(|preset| preset.name.clone())
        });
        // The slider block below re-sends the (pre-apply) effect every frame;
        // that send must be skipped on the frame an Apply button fires or it
        // overwrites the applied preset/fit in the same dispatch.
        let mut effect_applied_this_frame = false;
        {
            if let Some(chosen) = draw_preset_combo(
                ui,
                "Flame Preset",
                thyllore_effect_core::FLAME_PRESET_NAMES,
                applied_preset.as_deref(),
            ) {
                if selected_flame_entity.is_some() {
                    // Keyed scalar curves re-stamp their channels every
                    // frame and would silently pin the old look, so a
                    // preset stamp also clears them (undo restores).
                    ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
                    ui_events.send(UIEvent::ApplyFlamePreset(chosen));
                    effect_applied_this_frame = true;
                }
            }

            ui.separator();
            ui.text("Texture Fit");

            // Scan textures on first frame
            if !model.texture_fit_scan_done {
                let scan_dir = std::path::Path::new(crate::paths::FLAMES_TEXTURE_DIR);
                if let Ok(entries) = std::fs::read_dir(scan_dir) {
                    for entry in entries.flatten() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "png" {
                                if let Some(name) = entry.file_name().to_str() {
                                    model.texture_fit_scan.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                model.texture_fit_scan_done = true;
            }

            // Combo box for texture selection
            let mut scan_items: Vec<String> = vec!["(custom path)".to_string()];
            scan_items.extend(model.texture_fit_scan.iter().cloned());
            let mut scan_selected = 0usize;
            if ui.combo_simple_string("Fit Texture", &mut scan_selected, &scan_items) {
                if scan_selected > 0 {
                    let name = &model.texture_fit_scan[scan_selected - 1];
                    model.texture_fit_path =
                        format!("{}/{}", crate::paths::FLAMES_TEXTURE_DIR, name);
                } else {
                    model.texture_fit_path.clear();
                }
            }
            ui.same_line();
            if ui.small_button("Rescan") {
                model.texture_fit_scan_done = false;
                model.texture_fit_scan.clear();
            }

            ui.input_text("Fit Image (png)", &mut model.texture_fit_path)
                .build();
            if ui.small_button("Browse...") {
                model.texture_fit_browser_open = true;
                if model.texture_fit_browser_dir.is_empty() {
                    model.texture_fit_browser_dir = crate::paths::FLAMES_TEXTURE_DIR.to_string();
                }
                model.texture_fit_browser_dir = canonical_dir_or(&model.texture_fit_browser_dir);
            }
            ui.same_line();

            // Validation indicator: existence plus a lightweight PNG header read,
            // cached per path so the header is only parsed when the path changes.
            if model.texture_fit_path != model.texture_fit_path_validated {
                model.texture_fit_path_info = validate_texture_fit_path(&model.texture_fit_path);
                model.texture_fit_path_validated = model.texture_fit_path.clone();
            }
            if model.texture_fit_path.is_empty() {
                ui.text_disabled("enter a texture path");
            } else if model.texture_fit_path_info.starts_with("ok:") {
                ui.text_colored([0.3, 0.9, 0.3, 1.0], &model.texture_fit_path_info);
            } else {
                ui.text_colored([0.9, 0.3, 0.3, 1.0], &model.texture_fit_path_info);
            }

            build_texture_fit_browser(ui, model);

            ui.slider("Fit Blend", 0.0, 1.0, &mut model.texture_fit_blend);
            ui.checkbox("Silhouette", &mut model.texture_fit_groups[0]);
            ui.checkbox("Color", &mut model.texture_fit_groups[1]);
            {
                let _disabled = ui.begin_disabled(model.texture_fit_profile);
                ui.checkbox("Turbulence", &mut model.texture_fit_groups[2]);
            }
            if model.texture_fit_profile && ui.is_item_hovered() {
                ui.tooltip_text(
                    "Ignored in profile (reproduction) mode: the turbulence \
                     estimate is far below the calibrated pattern amplitude \
                     and would crush the noise",
                );
            }
            ui.checkbox("Tilt", &mut model.texture_fit_groups[3]);

            // Fidelity radio button
            let mut fidelity_mode: i32 = if model.texture_fit_profile { 1 } else { 0 };
            if ui.radio_button("statistics (projection)", &mut fidelity_mode, 0) {
                model.texture_fit_profile = false;
            }
            ui.same_line();
            if ui.radio_button("profile (reproduction)", &mut fidelity_mode, 1) {
                model.texture_fit_profile = true;
            }

            if ui.button("Apply Texture Fit") {
                let path = model.texture_fit_path.clone();
                let blend = model.texture_fit_blend;
                let groups = thyllore_effect_core::TextureFitGroups {
                    silhouette: model.texture_fit_groups[0],
                    color: model.texture_fit_groups[1],
                    turbulence: model.texture_fit_groups[2],
                    tilt: model.texture_fit_groups[3],
                };
                if selected_flame_entity.is_some() {
                    ui_events.send(UIEvent::ApplyFlameTextureFit {
                        path: path.clone(),
                        blend,
                        groups: [
                            groups.silhouette,
                            groups.color,
                            groups.turbulence,
                            groups.tilt,
                        ],
                        profile: model.texture_fit_profile,
                    });
                    effect_applied_this_frame = true;
                }
            }

            ui.separator();
            ui.text("Style");

            if !model.flame_style_scan_done {
                let scan_dir = std::path::Path::new(crate::paths::FLAMES_STYLE_DIR);
                if let Ok(entries) = std::fs::read_dir(scan_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".style.ron") {
                                model.flame_style_scan.push(name.to_string());
                            }
                        }
                    }
                    model.flame_style_scan.sort();
                }
                model.flame_style_scan_done = true;
            }

            if model.flame_style_scan.is_empty() {
                ui.text_disabled(format!("no styles in {}", crate::paths::FLAMES_STYLE_DIR));
            } else {
                let mut style_index = model
                    .flame_style_index
                    .min(model.flame_style_scan.len() - 1);
                ui.combo_simple_string("Style File", &mut style_index, &model.flame_style_scan);
                model.flame_style_index = style_index;
            }
            ui.same_line();
            if ui.small_button("Rescan##style") {
                model.flame_style_scan_done = false;
                model.flame_style_scan.clear();
            }

            ui.checkbox("Motion##style", &mut model.flame_style_groups[0]);
            ui.same_line();
            ui.checkbox("Texture##style", &mut model.flame_style_groups[1]);
            ui.same_line();
            ui.checkbox("Optics##style", &mut model.flame_style_groups[2]);

            if ui.button("Apply Style") {
                if let Some(name) = model.flame_style_scan.get(model.flame_style_index) {
                    if selected_flame_entity.is_some() {
                        ui_events.send(UIEvent::ApplyFlameStyle {
                            path: format!("{}/{}", crate::paths::FLAMES_STYLE_DIR, name),
                            groups: model.flame_style_groups,
                        });
                        effect_applied_this_frame = true;
                    }
                }
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(
                    "Apply the selected style file's parameters (Style-owned only, \
                     dimensionless: lengths scale with the flame's radius, opacity \
                     via tau0). Fields the style leaves unset keep their current \
                     values",
                );
            }
            if let Some(selected_flame) = selected_flame_entity {
                if let Some(applied) = ecs_world
                    .get_component::<crate::ecs::component::AppliedFlameStyle>(selected_flame)
                {
                    ui.same_line();
                    ui.text_disabled(format!("applied: {} v{}", applied.name, applied.version));
                }
            }

            ui.input_text("Save As##style", &mut model.flame_style_save_name)
                .build();
            ui.same_line();
            if ui.small_button("Save Style") {
                let name = model.flame_style_save_name.trim().to_string();
                if !name.is_empty() && selected_flame_entity.is_some() {
                    ui_events.send(UIEvent::SaveFlameStyle { name });
                }
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(
                    "Save the selected flame's current look as \
                     assets/flames/styles/<name>.style.ron (all Style-owned \
                     parameters, lengths over r0, opacity as tau0)",
                );
            }

            if let Some(selected_flame) = selected_flame_entity {
                if let Some(effect) = ecs_world.get_component::<FlameEffect>(selected_flame) {
                    let mut effect_copy = effect.clone();

                    let mut position = [
                        effect_copy.position.x,
                        effect_copy.position.y,
                        effect_copy.position.z,
                    ];
                    if ui.input_float3("Position", &mut position).build() {
                        effect_copy.position =
                            cgmath::Vector3::new(position[0], position[1], position[2]);
                    }

                    let emitter_labels: [&str; 3] = ["Cylinder", "Ring", "Mesh SDF"];
                    let mut emitter_selected = effect_copy.emitter.kind as usize;
                    if ui.combo_simple_string("Emitter", &mut emitter_selected, &emitter_labels) {
                        effect_copy.emitter.kind = emitter_selected as u32;
                    }

                    if effect_copy.emitter.kind == 1 {
                        ui.slider_config("Ring Radius", 0.2, 5.0)
                            .display_format("%.2f")
                            .build(&mut effect_copy.emitter.ring_major_radius);
                        ui.same_line();
                        let mut ring_speed = effect_copy.emitter.ring_angular_speed;
                        ui.slider_config("Ring Speed", 0.0, 6.28)
                            .display_format("%.2f")
                            .build(&mut ring_speed);
                        effect_copy.emitter.ring_angular_speed = ring_speed;
                    }

                    let colors_before = (effect_copy.color.base, effect_copy.color.tip);
                    let advanced_open = draw_tiered_params(
                        ui,
                        &thyllore_effect_core::FLAME_UI_PARAMS,
                        &thyllore_effect_core::FLAME_SCALAR_PARAMS,
                        &mut effect_copy,
                        &[],
                        |ui, edited| flame_key_button(ui, ecs_world, edited),
                    );
                    if advanced_open {
                        draw_flame_manual_params(ui, &mut effect_copy);
                    }
                    if (effect_copy.color.base, effect_copy.color.tip) != colors_before {
                        effect_copy.color.use_blackbody = false;
                    }

                    if ui.button("Clear Flame Keys") {
                        ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
                    }
                    ui.same_line();
                    if ui.button("Random Keys (Debug)") {
                        let seed = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        ecs_world.send_command(ScalarCurveEvent::InsertScalarDebugKeys { seed });
                    }
                    if ui.button("Curves") {
                        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
                    }
                    if ui.button("Add Flame") {
                        ecs_world.send_command(ScalarCurveEvent::AddEffect(FLAME_SPAWN_HOOK.key));
                    }

                    // Trail checkbox and slider
                    let trail_state = ecs_world
                        .get_component::<crate::ecs::component::FlameTrail>(selected_flame)
                        .map(|t| (t.state.enabled, t.state.fade_seconds))
                        .unwrap_or((false, 0.8));
                    let mut trail_enabled = trail_state.0;
                    let mut trail_fade = trail_state.1;
                    if ui.checkbox("Trail", &mut trail_enabled) {
                        ui_events.send(UIEvent::UpdateFlameTrailEnabled(trail_enabled));
                    }
                    ui.slider_config("Trail Fade", 0.1, 5.0)
                        .build(&mut trail_fade);
                    if (trail_fade - trail_state.1).abs() > 0.01 {
                        ui_events.send(UIEvent::UpdateFlameTrailFade(trail_fade));
                    }

                    // GPU Timings section (read-only)
                    let timings = ecs_world.get_resource::<crate::ecs::resource::GpuPassTimings>();
                    if let Some(timings) = timings {
                        if !timings.passes.is_empty() {
                            ui.separator();
                            ui.text("GPU Timings");
                            for (label, ms) in &timings.passes {
                                ui.text(format!("  {} {:.3} ms", label, ms));
                            }
                        }
                    }

                    if !effect_applied_this_frame {
                        ui_events.send(UIEvent::UpdateFlameEffect {
                            entity: selected_flame,
                            effect: Box::new(effect_copy),
                        });
                    }

                    if ui.collapsing_header("Flame Debug", imgui::TreeNodeFlags::empty()) {
                        draw_flame_render_settings(ui, ui_events, ecs_world);
                        if ui.button("Dump Probe") {
                            ui_events.send(UIEvent::DumpFlameWallProbe {
                                viewport_size: viewport.size,
                            });
                        }
                        if ui.is_item_hovered() {
                            ui.tooltip_text(
                                "Dump camera pose + wall-regime ray diagnostics to log/flame/",
                            );
                        }
                    }
                }
            }
        }
    }
}

fn draw_flame_render_settings(ui: &imgui::Ui, ui_events: &mut UIEventQueue, ecs_world: &World) {
    use crate::ecs::resource::{FlameDebugView, FlameRenderSettings, FlameShadingMode};
    use thyllore_effect_core::flame_wave::{
        read_env_wave_jitter, read_env_wave_jitter_freq, set_wave_jitter, set_wave_jitter_freq,
    };

    let Some(settings) = ecs_world.get_resource::<FlameRenderSettings>() else {
        return;
    };
    let mut settings_copy = *settings;
    drop(settings);

    if let Some(_token) = ui.begin_combo("Shading Mode", settings_copy.shading_mode.label()) {
        for mode in FlameShadingMode::ALL {
            if ui
                .selectable_config(mode.label())
                .selected(mode == settings_copy.shading_mode)
                .build()
            {
                settings_copy.shading_mode = mode;
            }
        }
    }
    if let Some(_token) = ui.begin_combo("Debug View", settings_copy.debug_view.label()) {
        for view in FlameDebugView::ALL {
            if ui
                .selectable_config(view.label())
                .selected(view == settings_copy.debug_view)
                .build()
            {
                settings_copy.debug_view = view;
            }
        }
    }

    let mut jitter = read_env_wave_jitter();
    if ui
        .slider_config("Jitter Depth", 0.0f32, 2.0f32)
        .build(&mut jitter)
    {
        set_wave_jitter(jitter);
    }
    let mut jitter_freq = read_env_wave_jitter_freq();
    if ui
        .slider_config("Jitter Freq", 0.25f32, 6.0f32)
        .build(&mut jitter_freq)
    {
        set_wave_jitter_freq(jitter_freq);
    }

    match settings_copy.shading_mode {
        FlameShadingMode::ReferenceRaymarch => {
            let mut steps = settings_copy.reference_step_count as i32;
            ui.slider_config("Reference Steps", 8, 512)
                .build(&mut steps);
            settings_copy.reference_step_count = steps.max(1) as u32;
        }
        FlameShadingMode::NoiseRaymarch => {
            let mut steps = settings_copy.noise_step_count as i32;
            ui.slider_config("Noise Steps", 4, 64).build(&mut steps);
            settings_copy.noise_step_count = steps.max(1) as u32;
        }
        FlameShadingMode::Analytic
        | FlameShadingMode::DebugThickness
        | FlameShadingMode::DebugDepthClamp => {}
    }

    ui_events.send(UIEvent::UpdateFlameRenderSettings(settings_copy));
}

/// Canonicalized directory, falling back to the typed text when the path
/// cannot be resolved (broken symlink, permissions, not yet existing).
fn canonical_dir_or(dir: &str) -> String {
    std::fs::canonicalize(dir)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| dir.to_string())
}

/// Lightweight validation for the fit path indicator: existence plus a PNG
/// header read (no pixel decode). "ok: ..." prefixed on success.
fn validate_texture_fit_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    if !std::path::Path::new(path).is_file() {
        return String::from("file not found");
    }
    match std::fs::File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|file| {
            png::Decoder::new(file)
                .read_info()
                .map_err(|e| e.to_string())
        }) {
        Ok(reader) => {
            let info = reader.info();
            format!("ok: {}x{} {:?}", info.width, info.height, info.color_type)
        }
        Err(error) => format!("not a readable png: {error}"),
    }
}

const TEXTURE_FIT_BROWSER_MAX_ENTRIES: usize = 2000;

/// In-app file browser for the fit texture (G9): breadcrumb + direct path
/// input, png filter, directory-first listing, double-click to descend /
/// confirm. Selection only fills the path field — applying stays on the
/// explicit Apply button. Unreadable entries render disabled instead of
/// failing the listing.
fn build_texture_fit_browser(ui: &imgui::Ui, model: &mut ModelState) {
    if !model.texture_fit_browser_open {
        return;
    }
    let mut open = true;
    let mut confirmed: Option<String> = None;
    ui.window("Select Fit Texture")
        .size([560.0, 430.0], imgui::Condition::FirstUseEver)
        .opened(&mut open)
        .build(|| {
            let dir_now = model.texture_fit_browser_dir.clone();
            let mut jump: Option<String> = None;

            if ui.small_button("/") {
                jump = Some(String::from("/"));
            }
            let mut accumulated = String::new();
            for (index, part) in dir_now.split('/').filter(|p| !p.is_empty()).enumerate() {
                accumulated.push('/');
                accumulated.push_str(part);
                ui.same_line();
                if ui.small_button(format!("{part}##crumb{index}")) {
                    jump = Some(accumulated.clone());
                }
            }

            ui.input_text("##fit_browser_dir", &mut model.texture_fit_browser_dir)
                .build();
            ui.same_line();
            if ui.small_button("Go") {
                jump = Some(model.texture_fit_browser_dir.clone());
            }
            ui.checkbox("all files", &mut model.texture_fit_browser_show_all);
            ui.same_line();
            ui.checkbox("hidden", &mut model.texture_fit_browser_show_hidden);
            ui.same_line();
            if ui.small_button("Up") {
                if let Some(parent) = std::path::Path::new(&dir_now).parent() {
                    jump = Some(parent.display().to_string());
                }
            }

            ui.child_window("##fit_browser_list")
                .size([0.0, -34.0])
                .build(|| {
                    let read = match std::fs::read_dir(&dir_now) {
                        Ok(read) => read,
                        Err(error) => {
                            ui.text_colored(
                                [0.9, 0.3, 0.3, 1.0],
                                format!("cannot read directory: {error}"),
                            );
                            return;
                        }
                    };
                    let mut rows: Vec<(String, bool, Option<u64>)> = Vec::new();
                    let mut truncated = false;
                    for entry in read.flatten() {
                        let name = match entry.file_name().into_string() {
                            Ok(name) => name,
                            Err(_) => continue,
                        };
                        if !model.texture_fit_browser_show_hidden && name.starts_with('.') {
                            continue;
                        }
                        let metadata = entry.metadata().ok();
                        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
                        if !is_dir
                            && !model.texture_fit_browser_show_all
                            && !name.to_ascii_lowercase().ends_with(".png")
                        {
                            continue;
                        }
                        if rows.len() >= TEXTURE_FIT_BROWSER_MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                        rows.push((name, is_dir, metadata.map(|m| m.len())));
                    }
                    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    for (name, is_dir, size) in &rows {
                        let label = if *is_dir {
                            format!("{name}/")
                        } else if let Some(size) = size {
                            format!("{name}  ({:.1} KB)", *size as f64 / 1024.0)
                        } else {
                            format!("{name}  (unreadable)")
                        };
                        if size.is_none() && !is_dir {
                            ui.text_disabled(label);
                            continue;
                        }
                        let selected = !is_dir && *name == model.texture_fit_browser_selected;
                        let clicked = ui.selectable_config(&label).selected(selected).build();
                        let double_clicked = ui.is_item_hovered()
                            && ui.is_mouse_double_clicked(imgui::MouseButton::Left);
                        if *is_dir {
                            if double_clicked {
                                jump = Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        } else {
                            if clicked {
                                model.texture_fit_browser_selected = name.clone();
                            }
                            if double_clicked {
                                confirmed =
                                    Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        }
                    }
                    if truncated {
                        ui.text_colored(
                            [0.9, 0.7, 0.3, 1.0],
                            format!("listing capped at {TEXTURE_FIT_BROWSER_MAX_ENTRIES} entries"),
                        );
                    }
                });

            let has_selection = !model.texture_fit_browser_selected.is_empty();
            ui.enabled(has_selection, || {
                if ui.button("Open") {
                    confirmed = Some(format!(
                        "{}/{}",
                        dir_now.trim_end_matches('/'),
                        model.texture_fit_browser_selected
                    ));
                }
            });
            ui.same_line();
            if ui.button("Cancel") {
                model.texture_fit_browser_open = false;
            }

            if let Some(target) = jump {
                model.texture_fit_browser_dir = canonical_dir_or(&target);
                model.texture_fit_browser_selected.clear();
            }
        });
    if let Some(path) = confirmed {
        model.texture_fit_path = path;
        model.texture_fit_browser_open = false;
    }
    if !open {
        model.texture_fit_browser_open = false;
    }
}

fn build_scene_overlay(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let mut ui_events = world.resource_mut::<UIEventQueue>();
    let mut model = world.resource_mut::<ModelState>();
    let viewport = world.resource::<ViewportInput>().clone();
    draw_scene_overlay(ui, &mut ui_events, &mut model, world, &viewport);
}

crate::ui_window!("scene_overlay", Overlay, 0, build_scene_overlay);
