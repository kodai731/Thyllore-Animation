use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::BoneId;

use super::clip::{ClipSpace, EditableAnimationClip};
use super::curve::{PropertyCurve, PropertyType};
use super::keyframe::{CurveId, EditableKeyframe, KeyframeId};
use super::track::BoneTrack;

pub const ANIMATION_FORMAT_VERSION: u32 = 2;

/// On-disk clip. Scalar curves are keyed by channel name because `PropertyType::Custom`
/// codes are process-local; the application resolves names when it saves and loads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipFile {
    pub version: u32,
    pub clip: EditableAnimationClip,
    #[serde(default)]
    pub scalar_curves: Vec<NamedScalarCurve>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedScalarCurve {
    pub channel: String,
    pub id: CurveId,
    pub keyframes: Vec<EditableKeyframe>,
    pub next_keyframe_id: KeyframeId,
}

#[derive(Debug, Clone)]
pub struct RoleSlot {
    pub index: BoneId,
    pub allows_translation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipFileError {
    UnnamedScalarCurve(PropertyType),
    UnknownScalarChannel(String),
    UnknownRole(String),
    RoleTranslationNotAllowed(String),
    RoleScaleNotAllowed(String),
}

impl std::fmt::Display for ClipFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipFileError::UnnamedScalarCurve(property_type) => {
                write!(f, "scalar curve {property_type:?} has no channel name")
            }
            ClipFileError::UnknownScalarChannel(channel) => {
                write!(f, "unknown scalar channel {channel}")
            }
            ClipFileError::UnknownRole(role) => {
                write!(f, "unknown role {role}")
            }
            ClipFileError::RoleTranslationNotAllowed(role) => {
                write!(f, "translation not allowed for role {role}")
            }
            ClipFileError::RoleScaleNotAllowed(role) => {
                write!(f, "scale not allowed for role {role}")
            }
        }
    }
}

impl std::error::Error for ClipFileError {}

impl AnimationClipFile {
    pub fn from_clip(
        clip: &EditableAnimationClip,
        channel_name: impl Fn(PropertyType) -> Option<String>,
    ) -> Result<Self, ClipFileError> {
        let scalar_curves = clip
            .scalar_curves
            .iter()
            .map(|curve| {
                let channel = channel_name(curve.property_type)
                    .ok_or(ClipFileError::UnnamedScalarCurve(curve.property_type))?;
                Ok(NamedScalarCurve {
                    channel,
                    id: curve.id,
                    keyframes: curve.keyframes.clone(),
                    next_keyframe_id: curve.next_keyframe_id(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut file_clip = clip.clone();
        file_clip.scalar_curves.clear();

        Ok(Self {
            version: ANIMATION_FORMAT_VERSION,
            clip: file_clip,
            scalar_curves,
        })
    }

    fn resolve_role_tracks(
        tracks: HashMap<BoneId, BoneTrack>,
        role_slot: impl Fn(&str) -> Option<RoleSlot>,
    ) -> Result<HashMap<BoneId, BoneTrack>, ClipFileError> {
        let mut new_tracks = HashMap::new();
        for (_old_key, mut track) in tracks.into_iter() {
            let slot = role_slot(&track.bone_name)
                .ok_or_else(|| ClipFileError::UnknownRole(track.bone_name.clone()))?;

            if !slot.allows_translation
                && (!track.translation_x.is_empty()
                    || !track.translation_y.is_empty()
                    || !track.translation_z.is_empty())
            {
                return Err(ClipFileError::RoleTranslationNotAllowed(track.bone_name));
            }

            if !track.scale_x.is_empty() || !track.scale_y.is_empty() || !track.scale_z.is_empty() {
                return Err(ClipFileError::RoleScaleNotAllowed(track.bone_name));
            }

            track.bone_id = slot.index;
            new_tracks.insert(slot.index, track);
        }
        Ok(new_tracks)
    }

    pub fn into_clip(
        self,
        property_type: impl Fn(&str) -> Option<PropertyType>,
        role_slot: impl Fn(&str) -> Option<RoleSlot>,
    ) -> Result<EditableAnimationClip, ClipFileError> {
        let mut clip = self.clip;

        if clip.space == ClipSpace::HumanoidRole {
            clip.tracks = Self::resolve_role_tracks(clip.tracks, role_slot)?;
        }

        clip.scalar_curves = self
            .scalar_curves
            .into_iter()
            .map(|curve| {
                let property_type = property_type(&curve.channel)
                    .ok_or(ClipFileError::UnknownScalarChannel(curve.channel))?;
                Ok(PropertyCurve::from_keyframes(
                    curve.id,
                    property_type,
                    curve.keyframes,
                    curve.next_keyframe_id,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(clip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable::systems::curve_ops::curve_add_keyframe;

    fn clip_with_scalar_curve(property_type: PropertyType) -> EditableAnimationClip {
        let mut clip = EditableAnimationClip::new(1, "clip".to_string());
        let curve = clip.get_or_add_scalar_curve(property_type);
        curve_add_keyframe(curve, 0.5, 2.0);
        clip
    }

    #[test]
    fn test_scalar_curves_round_trip_by_channel_name() {
        let clip = clip_with_scalar_curve(PropertyType::Custom(7));
        let file = AnimationClipFile::from_clip(&clip, |_| Some("probe_level".to_string()))
            .expect("named");
        assert_eq!(file.scalar_curves[0].channel, "probe_level");
        assert!(file.clip.scalar_curves.is_empty());

        let text = ron::to_string(&file).expect("serialize");
        assert!(!text.contains("Custom"), "{text}");
        let parsed: AnimationClipFile = ron::from_str(&text).expect("parse");
        let loaded = parsed
            .into_clip(
                |name| (name == "probe_level").then_some(PropertyType::Custom(9)),
                |_| None,
            )
            .expect("resolved");

        let curve = loaded
            .get_scalar_curve(PropertyType::Custom(9))
            .expect("curve under the new code");
        assert_eq!(curve.id, clip.scalar_curves[0].id);
        assert_eq!(curve.keyframes.len(), 1);
        assert_eq!(
            curve.next_keyframe_id(),
            clip.scalar_curves[0].next_keyframe_id()
        );
    }

    #[test]
    fn test_unresolved_names_fail_at_the_boundary() {
        let clip = clip_with_scalar_curve(PropertyType::Custom(7));
        assert_eq!(
            AnimationClipFile::from_clip(&clip, |_| None).err(),
            Some(ClipFileError::UnnamedScalarCurve(PropertyType::Custom(7)))
        );

        let file =
            AnimationClipFile::from_clip(&clip, |_| Some("gone".to_string())).expect("named");
        assert_eq!(
            file.into_clip(|_| None, |_| None).err(),
            Some(ClipFileError::UnknownScalarChannel("gone".to_string()))
        );
    }

    #[test]
    fn clip_file_v1_loads_as_bone_space() {
        let text = r#"(version: 1, clip: (id: 1, name: "v1", duration: 0.0, tracks: {}, source_path: None, next_curve_id: 1), scalar_curves: [])"#;
        let file: AnimationClipFile = ron::from_str(text).expect("parse v1 format");
        assert_eq!(file.clip.space, ClipSpace::Bone);
    }

    #[test]
    fn role_clip_rejects_non_hips_translation() {
        use crate::editable::components::curve::PropertyType;

        let mut clip = EditableAnimationClip::new(1, "role".to_string());
        clip.space = ClipSpace::HumanoidRole;
        let track = clip.add_track(0, "spine".to_string());
        curve_add_keyframe(track.get_curve_mut(PropertyType::TranslationX), 0.0, 1.0);

        let file = AnimationClipFile::from_clip(&clip, |_| None).expect("named");
        let result = file.into_clip(
            |_| None,
            |name| {
                if name == "spine" {
                    Some(RoleSlot {
                        index: 5,
                        allows_translation: false,
                    })
                } else {
                    None
                }
            },
        );
        assert_eq!(
            result.err(),
            Some(ClipFileError::RoleTranslationNotAllowed(
                "spine".to_string()
            ))
        );
    }

    #[test]
    fn role_clip_round_trip() {
        use crate::editable::components::curve::PropertyType;

        let mut clip = EditableAnimationClip::new(1, "role".to_string());
        clip.space = ClipSpace::HumanoidRole;
        let track = clip.add_track(0, "hips".to_string());
        curve_add_keyframe(track.get_curve_mut(PropertyType::RotationX), 0.5, 0.3);

        let file = AnimationClipFile::from_clip(&clip, |_| None).expect("named");
        let text = ron::to_string(&file).expect("serialize");
        let parsed: AnimationClipFile = ron::from_str(&text).expect("parse");

        let loaded = parsed
            .into_clip(
                |_| None,
                |name| {
                    if name == "hips" {
                        Some(RoleSlot {
                            index: 10,
                            allows_translation: true,
                        })
                    } else {
                        None
                    }
                },
            )
            .expect("resolved");

        let restored = loaded.get_track(10).expect("track at role index");
        assert_eq!(restored.bone_name, "hips");
        assert_eq!(restored.rotation_x.keyframes.len(), 1);
        assert!((restored.rotation_x.keyframes[0].time - 0.5).abs() < 1e-6);
        assert!((restored.rotation_x.keyframes[0].value - 0.3).abs() < 1e-6);
    }
}
