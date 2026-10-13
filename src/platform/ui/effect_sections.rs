use std::sync::OnceLock;

use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::ScalarChannelDomain;
use crate::ecs::World;

use super::param_widgets::EditedScalars;
use super::scene_overlay::send_key_button;

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
