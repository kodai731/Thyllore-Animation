use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::{WindParam, WindTornadoEffect};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{WindDebugCapture, WindDebugView, WindRenderSettings, WindShadingMode};
use crate::ecs::systems::wind::WindUiCommand;
use crate::ecs::systems::WIND_SPAWN_HOOK;
use crate::ecs::World;

use super::param_widgets::{draw_params, EditedScalars};
use super::SceneOverlayState;

fn wind_key_button(ui: &imgui::Ui, ui_events: &mut UIEventQueue, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WindParam::from_cli_name(name).map(|param| (param.property_type(), *value))
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

pub(super) fn build_wind_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    overlay_state: &mut SceneOverlayState,
    ecs_world: &World,
) {
    use crate::ecs::component::WindTornadoEffect;
    use crate::ecs::resource::{WindDebugView, WindRenderSettings, WindShadingMode};

    if !ui.collapsing_header("Wind", imgui::TreeNodeFlags::empty()) {
        return;
    }

    if let Some(settings) = ecs_world.get_resource::<WindRenderSettings>() {
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
        ui_events.send(UIEvent::Effect {
            key: WIND_SPAWN_HOOK.key,
            command: Rc::new(WindUiCommand::UpdateRenderSettings(settings_copy)),
        });
    }

    if ui.button("Add Wind") {
        ui_events.send(UIEvent::AddEffect(WIND_SPAWN_HOOK.key));
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
            ui_events.send(UIEvent::SelectEffectInstance {
                key: WIND_SPAWN_HOOK.key,
                index: current,
            });
        }
    }

    let presets: Vec<String> = thyllore_effect_core::WIND_PRESET_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut preset_index = overlay_state.wind_preset_index;
    let preset_changed = ui.combo_simple_string("Wind Preset", &mut preset_index, &presets);
    overlay_state.wind_preset_index = preset_index;
    let mut effect_applied_this_frame = false;
    if preset_changed && selected_wind_entity.is_some() {
        ui_events.send(UIEvent::ClearScalarKeys);
        ui_events.send(UIEvent::Effect {
            key: WIND_SPAWN_HOOK.key,
            command: Rc::new(WindUiCommand::ApplyPreset(presets[preset_index].clone())),
        });
        effect_applied_this_frame = true;
    }

    let Some(selected_wind) = selected_wind_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<WindTornadoEffect>(selected_wind) else {
        return;
    };
    let mut effect_copy = effect.clone();
    let mut drawn_groups: Vec<&str> = Vec::new();
    for group in thyllore_effect_core::WIND_UI_PARAMS
        .iter()
        .map(|param| param.group)
        .filter(|group| !group.is_empty())
    {
        if drawn_groups.contains(&group) {
            continue;
        }
        drawn_groups.push(group);

        let names: Vec<&str> = thyllore_effect_core::WIND_UI_PARAMS
            .iter()
            .filter(|param| param.group == group)
            .map(|param| param.name)
            .collect();
        draw_params(
            ui,
            &names,
            thyllore_effect_core::WIND_UI_PARAMS,
            thyllore_effect_core::WIND_SCALAR_PARAMS,
            &mut effect_copy,
            |ui, edited| wind_key_button(ui, ui_events, edited),
        );
    }
    if !effect_applied_this_frame {
        ui_events.send(UIEvent::Effect {
            key: WIND_SPAWN_HOOK.key,
            command: Rc::new(WindUiCommand::UpdateEffect(Box::new(effect_copy))),
        });
    }
    if ui.button("Curves") {
        ui_events.send(UIEvent::OpenScalarCurveEditor);
    }
    ui.same_line();
    if ui.button("Dump Debug") {
        ui_events.send(UIEvent::CaptureNow(Rc::new(WindDebugCapture)));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Write wind parameters, UBO, render settings, camera and a screenshot to log/wind/",
        );
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "wind",
    order: 2,
    draw: build_wind_section,
});
