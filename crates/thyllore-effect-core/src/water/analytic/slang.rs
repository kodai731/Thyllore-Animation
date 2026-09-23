use crate::water::WaterUBO;

include!(concat!(env!("OUT_DIR"), "/water_exports_bindings.rs"));

fn with_context<R>(
    ubo: &crate::water::WaterUBO,
    body: impl FnOnce(*const KernelContext) -> R,
) -> R {
    let ctx = KernelContext { water: *ubo };
    body(&ctx)
}

fn f2(x: f32, y: f32) -> Float2 {
    Float2 { x, y }
}

pub fn water_height_and_gradient_slang(
    ubo: &WaterUBO,
    u: f32,
    v: f32,
    time: f32,
    flow: (f32, f32),
    mode_count: i32,
    footprint: (f32, f32),
) -> (f32, f32, f32, f32) {
    let mut h = 0.0f32;
    let mut hu = 0.0f32;
    let mut hv = 0.0f32;
    let mut slope_variance = 0.0f32;
    with_context(ubo, |ctx| unsafe {
        thylloreWaterHeightAndGradient(
            f2(u, v),
            time,
            f2(flow.0, flow.1),
            mode_count,
            f2(footprint.0, footprint.1),
            &mut h,
            &mut hu,
            &mut hv,
            &mut slope_variance,
            ctx,
        )
    });
    (h, hu, hv, slope_variance)
}

pub fn water_lb_height_and_gradient_slang(
    ubo: &WaterUBO,
    uv: (f32, f32),
    time: f32,
    flow: (f32, f32),
) -> (f32, f32, f32) {
    let mut h = 0.0f32;
    let mut hu = 0.0f32;
    let mut hv = 0.0f32;
    with_context(ubo, |ctx| unsafe {
        thylloreWaterLbHeightAndGradient(
            f2(uv.0, uv.1),
            time,
            f2(flow.0, flow.1),
            &mut h,
            &mut hu,
            &mut hv,
            ctx,
        )
    });
    (h, hu, hv)
}
