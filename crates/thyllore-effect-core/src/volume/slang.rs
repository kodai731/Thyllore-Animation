// C ABI generated from shaders/cpu/exports.slang (see build.rs); scalar arguments keep the
// boundary free of Slang vector types.
use cgmath::Vector3;

use crate::volume::VolumeShell;

extern "C" {
    fn thylloreShellOpticalDepth(
        height: f32,
        radius_base: f32,
        radius_slope: f32,
        radius_offset_q: f32,
        width_q: f32,
        strength: f32,
        h_top: f32,
        top_fade: f32,
        sigma_t: f32,
        ox: f32,
        oy: f32,
        oz: f32,
        dx: f32,
        dy: f32,
        dz: f32,
        t_near: f32,
        t_far: f32,
    ) -> f32;

    fn thylloreShellOpticalDepthToward(
        height: f32,
        radius_base: f32,
        radius_slope: f32,
        radius_offset_q: f32,
        width_q: f32,
        strength: f32,
        h_top: f32,
        top_fade: f32,
        sigma_t: f32,
        ox: f32,
        oy: f32,
        oz: f32,
        dx: f32,
        dy: f32,
        dz: f32,
        t_max: f32,
    ) -> f32;

    fn thylloreShellDensityAt(
        height: f32,
        radius_base: f32,
        radius_slope: f32,
        radius_offset_q: f32,
        width_q: f32,
        strength: f32,
        h_top: f32,
        top_fade: f32,
        sigma_t: f32,
        px: f32,
        py: f32,
        pz: f32,
    ) -> f32;
}

pub fn shell_optical_depth(
    shell: &VolumeShell,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    unsafe {
        thylloreShellOpticalDepth(
            shell.height,
            shell.radius_base,
            shell.radius_slope,
            shell.radius_offset_q,
            shell.width_q,
            shell.strength,
            shell.h_top,
            shell.top_fade,
            shell.sigma_t,
            origin.x,
            origin.y,
            origin.z,
            direction.x,
            direction.y,
            direction.z,
            t_near,
            t_far,
        )
    }
}

pub fn shell_optical_depth_toward(
    shell: &VolumeShell,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_max: f32,
) -> f32 {
    unsafe {
        thylloreShellOpticalDepthToward(
            shell.height,
            shell.radius_base,
            shell.radius_slope,
            shell.radius_offset_q,
            shell.width_q,
            shell.strength,
            shell.h_top,
            shell.top_fade,
            shell.sigma_t,
            origin.x,
            origin.y,
            origin.z,
            direction.x,
            direction.y,
            direction.z,
            t_max,
        )
    }
}

pub fn shell_density_at(shell: &VolumeShell, point: Vector3<f32>) -> f32 {
    unsafe {
        thylloreShellDensityAt(
            shell.height,
            shell.radius_base,
            shell.radius_slope,
            shell.radius_offset_q,
            shell.width_q,
            shell.strength,
            shell.h_top,
            shell.top_fade,
            shell.sigma_t,
            point.x,
            point.y,
            point.z,
        )
    }
}
