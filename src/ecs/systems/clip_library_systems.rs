use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};

use crate::animation::editable::{EditableAnimationClip, SourceClip, SourceClipId};
use crate::animation::{AnimationClip, BoneId};
use crate::asset::{AnimationClipAsset, AssetStorage};
use crate::ecs::resource::ClipLibrary;
use crate::scene::AnimationClipFile;

pub fn clip_library_register_and_activate(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    editable: EditableAnimationClip,
) -> SourceClipId {
    let source_id = lib.next_source_id;
    lib.next_source_id += 1;

    let mut clip = editable;
    clip.id = source_id;

    let playable = clip.to_animation_clip();
    let asset_id = assets.add_animation_clip(AnimationClipAsset {
        id: 0,
        clip: playable,
    });

    let source = SourceClip::new(source_id, clip);
    lib.source_clips.insert(source_id, source);
    lib.source_to_asset_id.insert(source_id, asset_id);

    source_id
}

pub fn clip_library_create_from_imported(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    clip: &AnimationClip,
    bone_names: &HashMap<BoneId, String>,
) -> SourceClipId {
    let editable = EditableAnimationClip::from_animation_clip(0, clip, bone_names);
    clip_library_register_and_activate(lib, assets, editable)
}

pub fn clip_library_to_playable(lib: &ClipLibrary, id: SourceClipId) -> Option<AnimationClip> {
    lib.source_clips
        .get(&id)
        .map(|s| s.editable_clip.to_animation_clip())
}

pub fn clip_library_sync_dirty(lib: &mut ClipLibrary, assets: &mut AssetStorage) {
    for source_id in lib.dirty_sources.drain() {
        let (editable, asset_id) = match (
            lib.source_clips.get(&source_id),
            lib.source_to_asset_id.get(&source_id),
        ) {
            (Some(s), Some(&aid)) => (&s.editable_clip, aid),
            _ => continue,
        };

        let playable = editable.to_animation_clip();
        if let Some(asset) = assets.animation_clips.get_mut(&asset_id) {
            asset.clip = playable;
        }
    }
}

pub fn clip_library_clip_names(lib: &ClipLibrary) -> Vec<(SourceClipId, String)> {
    lib.source_clips
        .iter()
        .map(|(&id, s)| (id, s.editable_clip.name.clone()))
        .collect()
}

pub fn clip_library_save_to_file(lib: &ClipLibrary, id: SourceClipId, path: &Path) -> Result<()> {
    let source = lib.source_clips.get(&id).context("Clip not found")?;

    let clip_file = AnimationClipFile::new(source.editable_clip.clone());

    let file =
        fs::File::create(path).with_context(|| format!("Failed to create file: {:?}", path))?;
    let writer = BufWriter::new(file);

    ron::ser::to_writer_pretty(writer, &clip_file, ron::ser::PrettyConfig::default())
        .with_context(|| format!("Failed to serialize clip to: {:?}", path))?;

    crate::log!(
        "Saved animation clip '{}' to {:?}",
        source.editable_clip.name,
        path
    );
    Ok(())
}

pub fn clip_library_load_from_file(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    path: &Path,
    bone_name_to_id: Option<&HashMap<String, BoneId>>,
) -> Result<SourceClipId> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

    let mut clip = deserialize_clip(&content)
        .with_context(|| format!("Failed to deserialize clip from: {:?}", path))?;

    if let Some(name_to_id) = bone_name_to_id {
        let needs_remap = clip.tracks.values().any(|track| {
            name_to_id
                .get(&track.bone_name)
                .map_or(false, |&expected_id| expected_id != track.bone_id)
        });

        if needs_remap {
            let remapped_count = clip.tracks.len();
            clip.remap_bone_ids(name_to_id);
            crate::log!(
                "Remapped {} bone_ids by bone_name for '{}'",
                remapped_count,
                clip.name,
            );
        }
    }

    clip.source_path = Some(path.to_string_lossy().to_string());

    crate::log!("Loaded animation clip '{}' from {:?}", clip.name, path);

    let id = clip_library_register_and_activate(lib, assets, clip);
    Ok(id)
}

fn deserialize_clip(content: &str) -> Result<EditableAnimationClip> {
    if let Ok(clip_file) = ron::from_str::<AnimationClipFile>(content) {
        return Ok(clip_file.clip);
    }

    let clip = ron::from_str::<EditableAnimationClip>(content)?;
    Ok(clip)
}
