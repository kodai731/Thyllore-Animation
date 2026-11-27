use thyllore_effect_core::flame::emitter_ui_conditions;
use thyllore_scene_core::{ScalarParam, UiCondition, UiParam};

use crate::ecs::component::{AppliedFlamePreset, FlameEffect, ScalarChannelDomain, FLAME_DOMAIN};
use crate::ecs::resource::FlameRenderSettings;
use crate::ecs::systems::FLAME_SPAWN_HOOK;
use crate::ecs::world::{Entity, World};

use super::body_rows::{flame_debug_rows, flame_param_rows};
use super::head_rows::flame_extra_head_rows;
use super::section_view::{
    commit_flame_extra, edited_flame_effect, read_flame_extra, FlameExtra, FlameSectionView,
};
use crate::platform::ui::effect::sections::{
    build_effect_section, declared_param_rows, EffectPanel,
};
use crate::platform::ui::ui_rows::UiRow;

struct FlamePanel;

impl EffectPanel for FlamePanel {
    type Component = FlameEffect;
    type RenderSettings = FlameRenderSettings;
    type Extra = FlameExtra;

    const NAME: &'static str = "Flame";
    const ADD_LABEL: &'static str = "Add Flame";
    const SPAWN_HOOK_KEY: &'static str = FLAME_SPAWN_HOOK.key;
    const PRESET_NAMES: &'static [&'static str] = thyllore_effect_core::flame::FLAME_PRESET_NAMES;

    fn ui_params() -> &'static [UiParam] {
        &thyllore_effect_core::flame::FLAME_UI_PARAMS
    }

    fn scalar_params() -> &'static [ScalarParam<FlameEffect>] {
        &thyllore_effect_core::flame::FLAME_SCALAR_PARAMS
    }

    fn domain() -> &'static ScalarChannelDomain {
        &FLAME_DOMAIN
    }

    fn applied_preset_name(applied: &AppliedFlamePreset) -> String {
        applied.name.clone()
    }

    fn debug_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
        flame_debug_rows(view)
    }

    fn read_extra(world: &World, selected: Option<Entity>) -> FlameExtra {
        read_flame_extra(world, selected)
    }

    fn extra_head_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
        flame_extra_head_rows(view)
    }

    fn active_conditions(view: &FlameSectionView) -> Vec<&'static UiCondition> {
        emitter_ui_conditions(&view.effect.emitter)
    }

    fn param_rows(view: &FlameSectionView) -> Vec<UiRow<FlameEffect>> {
        flame_param_rows(declared_param_rows::<Self>(view))
    }

    fn edited_effect(shown: &FlameSectionView, view: &FlameSectionView) -> Option<FlameEffect> {
        edited_flame_effect(shown, view)
    }

    fn commit_extra(world: &World, shown: &FlameSectionView, view: &FlameSectionView) {
        commit_flame_extra(world, shown, view);
    }
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "flame",
    order: 4,
    section: build_effect_section::<FlamePanel>,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::platform::ui::effect::sections::{commit_effect_view, effect_head_rows};
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};
    use thyllore_effect_core::flame::FlameEmitterKind;

    const RING_PARAM_NAMES: [&str; 2] = ["emitter_ring_major_radius", "emitter_ring_angular_speed"];

    fn view_with_instances(count: usize) -> FlameSectionView {
        FlameSectionView {
            instances: (1..=count).map(|i| format!("Flame {i}")).collect(),
            instance_index: 0,
            preset: None,
            selected: Some(0),
            effect: FlameEffect::default(),
            render: FlameRenderSettings::default(),
            extra: FlameExtra::default(),
        }
    }

    fn view_with_emitter(kind: FlameEmitterKind) -> FlameSectionView {
        let mut view = view_with_instances(1);
        view.effect.emitter.kind = kind;
        view
    }

    fn committed_command_names(shown: &FlameSectionView, view: &FlameSectionView) -> Vec<String> {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        commit_effect_view::<FlamePanel>(&world, shown, view);

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    #[test]
    fn head_rows_with_two_instances_show_instance_choice() {
        let rows = effect_head_rows::<FlamePanel>(&view_with_instances(2));
        assert_eq!(visible_labels(&rows), ["Add Flame", "Instance", "Preset"]);
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = effect_head_rows::<FlamePanel>(&view_with_instances(1));
        assert_eq!(visible_labels(&rows), ["Add Flame", "Preset"]);
    }

    #[test]
    fn ring_params_follow_the_emitter_choice_only_for_a_ring_emitter() {
        let ring_rows = FlamePanel::param_rows(&view_with_emitter(FlameEmitterKind::Ring));
        assert_eq!(
            visible_labels(&ring_rows)[..3],
            ["Emitter", RING_PARAM_NAMES[0], RING_PARAM_NAMES[1]]
        );
        assert!(find_duplicate_labels(&ring_rows).is_empty());

        for kind in [FlameEmitterKind::Cylinder, FlameEmitterKind::MeshSdf] {
            let rows = FlamePanel::param_rows(&view_with_emitter(kind));
            let labels = visible_labels(&rows);
            assert!(
                RING_PARAM_NAMES.iter().all(|name| !labels.contains(name)),
                "{kind:?}: {labels:?}"
            );
        }
    }

    #[test]
    fn commit_without_change_sends_nothing() {
        let shown = view_with_instances(1);
        assert!(committed_command_names(&shown, &shown.clone()).is_empty());
    }

    #[test]
    fn commit_preset_change_clears_keys_and_applies_preset_without_update() {
        let shown = view_with_instances(1);
        let mut view = shown.clone();
        view.preset = Some(thyllore_effect_core::flame::FLAME_PRESET_NAMES[0].to_string());
        view.effect.wave_segments += 1;

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names[0].starts_with("ClearScalarKeys"), "{names:?}");
        assert!(names[1].starts_with("ApplyPreset"), "{names:?}");
    }

    #[test]
    fn commit_texture_fit_replaces_the_effect_update_of_the_same_frame() {
        let shown = view_with_instances(1);
        let mut view = shown.clone();
        view.effect.wave_segments += 1;
        view.extra.requests.apply_fit = true;

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("ApplyTextureFit"), "{names:?}");
    }

    #[test]
    fn commit_effect_change_sends_update() {
        let shown = view_with_instances(1);
        let mut view = shown.clone();
        view.effect.wave_segments += 1;

        let names = committed_command_names(&shown, &view);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("UpdateEffect"), "{names:?}");
    }
}
