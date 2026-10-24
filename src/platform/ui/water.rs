use std::rc::Rc;

use thyllore_effect_core::WaterSecondaryRays;

use super::effect_rows::{effect_instances, instance_options, preset_options};
use super::form_window::{FormSection, RowsSection};
use super::param_widgets::rows_from_params;
use super::ui_rows::{ChoiceOption, FieldRows, UiRow};
use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect, WATER_DOMAIN};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::water::WaterUiCommand;
use crate::ecs::systems::{resolve_selected_water, WATER_SPAWN_HOOK};
use crate::ecs::{Entity, World};

#[derive(Clone)]
struct WaterSectionView {
    instances: Vec<String>,
    instance_index: usize,
    preset: Option<String>,
    selected: Option<Entity>,
    effect: WaterTorusEffect,
    render: WaterRenderSettings,
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
    let effect = selected_water_entity
        .and_then(|entity| ecs_world.get_component::<WaterTorusEffect>(entity))
        .cloned()
        .unwrap_or_default();

    WaterSectionView {
        instances,
        instance_index,
        preset,
        selected: selected_water_entity,
        effect,
        render,
    }
}

fn water_section(world: &World) -> Option<Box<dyn FormSection>> {
    let waters = world.entities_with::<WaterTorusEffect>();
    let selected_water_entity = resolve_selected_water(world);
    let view = read_water_section_view(world, &waters, selected_water_entity);

    let mut rows = water_head_rows(&view);
    if view.selected.is_some() {
        rows.push(UiRow::Field(Box::new(FieldRows {
            field: |view: &mut WaterSectionView| &mut view.effect,
            rows: rows_from_params(
                &thyllore_effect_core::WATER_UI_PARAMS,
                &thyllore_effect_core::WATER_SCALAR_PARAMS,
                &[],
            ),
        })));
        rows.extend(water_tail_rows());
    }

    let group = UiRow::Group {
        title: "Water".into(),
        initially_open: false,
        rows,
    };

    Some(Box::new(RowsSection {
        rows: vec![group],
        view,
        commit: commit_water_view,
        keys: Some(&WATER_DOMAIN),
    }))
}

fn commit_water_view(world: &World, shown: &WaterSectionView, view: &WaterSectionView) {
    if view.instance_index != shown.instance_index {
        world.send_command(OverlayEvent::SelectEffectInstance {
            key: WATER_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, view.selected) {
        if view.preset != shown.preset {
            world.send_command(ScalarCurveEvent::ClearScalarKeys);
            world.send_command(WaterUiCommand::ApplyPreset(chosen.clone()));
            effect_applied_this_frame = true;
        }
    }

    let Some(entity) = view.selected else {
        return;
    };
    if !effect_applied_this_frame && view.effect != shown.effect {
        world.send_command(WaterUiCommand::UpdateEffect {
            entity,
            effect: Box::new(view.effect.clone()),
        });
    }

    if view.render != shown.render {
        world.send_command(WaterUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "water",
    order: 1,
    section: water_section,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::ui_command::UiCommandQueue;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> WaterSectionView {
        WaterSectionView {
            instances: (1..=count).map(|i| format!("Water {i}")).collect(),
            instance_index: 0,
            preset: None,
            selected: None,
            effect: WaterTorusEffect::default(),
            render: WaterRenderSettings::default(),
        }
    }

    fn committed_command_names(shown: &WaterSectionView, view: &WaterSectionView) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_water_view(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    fn selected_view() -> WaterSectionView {
        WaterSectionView {
            selected: Some(0),
            ..view_with_instances(1)
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

    #[test]
    fn commit_without_change_or_selection_sends_nothing() {
        let shown = view_with_instances(1);
        assert!(committed_command_names(&shown, &shown.clone()).is_empty());
    }

    #[test]
    fn commit_selected_same_effect_does_not_send_update() {
        let shown = selected_view();
        let view = shown.clone();
        let names = committed_command_names(&shown, &view);
        assert!(
            !names.iter().any(|name| name.starts_with("UpdateEffect")),
            "UpdateEffect should not be sent when effect is unchanged: {names:?}"
        );
    }

    #[test]
    fn commit_effect_change_sends_update() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.effect.time += 1.0;

        let names = committed_command_names(&shown, &view);
        assert!(
            names.iter().any(|name| name.starts_with("UpdateEffect")),
            "UpdateEffect should be sent when effect changes: {names:?}"
        );
    }

    #[test]
    fn commit_preset_change_clears_keys_and_applies_preset_without_update() {
        let shown = selected_view();
        let view = WaterSectionView {
            preset: Some(thyllore_effect_core::WATER_PRESET_NAMES[0].to_string()),
            ..shown.clone()
        };

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names[0].starts_with("ClearScalarKeys"), "{names:?}");
        assert!(names[1].starts_with("ApplyPreset"), "{names:?}");
    }

    #[test]
    fn commit_render_change_updates_render_settings() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.render.free_run_when_paused = !shown.render.free_run_when_paused;

        let names = committed_command_names(&shown, &view);
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("UpdateRenderSettings")),
            "{names:?}"
        );
    }
}
