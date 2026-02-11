use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};

use crate::animation::editable::{EditableAnimationClip, SourceClip, SourceClipId};
use crate::animation::{AnimationClip, BoneId};
use crate::asset::{AnimationClipAsset, AssetStorage};
use crate::ecs::resource::ClipLibrary;

pub fn clip_library_create_from_imported(
    lib: &mut ClipLibrary,
    clip: &AnimationClip,
    bone_names: &HashMap<BoneId, String>,
) -> SourceClipId {
    let id = lib.next_source_id;
    lib.next_source_id += 1;

    let editable = EditableAnimationClip::from_animation_clip(id, clip, bone_names);
    let source = SourceClip::new(id, editable);
    lib.source_clips.insert(id, source);
    lib.source_to_anim_id.insert(id, clip.id);
    id
}

pub fn clip_library_create_empty(
    lib: &mut ClipLibrary,
    name: String,
) -> SourceClipId {
    let id = lib.next_source_id;
    lib.next_source_id += 1;

    let editable = EditableAnimationClip::new(id, name);
    let source = SourceClip::new(id, editable);
    lib.source_clips.insert(id, source);
    id
}

pub fn clip_library_register_clip(
    lib: &mut ClipLibrary,
    mut clip: EditableAnimationClip,
) -> SourceClipId {
    let id = lib.next_source_id;
    lib.next_source_id += 1;
    clip.id = id;
    let source = SourceClip::new(id, clip);
    lib.source_clips.insert(id, source);
    id
}

pub fn clip_library_to_playable(
    lib: &ClipLibrary,
    id: SourceClipId,
) -> Option<AnimationClip> {
    lib.source_clips
        .get(&id)
        .map(|s| s.editable_clip.to_animation_clip())
}

pub fn clip_library_sync_dirty(lib: &mut ClipLibrary) {
    for source_id in lib.dirty_sources.drain() {
        let (clip, anim_id) = match (
            lib.source_clips.get(&source_id),
            lib.source_to_anim_id.get(&source_id),
        ) {
            (Some(s), Some(&id)) => (&s.editable_clip, id),
            _ => continue,
        };

        let mut playable = clip.to_animation_clip();
        playable.id = anim_id;

        if let Some(target) = lib.animation.clips.iter_mut().find(|c| c.id == anim_id) {
            *target = playable;
        }
    }
}

pub fn clip_library_clip_names(lib: &ClipLibrary) -> Vec<(SourceClipId, String)> {
    lib.source_clips
        .iter()
        .map(|(&id, s)| (id, s.editable_clip.name.clone()))
        .collect()
}

pub fn clip_library_save_to_file(
    lib: &ClipLibrary,
    id: SourceClipId,
    path: &Path,
) -> Result<()> {
    let source = lib
        .source_clips
        .get(&id)
        .context("Clip not found")?;

    let file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {:?}", path))?;
    let writer = BufWriter::new(file);

    ron::ser::to_writer_pretty(
        writer,
        &source.editable_clip,
        ron::ser::PrettyConfig::default(),
    )
    .with_context(|| {
        format!("Failed to serialize clip to: {:?}", path)
    })?;

    crate::log!(
        "Saved animation clip '{}' to {:?}",
        source.editable_clip.name,
        path
    );
    Ok(())
}

pub fn clip_library_ensure_playable(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    source_id: SourceClipId,
) -> bool {
    if lib.source_to_anim_id.contains_key(&source_id) {
        return false;
    }

    let playable = match lib.source_clips.get(&source_id) {
        Some(source) => source.editable_clip.to_animation_clip(),
        None => return false,
    };

    crate::log!(
        "[EnsurePlayable] source_id={}, channels={}, duration={:.3}",
        source_id,
        playable.channels.len(),
        playable.duration,
    );
    for (&bone_id, ch) in playable.channels.iter().take(3) {
        let rot_sample = ch.sample_rotation(0.0);
        let trans_sample = ch.sample_translation(0.0);
        crate::log!(
            "[EnsurePlayable]   bone_id={}: rot@0={:?}, trans@0={:?}, rot_kf={}, trans_kf={}",
            bone_id,
            rot_sample,
            trans_sample,
            ch.rotation.len(),
            ch.translation.len(),
        );
    }

    let existing_clip_ids: Vec<_> = assets
        .animation_clips
        .values()
        .map(|a| a.clip_id)
        .collect();
    crate::log!(
        "[EnsurePlayable] existing asset clip_ids={:?}",
        existing_clip_ids,
    );

    let anim_id = lib.animation.add_clip(playable.clone());
    lib.source_to_anim_id.insert(source_id, anim_id);

    crate::log!(
        "[EnsurePlayable] assigned anim_id={}, source_to_anim_id={:?}",
        anim_id,
        lib.source_to_anim_id,
    );

    let mut asset_clip = playable;
    asset_clip.id = anim_id;
    assets.add_animation_clip(AnimationClipAsset {
        id: 0,
        clip_id: anim_id,
        clip: asset_clip,
    });

    true
}

pub fn clip_library_load_from_file(
    lib: &mut ClipLibrary,
    path: &Path,
    bone_name_to_id: Option<&HashMap<String, BoneId>>,
) -> Result<SourceClipId> {
    let file = fs::File::open(path)
        .with_context(|| format!("Failed to open file: {:?}", path))?;
    let reader = BufReader::new(file);

    let mut clip: EditableAnimationClip = ron::de::from_reader(reader)
        .with_context(|| {
            format!("Failed to deserialize clip from: {:?}", path)
        })?;

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

    let id = lib.next_source_id;
    lib.next_source_id += 1;
    clip.id = id;
    clip.source_path = Some(path.to_string_lossy().to_string());

    crate::log!(
        "Loaded animation clip '{}' from {:?} (id={})",
        clip.name,
        path,
        id
    );

    let source = SourceClip::new(id, clip);
    lib.source_clips.insert(id, source);
    Ok(id)
}
