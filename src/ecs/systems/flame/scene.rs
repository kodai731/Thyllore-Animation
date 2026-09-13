use thyllore_scene_core::SceneComponent;

use crate::asset::AssetStorage;
use crate::ecs::component::{AppliedFlameStyle, FlameEffect};
use crate::ecs::world::{Entity, World};
use crate::hooks::scene::{
    capture_component, decode_component, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};

pub const FLAME_SCENE_COMPONENTS: &[SceneComponentHook] = &[
    SceneComponentHook {
        type_key: FlameEffect::TYPE_KEY,
        role: SceneComponentRole::Owner,
        entities: entities_with::<FlameEffect>,
        capture: capture_component::<FlameEffect>,
        apply: apply_flame,
    },
    SceneComponentHook::attachment::<AppliedFlameStyle>(),
];

fn apply_flame(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    let mut effect: FlameEffect = decode_component(value)?;
    thyllore_effect_core::refresh_flame_coefficients(&mut effect, &Default::default());
    super::attach_flame(world, entity, effect);
    Ok(())
}
