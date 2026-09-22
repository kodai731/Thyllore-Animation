use cgmath::{InnerSpace, Matrix4, Vector3, Vector4};

use super::slang::{water_height_and_gradient_slang, water_lb_height_and_gradient_slang};
use crate::water::effect::WaterTorusEffect;
use crate::water::gpu::systems::build_water_ubo;
use thyllore_math_core::intersect_torus;

pub fn pick_torus(
    ray_origin: Vector3<f32>,
    ray_dir: Vector3<f32>,
    model: Matrix4<f32>,
    inverse_model: Matrix4<f32>,
    major_radius: f32,
    r: f32,
) -> Option<f32> {
    let local_origin = inverse_model * Vector4::new(ray_origin.x, ray_origin.y, ray_origin.z, 1.0);
    let local_dir = inverse_model * Vector4::new(ray_dir.x, ray_dir.y, ray_dir.z, 0.0);

    let hits = intersect_torus(
        local_origin.truncate(),
        local_dir.truncate(),
        major_radius,
        r,
    );
    if hits.count == 0 {
        return None;
    }

    let t_local = hits.roots[0];
    let world_hit = model
        * Vector4::new(
            local_origin.x + local_dir.x * t_local,
            local_origin.y + local_dir.y * t_local,
            local_origin.z + local_dir.z * t_local,
            1.0,
        );

    let hit_world = world_hit.truncate();
    let dist = (hit_world - ray_origin).magnitude();
    Some(dist)
}

/// Sum of the flat wave modes and the LB modes, matching what the shader evaluates.
/// Returns (h, h_u, h_v).
pub fn water_total_height_and_gradient(
    effect: &WaterTorusEffect,
    u: f32,
    v: f32,
    frame_index: u32,
) -> (f32, f32, f32) {
    let flow = (effect.flow_longitudinal, effect.flow_meridional);

    let ubo = build_water_ubo(effect, frame_index);

    let (flat_h, flat_h_u, flat_h_v, _) =
        water_height_and_gradient_slang(&ubo, u, v, effect.time, flow, 8, (0.0, 0.0));

    let (laplace_beltrami_h, laplace_beltrami_h_u, laplace_beltrami_h_v) =
        water_lb_height_and_gradient_slang(&ubo, (u, v), effect.time, flow);

    (
        flat_h + laplace_beltrami_h,
        flat_h_u + laplace_beltrami_h_u,
        flat_h_v + laplace_beltrami_h_v,
    )
}
