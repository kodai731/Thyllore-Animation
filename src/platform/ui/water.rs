use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect, WATER_DOMAIN};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::water::WaterUiCommand;
use crate::ecs::systems::{resolve_selected_water, WATER_SPAWN_HOOK};
use crate::ecs::World;
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::param_widgets::{draw_preset_combo, draw_tiered_params, EditedScalars};
use super::scene_overlay::send_key_button;

fn water_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WATER_DOMAIN
                .property_type_for_cli_name(name)
                .map(|property_type| (property_type, *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

pub(super) fn build_water_section(ui: &imgui::Ui, ecs_world: &World) {
    if !section_header(ui, ecs_world, "Water", SectionDefault::Closed) {
        return;
    }
    let _section_id = ui.push_id("water");

    if ui.button("Add Water") {
        ecs_world.send_command(ScalarCurveEvent::AddEffect(WATER_SPAWN_HOOK.key));
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
            ecs_world.send_command(OverlayEvent::SelectEffectInstance {
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
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ecs_world.send_command(WaterUiCommand::ApplyPreset(chosen));
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
        ecs_world,
        &thyllore_effect_core::WATER_UI_PARAMS,
        &thyllore_effect_core::WATER_SCALAR_PARAMS,
        &mut effect_copy,
        &[],
        |ui, edited| water_key_button(ui, ecs_world, edited),
    );
    if !effect_applied_this_frame {
        ecs_world.send_command(WaterUiCommand::UpdateEffect {
            entity: selected_water,
            effect: Box::new(effect_copy),
        });
    }
    if ui.button("Curves") {
        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
    }
    if section_header(ui, ecs_world, "Water Debug", SectionDefault::Closed) {
        draw_water_render_settings(ui, ecs_world);
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

fn draw_water_render_settings(ui: &imgui::Ui, ecs_world: &World) {
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
    ecs_world.send_command(WaterUiCommand::UpdateRenderSettings(settings_copy));
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    draw: build_water_section,
});
