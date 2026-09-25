use cgmath::Vector3;

use super::path::adopt_lightning_child;
use crate::ecs::component::{LightningEffect, LightningTarget, Locator};
use crate::ecs::world::{Entity, Name, Transform, World};
use crate::ecs::FrameContext;
use crate::hooks::scene::spawn_scene_owner;

pub fn find_entity_named(world: &World, name: &str) -> Option<Entity> {
    world
        .iter_components::<Name>()
        .find(|(_, entity_name)| entity_name.0 == name)
        .map(|(entity, _)| entity)
}

pub fn resolve_lightning_target(world: &World, lightning: Entity) -> Option<Entity> {
    let target = world.get_component::<LightningTarget>(lightning)?;
    find_entity_named(world, &target.entity_name)
}

/// Spawns a locator child at the bolt's end point and links the bolt to it.
pub fn spawn_lightning_target(world: &mut World, lightning: Entity) -> Option<Entity> {
    let end_offset = world
        .get_component::<LightningEffect>(lightning)
        .map(|effect| effect.end_offset)?;
    let lightning_name = world
        .get_component::<Name>(lightning)
        .map(|name| name.0.clone())
        .unwrap_or_else(|| "Lightning".to_string());

    let target_name = format!("{lightning_name} Target");
    let target = spawn_scene_owner(world, &target_name, Locator::at(Vector3::from(end_offset)));
    adopt_lightning_child(world, lightning, target);
    world.insert_component(
        lightning,
        LightningTarget {
            entity_name: target_name,
        },
    );
    Some(target)
}

/// Unlinks the bolt; the locator entity stays in the scene.
pub fn clear_lightning_target(world: &mut World, lightning: Entity) {
    world.remove_component::<LightningTarget>(lightning);
}

/// Copies every linked target's local translation into its bolt's `end_offset`.
pub fn follow_lightning_targets(world: &mut World) {
    for lightning in world.entities_with::<LightningTarget>() {
        let Some(target) = resolve_lightning_target(world, lightning) else {
            continue;
        };
        adopt_lightning_child(world, lightning, target);

        let Some(local_end) = world
            .get_component::<Transform>(target)
            .map(|transform| transform.translation)
        else {
            continue;
        };
        if let Some(effect) = world.get_component_mut::<LightningEffect>(lightning) {
            effect.end_offset = local_end.into();
        }
    }
}

fn lightning_target_advance(ctx: &mut FrameContext) {
    follow_lightning_targets(ctx.world);
}

crate::frame_prep_hook!("lightning_target", Advance, lightning_target_advance);
