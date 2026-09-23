use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{WaterParam, WaterTorusEffect};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::water::WaterUiCommand;
use crate::ecs::systems::WATER_SPAWN_HOOK;
use crate::ecs::World;

use super::param_widgets::{draw_params, EditedScalars};
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

fn send_key_button(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    edited: EditedScalars,
    keys: Vec<(PropertyType, f32)>,
) {
    let (Some((first_name, _)), false) = (edited.first(), keys.is_empty()) else {
        return;
    };
    ui.same_line();
    if ui.small_button(format!("K##{first_name}")) {
        for (property_type, value) in keys {
            ui_events.send(UIEvent::InsertScalarKey {
                property_type,
                value,
            });
        }
    }
}

pub(super) fn build_water_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    _overlay_state: &mut SceneOverlayState,
    ecs_world: &World,
) {
    use crate::ecs::component::WaterTorusEffect;
    use crate::ecs::resource::WaterRenderSettings;

    if ui.collapsing_header("Water", imgui::TreeNodeFlags::empty()) {
        // WaterRenderSettings: secondary_rays combo and debug_view slider
        if let Some(settings) = ecs_world.get_resource::<WaterRenderSettings>() {
            let mut settings_copy = *settings;
            drop(settings);

            if let Some(_token) =
                ui.begin_combo("Secondary Rays", settings_copy.secondary_rays.label())
            {
                for mode in thyllore_effect_core::WaterSecondaryRays::ALL {
                    let selected = mode == settings_copy.secondary_rays;
                    if ui
                        .selectable_config(mode.label())
                        .selected(selected)
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

            ui_events.send(UIEvent::Effect {
                key: WATER_SPAWN_HOOK.key,
                command: Rc::new(WaterUiCommand::UpdateRenderSettings(settings_copy)),
            });
        }

        if ui.button("Add Water") {
            ui_events.send(UIEvent::AddEffect(WATER_SPAWN_HOOK.key));
        }
        ui.same_line();
        if ui.button("Dump Debug") {
            ui_events.send(UIEvent::CaptureNow(Rc::new(WaterDebugCapture)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Write water parameters, UBO, camera, render settings and a screenshot to log/water/",
            );
        }

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
                ui_events.send(UIEvent::SelectEffectInstance {
                    key: WATER_SPAWN_HOOK.key,
                    index: current,
                });
            }
        }

        let presets: Vec<String> = thyllore_effect_core::WATER_PRESET_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut effect_applied_this_frame = false;
        {
            let mut preset_index = ecs_world
                .resource_mut::<crate::ecs::WaterUIState>()
                .preset_index;
            let preset_changed =
                ui.combo_simple_string("Water Preset", &mut preset_index, &presets);
            ecs_world
                .resource_mut::<crate::ecs::WaterUIState>()
                .preset_index = preset_index;
            if preset_changed {
                if selected_water_entity.is_some() {
                    ui_events.send(UIEvent::ClearScalarKeys);
                    ui_events.send(UIEvent::Effect {
                        key: WATER_SPAWN_HOOK.key,
                        command: Rc::new(WaterUiCommand::ApplyPreset(
                            presets[preset_index].clone(),
                        )),
                    });
                    effect_applied_this_frame = true;
                }
            }

            if let Some(selected_water) = selected_water_entity {
                if let Some(effect) = ecs_world.get_component::<WaterTorusEffect>(selected_water) {
                    let mut effect_copy = effect.clone();

                    let mut drawn_groups: Vec<&str> = Vec::new();
                    for group in thyllore_effect_core::WATER_UI_PARAMS
                        .iter()
                        .map(|param| param.group)
                        .filter(|group| !group.is_empty())
                    {
                        if drawn_groups.contains(&group) {
                            continue;
                        }
                        drawn_groups.push(group);

                        let names: Vec<&str> = thyllore_effect_core::WATER_UI_PARAMS
                            .iter()
                            .filter(|param| param.group == group)
                            .map(|param| param.name)
                            .collect();
                        draw_params(
                            ui,
                            &names,
                            thyllore_effect_core::WATER_UI_PARAMS,
                            thyllore_effect_core::WATER_SCALAR_PARAMS,
                            &mut effect_copy,
                            |ui, edited| water_key_button(ui, ui_events, edited),
                        );
                    }

                    if !effect_applied_this_frame {
                        ui_events.send(UIEvent::Effect {
                            key: WATER_SPAWN_HOOK.key,
                            command: Rc::new(WaterUiCommand::UpdateEffect(Box::new(effect_copy))),
                        });
                    }

                    if ui.button("Curves") {
                        ui_events.send(UIEvent::OpenScalarCurveEditor);
                    }
                }
            }
        }
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    draw: build_water_section,
});
