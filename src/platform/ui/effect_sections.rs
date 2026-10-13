use std::sync::OnceLock;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::ScalarChannelDomain;
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::{resolve_selected_effect, EffectEdit};
use crate::ecs::World;

use super::param_widgets::{
    draw_preset_combo, draw_tiered_params, EditedScalars, ScalarParam, UiParam,
};
use super::scene_overlay::send_key_button;
use super::theme::{section_header, SectionDefault};

pub type EffectSectionDrawFn = fn(&imgui::Ui, &World);

pub struct EffectSectionHook {
    pub key: &'static str,
    pub order: u32,
    pub draw: EffectSectionDrawFn,
}

inventory::collect!(EffectSectionHook);

#[macro_export]
macro_rules! effect_section_hook {
    ($hook:expr) => {
        inventory::submit! { $hook }
    };
}

pub fn collect_effect_sections() -> &'static [&'static EffectSectionHook] {
    static SECTIONS: OnceLock<Vec<&'static EffectSectionHook>> = OnceLock::new();
    SECTIONS.get_or_init(|| {
        let mut sections: Vec<&'static EffectSectionHook> =
            inventory::iter::<EffectSectionHook>().collect();
        sections.sort_by_key(|hook| (hook.order, hook.key));
        for pair in sections.windows(2) {
            assert!(
                pair[0].key != pair[1].key,
                "effect section hook {} registered twice",
                pair[0].key
            );
        }
        sections
    })
}

pub trait EffectPanel {
    type Component: crate::ecs::systems::EffectPreset + Clone + std::fmt::Debug;
    type RenderSettings: Clone + std::fmt::Debug + 'static;

    const NAME: &'static str;
    const SPAWN_HOOK_KEY: &'static str;
    const PRESET_NAMES: &'static [&'static str];

    fn ui_params() -> &'static [UiParam];
    fn scalar_params() -> &'static [ScalarParam<Self::Component>];
    fn domain() -> &'static ScalarChannelDomain;

    fn applied_preset_name(
        applied: &<Self::Component as crate::ecs::systems::EffectPreset>::Applied,
    ) -> String;

    fn draw_extra(
        ui: &imgui::Ui,
        world: &World,
        entity: crate::ecs::world::Entity,
        effect: &mut Self::Component,
    ) {
        let _ = (ui, world, entity, effect);
    }

    fn draw_debug(ui: &imgui::Ui, world: &World);
}

pub(super) fn build_effect_section<P: EffectPanel>(ui: &imgui::Ui, ecs_world: &World) {
    if !section_header(ui, ecs_world, P::NAME, SectionDefault::Closed) {
        return;
    }
    let _section_id = ui.push_id(P::NAME);

    if ui.button(format!("Add {}", P::NAME).as_str()) {
        ecs_world.send_command(ScalarCurveEvent::AddEffect(P::SPAWN_HOOK_KEY));
    }

    let instances: Vec<crate::ecs::world::Entity> = ecs_world.entities_with::<P::Component>();
    let selected_entity = resolve_selected_effect::<P::Component>(ecs_world);
    if instances.len() > 1 {
        let mut current = selected_entity
            .and_then(|entity| instances.iter().position(|&e| e == entity))
            .unwrap_or(0);
        let items: Vec<String> = instances
            .iter()
            .enumerate()
            .map(|(i, &entity)| {
                ecs_world
                    .get_component::<crate::ecs::world::Name>(entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("{} {}", P::NAME, i + 1))
            })
            .collect();
        if ui.combo_simple_string("Instance", &mut current, &items) {
            ecs_world.send_command(OverlayEvent::SelectEffectInstance {
                key: P::SPAWN_HOOK_KEY,
                index: current,
            });
        }
    }

    let applied_preset = selected_entity.and_then(|entity| {
        ecs_world
            .get_component::<<P::Component as crate::ecs::systems::EffectPreset>::Applied>(entity)
            .map(|applied| P::applied_preset_name(applied))
    });
    let mut effect_applied_this_frame = false;
    if let Some(chosen) = draw_preset_combo(
        ui,
        format!("{} Preset", P::NAME).as_str(),
        P::PRESET_NAMES,
        applied_preset.as_deref(),
    ) {
        if selected_entity.is_some() {
            ecs_world.send_command(ScalarCurveEvent::ClearScalarKeys);
            ecs_world.send_command(EffectEdit::<P::Component, P::RenderSettings>::ApplyPreset(
                chosen,
            ));
            effect_applied_this_frame = true;
        }
    }

    let Some(selected) = selected_entity else {
        return;
    };
    let Some(effect) = ecs_world.get_component::<P::Component>(selected) else {
        return;
    };
    let mut effect_copy = effect.clone();
    draw_tiered_params(
        ui,
        ecs_world,
        P::ui_params(),
        P::scalar_params(),
        &mut effect_copy,
        &[],
        |ui, edited| key_edited_scalars(ui, ecs_world, P::domain(), edited),
    );
    P::draw_extra(ui, ecs_world, selected, &mut effect_copy);
    if !effect_applied_this_frame {
        ecs_world.send_command(
            EffectEdit::<P::Component, P::RenderSettings>::UpdateEffect {
                entity: selected,
                effect: Box::new(effect_copy),
            },
        );
    }
    if ui.button("Curves") {
        ecs_world.send_command(ScalarCurveEvent::OpenScalarCurveEditor);
    }
    if section_header(
        ui,
        ecs_world,
        format!("{} Debug", P::NAME).as_str(),
        SectionDefault::Closed,
    ) {
        P::draw_debug(ui, ecs_world);
    }
}

pub(super) fn key_edited_scalars(
    ui: &imgui::Ui,
    ecs_world: &World,
    domain: &ScalarChannelDomain,
    edited: EditedScalars,
) {
    let keys: Vec<(PropertyType, f32)> = edited
        .iter()
        .filter_map(|(name, value)| {
            domain
                .property_type_for_cli_name(name)
                .map(|property_type| (property_type, *value))
        })
        .collect();
    send_key_button(ui, ecs_world, edited, keys);
}
