use crate::ecs::resource::PickRay;
use crate::ecs::world::{Entity, World};

pub type PickFn = fn(&World, &PickRay) -> Option<(Entity, f32)>;

#[derive(Clone, Copy)]
pub struct PickHook {
    pub key: &'static str,
    pub find: PickFn,
}

inventory::collect!(PickHook);

#[macro_export]
macro_rules! pick_hook {
    ($key:literal, $find:path) => {
        inventory::submit! {
            $crate::hooks::pick::PickHook {
                key: $key,
                find: $find,
            }
        }
    };
}

pub struct PickHooks {
    entries: Vec<PickHook>,
}

impl PickHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<PickHook> = inventory::iter::<PickHook>.into_iter().copied().collect();
        entries.sort_by_key(|hook| hook.key);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].key != pair[1].key,
                "pick hook {} registered twice",
                pair[0].key
            );
        }
        Ok(Self { entries })
    }

    pub fn iter(&self) -> impl Iterator<Item = &PickHook> {
        self.entries.iter()
    }
}

impl std::fmt::Debug for PickHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PickHooks")
            .field(
                "keys",
                &self.entries.iter().map(|h| h.key).collect::<Vec<_>>(),
            )
            .finish()
    }
}
