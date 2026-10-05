use std::fs;
use std::path::Path;

use thyllore_anim_core::editable::{PropertyType, RoleSlot};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::editable::EditableAnimationClip;
use crate::ecs::component::{scalar_channel_for_cli_name, scalar_channel_for_property};

use super::error::{SceneError, SceneResult};
use super::file::{AnimationClipFile, ANIMATION_FORMAT_VERSION};

pub fn save_animation_clip(path: &Path, clip: &EditableAnimationClip) -> SceneResult<()> {
    let clip_file =
        AnimationClipFile::from_clip(clip, scalar_channel_name).map_err(SceneError::ClipFile)?;
    thyllore_exporter_core::systems::ron::export_ron_clip(&clip_file, path)
        .map_err(SceneError::Export)
}

pub fn load_animation_clip(path: &Path) -> SceneResult<EditableAnimationClip> {
    if !path.exists() {
        return Err(SceneError::AnimationNotFound(path.to_path_buf()));
    }

    let content = fs::read_to_string(path)?;
    let clip = parse_animation_clip(&content)?;

    log!("Loaded animation clip from: {}", path.display());
    Ok(clip)
}

/// Decodes a clip file, resolving every scalar curve's channel name to its registered code.
pub fn parse_animation_clip(content: &str) -> SceneResult<EditableAnimationClip> {
    let clip_file: AnimationClipFile = ron::from_str(content)?;

    if !(clip_file.version == 1 || clip_file.version == ANIMATION_FORMAT_VERSION) {
        return Err(SceneError::VersionMismatch {
            expected: ANIMATION_FORMAT_VERSION,
            found: clip_file.version,
        });
    }

    clip_file
        .into_clip(scalar_channel_property, humanoid_role_slot)
        .map_err(SceneError::ClipFile)
}

fn scalar_channel_name(property_type: PropertyType) -> Option<String> {
    scalar_channel_for_property(property_type).map(|(_, channel)| channel.cli_name.to_string())
}

fn scalar_channel_property(name: &str) -> Option<PropertyType> {
    scalar_channel_for_cli_name(name).and_then(|(domain, channel)| domain.property_type_of(channel))
}

fn humanoid_role_slot(name: &str) -> Option<RoleSlot> {
    let role = HumanoidRole::from_unity_name(name)?;
    Some(RoleSlot {
        index: role.index() as thyllore_anim_core::BoneId,
        allows_translation: role.allows_translation(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::scalar_clip_systems::test_support::{probe_property, PROBE_LEVEL};
    use thyllore_anim_core::editable::{curve_add_keyframe, ClipSpace};

    #[test]
    fn test_clip_files_persist_scalar_curves_by_channel_name() {
        let mut clip = EditableAnimationClip::new(1, "probe".to_string());
        let property_type = probe_property(&PROBE_LEVEL);
        curve_add_keyframe(clip.get_or_add_scalar_curve(property_type), 0.25, 0.5);

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("probe.anim.ron");
        save_animation_clip(&path, &clip).expect("save");

        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains("\"probe_level\""), "{text}");
        assert!(!text.contains("Custom"), "{text}");

        let loaded = load_animation_clip(&path).expect("load");
        let curve = loaded
            .get_scalar_curve(property_type)
            .expect("curve resolved back to the probe channel");
        assert_eq!(curve.keyframes.len(), 1);
    }

    #[test]
    fn test_unknown_channel_names_are_rejected() {
        let text = r#"(version: 2, clip: (id: 1, name: "x", duration: 0.0, tracks: {}, source_path: None, next_curve_id: 1), scalar_curves: [(channel: "no_such_channel", id: 1, keyframes: [], next_keyframe_id: 1)])"#;
        assert!(matches!(
            parse_animation_clip(text),
            Err(SceneError::ClipFile(_))
        ));
    }

    #[test]
    fn role_clip_file_resolves_role_indices() {
        let text = r#"(version: 2, clip: (id: 1, name: "role", duration: 1.0, space: HumanoidRole, tracks: {0: (bone_id: 0, bone_name: "LeftUpperArm", translation_x: (id: 0, property_type: TranslationX, keyframes: [], next_keyframe_id: 1), translation_y: (id: 1, property_type: TranslationY, keyframes: [], next_keyframe_id: 1), translation_z: (id: 2, property_type: TranslationZ, keyframes: [], next_keyframe_id: 1), rotation_x: (id: 3, property_type: RotationX, keyframes: [(id: 1, time: 0.0, value: 0.5, in_tangent: (time_offset: 0.0, value_offset: 0.0), out_tangent: (time_offset: 0.0, value_offset: 0.0))], next_keyframe_id: 2), rotation_y: (id: 4, property_type: RotationY, keyframes: [], next_keyframe_id: 1), rotation_z: (id: 5, property_type: RotationZ, keyframes: [], next_keyframe_id: 1), scale_x: (id: 6, property_type: ScaleX, keyframes: [], next_keyframe_id: 1), scale_y: (id: 7, property_type: ScaleY, keyframes: [], next_keyframe_id: 1), scale_z: (id: 8, property_type: ScaleZ, keyframes: [], next_keyframe_id: 1))}, source_path: None, next_curve_id: 9), scalar_curves: [])"#;
        let clip = parse_animation_clip(text).expect("parse");
        let expected: u32 = HumanoidRole::LeftUpperArm.index() as u32;
        let track = clip.get_track(expected).expect("track at role index");
        assert_eq!(track.bone_id, expected);
        assert!(
            clip.get_track(0).is_none(),
            "track at index 0 should not exist"
        );
    }

    #[test]
    fn v1_clip_file_still_loads() {
        let text = r#"(version: 1, clip: (id: 1, name: "v1", duration: 0.0, tracks: {}, source_path: None, next_curve_id: 1), scalar_curves: [])"#;
        let clip = parse_animation_clip(text).expect("parse v1");
        assert_eq!(clip.name, "v1");
        assert_eq!(clip.space, ClipSpace::Bone);
    }
}
