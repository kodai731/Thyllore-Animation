use std::collections::HashMap;

use cgmath::{Quaternion, Vector3};

use crate::animation::editable::components::clip::EditableAnimationClip;
use crate::animation::editable::components::curve::PropertyCurve;
use crate::animation::editable::components::keyframe::{
    EditableKeyframe, InterpolationType, SourceClipId,
};
use crate::animation::editable::components::track::BoneTrack;
use crate::animation::editable::systems::curve_ops::{curve_add_keyframe, curve_sample};
use crate::animation::{AnimationClip, BoneId, Keyframe, TransformChannel};

pub fn clip_from_animation(
    id: SourceClipId,
    clip: &AnimationClip,
    bone_names: &HashMap<BoneId, String>,
) -> EditableAnimationClip {
    use crate::animation::Interpolation;

    let mut editable = EditableAnimationClip::new(id, clip.name.clone());
    editable.duration = clip.duration;

    for (&bone_id, channel) in &clip.channels {
        let bone_name = bone_names
            .get(&bone_id)
            .cloned()
            .unwrap_or_else(|| format!("Bone_{}", bone_id));

        let mut track = editable.add_track(bone_id, bone_name).clone();

        import_vec3_keyframes(
            &channel.translation,
            &mut [
                &mut track.translation_x,
                &mut track.translation_y,
                &mut track.translation_z,
            ],
        );

        for (idx, kf) in channel.rotation.iter().enumerate() {
            let euler = quaternion_to_euler_degrees(&kf.value);
            let kf_id_x = curve_add_keyframe(&mut track.rotation_x, kf.time, euler.x);
            let kf_id_y = curve_add_keyframe(&mut track.rotation_y, kf.time, euler.y);
            let kf_id_z = curve_add_keyframe(&mut track.rotation_z, kf.time, euler.z);

            if kf.interpolation == Interpolation::CubicSpline {
                let next_kf = channel.rotation.get(idx + 1);
                let dt = next_kf.map(|n| n.time - kf.time).unwrap_or(0.1);

                if let Some(out_t) = &kf.out_tangent {
                    let out_euler = quaternion_to_euler_degrees(out_t);
                    set_cubic_bezier_handles(&mut track.rotation_x, kf_id_x, dt, out_euler.x);
                    set_cubic_bezier_handles(&mut track.rotation_y, kf_id_y, dt, out_euler.y);
                    set_cubic_bezier_handles(&mut track.rotation_z, kf_id_z, dt, out_euler.z);
                }

                if let Some(in_t) = &kf.in_tangent {
                    let in_euler = quaternion_to_euler_degrees(in_t);
                    set_cubic_bezier_in_handles(&mut track.rotation_x, kf_id_x, dt, in_euler.x);
                    set_cubic_bezier_in_handles(&mut track.rotation_y, kf_id_y, dt, in_euler.y);
                    set_cubic_bezier_in_handles(&mut track.rotation_z, kf_id_z, dt, in_euler.z);
                }
            }
        }

        import_vec3_keyframes(
            &channel.scale,
            &mut [&mut track.scale_x, &mut track.scale_y, &mut track.scale_z],
        );

        editable.tracks.insert(bone_id, track);
    }

    editable
}

pub fn clip_to_animation(clip: &EditableAnimationClip) -> AnimationClip {
    let mut anim = AnimationClip::new(&clip.name);
    anim.duration = clip.duration;

    for (&bone_id, track) in &clip.tracks {
        let mut channel = TransformChannel::default();

        let translation_curves = [
            &track.translation_x,
            &track.translation_y,
            &track.translation_z,
        ];
        for time in collect_bake_times(&translation_curves) {
            let x = curve_sample(&track.translation_x, time).unwrap_or(0.0);
            let y = curve_sample(&track.translation_y, time).unwrap_or(0.0);
            let z = curve_sample(&track.translation_z, time).unwrap_or(0.0);
            channel
                .translation
                .push(Keyframe::new(time, Vector3::new(x, y, z)));
        }

        let rotation_curves = [&track.rotation_x, &track.rotation_y, &track.rotation_z];
        for time in collect_bake_times(&rotation_curves) {
            let ex = curve_sample(&track.rotation_x, time).unwrap_or(0.0);
            let ey = curve_sample(&track.rotation_y, time).unwrap_or(0.0);
            let ez = curve_sample(&track.rotation_z, time).unwrap_or(0.0);
            let q = euler_degrees_to_quaternion(ex, ey, ez);
            channel.rotation.push(Keyframe::new(time, q));
        }

        let scale_curves = [&track.scale_x, &track.scale_y, &track.scale_z];
        for time in collect_bake_times(&scale_curves) {
            let x = curve_sample(&track.scale_x, time).unwrap_or(1.0);
            let y = curve_sample(&track.scale_y, time).unwrap_or(1.0);
            let z = curve_sample(&track.scale_z, time).unwrap_or(1.0);
            channel
                .scale
                .push(Keyframe::new(time, Vector3::new(x, y, z)));
        }

        if !channel.translation.is_empty()
            || !channel.rotation.is_empty()
            || !channel.scale.is_empty()
        {
            anim.add_channel(bone_id, channel);
        }
    }

    anim
}

pub fn clip_recalculate_duration(clip: &mut EditableAnimationClip) {
    let mut max_time: f32 = 0.0;

    for track in clip.tracks.values() {
        for curve in track.all_curves() {
            if let Some(last_kf) = curve.keyframes.last() {
                max_time = max_time.max(last_kf.time);
            }
        }
    }

    clip.duration = max_time;
}

pub fn clip_remap_bone_ids(
    clip: &mut EditableAnimationClip,
    name_to_new_id: &HashMap<String, BoneId>,
) {
    let old_tracks: Vec<(BoneId, BoneTrack)> = clip.tracks.drain().collect();
    for (_, mut track) in old_tracks {
        let new_id = match name_to_new_id.get(&track.bone_name) {
            Some(&id) => id,
            None => continue,
        };
        track.bone_id = new_id;
        clip.tracks.insert(new_id, track);
    }
}

pub fn quaternion_to_euler_degrees(q: &Quaternion<f32>) -> Vector3<f32> {
    let w = q.s;
    let x = q.v.x;
    let y = q.v.y;
    let z = q.v.z;

    let sinp = 2.0 * (w * x + y * z);
    let cosp = 1.0 - 2.0 * (x * x + y * y);
    let pitch = sinp.atan2(cosp);

    let siny = 2.0 * (w * y - z * x);
    let yaw = if siny.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(siny)
    } else {
        siny.asin()
    };

    let sinr = 2.0 * (w * z + x * y);
    let cosr = 1.0 - 2.0 * (y * y + z * z);
    let roll = sinr.atan2(cosr);

    Vector3::new(pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees())
}

fn euler_degrees_to_quaternion(x_deg: f32, y_deg: f32, z_deg: f32) -> Quaternion<f32> {
    let x_rad = x_deg.to_radians();
    let y_rad = y_deg.to_radians();
    let z_rad = z_deg.to_radians();

    let cx = (x_rad * 0.5).cos();
    let sx = (x_rad * 0.5).sin();
    let cy = (y_rad * 0.5).cos();
    let sy = (y_rad * 0.5).sin();
    let cz = (z_rad * 0.5).cos();
    let sz = (z_rad * 0.5).sin();

    Quaternion::new(
        cx * cy * cz + sx * sy * sz,
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
    )
}

fn import_vec3_keyframes(
    keyframes: &[Keyframe<Vector3<f32>>],
    curves: &mut [&mut PropertyCurve; 3],
) {
    use crate::animation::Interpolation;

    for (idx, kf) in keyframes.iter().enumerate() {
        let values = [kf.value.x, kf.value.y, kf.value.z];
        let is_cubic = kf.interpolation == Interpolation::CubicSpline;
        let next_kf = keyframes.get(idx + 1);
        let dt = next_kf.map(|n| n.time - kf.time).unwrap_or(0.1);

        let out_tangent = kf.out_tangent.map(|t| [t.x, t.y, t.z]);
        let in_tangent = kf.in_tangent.map(|t| [t.x, t.y, t.z]);

        for (c_idx, curve) in curves.iter_mut().enumerate() {
            let kf_id = curve_add_keyframe(curve, kf.time, values[c_idx]);

            if is_cubic {
                if let Some(out_t) = &out_tangent {
                    set_cubic_bezier_handles(curve, kf_id, dt, out_t[c_idx]);
                }
                if let Some(in_t) = &in_tangent {
                    set_cubic_bezier_in_handles(curve, kf_id, dt, in_t[c_idx]);
                }
            }
        }
    }
}

fn set_cubic_bezier_handles(curve: &mut PropertyCurve, kf_id: u64, dt: f32, tangent_value: f32) {
    use crate::animation::editable::components::keyframe::BezierHandle;

    if let Some(kf) = curve.get_keyframe_mut(kf_id) {
        kf.interpolation = InterpolationType::Bezier;
        let handle_time = dt / 3.0;
        let handle_value = tangent_value * dt / 3.0;
        kf.out_tangent = BezierHandle::new(handle_time, handle_value);
    }
}

fn set_cubic_bezier_in_handles(curve: &mut PropertyCurve, kf_id: u64, dt: f32, tangent_value: f32) {
    use crate::animation::editable::components::keyframe::BezierHandle;

    if let Some(kf) = curve.get_keyframe_mut(kf_id) {
        let handle_time = dt / 3.0;
        let handle_value = tangent_value * dt / 3.0;
        kf.in_tangent = BezierHandle::new(-handle_time, -handle_value);
    }
}

fn collect_unique_times(curves: &[&PropertyCurve]) -> Vec<f32> {
    let mut times: Vec<f32> = curves
        .iter()
        .flat_map(|c| c.keyframes.iter().map(|k| k.time))
        .collect();

    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    times
}

fn estimate_bezier_curvature(k0: &EditableKeyframe, k1: &EditableKeyframe) -> f32 {
    let dt = k1.time - k0.time;
    if dt.abs() < 1e-8 {
        return 0.0;
    }

    let linear_slope = (k1.value - k0.value) / dt;
    let out_deviation =
        (k0.out_tangent.value_offset - linear_slope * k0.out_tangent.time_offset).abs();
    let in_deviation =
        (k1.in_tangent.value_offset - linear_slope * k1.in_tangent.time_offset).abs();

    let value_range = (k1.value - k0.value).abs().max(1.0);
    (out_deviation + in_deviation) / value_range
}

fn compute_bezier_subdivisions(k0: &EditableKeyframe, k1: &EditableKeyframe) -> usize {
    let duration = k1.time - k0.time;
    let fps_based = (duration * 30.0).ceil() as usize;

    let curvature = estimate_bezier_curvature(k0, k1);
    let curvature_multiplier = 1.0 + curvature * 2.0;

    let subdivisions = (fps_based as f32 * curvature_multiplier).ceil() as usize;
    subdivisions.clamp(2, 64)
}

fn collect_bake_times(curves: &[&PropertyCurve]) -> Vec<f32> {
    let has_bezier = curves.iter().any(|c| c.has_bezier_keyframes());

    if !has_bezier {
        return collect_unique_times(curves);
    }

    let mut times: Vec<f32> = Vec::new();

    for curve in curves {
        for kf in &curve.keyframes {
            times.push(kf.time);
        }

        for i in 0..curve.keyframes.len().saturating_sub(1) {
            let k0 = &curve.keyframes[i];
            let k1 = &curve.keyframes[i + 1];

            if k0.interpolation == InterpolationType::Bezier {
                let subdivisions = compute_bezier_subdivisions(k0, k1);
                for s in 1..subdivisions {
                    let frac = s as f32 / subdivisions as f32;
                    let mid_time = k0.time + (k1.time - k0.time) * frac;
                    times.push(mid_time);
                }
            }
        }
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    times
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{BezierHandle, EditableKeyframe, InterpolationType};

    fn make_bezier_kf(
        time: f32,
        value: f32,
        out_t: f32,
        out_v: f32,
        in_t: f32,
        in_v: f32,
    ) -> EditableKeyframe {
        let mut kf = EditableKeyframe::new(0, time, value);
        kf.interpolation = InterpolationType::Bezier;
        kf.out_tangent = BezierHandle::new(out_t, out_v);
        kf.in_tangent = BezierHandle::new(in_t, in_v);
        kf
    }

    #[test]
    fn test_short_segment_few_subdivisions() {
        let k0 = make_bezier_kf(0.0, 0.0, 0.01, 0.0, 0.0, 0.0);
        let k1 = make_bezier_kf(0.05, 1.0, 0.0, 0.0, -0.01, 0.0);
        let subs = compute_bezier_subdivisions(&k0, &k1);
        assert!(subs >= 2 && subs <= 10, "short segment: got {}", subs);
    }

    #[test]
    fn test_long_segment_many_subdivisions() {
        let k0 = make_bezier_kf(0.0, 0.0, 0.3, 0.0, 0.0, 0.0);
        let k1 = make_bezier_kf(3.0, 10.0, 0.0, 0.0, -0.3, 0.0);
        let subs = compute_bezier_subdivisions(&k0, &k1);
        assert!(subs >= 60, "long segment: got {}", subs);
    }

    #[test]
    fn test_linear_bezier_minimal_subdivisions() {
        let k0 = make_bezier_kf(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let k1 = make_bezier_kf(1.0, 1.0, 0.0, 0.0, 0.0, 0.0);
        let subs = compute_bezier_subdivisions(&k0, &k1);
        assert!(subs <= 31, "linear bezier: got {}", subs);
    }

    #[test]
    fn test_s_curve_high_subdivisions() {
        let k0 = make_bezier_kf(0.0, 0.0, 0.3, 5.0, 0.0, 0.0);
        let k1 = make_bezier_kf(1.0, 1.0, 0.0, 0.0, -0.3, -5.0);
        let subs = compute_bezier_subdivisions(&k0, &k1);
        assert!(subs > 30, "s-curve: got {}", subs);
    }

    #[test]
    fn test_curvature_zero_for_linear_handles() {
        let k0 = make_bezier_kf(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let k1 = make_bezier_kf(1.0, 1.0, 0.0, 0.0, 0.0, 0.0);
        let curvature = estimate_bezier_curvature(&k0, &k1);
        assert!(curvature.abs() < 0.01, "curvature: {}", curvature);
    }
}
