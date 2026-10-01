use std::collections::HashMap;

use cgmath::Vector3;

use crate::editable::components::clip::EditableAnimationClip;
use crate::editable::components::curve::PropertyCurve;
use crate::editable::components::keyframe::{InterpolationType, SourceClipId};
use crate::editable::systems::bake::collect_bake_times;
use crate::editable::systems::curve_ops::{curve_add_keyframe, curve_sample};
use crate::{AnimationClip, BoneId, Interpolation, Keyframe, MorphWeightChannel, TransformChannel};
use thyllore_math_core::{euler_degrees_to_quaternion, quaternion_to_euler_degrees};

pub fn clip_from_animation(
    id: SourceClipId,
    clip: &AnimationClip,
    bone_names: &HashMap<BoneId, String>,
) -> EditableAnimationClip {
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

    for morph_channel in &clip.morph_channels {
        let curve = &mut editable
            .get_or_add_morph_track(&morph_channel.source_mesh, &morph_channel.channel)
            .curve;
        import_scalar_keyframes(&morph_channel.keyframes, curve);
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
            let q = euler_degrees_to_quaternion(&Vector3::new(ex, ey, ez));
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

    for track in &clip.morph_tracks {
        if track.curve.is_empty() {
            continue;
        }
        let keyframes = collect_bake_times(&[&track.curve])
            .into_iter()
            .filter_map(|time| curve_sample(&track.curve, time).map(|w| Keyframe::new(time, w)))
            .collect();
        anim.add_morph_channel(MorphWeightChannel {
            source_mesh: track.source_mesh.clone(),
            channel: track.channel.clone(),
            keyframes,
        });
    }

    anim
}

fn import_scalar_keyframes(keyframes: &[Keyframe<f32>], curve: &mut PropertyCurve) {
    for (idx, kf) in keyframes.iter().enumerate() {
        let kf_id = curve_add_keyframe(curve, kf.time, kf.value);
        match kf.interpolation {
            Interpolation::Linear => {}
            Interpolation::Step => {
                if let Some(editable_kf) = curve.get_keyframe_mut(kf_id) {
                    editable_kf.interpolation = InterpolationType::Stepped;
                }
            }
            Interpolation::CubicSpline => {
                let dt = keyframes
                    .get(idx + 1)
                    .map(|n| n.time - kf.time)
                    .unwrap_or(0.1);
                if let Some(out_t) = kf.out_tangent {
                    set_cubic_bezier_handles(curve, kf_id, dt, out_t);
                }
                if let Some(in_t) = kf.in_tangent {
                    set_cubic_bezier_in_handles(curve, kf_id, dt, in_t);
                }
            }
        }
    }
}

fn import_vec3_keyframes(
    keyframes: &[Keyframe<Vector3<f32>>],
    curves: &mut [&mut PropertyCurve; 3],
) {
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
    use crate::editable::components::keyframe::BezierHandle;

    if let Some(kf) = curve.get_keyframe_mut(kf_id) {
        kf.interpolation = InterpolationType::Bezier;
        let handle_time = dt / 3.0;
        let handle_value = tangent_value * dt / 3.0;
        kf.out_tangent = BezierHandle::new(handle_time, handle_value);
    }
}

fn set_cubic_bezier_in_handles(curve: &mut PropertyCurve, kf_id: u64, dt: f32, tangent_value: f32) {
    use crate::editable::components::keyframe::BezierHandle;

    if let Some(kf) = curve.get_keyframe_mut(kf_id) {
        let handle_time = dt / 3.0;
        let handle_value = tangent_value * dt / 3.0;
        kf.in_tangent = BezierHandle::new(-handle_time, -handle_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn morph_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("face");
        clip.add_morph_channel(MorphWeightChannel {
            source_mesh: "head".to_string(),
            channel: "smile".to_string(),
            keyframes: vec![
                Keyframe::new(0.0, 0.0),
                Keyframe::new(1.0, 1.0),
                Keyframe::with_interpolation(2.0, 0.25, Interpolation::Step),
            ],
        });
        clip
    }

    #[test]
    fn morph_channels_become_editable_morph_tracks() {
        let editable = clip_from_animation(1, &morph_clip(), &HashMap::new());

        let track = editable
            .get_morph_track("head", "smile")
            .expect("morph track imported");
        let times: Vec<f32> = track.curve.keyframes.iter().map(|k| k.time).collect();
        assert_eq!(times, vec![0.0, 1.0, 2.0]);
        assert_eq!(
            track.curve.keyframes[2].interpolation,
            InterpolationType::Stepped
        );
        assert_eq!(editable.duration, 2.0);
    }

    #[test]
    fn morph_tracks_round_trip_to_animation_clip() {
        let editable = clip_from_animation(1, &morph_clip(), &HashMap::new());

        let clip = clip_to_animation(&editable);

        assert_eq!(clip.morph_channels.len(), 1);
        let channel = &clip.morph_channels[0];
        assert_eq!(channel.source_mesh, "head");
        assert_eq!(channel.channel, "smile");
        let values: Vec<f32> = channel.keyframes.iter().map(|k| k.value).collect();
        assert_eq!(values, vec![0.0, 1.0, 0.25]);
        assert_eq!(clip.duration, 2.0);
    }
}
