use cgmath::Vector3;

use crate::ecs::world::{Entity, World};

pub use thyllore_render_core::DistanceAttenuation;

pub const DEFAULT_LIGHT_POSITION: Vector3<f32> = Vector3 {
    x: 1.0,
    y: 1.0,
    z: 2.0,
};

#[derive(Clone, Debug)]
pub struct Light {
    pub shadow_strength: f32,
    pub shadow_normal_offset: f32,
    pub distance_attenuation: DistanceAttenuation,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            distance_attenuation: DistanceAttenuation::Disabled,
        }
    }
}

pub fn light_entity(world: &World) -> Option<Entity> {
    world.component_entities::<Light>().first().copied()
}

pub fn light_position(world: &World) -> Option<Vector3<f32>> {
    let entity = light_entity(world)?;
    let transform = world.get_component::<crate::ecs::world::Transform>(entity)?;
    Some(transform.translation)
}

pub fn set_light_position(world: &mut World, position: Vector3<f32>) {
    if let Some(entity) = light_entity(world) {
        if let Some(transform) = world.get_component_mut::<crate::ecs::world::Transform>(entity) {
            transform.translation = position;
        }
    }
}

pub fn light(world: &World) -> Option<Light> {
    let entity = light_entity(world)?;
    world.get_component::<Light>(entity).cloned()
}
