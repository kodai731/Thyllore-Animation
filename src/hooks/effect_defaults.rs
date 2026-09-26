use crate::ecs::world::World;

pub type EffectDefaultResourceFn = fn(&mut World);

#[derive(Clone, Copy)]
pub struct EffectDefaultResource {
    pub name: &'static str,
    pub insert: EffectDefaultResourceFn,
}

#[macro_export]
macro_rules! effect_default_resource {
    ($name:literal, $insert:path) => {
        inventory::submit! {
            $crate::hooks::effect_defaults::EffectDefaultResource {
                name: $name,
                insert: $insert,
            }
        }
    };
}

inventory::collect!(EffectDefaultResource);

pub fn apply_effect_default_resources(world: &mut World) {
    let mut entries: Vec<EffectDefaultResource> = inventory::iter::<EffectDefaultResource>
        .into_iter()
        .copied()
        .collect();
    entries.sort_by_key(|entry| entry.name);
    for entry in entries {
        (entry.insert)(world);
    }
}
