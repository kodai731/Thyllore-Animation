use cgmath::Vector3;
use thyllore_effect_core::LIGHTNING_MAX_WAYPOINTS;

use super::target::find_entity_named;
use crate::ecs::component::{EditorDisplay, LightningEffect, LightningPath, Locator};
use crate::ecs::world::{Entity, Name, Parent, Transform, World};
use crate::ecs::FrameContext;
use crate::hooks::scene::spawn_scene_owner;

/// Makes the locator a child of the bolt so moving the bolt moves it; its Transform is then in
/// bolt space. Already-adopted locators are left as they are.
pub fn adopt_lightning_child(world: &mut World, lightning: Entity, child: Entity) {
    if world.has_component::<Parent>(child) {
        return;
    }
    world.insert_component(child, Parent(lightning));
    world.add_child(lightning, child);
    if let Some(display) = world.get_component_mut::<EditorDisplay>(lightning) {
        display.expanded = true;
    }
}

/// Spawns a locator child halfway between the last waypoint (or the bolt origin) and the end
/// point, and appends it to the bolt's path.
pub fn spawn_lightning_waypoint(world: &mut World, lightning: Entity) -> Option<Entity> {
    let end_offset = world
        .get_component::<LightningEffect>(lightning)
        .map(|effect| Vector3::from(effect.end_offset))?;
    let lightning_name = world
        .get_component::<Name>(lightning)
        .map(|name| name.0.clone())
        .unwrap_or_else(|| "Lightning".to_string());
    let waypoint_names = world
        .get_component::<LightningPath>(lightning)
        .map(|path| path.waypoints.clone())
        .unwrap_or_default();
    if waypoint_names.len() >= LIGHTNING_MAX_WAYPOINTS {
        return None;
    }

    let previous_point = waypoint_names
        .iter()
        .rev()
        .find_map(|name| find_waypoint_translation(world, name))
        .unwrap_or(Vector3::new(0.0, 0.0, 0.0));
    let midpoint = (previous_point + end_offset) * 0.5;

    let waypoint_name = find_unused_waypoint_name(world, &lightning_name, waypoint_names.len() + 1);
    let waypoint = spawn_scene_owner(world, &waypoint_name, Locator::at(midpoint));
    adopt_lightning_child(world, lightning, waypoint);

    let mut waypoints = waypoint_names;
    waypoints.push(waypoint_name);
    world.insert_component(lightning, LightningPath { waypoints });
    Some(waypoint)
}

/// Drops the waypoint from the bolt's path; the locator entity stays in the scene.
pub fn remove_lightning_waypoint(world: &mut World, lightning: Entity, index: usize) {
    let Some(path) = world.get_component_mut::<LightningPath>(lightning) else {
        return;
    };
    if index < path.waypoints.len() {
        path.waypoints.remove(index);
    }
    if path.waypoints.is_empty() {
        world.remove_component::<LightningPath>(lightning);
    }
}

/// Copies every resolvable waypoint's local translation, in path order, into its bolt's
/// `waypoints`.
pub fn follow_lightning_path(world: &mut World) {
    for lightning in world.entities_with::<LightningEffect>() {
        let waypoint_names = world
            .get_component::<LightningPath>(lightning)
            .map(|path| path.waypoints.clone())
            .unwrap_or_default();

        let mut waypoints = Vec::with_capacity(LIGHTNING_MAX_WAYPOINTS);
        for name in &waypoint_names {
            let Some(waypoint) = find_entity_named(world, name) else {
                continue;
            };
            adopt_lightning_child(world, lightning, waypoint);
            if let Some(transform) = world.get_component::<Transform>(waypoint) {
                waypoints.push(transform.translation);
            }
        }
        waypoints.truncate(LIGHTNING_MAX_WAYPOINTS);

        if let Some(effect) = world.get_component_mut::<LightningEffect>(lightning) {
            for (slot, waypoint) in effect.waypoints.iter_mut().zip(&waypoints) {
                *slot = (*waypoint).into();
            }
            effect.waypoint_count = waypoints.len() as u32;
        }
    }
}

fn find_waypoint_translation(world: &World, name: &str) -> Option<Vector3<f32>> {
    let waypoint = find_entity_named(world, name)?;
    world
        .get_component::<Transform>(waypoint)
        .map(|transform| transform.translation)
}

fn find_unused_waypoint_name(world: &World, lightning_name: &str, first_number: usize) -> String {
    (first_number..)
        .map(|number| format!("{lightning_name} Waypoint {number}"))
        .find(|name| find_entity_named(world, name).is_none())
        .unwrap_or_else(|| format!("{lightning_name} Waypoint"))
}

fn lightning_path_advance(ctx: &mut FrameContext) {
    follow_lightning_path(ctx.world);
}

crate::frame_prep_hook!("lightning_path", Advance, lightning_path_advance);
