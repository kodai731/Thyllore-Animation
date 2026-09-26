use crate::asset::AssetStorage;
use crate::ecs::world::World;

pub struct EffectUiQueue<C> {
    commands: Vec<C>,
}

impl<C> Default for EffectUiQueue<C> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<C> EffectUiQueue<C> {
    pub fn push(&mut self, command: C) {
        self.commands.push(command);
    }

    pub fn drain(&mut self) -> Vec<C> {
        std::mem::take(&mut self.commands)
    }
}

pub fn send_effect_ui_command<C: 'static>(world: &World, command: C) {
    let Some(mut queue) = world.get_resource_mut::<EffectUiQueue<C>>() else {
        log_warn!(
            "EffectUiQueue<{}> is not inserted; command dropped",
            std::any::type_name::<C>()
        );
        return;
    };
    queue.push(command);
}

pub type EffectUiDispatchFn = fn(&mut World, &mut AssetStorage);

#[derive(Clone, Copy)]
pub struct EffectUiEventDispatchHook {
    pub key: &'static str,
    pub dispatch: EffectUiDispatchFn,
}

inventory::collect!(EffectUiEventDispatchHook);

#[macro_export]
macro_rules! ui_event_hook {
    ($key:expr, $dispatch_fn:path) => {
        inventory::submit! {
            $crate::hooks::effect_ui_event::EffectUiEventDispatchHook {
                key: $key,
                dispatch: $dispatch_fn,
            }
        }
    };
}

pub struct EffectUiEventDispatchHooks {
    entries: Vec<EffectUiEventDispatchHook>,
}

impl EffectUiEventDispatchHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<EffectUiEventDispatchHook> =
            inventory::iter::<EffectUiEventDispatchHook>
                .into_iter()
                .copied()
                .collect();
        entries.sort_by_key(|hook| hook.key);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].key != pair[1].key,
                "effect ui dispatch hook {} registered twice",
                pair[0].key
            );
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> Vec<EffectUiEventDispatchHook> {
        self.entries.clone()
    }

    pub fn keys(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.key).collect()
    }
}

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
    use crate::hooks::effect_spawn::EffectSpawnHooks;

    #[test]
    fn ui_event_hook_keys_are_subset_of_spawn_hook_keys() {
        let spawn_hooks = EffectSpawnHooks::collect().expect("spawn hooks collected");
        let ui_hooks = EffectUiEventHooks::collect();
        let dispatch_hooks =
            EffectUiEventDispatchHooks::collect().expect("dispatch hooks collected");

        let spawn_keys: Vec<_> = spawn_hooks.keys();

        for hook in &ui_hooks {
            assert!(
                spawn_keys.contains(&hook.key),
                "EffectUiEventHook has key {:?} but no matching EffectSpawnHook",
                hook.key
            );
        }
        for key in dispatch_hooks.keys() {
            assert!(
                spawn_keys.contains(&key),
                "EffectUiEventDispatchHook has key {:?} but no matching EffectSpawnHook",
                key
            );
        }
    }

    #[test]
    fn ui_event_hooks_exist_for_all_spawn_hook_keys() {
        let spawn_hooks = EffectSpawnHooks::collect().expect("spawn hooks collected");
        let ui_hooks = EffectUiEventHooks::collect();
        let dispatch_hooks =
            EffectUiEventDispatchHooks::collect().expect("dispatch hooks collected");
        let all_keys: Vec<_> = ui_hooks
            .iter()
            .map(|h| h.key)
            .chain(dispatch_hooks.keys())
            .collect();

        for key in spawn_hooks.keys() {
            assert!(
                all_keys.contains(&key),
                "missing EffectUiEventHook or EffectUiEventDispatchHook for key {:?}",
                key
            );
        }
    }
}
