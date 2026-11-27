use thyllore_effect_core::lightning::END_FOLLOWS_TARGET;
use thyllore_scene_core::{ScalarParam, UiCondition, UiParam};

use crate::ecs::component::{
    AppliedLightningPreset, LightningEffect, LightningPath, LightningTarget, ScalarChannelDomain,
    LIGHTNING_DOMAIN,
};
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::LIGHTNING_SPAWN_HOOK;
use crate::ecs::world::{Entity, World};

use super::rows::{
    lightning_debug_rows, lightning_extra_head_rows, LightningExtra, LightningSectionView,
};
use crate::platform::ui::effect::sections::{build_effect_section, EffectPanel};
use crate::platform::ui::ui_rows::UiRow;

struct LightningPanel;

impl EffectPanel for LightningPanel {
    type Component = LightningEffect;
    type RenderSettings = LightningRenderSettings;
    type Extra = LightningExtra;

    const NAME: &'static str = "Lightning";
    const ADD_LABEL: &'static str = "Add Lightning";
    const SPAWN_HOOK_KEY: &'static str = LIGHTNING_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] =
        thyllore_effect_core::lightning::LIGHTNING_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &thyllore_effect_core::lightning::LIGHTNING_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<LightningEffect>] {
        &thyllore_effect_core::lightning::LIGHTNING_SCALAR_PARAMS
    }

    fn domain() -> &'static ScalarChannelDomain {
        &LIGHTNING_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedLightningPreset) -> String {
        applied.name.clone()
    }

    fn debug_rows(_view: &LightningSectionView) -> Vec<UiRow<LightningSectionView>> {
        lightning_debug_rows()
    }

    fn read_extra(world: &World, selected: Option<Entity>) -> LightningExtra {
        let target_name = selected.and_then(|entity| {
            world
                .get_component::<LightningTarget>(entity)
                .map(|target| target.entity_name.clone())
        });
        let waypoints = selected
            .and_then(|entity| {
                world
                    .get_component::<LightningPath>(entity)
                    .map(|path| path.waypoints.clone())
            })
            .unwrap_or_default();

        LightningExtra {
            target_name,
            waypoints,
        }
    }

    fn extra_head_rows(view: &LightningSectionView) -> Vec<UiRow<LightningSectionView>> {
        lightning_extra_head_rows(view)
    }

    fn active_conditions(view: &LightningSectionView) -> Vec<&'static UiCondition> {
        match view.extra.target_name {
            Some(_) => vec![&END_FOLLOWS_TARGET],
            None => Vec::new(),
        }
    }
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "lightning",
    order: 3,
    section: build_effect_section::<LightningPanel>,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::platform::ui::effect::sections::{commit_effect_view, effect_head_rows};
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_instances(count: usize) -> LightningSectionView {
        LightningSectionView {
            instances: (1..=count).map(|i| format!("Lightning {i}")).collect(),
            instance_index: 0,
            preset: None,
            selected: None,
            effect: LightningEffect::default(),
            render: LightningRenderSettings::default(),
            extra: LightningExtra::default(),
        }
    }

    fn selected_view() -> LightningSectionView {
        LightningSectionView {
            selected: Some(0),
            ..view_with_instances(1)
        }
    }

    fn committed_command_names(
        shown: &LightningSectionView,
        view: &LightningSectionView,
    ) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_effect_view::<LightningPanel>(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    #[test]
    fn head_rows_with_two_instances_show_instance_choice() {
        let rows = effect_head_rows::<LightningPanel>(&view_with_instances(2));
        assert_eq!(
            visible_labels(&rows),
            ["Add Lightning", "Instance", "Preset"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = effect_head_rows::<LightningPanel>(&view_with_instances(1));
        assert_eq!(visible_labels(&rows), ["Add Lightning", "Preset"]);
    }

    #[test]
    fn end_follows_target_only_while_a_target_is_linked() {
        let mut view = selected_view();
        assert!(LightningPanel::active_conditions(&view).is_empty());

        view.extra.target_name = Some("Target Locator".to_string());
        let active = LightningPanel::active_conditions(&view);
        assert_eq!(active.len(), 1);
        assert!(std::ptr::eq(active[0], &END_FOLLOWS_TARGET));
    }

    #[test]
    fn commit_without_change_sends_nothing() {
        let shown = selected_view();
        assert!(committed_command_names(&shown, &shown.clone()).is_empty());
    }

    #[test]
    fn commit_preset_change_clears_keys_and_applies_preset_without_update() {
        let shown = selected_view();
        let view = LightningSectionView {
            preset: Some(thyllore_effect_core::lightning::LIGHTNING_PRESET_NAMES[0].to_string()),
            ..shown.clone()
        };

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names[0].starts_with("ClearScalarKeys"), "{names:?}");
        assert!(names[1].starts_with("ApplyPreset"), "{names:?}");
    }

    #[test]
    fn commit_effect_change_sends_update() {
        let shown = selected_view();
        let mut view = shown.clone();
        view.effect.time += 1.0;

        let names = committed_command_names(&shown, &view);
        assert!(
            names.iter().any(|name| name.starts_with("UpdateEffect")),
            "{names:?}"
        );
    }
}
