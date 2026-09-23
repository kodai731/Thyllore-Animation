use crate::flame::FlameUBO;

include!(concat!(env!("OUT_DIR"), "/flame_exports_bindings.rs"));

fn with_context<R>(
    ubo: &crate::flame::FlameUBO,
    body: impl FnOnce(*const KernelContext) -> R,
) -> R {
    let global_params = GlobalParams::new(ubo);
    let ctx = KernelContext {
        global_params: &global_params,
    };
    body(&ctx)
}

fn v3(p: [f32; 3]) -> Float3 {
    Float3 {
        x: p[0],
        y: p[1],
        z: p[2],
    }
}

fn f3(v: Float3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

pub fn flame_branch_pull_back_slang(ubo: &FlameUBO, p: [f32; 3]) -> [f32; 3] {
    with_context(ubo, |ctx| unsafe {
        f3(thylloreFlameBranchPullBack(v3(p), ctx))
    })
}

pub fn flame_branch_pull_back_jvp_slang(
    ubo: &FlameUBO,
    p: [f32; 3],
    dir: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let mut pulled = Float3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut dir_out = Float3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    with_context(ubo, |ctx| unsafe {
        thylloreFlameBranchPullBackJvp(v3(p), v3(dir), &mut pulled, &mut dir_out, ctx)
    });
    (f3(pulled), f3(dir_out))
}

pub fn flame_branch_burnout_mask_slang(ubo: &FlameUBO, p: [f32; 3]) -> f32 {
    with_context(ubo, |ctx| unsafe {
        thylloreFlameBranchBurnoutMask(v3(p), ctx)
    })
}

pub fn flame_self_shadow_tau_slang(ubo: &FlameUBO, p: [f32; 3], light_dir: [f32; 3]) -> f32 {
    with_context(ubo, |ctx| unsafe {
        thylloreFlameSelfShadowTau(v3(p), v3(light_dir), ctx)
    })
}
