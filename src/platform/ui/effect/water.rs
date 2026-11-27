use std::rc::Rc;

use thyllore_effect_core::water::WaterSecondaryRays;
use thyllore_scene_core::{ScalarParam, UiParam};

use crate::ecs::component::{
    AppliedWaterPreset, ScalarChannelDomain, WaterTorusEffect, WATER_DOMAIN,
};
use crate::ecs::resource::{WaterDebugCapture, WaterRenderSettings};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::WATER_SPAWN_HOOK;
use crate::platform::ui::effect::sections::{build_effect_section, EffectPanel, EffectSectionView};
use crate::platform::ui::ui_rows::UiRow;

type WaterSectionView = EffectSectionView<WaterTorusEffect, WaterRenderSettings>;

fn water_debug_rows() -> Vec<UiRow<WaterSectionView>> {
    vec![
        crate::variant_choice_row!(
            "Secondary Rays",
            WaterSectionView,
            WaterSecondaryRays,
            render.secondary_rays
        ),
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

struct WaterPanel;

impl EffectPanel for WaterPanel {
    type Component = WaterTorusEffect;
    type RenderSettings = WaterRenderSettings;
    type Extra = ();

    const NAME: &'static str = "Water";
    const ADD_LABEL: &'static str = "Add Water";
    const SPAWN_HOOK_KEY: &'static str = WATER_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] = thyllore_effect_core::water::WATER_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &thyllore_effect_core::water::WATER_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<WaterTorusEffect>] {
        &thyllore_effect_core::water::WATER_SCALAR_PARAMS
    }

    fn domain() -> &'static ScalarChannelDomain {
        &WATER_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedWaterPreset) -> String {
        applied.name.clone()
    }

    fn debug_rows(_view: &WaterSectionView) -> Vec<UiRow<WaterSectionView>> {
        water_debug_rows()
    }
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "water",
    order: 1,
    section: build_effect_section::<WaterPanel>,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::ecs::world::World;
    use crate::platform::ui::effect::sections::{commit_effect_view, effect_head_rows};
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> WaterSectionView {
        WaterSectionView {
            instances: (1..=count).map(|i| format!("Water {i}")).collect(),
            instance_index: 0,
            preset: None,
            selected: None,
            effect: WaterTorusEffect::default(),
            render: WaterRenderSettings::default(),
            extra: (),
        }
    }

    fn committed_command_names(shown: &WaterSectionView, view: &WaterSectionView) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_effect_view::<WaterPanel>(&world, shown, view);

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
        let rows = effect_head_rows::<WaterPanel>(&view_with_instances(2));
        assert_eq!(visible_labels(&rows), ["Add Water", "Instance", "Preset"]);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = effect_head_rows::<WaterPanel>(&view_with_instances(1));
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
            preset: Some(thyllore_effect_core::water::WATER_PRESET_NAMES[0].to_string()),
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
