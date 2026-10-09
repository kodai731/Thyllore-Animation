use anyhow::Context;
use thyllore_scene_core::SceneComponent;

use super::scene::{decode_scene_value, encode_scene_value, SceneValue};
use crate::ecs::world::World;

pub type SceneResourceCaptureFn = fn(&World) -> Option<SceneValue>;
pub type SceneResourceApplyFn = fn(&mut World, &SceneValue) -> anyhow::Result<()>;

/// A world resource stored in the scene under `type_key`.
#[derive(Clone, Copy)]
pub struct SceneResourceHook {
    pub type_key: &'static str,
    pub capture: SceneResourceCaptureFn,
    pub apply: SceneResourceApplyFn,
}

inventory::collect!(SceneResourceHook);

/// Registers a resource stored as its serde form and restored onto the live resource.
#[macro_export]
macro_rules! scene_resource {
    ($resource:ty) => {
        inventory::submit! { $crate::hooks::scene_resource::SceneResourceHook::of::<$resource>() }
    };
}

impl SceneResourceHook {
    pub const fn of<R: SceneComponent>() -> Self {
        Self {
            type_key: R::TYPE_KEY,
            capture: capture_resource::<R>,
            apply: overwrite_resource::<R>,
        }
    }
}

fn capture_resource<R: SceneComponent>(world: &World) -> Option<SceneValue> {
    let resource = world.get_resource::<R>()?;
    match encode_scene_value(&*resource) {
        Ok(value) => Some(value),
        Err(error) => {
            log_warn!(
                "Failed to encode scene resource {}: {:#}",
                R::TYPE_KEY,
                error
            );
            None
        }
    }
}

fn overwrite_resource<R: SceneComponent>(
    world: &mut World,
    value: &SceneValue,
) -> anyhow::Result<()> {
    let loaded: R =
        decode_scene_value(value).with_context(|| format!("scene resource {}", R::TYPE_KEY))?;
    let overwritten = match world.get_resource_mut::<R>() {
        Some(mut resource) => {
            resource.overwrite_persisted_fields(&loaded);
            true
        }
        None => false,
    };
    if !overwritten {
        world.insert_resource(loaded);
    }
    Ok(())
}

/// Every resource hook submitted at link time, sorted by type key.
pub struct SceneResourceHooks {
    entries: Vec<SceneResourceHook>,
}

impl SceneResourceHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<SceneResourceHook> = inventory::iter::<SceneResourceHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| hook.type_key);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].type_key != pair[1].type_key,
                "scene resource type key {} registered twice",
                pair[0].type_key
            );
        }
        Ok(Self { entries })
    }

    pub fn iter(&self) -> impl Iterator<Item = &SceneResourceHook> {
        self.entries.iter()
    }

    pub fn type_keys(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.type_key).collect()
    }
}

impl std::fmt::Debug for SceneResourceHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneResourceHooks")
            .field("type_keys", &self.type_keys())
            .finish()
    }
}
