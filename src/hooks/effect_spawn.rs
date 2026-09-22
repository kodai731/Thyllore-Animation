use crate::asset::AssetStorage;
use crate::ecs::world::{Entity, World};

pub type EffectSpawnFn = fn(&mut World, &mut AssetStorage, usize) -> Entity;
pub type EffectEntitiesFn = fn(&World) -> Vec<Entity>;

/// How a feature's entities are created from generic code: the UI "add" button, the batch
/// `add_<key>` action and the empty scene at startup all go through this registry, so no
/// dispatcher lists the effects. `spawn` receives the ordinal of the new instance (the number
/// of instances that already exist) so the feature decides its own name and placement.
#[derive(Clone, Copy)]
pub struct EffectSpawnHook {
    pub key: &'static str,
    pub max_instances: usize,
    pub spawn: EffectSpawnFn,
    pub entities: EffectEntitiesFn,
    pub default_in_empty_scene: bool,
}

inventory::collect!(EffectSpawnHook);

#[macro_export]
macro_rules! effect_spawn_hook {
    ($hook:expr) => {
        inventory::submit! { $hook }
    };
}

pub struct EffectSpawnHooks {
    entries: Vec<EffectSpawnHook>,
}

impl EffectSpawnHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<EffectSpawnHook> = inventory::iter::<EffectSpawnHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| hook.key);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].key != pair[1].key,
                "effect spawn hook {} registered twice",
                pair[0].key
            );
        }
        Ok(Self { entries })
    }

    pub fn get(&self, key: &str) -> Option<&EffectSpawnHook> {
        self.entries.iter().find(|hook| hook.key == key)
    }

    pub fn ordered(&self) -> impl Iterator<Item = &EffectSpawnHook> {
        self.entries.iter()
    }

    pub fn keys(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.key).collect()
    }
}

impl std::fmt::Debug for EffectSpawnHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectSpawnHooks")
            .field("keys", &self.keys())
            .finish()
    }
}

/// Spawns one more instance of `key` unless the feature's instance limit is reached.
pub fn spawn_effect_instance(
    world: &mut World,
    assets: &mut AssetStorage,
    key: &str,
) -> Option<Entity> {
    let hook = *world.get_resource::<EffectSpawnHooks>()?.get(key)?;
    let ordinal = (hook.entities)(world).len();
    if ordinal >= hook.max_instances {
        return None;
    }
    Some((hook.spawn)(world, assets, ordinal))
}

/// Spawns every feature that wants to be present when the app starts without a scene.
pub fn spawn_empty_scene_defaults(world: &mut World, assets: &mut AssetStorage) {
    let defaults: Vec<EffectSpawnHook> = match world.get_resource::<EffectSpawnHooks>() {
        Some(hooks) => hooks
            .ordered()
            .filter(|hook| hook.default_in_empty_scene)
            .copied()
            .collect(),
        None => Vec::new(),
    };
    for hook in defaults {
        (hook.spawn)(world, assets, 0);
    }
}
