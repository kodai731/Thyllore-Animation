use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{
    AppliedLightningPreset, LightningEffect, LightningParam, LightningPath, LightningTarget,
};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    LightningDebugCapture, LightningDebugView, LightningRenderSettings, LightningShadingMode,
};
use crate::ecs::systems::lightning::{resolve_selected_lightning, LightningUiCommand};
use crate::ecs::systems::LIGHTNING_SPAWN_HOOK;
use crate::ecs::world::Entity;
use crate::ecs::World;
use crate::hooks::effect_ui_event::send_effect_ui_command;

use super::param_widgets::{draw_params, draw_preset_combo, draw_tiered_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::SceneOverlayState;

fn lightning_key_button(ui: &imgui::Ui, ui_events: &mut UIEventQueue, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            LightningParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ui_events, edited, keys);
}

pub(super) fn build_lightning_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    _overlay_state: &mut SceneOverlayState,
    ecs_world: &World,
) {
    if !ui.collapsing_header("Lightning", imgui::TreeNodeFlags::empty()) {
        return;
    }
    let _section_id = ui.push_id("lightning");

    if ui.button("Add Lightning") {
        ui_events.send(UIEvent::AddEffect(LIGHTNING_SPAWN_HOOK.key));
    }

    let lightnings = ecs_world.entities_with::<LightningEffect>();
    let selected_entity = resolve_selected_lightning(ecs_world);
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
            ui_events.send(UIEvent::SelectEffectInstance {
                key: LIGHTNING_SPAWN_HOOK.key,
                index: current,
            });
        }
    }

    let applied_preset = selected_entity.and_then(|entity| {
        ecs_world
            .get_component::<AppliedLightningPreset>(entity)
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
            ui_events.send(UIEvent::ClearScalarKeys);
            send_effect_ui_command(ecs_world, LightningUiCommand::ApplyPreset(chosen));
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
            &["end_offset"],
            thyllore_effect_core::LIGHTNING_UI_PARAMS,
            thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
            &mut effect_copy,
            |ui, edited| lightning_key_button(ui, ui_events, edited),
        );
    }

    draw_tiered_params(
        ui,
        thyllore_effect_core::LIGHTNING_UI_PARAMS,
        thyllore_effect_core::LIGHTNING_SCALAR_PARAMS,
        &mut effect_copy,
        &["end_offset"],
        |ui, edited| lightning_key_button(ui, ui_events, edited),
    );

    if !effect_applied_this_frame {
        send_effect_ui_command(
            ecs_world,
            LightningUiCommand::UpdateEffect {
                entity: selected,
                effect: Box::new(effect_copy),
            },
        );
    }
    if ui.button("Curves") {
        ui_events.send(UIEvent::OpenScalarCurveEditor);
    }
    if !ui.collapsing_header("Lightning Debug", imgui::TreeNodeFlags::empty()) {
        return;
    }
    draw_lightning_render_settings(ui, ui_events, ecs_world);
    if ui.button("Dump Debug") {
        ui_events.send(UIEvent::CaptureNow(Rc::new(LightningDebugCapture)));
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
    lightning: Entity,
) -> Option<String> {
    let target_name = ecs_world
        .get_component::<LightningTarget>(lightning)
        .map(|target| target.entity_name.clone());
    match &target_name {
        Some(name) => {
            ui.text(format!("Target: {name}"));
            ui.same_line();
            if ui.small_button("Clear##lightning_target") {
                send_effect_ui_command(ecs_world, LightningUiCommand::ClearTarget);
            }
        }
        None => {
            if ui.button("Add Target") {
                send_effect_ui_command(ecs_world, LightningUiCommand::AddTarget);
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
    lightning: Entity,
) {
    let waypoints = ecs_world
        .get_component::<LightningPath>(lightning)
        .map(|path| path.waypoints.clone())
        .unwrap_or_default();

    for (i, name) in waypoints.iter().enumerate() {
        ui.text(name);
        ui.same_line();
        if ui.small_button(format!("x##waypoint{i}")) {
            send_effect_ui_command(ecs_world, LightningUiCommand::RemoveWaypoint(i));
        }
    }

    if ui.button("Add Waypoint") {
        send_effect_ui_command(ecs_world, LightningUiCommand::AddWaypoint);
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Spawn a locator halfway to the end point; the bolt passes through waypoints in order",
        );
    }
}

fn draw_lightning_render_settings(
    ui: &imgui::Ui,
    _ui_events: &mut UIEventQueue,
    ecs_world: &World,
) {
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
    send_effect_ui_command(
        ecs_world,
        LightningUiCommand::UpdateRenderSettings(settings_copy),
    );
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "lightning",
    order: 3,
    draw: build_lightning_section,
});
