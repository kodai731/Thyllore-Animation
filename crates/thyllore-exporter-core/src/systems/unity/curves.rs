use std::collections::BTreeMap;

use thyllore_anim_core::editable::{EditableAnimationClip, EditableKeyframe, InterpolationType};

use crate::components::morph::MORPH_WEIGHT_TO_PERCENT;
use crate::components::unity_anim::{UnityFloatCurve, UnityKey};
use crate::systems::fbx::animation::convert_tangent_to_fbx_slope_weight;

const ONE_FRAME_DURATION: f32 = 1.0 / 60.0;

pub fn expression_curves(mesh_path: &str, weights: &BTreeMap<String, f32>) -> Vec<UnityFloatCurve> {
    weights
        .iter()
        .map(|(channel, weight)| UnityFloatCurve {
            path: mesh_path.to_string(),
            attribute: format!("blendShape.{}", channel),
            keys: vec![UnityKey {
                time: 0.0,
                value: weight * MORPH_WEIGHT_TO_PERCENT,
                in_slope: 0.0,
                out_slope: 0.0,
            }],
        })
        .collect()
}

pub fn morph_track_curves(clip: &EditableAnimationClip) -> Vec<UnityFloatCurve> {
    clip.morph_tracks
        .iter()
        .filter(|track| !track.curve.is_empty())
        .map(|track| UnityFloatCurve {
            path: track.source_mesh.clone(),
            attribute: format!("blendShape.{}", track.channel),
            keys: convert_keyframes_to_unity_keys(&track.curve.keyframes, MORPH_WEIGHT_TO_PERCENT),
        })
        .collect()
}

fn convert_keyframes_to_unity_keys(
    keyframes: &[EditableKeyframe],
    value_scale: f32,
) -> Vec<UnityKey> {
    let segment_slopes: Vec<SegmentSlopes> = keyframes
        .windows(2)
        .map(|segment| compute_segment_slopes(&segment[0], &segment[1]))
        .collect();

    keyframes
        .iter()
        .enumerate()
        .map(|(index, keyframe)| {
            let arriving = index.checked_sub(1).and_then(|i| segment_slopes.get(i));
            let leaving = segment_slopes.get(index);

            UnityKey {
                time: keyframe.time,
                value: keyframe.value * value_scale,
                in_slope: arriving.map_or(0.0, |slopes| slopes.at_end) * value_scale,
                out_slope: leaving.map_or(0.0, |slopes| slopes.at_start) * value_scale,
            }
        })
        .collect()
}

struct SegmentSlopes {
    at_start: f32,
    at_end: f32,
}

// Mirrors `keyframes_sample`: stepped holds the first key, a non-bezier end of a segment is the secant.
fn compute_segment_slopes(start: &EditableKeyframe, end: &EditableKeyframe) -> SegmentSlopes {
    if start.interpolation == InterpolationType::Stepped {
        return SegmentSlopes {
            at_start: f32::INFINITY,
            at_end: f32::INFINITY,
        };
    }

    let duration = end.time - start.time;
    let secant_slope = if duration.abs() > f32::EPSILON {
        (end.value - start.value) / duration
    } else {
        0.0
    };
    let handle_slope = |handle| convert_tangent_to_fbx_slope_weight(handle, duration).0;

    SegmentSlopes {
        at_start: match start.interpolation {
            InterpolationType::Bezier => handle_slope(&start.out_tangent),
            InterpolationType::Linear | InterpolationType::Stepped => secant_slope,
        },
        at_end: match end.interpolation {
            InterpolationType::Bezier => handle_slope(&end.in_tangent),
            InterpolationType::Linear | InterpolationType::Stepped => secant_slope,
        },
    }
}

pub fn write_expression_anim(
    name: &str,
    mesh_path: &str,
    weights: &BTreeMap<String, f32>,
) -> String {
    let curves = expression_curves(mesh_path, weights);
    crate::systems::unity::anim::write_unity_anim(name, &curves, ONE_FRAME_DURATION, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_curves() {
        let mut weights = BTreeMap::new();
        weights.insert("smile".to_string(), 0.5);

        let curves = expression_curves("Body", &weights);

        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].path, "Body");
        assert_eq!(curves[0].attribute, "blendShape.smile");
        assert_eq!(curves[0].keys.len(), 1);
        assert_eq!(curves[0].keys[0].value, 50.0);
    }

    #[test]
    fn test_morph_track_curves() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let track = clip.get_or_add_morph_track("Body", "blink");
        curve_add_keyframe(&mut track.curve, 0.0, 0.0);
        curve_add_keyframe(&mut track.curve, 1.0, 1.0);

        let curves = morph_track_curves(&clip);

        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].path, "Body");
        assert_eq!(curves[0].attribute, "blendShape.blink");
        assert_eq!(curves[0].keys.len(), 2);
        assert_eq!(curves[0].keys[0].time, 0.0);
        assert_eq!(curves[0].keys[0].value, 0.0);
        assert_eq!(curves[0].keys[1].time, 1.0);
        assert_eq!(curves[0].keys[1].value, 100.0);
    }

    #[test]
    fn test_linear_keys_export_the_secant_slope() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let track = clip.get_or_add_morph_track("Body", "blink");
        curve_add_keyframe(&mut track.curve, 0.0, 1.0);
        curve_add_keyframe(&mut track.curve, 0.5, 0.0);
        curve_add_keyframe(&mut track.curve, 1.0, 0.5);

        let keys = &morph_track_curves(&clip)[0].keys;

        assert_eq!(keys[0].out_slope, -200.0);
        assert_eq!(keys[1].in_slope, -200.0);
        assert_eq!(keys[1].out_slope, 100.0);
        assert_eq!(keys[2].in_slope, 100.0);
    }

    #[test]
    fn test_stepped_key_exports_a_constant_segment() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let track = clip.get_or_add_morph_track("Body", "blink");
        curve_add_keyframe(&mut track.curve, 0.0, 1.0);
        curve_add_keyframe(&mut track.curve, 1.0, 0.0);
        track.curve.keyframes[0].interpolation = InterpolationType::Stepped;

        let keys = &morph_track_curves(&clip)[0].keys;

        assert_eq!(keys[0].out_slope, f32::INFINITY);
        assert_eq!(keys[1].in_slope, f32::INFINITY);
    }

    #[test]
    fn test_bezier_key_exports_its_handle_slope() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let track = clip.get_or_add_morph_track("Body", "blink");
        curve_add_keyframe(&mut track.curve, 0.0, 0.0);
        curve_add_keyframe(&mut track.curve, 1.0, 1.0);
        track.curve.keyframes[0].interpolation = InterpolationType::Bezier;
        track.curve.keyframes[0].out_tangent = BezierHandle::new(1.0 / 3.0, 0.0);

        let keys = &morph_track_curves(&clip)[0].keys;

        assert_eq!(keys[0].out_slope, 0.0);
        assert_eq!(keys[1].in_slope, 100.0);
    }

    use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
    use thyllore_anim_core::editable::BezierHandle;
}
