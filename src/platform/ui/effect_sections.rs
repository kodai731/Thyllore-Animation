use crate::ecs::events::UIEventQueue;
use crate::ecs::World;

use super::SceneOverlayState;

pub type EffectSectionDrawFn = fn(&imgui::Ui, &mut UIEventQueue, &mut SceneOverlayState, &World);

#[derive(Clone)]
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

pub fn collect_effect_sections() -> Vec<EffectSectionHook> {
    let mut sections: Vec<&EffectSectionHook> = inventory::iter::<EffectSectionHook>().collect();
    sections.sort_by_key(|h| (h.order, h.key));
    let mut last_key: Option<&str> = None;
    for hook in &sections {
        if let Some(prev) = last_key {
            assert!(
                prev != hook.key,
                "duplicate effect section key: {}",
                hook.key
            );
        }
        last_key = Some(hook.key);
    }
    sections.into_iter().map(|h| (*h).clone()).collect()
}
