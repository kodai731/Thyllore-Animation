use std::sync::OnceLock;

use crate::ecs::events::UIEventQueue;
use crate::ecs::World;

use super::SceneOverlayState;

pub type EffectSectionDrawFn = fn(&imgui::Ui, &mut UIEventQueue, &mut SceneOverlayState, &World);

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
