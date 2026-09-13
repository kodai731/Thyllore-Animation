use thyllore_scene_core::SceneComponent;

use crate::asset::AssetStorage;
use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect};
use crate::ecs::world::{Entity, World};
use crate::hooks::scene::{
    capture_component, decode_component, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};

pub const WIND_SCENE_COMPONENTS: &[SceneComponentHook] = &[
    SceneComponentHook {
        type_key: WindTornadoEffect::TYPE_KEY,
        role: SceneComponentRole::Owner,
        entities: entities_with::<WindTornadoEffect>,
        capture: capture_component::<WindTornadoEffect>,
        apply: apply_wind,
    },
    SceneComponentHook::attachment::<AppliedWindPreset>(),
];

fn apply_wind(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    super::attach_wind(world, entity, decode_component(value)?);
    Ok(())
}
