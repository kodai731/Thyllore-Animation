use crate::asset::AssetStorage;
use crate::ecs::world::World;

pub type EffectUiEventApplyFn = fn(&mut World, &mut AssetStorage, &dyn std::any::Any);

#[derive(Clone, Copy)]
pub struct EffectUiEventHook {
    pub key: &'static str,
    pub apply: EffectUiEventApplyFn,
}

inventory::collect!(EffectUiEventHook);

#[macro_export]
macro_rules! effect_ui_event_hook {
    ($key:expr, $apply_fn:expr) => {
        inventory::submit! {
            $crate::hooks::effect_ui_event::EffectUiEventHook {
                key: $key,
                apply: $apply_fn,
            }
        }
    };
}

pub struct EffectUiEventHooks;

impl EffectUiEventHooks {
    pub fn collect() -> Vec<EffectUiEventHook> {
        let mut hooks: Vec<EffectUiEventHook> = inventory::iter::<EffectUiEventHook>
            .into_iter()
            .copied()
            .collect();
        hooks.sort_by_key(|h| h.key);
        for i in 0..hooks.len().saturating_sub(1) {
            if hooks[i].key == hooks[i + 1].key {
                panic!("Duplicate EffectUiEventHook key found: {}", hooks[i].key);
            }
        }
        hooks
    }
}
