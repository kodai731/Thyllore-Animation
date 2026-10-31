use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::resource::{WindDebugCapture, WindDebugView, WindRenderSettings, WindShadingMode};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::wind::WindUiCommand;
use crate::ecs::systems::{resolve_selected_wind, WIND_SPAWN_HOOK};
use crate::ecs::{Entity, World};
use crate::platform::ui::effect::rows::{effect_instances, instance_options, preset_options};
use crate::platform::ui::form_window::{FormSection, RowsSection};
use crate::platform::ui::param_widgets::rows_from_params;
use crate::platform::ui::ui_rows::{ChoiceOption, FieldRows, UiRow};

#[derive(Clone)]
struct WindSectionView {
    instances: Vec<String>,
    instance_index: usize,
    preset: Option<String>,
    selected: Option<Entity>,
    effect: WindTornadoEffect,
    render: WindRenderSettings,
}

struct WindRenderPath;

impl FieldPath<WindSectionView> for WindRenderPath {
    type Field = WindRenderSettings;

    fn get(root: &WindSectionView) -> &WindRenderSettings {
        &root.render
    }

    fn get_mut(root: &mut WindSectionView) -> &mut WindRenderSettings {
        &mut root.render
    }
}

static RENDER_SCALARS: LazyLock<Vec<ScalarParam<WindSectionView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    WindRenderSettings::collect_scalars::<WindSectionView, WindRenderPath>("", &mut out);
    out
});

static RENDER_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    WindRenderSettings::collect_ui("", "", &mut out);
    out
});

fn wind_head_rows(view: &WindSectionView) -> Vec<UiRow<WindSectionView>> {
    let mut rows = vec![UiRow::Action {
        label: "Add Wind",
        enabled: |_, _| true,
        on_click: |_| Box::new(ScalarCurveEvent::AddEffect(WIND_SPAWN_HOOK.key)),
    }];

    if view.instances.len() > 1 {
        rows.push(UiRow::Choice {
            label: "Instance",
            options: |_, view: &WindSectionView| {
                instance_options(&view.instances, view.instance_index)
            },
            pick: |view, index| view.instance_index = index,
        });
    }

    rows.push(UiRow::Choice {
        label: "Preset",
        options: |_, view: &WindSectionView| {
            preset_options(
                thyllore_effect_core::WIND_PRESET_NAMES,
                view.preset.as_deref(),
            )
        },
        pick: |view, index| {
            view.preset = Some(thyllore_effect_core::WIND_PRESET_NAMES[index].to_string());
        },
    });

    rows
}

fn wind_tail_rows() -> Vec<UiRow<WindSectionView>> {
    vec![
        UiRow::Action {
            label: "Curves",
            enabled: |_, _| true,
            on_click: |_| Box::new(ScalarCurveEvent::OpenScalarCurveEditor),
        },
        UiRow::Group {
            title: "Wind Debug".into(),
            initially_open: false,
            rows: wind_debug_rows(),
        },
    ]
}

fn wind_debug_rows() -> Vec<UiRow<WindSectionView>> {
    let mut rows = vec![
        UiRow::Choice {
            label: "Shading Mode",
            options: |_, view: &WindSectionView| {
                WindShadingMode::ALL
                    .iter()
                    .map(|&mode| ChoiceOption {
                        label: mode.label().to_string(),
                        selected: mode == view.render.shading_mode,
                    })
                    .collect()
            },
            pick: |view, index| view.render.shading_mode = WindShadingMode::ALL[index],
        },
        UiRow::Choice {
            label: "Debug View",
            options: |_, view: &WindSectionView| {
                WindDebugView::ALL
                    .iter()
                    .map(|&debug_view| ChoiceOption {
                        label: debug_view.label().to_string(),
                        selected: debug_view == view.render.debug_view,
                    })
                    .collect()
            },
            pick: |view, index| view.render.debug_view = WindDebugView::ALL[index],
        },
    ];

    rows.extend(RENDER_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &RENDER_SCALARS,
    }));

    rows.push(UiRow::Toggle {
        label: "Animate When Paused",
        get: |view| view.render.free_run_when_paused,
        set: |view, value| view.render.free_run_when_paused = value,
    });
    rows.push(UiRow::Action {
        label: "Dump Debug",
        enabled: |_, _| true,
        on_click: |_| Box::new(CameraEvent::CaptureNow(Rc::new(WindDebugCapture))),
    });
    rows.push(UiRow::Tooltip(
        "Write wind parameters, UBO, render settings, camera and a screenshot to log/wind/",
    ));
    rows
}

fn read_wind_section_view(
    ecs_world: &World,
    winds: &[Entity],
    selected_wind_entity: Option<Entity>,
) -> WindSectionView {
    let instance_index = selected_wind_entity
        .and_then(|entity| winds.iter().position(|&e| e == entity))
        .unwrap_or(0);
    let (instances, _) = effect_instances(ecs_world, winds, instance_index, "Wind");
    let preset = selected_wind_entity.and_then(|entity| {
        ecs_world
            .get_component::<AppliedWindPreset>(entity)
            .map(|preset| preset.name.clone())
    });
    let render = ecs_world
        .get_resource::<WindRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();
    let effect = selected_wind_entity
        .and_then(|entity| ecs_world.get_component::<WindTornadoEffect>(entity))
        .cloned()
        .unwrap_or_default();

    WindSectionView {
        instances,
        instance_index,
        preset,
        selected: selected_wind_entity,
        effect,
        render,
    }
}

fn wind_section(world: &World) -> Option<Box<dyn FormSection>> {
    let winds = world.entities_with::<WindTornadoEffect>();
    let selected_wind_entity = resolve_selected_wind(world);
    let view = read_wind_section_view(world, &winds, selected_wind_entity);

    let mut rows = wind_head_rows(&view);
    if view.selected.is_some() {
        rows.push(UiRow::Field(Box::new(FieldRows {
            scope: "effect",
            field: |view: &mut WindSectionView| &mut view.effect,
            rows: rows_from_params(
                &thyllore_effect_core::WIND_UI_PARAMS,
                &thyllore_effect_core::WIND_SCALAR_PARAMS,
                &[],
            ),
        })));
        rows.extend(wind_tail_rows());
    }

    let group = UiRow::Group {
        title: "Wind".into(),
        initially_open: false,
        rows,
    };

    Some(Box::new(RowsSection {
        rows: vec![group],
        view,
        commit: commit_wind_view,
        keys: Some(&WIND_DOMAIN),
    }))
}

fn commit_wind_view(world: &World, shown: &WindSectionView, view: &WindSectionView) {
    if view.instance_index != shown.instance_index {
        world.send_command(OverlayEvent::SelectEffectInstance {
            key: WIND_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, view.selected) {
        if view.preset != shown.preset {
            world.send_command(ScalarCurveEvent::ClearScalarKeys);
            world.send_command(WindUiCommand::ApplyPreset(chosen.clone()));
            effect_applied_this_frame = true;
        }
    }

    let Some(entity) = view.selected else {
        return;
    };
    if !effect_applied_this_frame && view.effect != shown.effect {
        world.send_command(WindUiCommand::UpdateEffect {
            entity,
            effect: Box::new(view.effect.clone()),
        });
    }

    if view.render != shown.render {
        world.send_command(WindUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "wind",
    order: 2,
    section: wind_section,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> WindSectionView {
        WindSectionView {
            instances: (1..=count).map(|i| format!("Wind {i}")).collect(),
            instance_index: 0,
            preset: None,
            selected: None,
            effect: WindTornadoEffect::default(),
            render: WindRenderSettings::default(),
        }
    }

    #[test]
    fn head_rows_with_two_instances_show_instance_choice() {
        let rows = wind_head_rows(&view_with_instances(2));
        assert_eq!(visible_labels(&rows), ["Add Wind", "Instance", "Preset"]);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = wind_head_rows(&view_with_instances(1));
        assert_eq!(visible_labels(&rows), ["Add Wind", "Preset"]);
    }

    #[test]
    fn debug_rows_keep_order_without_duplicates() {
        let rows = wind_debug_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Shading Mode",
                "Debug View",
                "reference_step_count",
                "Animate When Paused",
                "Dump Debug",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    fn committed_command_names(shown: &WindSectionView, view: &WindSectionView) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_wind_view(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    fn selected_view() -> WindSectionView {
        WindSectionView {
            selected: Some(0),
            ..view_with_instances(1)
        }
    }

    #[test]
    fn commit_without_change_or_selection_sends_nothing() {
        let shown = view_with_instances(1);
        assert!(committed_command_names(&shown, &shown.clone()).is_empty());
    }

    #[test]
    fn commit_preset_change_clears_keys_and_applies_preset_without_update() {
        let shown = selected_view();
        let view = WindSectionView {
            preset: Some(thyllore_effect_core::WIND_PRESET_NAMES[0].to_string()),
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
}
