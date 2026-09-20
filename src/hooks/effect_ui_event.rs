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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::{FLAME_SPAWN_HOOK, WATER_SPAWN_HOOK, WIND_SPAWN_HOOK};
    use crate::hooks::effect_spawn::EffectSpawnHooks;

    #[test]
    fn ui_event_hook_keys_are_subset_of_spawn_hook_keys() {
        let spawn_hooks = EffectSpawnHooks::collect().expect("spawn hooks collected");
        let ui_hooks = EffectUiEventHooks::collect();

        let spawn_keys: Vec<_> = spawn_hooks.keys();

        for hook in &ui_hooks {
            assert!(
                spawn_keys.contains(&hook.key),
                "EffectUiEventHook has key {:?} but no matching EffectSpawnHook",
                hook.key
            );
        }
    }

    #[test]
    fn ui_event_hooks_exist_for_flame_water_wind() {
        let ui_hooks = EffectUiEventHooks::collect();
        let ui_keys: Vec<_> = ui_hooks.iter().map(|h| h.key).collect();

        for expected in &[
            FLAME_SPAWN_HOOK.key,
            WATER_SPAWN_HOOK.key,
            WIND_SPAWN_HOOK.key,
        ] {
            assert!(
                ui_keys.contains(expected),
                "missing EffectUiEventHook for key {:?}",
                expected
            );
        }
    }
}
