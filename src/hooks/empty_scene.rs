use crate::asset::AssetStorage;
use crate::ecs::world::World;

pub type EmptySceneApplyFn = fn(&mut World, &mut AssetStorage);

#[derive(Clone, Copy)]
pub struct EmptySceneHook {
    pub name: &'static str,
    pub apply: EmptySceneApplyFn,
}

inventory::collect!(EmptySceneHook);

#[macro_export]
macro_rules! empty_scene_hook {
    ($name:literal, $apply:path) => {
        inventory::submit! {
            $crate::hooks::empty_scene::EmptySceneHook {
                name: $name,
                apply: $apply,
            }
        }
    };
}

pub struct EmptySceneHooks {
    entries: Vec<EmptySceneHook>,
}

impl EmptySceneHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<EmptySceneHook> = inventory::iter::<EmptySceneHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| hook.name);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].name != pair[1].name,
                "empty scene hook {} registered twice",
                pair[0].name
            );
        }
        Ok(Self { entries })
    }

    pub fn ordered(&self) -> impl Iterator<Item = &EmptySceneHook> {
        self.entries.iter()
    }
}
