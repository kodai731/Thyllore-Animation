use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::ecs::component::{
    AppliedWindPreset, ScalarChannelDomain, WindTornadoEffect, WIND_DOMAIN,
};
use crate::ecs::resource::{WindDebugCapture, WindDebugView, WindRenderSettings, WindShadingMode};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::WIND_SPAWN_HOOK;
use crate::platform::ui::effect::sections::{build_effect_section, EffectPanel, EffectSectionView};
use crate::platform::ui::ui_rows::{ChoiceOption, UiRow};

type WindSectionView = EffectSectionView<WindTornadoEffect, WindRenderSettings>;

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

struct WindPanel;

impl EffectPanel for WindPanel {
    type Component = WindTornadoEffect;
    type RenderSettings = WindRenderSettings;

    const NAME: &'static str = "Wind";
    const ADD_LABEL: &'static str = "Add Wind";
    const SPAWN_HOOK_KEY: &'static str = WIND_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] = thyllore_effect_core::wind::WIND_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &thyllore_effect_core::wind::WIND_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<WindTornadoEffect>] {
        &thyllore_effect_core::wind::WIND_SCALAR_PARAMS
    }

    fn domain() -> &'static ScalarChannelDomain {
        &WIND_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedWindPreset) -> String {
        applied.name.clone()
    }

    fn debug_rows() -> Vec<UiRow<WindSectionView>> {
        wind_debug_rows()
    }
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "wind",
    order: 2,
    section: build_effect_section::<WindPanel>,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::ecs::world::World;
    use crate::platform::ui::effect::sections::{commit_effect_view, effect_head_rows};
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
        let rows = effect_head_rows::<WindPanel>(&view_with_instances(2));
        assert_eq!(visible_labels(&rows), ["Add Wind", "Instance", "Preset"]);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = effect_head_rows::<WindPanel>(&view_with_instances(1));
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
        commit_effect_view::<WindPanel>(&world, shown, view);

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
            preset: Some(thyllore_effect_core::wind::WIND_PRESET_NAMES[0].to_string()),
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
