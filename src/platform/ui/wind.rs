use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_anim_core::editable::PropertyType;
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect, WIND_DOMAIN};
use crate::ecs::resource::{WindDebugCapture, WindDebugView, WindRenderSettings, WindShadingMode};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::wind::WindUiCommand;
use crate::ecs::systems::{resolve_selected_wind, WIND_SPAWN_HOOK};
use crate::ecs::{Entity, World};
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;

use super::effect_rows::{effect_instances, instance_options, preset_options};
use super::param_widgets::{rows_from_params, EditedScalars};
use super::scene_overlay::send_key_button;
use super::ui_rows::{draw_rows, ChoiceOption, UiRow};

#[derive(Clone)]
struct WindSectionView {
    instances: Vec<String>,
    instance_index: usize,
    preset: Option<String>,
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

fn wind_key_button(ui: &imgui::Ui, ecs_world: &World, edited: EditedScalars) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            WIND_DOMAIN
                .property_type_for_cli_name(name)
                .map(|property_type| (property_type, *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}

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
            title: "Wind Debug",
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

    WindSectionView {
        instances,
        instance_index,
        preset,
        render,
    }
}

pub(super) fn build_wind_section(ui: &imgui::Ui, ecs_world: &World) {
    if !section_header(ui, ecs_world, "Wind", SectionDefault::Closed) {
        return;
    }
    let _section_id = ui.push_id("wind");

    let winds = ecs_world.entities_with::<WindTornadoEffect>();
    let selected_wind_entity = resolve_selected_wind(ecs_world);
    let shown = read_wind_section_view(ecs_world, &winds, selected_wind_entity);
    let mut view = shown.clone();

    draw_rows(ui, ecs_world, &wind_head_rows(&view), &mut view, |_, _| {});

    if view.instance_index != shown.instance_index {
        ecs_world.send_command(OverlayEvent::SelectEffectInstance {
            key: WIND_SPAWN_HOOK.key,
            index: view.instance_index,
        });
    }

    let mut effect_applied_this_frame = false;
    if let (Some(chosen), Some(_)) = (&view.preset, selected_wind_entity) {
        if view.preset != shown.preset {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ecs_world.send_command(WindUiCommand::ApplyPreset(chosen.clone()));
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
    let rows = rows_from_params(
        &thyllore_effect_core::WIND_UI_PARAMS,
        &thyllore_effect_core::WIND_SCALAR_PARAMS,
        &[],
    );
    draw_rows(ui, ecs_world, &rows, &mut effect_copy, |ui, edited| {
        wind_key_button(ui, ecs_world, edited);
    });
    if !effect_applied_this_frame {
        ecs_world.send_command(WindUiCommand::UpdateEffect {
            entity: selected_wind,
            effect: Box::new(effect_copy),
        });
    }

    draw_rows(ui, ecs_world, &wind_tail_rows(), &mut view, |_, _| {});
    if view.render != shown.render {
        ecs_world.send_command(WindUiCommand::UpdateRenderSettings(view.render));
    }
}

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "wind",
    order: 2,
    draw: build_wind_section,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> WindSectionView {
        WindSectionView {
            instances: (1..=count).map(|i| format!("Wind {i}")).collect(),
            instance_index: 0,
            preset: None,
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
}
