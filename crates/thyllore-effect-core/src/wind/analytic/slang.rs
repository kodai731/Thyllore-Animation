use cgmath::Vector3;

use crate::wind::WindUBO;

include!(concat!(env!("OUT_DIR"), "/wind_exports_bindings.rs"));

fn with_context<R>(ubo: &crate::wind::WindUBO, body: impl FnOnce(*const KernelContext) -> R) -> R {
    let global_params = GlobalParams { wind: ubo };
    let ctx = KernelContext {
        global_params: &global_params,
    };
    body(&ctx)
}

fn v3(v: Vector3<f32>) -> Float3 {
    Float3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

pub fn wind_optical_depth_slang(
    ubo: &WindUBO,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    with_context(ubo, |ctx| unsafe {
        thylloreWindOpticalDepth(v3(origin), v3(direction), t_near, t_far, ctx)
    })
}

pub fn wind_density_at_slang(ubo: &WindUBO, point: Vector3<f32>) -> f32 {
    with_context(ubo, |ctx| unsafe { thylloreWindDensityAt(v3(point), ctx) })
}

pub fn wind_optical_depth_toward_slang(
    ubo: &WindUBO,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) -> f32 {
    with_context(ubo, |ctx| unsafe {
        thylloreWindOpticalDepthToward(v3(origin), v3(direction), ctx)
    })
}

pub fn wind_clamp_ray_to_cone_slang(
    ubo: &WindUBO,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> Option<(f32, f32)> {
    let mut out_near = 0.0f32;
    let mut out_far = 0.0f32;
    let hit = with_context(ubo, |ctx| unsafe {
        thylloreWindClampRayToCone(
            v3(origin),
            v3(direction),
            t_near,
            t_far,
            &mut out_near,
            &mut out_far,
            ctx,
        )
    });
    if hit {
        Some((out_near, out_far))
    } else {
        None
    }
}

pub fn wind_envelope_radius_slang(ubo: &WindUBO, h: f32) -> f32 {
    with_context(ubo, |ctx| unsafe { thylloreWindEnvelopeRadius(h, ctx) })
}
