use crate::ecs::resource::PickRay;
use crate::ecs::world::{Entity, World};

pub type ObjectPickFn = fn(&World, &PickRay) -> Option<(Entity, f32)>;

#[derive(Clone, Copy)]
pub struct ObjectPickHook {
    pub key: &'static str,
    pub find: ObjectPickFn,
}

inventory::collect!(ObjectPickHook);

#[macro_export]
macro_rules! object_pick_hook {
    ($key:literal, $find:path) => {
        inventory::submit! {
            $crate::hooks::object_pick::ObjectPickHook {
                key: $key,
                find: $find,
            }
        }
    };
}

pub struct ObjectPickHooks {
    entries: Vec<ObjectPickHook>,
}

impl ObjectPickHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<ObjectPickHook> = inventory::iter::<ObjectPickHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| hook.key);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].key != pair[1].key,
                "object pick hook {} registered twice",
                pair[0].key
            );
        }
        Ok(Self { entries })
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectPickHook> {
        self.entries.iter()
    }
}

impl std::fmt::Debug for ObjectPickHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectPickHooks")
            .field(
                "keys",
                &self.entries.iter().map(|h| h.key).collect::<Vec<_>>(),
            )
            .finish()
    }
}
