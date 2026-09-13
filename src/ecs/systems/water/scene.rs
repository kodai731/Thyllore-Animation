use thyllore_scene_core::SceneComponent;

use crate::asset::AssetStorage;
use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect};
use crate::ecs::world::{Entity, World};
use crate::hooks::scene::{
    capture_component, decode_component, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};

pub const WATER_SCENE_COMPONENTS: &[SceneComponentHook] = &[
    SceneComponentHook {
        type_key: WaterTorusEffect::TYPE_KEY,
        role: SceneComponentRole::Owner,
        entities: entities_with::<WaterTorusEffect>,
        capture: capture_component::<WaterTorusEffect>,
        apply: apply_water,
    },
    SceneComponentHook::attachment::<AppliedWaterPreset>(),
];

fn apply_water(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    super::attach_water(world, entity, decode_component(value)?);
    Ok(())
}
