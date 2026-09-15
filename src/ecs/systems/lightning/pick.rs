use crate::ecs::component::LightningEffect;
use crate::ecs::resource::PickRay;
use crate::ecs::world::{Entity, World};
use cgmath::{InnerSpace, Vector3};

/// Nearest lightning whose bounding sphere the ray enters, with the distance at which it enters.
pub fn find_lightning_by_pick_ray(world: &World, ray: &PickRay) -> Option<(Entity, f32)> {
    world
        .query_lightnings()
        .into_iter()
        .filter_map(|entity| {
            let effect = world.get_component::<LightningEffect>(entity)?;
            let (center, radius) = strike_bounding_sphere(&effect);
            let distance = ray_sphere_entry(ray.origin, ray.direction, center, radius)?;
            Some((entity, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
}

fn strike_bounding_sphere(effect: &LightningEffect) -> (Vector3<f32>, f32) {
    let start = effect.position;
    let end = start + Vector3::from(effect.end_offset);

    let tip_radius = effect.core_radius * effect.tip_radius_ratio;
    let sheath_radius = effect.core_radius.max(tip_radius) * effect.glow_ratio;

    let center = (start + end) * 0.5;
    let radius = (end - start).magnitude() * 0.5 + sheath_radius;
    (center, radius)
}

fn ray_sphere_entry(
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    center: Vector3<f32>,
    radius: f32,
) -> Option<f32> {
    let to_center = center - origin;
    let projection = to_center.dot(direction);
    let gap_squared = to_center.magnitude2() - projection * projection;
    let half_chord_squared = radius * radius - gap_squared;
    if half_chord_squared < 0.0 {
        return None;
    }

    let entry = projection - half_chord_squared.sqrt();
    if entry >= 0.0 {
        Some(entry)
    } else if projection + half_chord_squared.sqrt() >= 0.0 {
        Some(0.0)
    } else {
        None
    }
}
