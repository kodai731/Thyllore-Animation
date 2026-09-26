use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{AppliedWaterPreset, WaterParam, WaterTorusEffect};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::water::WaterUiCommand;
use crate::ecs::systems::{resolve_selected_water, WATER_SPAWN_HOOK};
use crate::ecs::World;
use crate::hooks::effect_ui_event::send_effect_ui_command;

use super::param_widgets::{draw_preset_combo, draw_tiered_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::SceneOverlayState;

fn water_key_button(ui: &imgui::Ui, ui_events: &mut UIEventQueue, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WaterParam::from_cli_name(name).map(|param| (param.property_type(), *value))
        })
        .collect();
    send_key_button(ui, ui_events, edited, keys);
}

pub(super) fn build_water_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    _overlay_state: &mut SceneOverlayState,
    ecs_world: &World,
) {
    if !ui.collapsing_header("Water", imgui::TreeNodeFlags::empty()) {
        return;
    }
    let _section_id = ui.push_id("water");

    if ui.button("Add Water") {
        ui_events.send(UIEvent::AddEffect(WATER_SPAWN_HOOK.key));
    }

    let waters = ecs_world.entities_with::<WaterTorusEffect>();
    let selected_water_entity = resolve_selected_water(ecs_world);
    if waters.len() > 1 {
        let mut current = selected_water_entity
            .and_then(|entity| waters.iter().position(|&e| e == entity))
            .unwrap_or(0);
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
            ui_events.send(UIEvent::SelectEffectInstance {
                key: WATER_SPAWN_HOOK.key,
                index: current,
            });
        }
    }

    let applied_preset = selected_water_entity.and_then(|entity| {
        ecs_world
            .get_component::<AppliedWaterPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let mut effect_applied_this_frame = false;
    if let Some(chosen) = draw_preset_combo(
        ui,
        "Water Preset",
        thyllore_effect_core::WATER_PRESET_NAMES,
        applied_preset.as_deref(),
    ) {
        if selected_water_entity.is_some() {
            ui_events.send(UIEvent::ClearScalarKeys);
            send_effect_ui_command(ecs_world, WaterUiCommand::ApplyPreset(chosen));
            effect_applied_this_frame = true;
        }
    }

    let Some(selected_water) = selected_water_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<WaterTorusEffect>(selected_water) else {
        return;
    };
    let mut effect_copy = effect.clone();
    draw_tiered_params(
        ui,
        thyllore_effect_core::WATER_UI_PARAMS,
        thyllore_effect_core::WATER_SCALAR_PARAMS,
        &mut effect_copy,
        &[],
        |ui, edited| water_key_button(ui, ui_events, edited),
    );
    if !effect_applied_this_frame {
        send_effect_ui_command(
            ecs_world,
            WaterUiCommand::UpdateEffect {
                entity: selected_water,
                effect: Box::new(effect_copy),
            },
        );
    }
    if ui.button("Curves") {
        ui_events.send(UIEvent::OpenScalarCurveEditor);
    }
    if ui.collapsing_header("Water Debug", imgui::TreeNodeFlags::empty()) {
        draw_water_render_settings(ui, ui_events, ecs_world);
        if ui.button("Dump Debug") {
            ui_events.send(UIEvent::CaptureNow(Rc::new(WaterDebugCapture)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Write water parameters, UBO, camera, render settings and a screenshot to log/water/",
            );
        }
    }
}

fn draw_water_render_settings(ui: &imgui::Ui, _ui_events: &mut UIEventQueue, ecs_world: &World) {
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
    send_effect_ui_command(
        ecs_world,
        WaterUiCommand::UpdateRenderSettings(settings_copy),
    );
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    draw: build_water_section,
});
