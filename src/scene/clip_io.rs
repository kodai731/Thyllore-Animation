use std::fs;
use std::path::Path;

use crate::animation::editable::EditableAnimationClip;

use super::error::{SceneError, SceneResult};
use super::format::{AnimationClipFile, ANIMATION_FORMAT_VERSION};

pub fn save_animation_clip(path: &Path, clip: &EditableAnimationClip) -> SceneResult<()> {
    let clip_file = AnimationClipFile::new(clip.clone());

    let config = ron::ser::PrettyConfig::new()
        .depth_limit(10)
        .separate_tuple_members(true)
        .enumerate_arrays(false);

    let content = ron::ser::to_string_pretty(&clip_file, config)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    crate::log!("Saved animation clip to: {}", path.display());

    Ok(())
}

pub fn load_animation_clip(path: &Path) -> SceneResult<EditableAnimationClip> {
    if !path.exists() {
        return Err(SceneError::AnimationNotFound(path.to_path_buf()));
    }

    let content = fs::read_to_string(path)?;
    let clip_file: AnimationClipFile = ron::from_str(&content)?;

    if clip_file.version != ANIMATION_FORMAT_VERSION {
        return Err(SceneError::VersionMismatch {
            expected: ANIMATION_FORMAT_VERSION,
            found: clip_file.version,
        });
    }

    crate::log!("Loaded animation clip from: {}", path.display());
    Ok(clip_file.clip)
}
