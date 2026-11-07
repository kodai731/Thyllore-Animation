use glam::Vec3;

use crate::volume::VolumeShell;

include!(concat!(env!("OUT_DIR"), "/volume_exports_bindings.rs"));

fn v3(v: Vec3) -> Float3 {
    Float3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

pub fn shell_optical_depth(
    shell: &VolumeShell,
    origin: Vec3,
    direction: Vec3,
    t_near: f32,
    t_far: f32,
) -> f32 {
    let mut shell = *shell;
    unsafe { thylloreShellOpticalDepth(&mut shell, v3(origin), v3(direction), t_near, t_far) }
}

pub fn shell_optical_depth_toward(
    shell: &VolumeShell,
    origin: Vec3,
    direction: Vec3,
    t_max: f32,
) -> f32 {
    let mut shell = *shell;
    unsafe { thylloreShellOpticalDepthToward(&mut shell, v3(origin), v3(direction), t_max) }
}

pub fn shell_density_at(shell: &VolumeShell, point: Vec3) -> f32 {
    let mut shell = *shell;
    unsafe { thylloreShellDensityAt(&mut shell, v3(point)) }
}
