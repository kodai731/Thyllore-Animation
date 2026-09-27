use std::collections::BTreeMap;

use thyllore_anim_core::editable::EditableAnimationClip;

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
        .map(|track| {
            let keyframes = &track.curve.keyframes;
            let keys: Vec<UnityKey> = keyframes
                .iter()
                .enumerate()
                .map(|(i, kf)| {
                    let next_kf = keyframes.get(i + 1);
                    let key_interval = next_kf.map(|n| n.time - kf.time).unwrap_or(1.0 / 30.0);

                    let (out_slope, _) =
                        convert_tangent_to_fbx_slope_weight(&kf.out_tangent, key_interval);
                    let (in_slope, _) =
                        convert_tangent_to_fbx_slope_weight(&kf.in_tangent, key_interval);

                    UnityKey {
                        time: kf.time,
                        value: kf.value * MORPH_WEIGHT_TO_PERCENT,
                        in_slope: in_slope * MORPH_WEIGHT_TO_PERCENT,
                        out_slope: out_slope * MORPH_WEIGHT_TO_PERCENT,
                    }
                })
                .collect();

            UnityFloatCurve {
                path: track.source_mesh.clone(),
                attribute: format!("blendShape.{}", track.channel),
                keys,
            }
        })
        .collect()
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

    use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
}
