use cgmath::{Quaternion, Vector3};

use crate::animation::editable::EaseType;
use crate::animation::{normalize_quat, slerp, BoneLocalPose, SkeletonPose};

pub fn blend_poses_override(
    base: &SkeletonPose,
    overlay: &SkeletonPose,
    weight: f32,
) -> SkeletonPose {
    let bone_count = base.bone_poses.len().min(overlay.bone_poses.len());
    let mut result = base.clone();

    for i in 0..bone_count {
        let b = &base.bone_poses[i];
        let o = &overlay.bone_poses[i];

        result.bone_poses[i] = BoneLocalPose {
            translation: lerp_vec3(b.translation, o.translation, weight),
            rotation: slerp(b.rotation, o.rotation, weight),
            scale: lerp_vec3(b.scale, o.scale, weight),
        };
    }

    result
}

pub fn blend_poses_additive(
    base: &SkeletonPose,
    additive: &SkeletonPose,
    rest: &SkeletonPose,
    weight: f32,
) -> SkeletonPose {
    let bone_count = base
        .bone_poses
        .len()
        .min(additive.bone_poses.len())
        .min(rest.bone_poses.len());
    let mut result = base.clone();
    let identity = Quaternion::new(1.0, 0.0, 0.0, 0.0);

    for i in 0..bone_count {
        let b = &base.bone_poses[i];
        let a = &additive.bone_poses[i];
        let r = &rest.bone_poses[i];

        let delta_t = (a.translation - r.translation) * weight;
        let inv_rest_r = conjugate_quat(r.rotation);
        let delta_r = slerp(identity, quat_mul(inv_rest_r, a.rotation), weight);
        let delta_s = (a.scale - r.scale) * weight;

        result.bone_poses[i] = BoneLocalPose {
            translation: b.translation + delta_t,
            rotation: normalize_quat(quat_mul(b.rotation, delta_r)),
            scale: b.scale + delta_s,
        };
    }

    result
}

pub fn apply_ease(t: f32, ease: EaseType) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        EaseType::Linear => t,
        EaseType::EaseIn => t * t,
        EaseType::EaseOut => t * (2.0 - t),
        EaseType::EaseInOut => 3.0 * t * t - 2.0 * t * t * t,
        EaseType::Stepped => {
            if t >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
    }
}

pub fn compute_crossfade_factor(
    current_time: f32,
    earlier_end: f32,
    later_start: f32,
    ease_out: EaseType,
    ease_in: EaseType,
) -> f32 {
    let overlap = earlier_end - later_start;
    if overlap <= 0.0 {
        return 1.0;
    }

    let t = ((current_time - later_start) / overlap).clamp(0.0, 1.0);
    let fade_out = apply_ease(t, ease_out);
    let fade_in = apply_ease(t, ease_in);

    fade_in / (fade_in + (1.0 - fade_out)).max(0.001)
}

pub fn compute_local_time(
    global_time: f32,
    start_time: f32,
    clip_in: f32,
    clip_out: f32,
    speed: f32,
    cycle_count: f32,
    looping: bool,
) -> f32 {
    let elapsed = (global_time - start_time) * speed;
    let clip_duration = clip_out - clip_in;

    if clip_duration <= 0.0 {
        return clip_in;
    }

    let total_duration = clip_duration * cycle_count;

    if looping {
        clip_in + (elapsed % clip_duration)
    } else if elapsed >= total_duration {
        clip_out
    } else {
        clip_in + (elapsed % clip_duration)
    }
}

fn lerp_vec3(a: Vector3<f32>, b: Vector3<f32>, t: f32) -> Vector3<f32> {
    Vector3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

fn conjugate_quat(q: Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(q.s, -q.v.x, -q.v.y, -q.v.z)
}

fn quat_mul(a: Quaternion<f32>, b: Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(
        a.s * b.s - a.v.x * b.v.x - a.v.y * b.v.y - a.v.z * b.v.z,
        a.s * b.v.x + a.v.x * b.s + a.v.y * b.v.z - a.v.z * b.v.y,
        a.s * b.v.y - a.v.x * b.v.z + a.v.y * b.s + a.v.z * b.v.x,
        a.s * b.v.z + a.v.x * b.v.y - a.v.y * b.v.x + a.v.z * b.s,
    )
}
