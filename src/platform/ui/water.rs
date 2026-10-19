use std::rc::Rc;

use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::WaterSecondaryRays;

use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect, WATER_DOMAIN};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::water::WaterUiCommand;
use crate::ecs::systems::{resolve_selected_water, WATER_SPAWN_HOOK};
use crate::ecs::{Entity, World};
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::effect_rows::{effect_instances, instance_options, preset_options};
use super::param_widgets::{rows_from_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::ui_rows::{draw_rows, ChoiceOption, UiRow};

#[derive(Clone)]
struct WaterSectionView {
    instances: Vec<String>,
    instance_index: usize,
    preset: Option<String>,
    render: WaterRenderSettings,
}

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

fn water_head_rows(view: &WaterSectionView) -> Vec<UiRow<WaterSectionView>> {
    let mut rows = vec![UiRow::Action {
        label: "Add Water",
        enabled: |_, _| true,
        on_click: |_| Box::new(ScalarCurveEvent::AddEffect(WATER_SPAWN_HOOK.key)),
    }];

    if view.instances.len() > 1 {
        rows.push(UiRow::Choice {
            label: "Instance",
            options: |_, view: &WaterSectionView| {
                instance_options(&view.instances, view.instance_index)
            },
            pick: |view, index| view.instance_index = index,
        });
    }

    rows.push(UiRow::Choice {
        label: "Preset",
        options: |_, view: &WaterSectionView| {
            preset_options(
                thyllore_effect_core::WATER_PRESET_NAMES,
                view.preset.as_deref(),
            )
        },
        pick: |view, index| {
            view.preset = Some(thyllore_effect_core::WATER_PRESET_NAMES[index].to_string());
        },
    });

    rows
}

fn water_tail_rows() -> Vec<UiRow<WaterSectionView>> {
    vec![
        UiRow::Action {
            label: "Curves",
            enabled: |_, _| true,
            on_click: |_| Box::new(ScalarCurveEvent::OpenScalarCurveEditor),
        },
        UiRow::Group {
            title: "Water Debug".into(),
            initially_open: false,
            rows: water_debug_rows(),
        },
    ]
}

fn water_debug_rows() -> Vec<UiRow<WaterSectionView>> {
    vec![
        UiRow::Choice {
            label: "Secondary Rays",
            options: |_, view: &WaterSectionView| {
                WaterSecondaryRays::ALL
                    .iter()
                    .map(|&mode| ChoiceOption {
                        label: mode.label().to_string(),
                        selected: mode == view.render.secondary_rays,
                    })
                    .collect()
            },
            pick: |view, index| view.render.secondary_rays = WaterSecondaryRays::ALL[index],
        },
        UiRow::Toggle {
            label: "Debug View",
            get: |view| view.render.debug_view != 0,
            set: |view, value| view.render.debug_view = i32::from(value),
        },
        UiRow::Toggle {
            label: "Animate When Paused",
            get: |view| view.render.free_run_when_paused,
            set: |view, value| view.render.free_run_when_paused = value,
        },
        UiRow::Action {
            label: "Dump Debug",
            enabled: |_, _| true,
            on_click: |_| Box::new(CameraEvent::CaptureNow(Rc::new(WaterDebugCapture))),
        },
        UiRow::Tooltip(
            "Write water parameters, UBO, camera, render settings and a screenshot to log/water/",
        ),
    ]
}

fn read_water_section_view(
    ecs_world: &World,
    waters: &[Entity],
    selected_water_entity: Option<Entity>,
) -> WaterSectionView {
    let instance_index = selected_water_entity
        .and_then(|entity| waters.iter().position(|&e| e == entity))
        .unwrap_or(0);
    let (instances, _) = effect_instances(ecs_world, waters, instance_index, "Water");
    let preset = selected_water_entity.and_then(|entity| {
        ecs_world
            .get_component::<AppliedWaterPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let render = ecs_world
        .get_resource::<WaterRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();

    WaterSectionView {
        instances,
        instance_index,
        preset,
        render,
    }
}

pub(super) fn build_water_section(ui: &imgui::Ui, ecs_world: &World) {
    if !section_header(ui, ecs_world, "Water", SectionDefault::Closed) {
        return;
    }
    let _section_id = ui.push_id("water");

    let waters = ecs_world.entities_with::<WaterTorusEffect>();
    let selected_water_entity = resolve_selected_water(ecs_world);
    let shown = read_water_section_view(ecs_world, &waters, selected_water_entity);
    let mut view = shown.clone();

    draw_rows(ui, ecs_world, &water_head_rows(&view), &mut view, |_, _| {});

    if view.instance_index != shown.instance_index {
        ecs_world.send_command(OverlayEvent::SelectEffectInstance {
            key: WATER_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, selected_water_entity) {
        if view.preset != shown.preset {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ecs_world.send_command(WaterUiCommand::ApplyPreset(chosen.clone()));
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
    let rows = rows_from_params(
        &thyllore_effect_core::WATER_UI_PARAMS,
        &thyllore_effect_core::WATER_SCALAR_PARAMS,
        &[],
    );
    draw_rows(ui, ecs_world, &rows, &mut effect_copy, |ui, edited| {
        water_key_button(ui, ecs_world, edited);
    });
    if !effect_applied_this_frame {
        ecs_world.send_command(WaterUiCommand::UpdateEffect {
            entity: selected_water,
            effect: Box::new(effect_copy),
        });
    }

    draw_rows(ui, ecs_world, &water_tail_rows(), &mut view, |_, _| {});
    if view.render != shown.render {
        ecs_world.send_command(WaterUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    draw: build_water_section,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> WaterSectionView {
        WaterSectionView {
            instances: (1..=count).map(|i| format!("Water {i}")).collect(),
            instance_index: 0,
            preset: None,
            render: WaterRenderSettings::default(),
        }
    }

    #[test]
    fn head_rows_with_two_instances_show_instance_choice() {
        let rows = water_head_rows(&view_with_instances(2));
        assert_eq!(visible_labels(&rows), ["Add Water", "Instance", "Preset"]);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = water_head_rows(&view_with_instances(1));
        assert_eq!(visible_labels(&rows), ["Add Water", "Preset"]);
    }

    #[test]
    fn debug_rows_keep_order_without_duplicates() {
        let rows = water_debug_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Secondary Rays",
                "Debug View",
                "Animate When Paused",
                "Dump Debug",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
