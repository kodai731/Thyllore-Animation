use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use crate::asset::AssetStorage;
use crate::ecs::component::DebugPrimitiveTag;
use crate::ecs::events::DebugPrimitiveKind;
use crate::ecs::world::{Entity, GlobalTransform, MeshRef, Transform, World};
use crate::hooks::scene::{
    decode_component, encode_scene_value, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};

/// Persisted form of a debug primitive: its kind and where it stands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebugPrimitiveRecord {
    pub kind: String,
    pub position: [f32; 3],
}

impl DebugPrimitiveRecord {
    pub fn kind(&self) -> anyhow::Result<DebugPrimitiveKind> {
        DebugPrimitiveKind::from_name(&self.kind)
            .ok_or_else(|| anyhow::anyhow!("unknown debug primitive kind {}", self.kind))
    }
}

impl SceneComponent for DebugPrimitiveRecord {
    const TYPE_KEY: &'static str = "debug_primitive";
    const PERSISTED_FIELDS: &'static [&'static str] = &["kind", "position"];
}

inventory::submit! { DEBUG_PRIMITIVE_SCENE_COMPONENT }

const DEBUG_PRIMITIVE_SCENE_COMPONENT: SceneComponentHook = SceneComponentHook {
    type_key: DebugPrimitiveRecord::TYPE_KEY,
    role: SceneComponentRole::Owner,
    entities: entities_with::<DebugPrimitiveTag>,
    capture: capture_debug_primitive,
    apply: apply_debug_primitive,
};

fn capture_debug_primitive(world: &World, entity: Entity) -> Option<SceneValue> {
    let tag = world.get_component::<DebugPrimitiveTag>(entity)?;
    let transform = world.get_component::<Transform>(entity)?;
    encode_scene_value(&DebugPrimitiveRecord {
        kind: tag.kind.name().to_string(),
        position: transform.translation.into(),
    })
    .ok()
}

/// Restores the tag and placement only; the mesh needs the GPU, so the app attaches it later
/// through `take_debug_primitives_awaiting_mesh`.
fn apply_debug_primitive(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    let record: DebugPrimitiveRecord = decode_component(value)?;
    world.insert_component(
        entity,
        DebugPrimitiveTag {
            kind: record.kind()?,
        },
    );
    world.insert_component(
        entity,
        Transform {
            translation: record.position.into(),
            ..Default::default()
        },
    );
    world.insert_component(entity, GlobalTransform::new());
    Ok(())
}

/// Removes every tagged entity that has no mesh yet and returns what to spawn in its place.
pub fn take_debug_primitives_awaiting_mesh(world: &mut World) -> Vec<DebugPrimitiveRecord> {
    let awaiting: Vec<(Entity, DebugPrimitiveRecord)> = entities_with::<DebugPrimitiveTag>(world)
        .into_iter()
        .filter(|entity| !world.has_component::<MeshRef>(*entity))
        .filter_map(|entity| {
            let record: DebugPrimitiveRecord =
                decode_component(&capture_debug_primitive(world, entity)?).ok()?;
            Some((entity, record))
        })
        .collect();

    awaiting
        .into_iter()
        .map(|(entity, record)| {
            world.despawn(entity);
            record
        })
        .collect()
}
