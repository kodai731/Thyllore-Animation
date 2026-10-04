use serde::{Deserialize, Serialize};

use super::clip::EditableAnimationClip;
use super::curve::{PropertyCurve, PropertyType};
use super::keyframe::{CurveId, EditableKeyframe, KeyframeId};

pub const ANIMATION_FORMAT_VERSION: u32 = 1;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipFileError {
    UnnamedScalarCurve(PropertyType),
    UnknownScalarChannel(String),
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

    pub fn into_clip(
        self,
        property_type: impl Fn(&str) -> Option<PropertyType>,
    ) -> Result<EditableAnimationClip, ClipFileError> {
        let mut clip = self.clip;
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
            .into_clip(|name| (name == "probe_level").then_some(PropertyType::Custom(9)))
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
            file.into_clip(|_| None).err(),
            Some(ClipFileError::UnknownScalarChannel("gone".to_string()))
        );
    }
}
