use cgmath::Vector3;
use thyllore_anim_core::editable::PropertyType;

use super::{spawn_flame, spawn_flame_with_clip, DEFAULT_FLAME_NAME};
use crate::asset::AssetStorage;
use crate::ecs::component::{FlameEffect, FLAME_DOMAIN};
use crate::ecs::resource::{ClipLibrary, PickRay};
use crate::ecs::systems::{resolve_engine_cli_overrides, EngineCliOverrides};
use crate::ecs::world::{Entity, World};
use crate::hooks::object_pick::ObjectPickHooks;

const RAY_START_Z: f32 = -10.0;

pub(super) fn spawn_default_flame(world: &mut World, name: &str) -> Entity {
    spawn_flame(world, name, FlameEffect::default())
}

pub(super) fn flame_height_property() -> PropertyType {
    FLAME_DOMAIN
        .property_type_for_cli_name("height")
        .expect("flame declares a height channel")
}

pub(super) fn flame_with_keyed_clip(world: &mut World, assets: &mut AssetStorage) -> Entity {
    use thyllore_anim_core::editable::{curve_add_keyframe, InterpolationType};

    let entity = spawn_flame_with_clip(world, assets, DEFAULT_FLAME_NAME, FlameEffect::default());
    let clip_id = crate::ecs::systems::find_entity_clip_id(world, entity).expect("flame clip");
    let mut library = world.resource_mut::<ClipLibrary>();
    let clip = library.get_mut(clip_id).expect("clip registered");
    let curve = clip.get_or_add_scalar_curve(flame_height_property());
    let key = curve_add_keyframe(curve, 1.0, 2.0);
    curve
        .get_keyframe_mut(key)
        .expect("key inserted")
        .interpolation = InterpolationType::Bezier;
    entity
}

pub(super) fn wall_probe_overrides(extra: &[&str]) -> EngineCliOverrides {
    let mut args: Vec<String> = vec!["bin".to_string()];
    args.extend(extra.iter().map(|s| s.to_string()));
    args.push("--batch-debug-action".to_string());
    args.push("dump_wall_probe".to_string());
    resolve_engine_cli_overrides(&args).expect("engine overrides parse")
}

pub(super) fn world_with_flame_at(x: f32) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(ObjectPickHooks::collect().expect("object pick hooks"));
    let effect = FlameEffect {
        position: Vector3::new(x, 0.0, 0.0),
        ..FlameEffect::default()
    };
    let entity = spawn_flame(&mut world, "Flame", effect);
    (world, entity)
}

pub(super) fn ray_towards_origin() -> PickRay {
    PickRay {
        origin: Vector3::new(0.0, 0.5, RAY_START_Z),
        direction: Vector3::new(0.0, 0.0, 1.0),
    }
}
