use crate::clip::AnimationClip;
use crate::editable::{BoneTrack, EditableAnimationClip, PropertyCurve};
use crate::spring_bone::{ColliderShape, SpringBoneSetup};
use crate::BoneId;

pub fn scale_spring_setup(setup: &mut SpringBoneSetup, factor: f32) {
    for collider in &mut setup.colliders {
        collider.offset *= factor;
        collider.shape = match collider.shape {
            ColliderShape::Sphere { radius } => ColliderShape::Sphere {
                radius: radius * factor,
            },
            ColliderShape::Capsule { radius, tail } => ColliderShape::Capsule {
                radius: radius * factor,
                tail: tail * factor,
            },
        };
    }
}

pub fn scale_animation_clip_translations(clip: &mut AnimationClip, factor: f32) {
    for channel in clip.channels.values_mut() {
        for key in &mut channel.translation {
            key.value *= factor;
            key.in_tangent = key.in_tangent.map(|tangent| tangent * factor);
            key.out_tangent = key.out_tangent.map(|tangent| tangent * factor);
        }
    }
}

/// Scale the translation curves of the tracks `is_model_space` accepts; humanoid tracks keep their normalised values.
pub fn scale_editable_clip_translations(
    clip: &mut EditableAnimationClip,
    factor: f32,
    is_model_space: impl Fn(BoneId) -> bool,
) {
    for (bone_id, track) in &mut clip.tracks {
        if is_model_space(*bone_id) {
            scale_track_translations(track, factor);
        }
    }
}

fn scale_track_translations(track: &mut BoneTrack, factor: f32) {
    for curve in [
        &mut track.translation_x,
        &mut track.translation_y,
        &mut track.translation_z,
    ] {
        scale_curve_values(curve, factor);
    }
}

fn scale_curve_values(curve: &mut PropertyCurve, factor: f32) {
    for keyframe in &mut curve.keyframes {
        keyframe.value *= factor;
        keyframe.in_tangent.value_offset *= factor;
        keyframe.out_tangent.value_offset *= factor;
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;
    use crate::clip::{Keyframe, TransformChannel};
    use crate::editable::EditableKeyframe;
    use crate::spring_bone::{SpringChain, SpringColliderDef, SpringJointParam};

    #[test]
    fn spring_setup_scale_touches_collider_geometry_only() {
        let mut setup = SpringBoneSetup::default();
        setup.chains.push(SpringChain {
            joints: vec![SpringJointParam {
                hit_radius: 0.02,
                stiffness: 0.5,
                ..Default::default()
            }],
            ..Default::default()
        });
        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vector3::new(1.0, 0.0, 0.0),
            shape: ColliderShape::Capsule {
                radius: 0.1,
                tail: Vector3::new(0.0, 2.0, 0.0),
            },
        });

        scale_spring_setup(&mut setup, 100.0);

        let joint = &setup.chains[0].joints[0];
        assert_eq!(joint.hit_radius, 0.02);
        assert_eq!(joint.stiffness, 0.5);
        assert_eq!(setup.colliders[0].offset, Vector3::new(100.0, 0.0, 0.0));
        match setup.colliders[0].shape {
            ColliderShape::Capsule { radius, tail } => {
                assert!((radius - 10.0).abs() < 1e-5);
                assert_eq!(tail, Vector3::new(0.0, 200.0, 0.0));
            }
            ColliderShape::Sphere { .. } => panic!("shape kind changed"),
        }
    }

    #[test]
    fn animation_clip_scale_touches_translation_keys_only() {
        let mut clip = AnimationClip::default();
        clip.channels.insert(
            0,
            TransformChannel {
                translation: vec![Keyframe::new(0.0, Vector3::new(1.0, 2.0, 3.0))],
                rotation: Vec::new(),
                scale: vec![Keyframe::new(0.0, Vector3::new(1.0, 1.0, 1.0))],
            },
        );

        scale_animation_clip_translations(&mut clip, 0.5);

        assert_eq!(
            clip.channels[&0].translation[0].value,
            Vector3::new(0.5, 1.0, 1.5)
        );
        assert_eq!(
            clip.channels[&0].scale[0].value,
            Vector3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn editable_clip_scale_skips_tracks_outside_model_space() {
        let mut clip = EditableAnimationClip::new(1, "c".into());
        for bone in [0u32, 1] {
            let track = clip
                .tracks
                .entry(bone)
                .or_insert_with(|| BoneTrack::new(bone, format!("b{bone}"), u64::from(bone) * 10));
            let mut key = EditableKeyframe::new(1, 0.0, 4.0);
            key.out_tangent.value_offset = 2.0;
            track.translation_x.keyframes.push(key);
            track
                .rotation_x
                .keyframes
                .push(EditableKeyframe::new(2, 0.0, 4.0));
        }

        scale_editable_clip_translations(&mut clip, 0.25, |bone| bone == 1);

        assert_eq!(clip.tracks[&0].translation_x.keyframes[0].value, 4.0);
        assert_eq!(clip.tracks[&1].translation_x.keyframes[0].value, 1.0);
        assert_eq!(
            clip.tracks[&1].translation_x.keyframes[0]
                .out_tangent
                .value_offset,
            0.5
        );
        assert_eq!(clip.tracks[&1].rotation_x.keyframes[0].value, 4.0);
    }
}
