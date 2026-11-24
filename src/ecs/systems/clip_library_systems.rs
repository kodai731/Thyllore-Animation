use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::asset::{AnimationClipAsset, AssetStorage};
use crate::ecs::resource::{AssetEditCommand, AssetEditQueue, ClipLibrary};
use crate::ecs::world::World;
use crate::scene::SceneError;
use thyllore_anim_core::editable::{
    clip_remap_bone_ids, clip_to_animation, EditableAnimationClip, SourceClip, SourceClipId,
};
use thyllore_anim_core::{AnimationClip, BoneId};

pub const EMPTY_MODEL_CLIP_DURATION_SECONDS: f32 = 5.0;

/// A clip file dropped on the window loads the way the clip browser's Load button does.
fn load_dropped_clip_file(world: &mut World, path: &str) {
    world
        .resource_mut::<AssetEditQueue>()
        .push(AssetEditCommand::LoadClipFromFile {
            path: PathBuf::from(path),
        });
}

crate::dropped_file_hook!(".anim.ron", load_dropped_clip_file);

/// The clip a model without animation is given, so the model can always be keyed.
pub fn clip_library_register_empty_model_clip(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
) -> SourceClipId {
    let mut editable = EditableAnimationClip::new(0, "New Animation".to_string());
    editable.duration = EMPTY_MODEL_CLIP_DURATION_SECONDS;
    editable.min_duration = EMPTY_MODEL_CLIP_DURATION_SECONDS;

    let source_id = clip_library_register_and_activate(lib, assets, editable);
    lib.model_clip_ids.insert(source_id);
    source_id
}

pub fn clip_library_register_and_activate(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    editable: EditableAnimationClip,
) -> SourceClipId {
    let source_id = lib.next_source_id;
    lib.next_source_id += 1;

    let mut clip = editable;
    clip.id = source_id;

    let playable = clip_to_animation(&clip);
    let asset_id = assets.add_animation_clip(AnimationClipAsset {
        id: 0,
        clip: playable,
    });
    lib.source_to_asset_id.insert(source_id, asset_id);

    let source = SourceClip::new(source_id, clip);
    lib.source_clips.insert(source_id, source);

    source_id
}

/// Gives every library clip its playable asset again once the asset storage was cleared.
pub fn clip_library_rebuild_playable_assets(lib: &mut ClipLibrary, assets: &mut AssetStorage) {
    let mut source_ids: Vec<SourceClipId> = lib.all_clip_ids().copied().collect();
    source_ids.sort_unstable();

    for source_id in source_ids {
        let Some(playable) = clip_library_to_playable(lib, source_id) else {
            continue;
        };
        let asset_id = assets.add_animation_clip(AnimationClipAsset {
            id: 0,
            clip: playable,
        });
        lib.source_to_asset_id.insert(source_id, asset_id);
    }
}

/// Registers clips read from disk, returning each new id with the clip's name.
pub fn clip_library_register_loaded(
    world: &mut crate::ecs::world::World,
    assets: &mut AssetStorage,
    clips: Vec<EditableAnimationClip>,
) -> Vec<(SourceClipId, String)> {
    let mut clip_library = world.resource_mut::<ClipLibrary>();
    clips
        .into_iter()
        .map(|clip| {
            let name = clip.name.clone();
            let id = clip_library_register_and_activate(&mut clip_library, assets, clip);
            (id, name)
        })
        .collect()
}

pub fn clip_library_to_playable(lib: &ClipLibrary, id: SourceClipId) -> Option<AnimationClip> {
    lib.source_clips
        .get(&id)
        .map(|s| clip_to_animation(&s.editable_clip))
}

/// Releases the playable assets of removed clips and rewrites those of edited clips.
pub fn clip_library_sync_playable_assets(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
) -> Vec<SourceClipId> {
    for asset_id in lib.take_released_asset_ids() {
        assets.animation_clips.remove(&asset_id);
    }

    let mut synced_ids: Vec<SourceClipId> = Vec::new();
    for source_id in std::mem::take(&mut lib.dirty_sources) {
        let Some(playable) = clip_library_to_playable(lib, source_id) else {
            continue;
        };
        write_playable_asset(lib, assets, source_id, playable);
        synced_ids.push(source_id);
    }
    synced_ids
}

fn write_playable_asset(
    lib: &mut ClipLibrary,
    assets: &mut AssetStorage,
    source_id: SourceClipId,
    playable: AnimationClip,
) {
    let existing_asset = lib
        .get_asset_id_for_source(source_id)
        .and_then(|asset_id| assets.animation_clips.get_mut(&asset_id));

    if let Some(asset) = existing_asset {
        asset.clip = playable;
        return;
    }

    let asset_id = assets.add_animation_clip(AnimationClipAsset {
        id: 0,
        clip: playable,
    });
    lib.source_to_asset_id.insert(source_id, asset_id);
}

/// Listed in registration order, so the rows keep their place between frames and runs.
pub fn clip_library_clip_names(lib: &ClipLibrary) -> Vec<(SourceClipId, String)> {
    let mut clip_names: Vec<(SourceClipId, String)> = lib
        .source_clips
        .iter()
        .map(|(&id, s)| (id, s.editable_clip.name.clone()))
        .collect();
    clip_names.sort_unstable_by_key(|(id, _)| *id);
    clip_names
}

pub fn clip_library_update_save_metadata(
    lib: &mut ClipLibrary,
    id: SourceClipId,
    name: String,
    path: &Path,
) {
    if let Some(clip) = lib.get_mut(id) {
        clip.name = name;
        clip.source_path = Some(path.to_string_lossy().to_string());
    }
}

pub fn clip_library_save_to_file(lib: &ClipLibrary, id: SourceClipId, path: &Path) -> Result<()> {
    let source = lib.source_clips.get(&id).context("Clip not found")?;

    crate::scene::save_animation_clip(path, &source.editable_clip)?;

    log!(
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
        let has_misbound_track = clip.tracks.values().any(|track| {
            name_to_id
                .get(&track.bone_name)
                .map_or(false, |&expected_id| expected_id != track.bone_id)
        });

        if has_misbound_track || !clip.unbound_tracks.is_empty() {
            let remapped_count = clip.tracks.len();
            clip_remap_bone_ids(&mut clip, name_to_id);
            log!(
                "Remapped {} bone_ids by bone_name for '{}'",
                remapped_count,
                clip.name,
            );
        }
    }

    clip.source_path = Some(path.to_string_lossy().to_string());

    msg_info!("Loaded animation clip '{}' from {:?}", clip.name, path);

    let id = clip_library_register_and_activate(lib, assets, clip);
    Ok(id)
}

pub fn find_clip_schedule_owner(
    world: &crate::ecs::world::World,
    entity: crate::ecs::world::Entity,
) -> Option<crate::ecs::world::Entity> {
    use crate::ecs::component::ClipSchedule;
    use crate::ecs::world::Parent;

    let mut current = entity;
    loop {
        if world.get_component::<ClipSchedule>(current).is_some() {
            return Some(current);
        }
        match world.get_component::<Parent>(current) {
            Some(parent) => current = parent.0,
            None => return None,
        }
    }
}

pub fn query_selected_model_clip_ids(world: &crate::ecs::world::World) -> Vec<SourceClipId> {
    use crate::ecs::component::ClipSchedule;
    use crate::ecs::resource::HierarchyState;

    let Some(selected) = world.resource::<HierarchyState>().selected_entity else {
        return Vec::new();
    };
    let Some(owner) = find_clip_schedule_owner(world, selected) else {
        return Vec::new();
    };
    let Some(schedule) = world.get_component::<ClipSchedule>(owner) else {
        return Vec::new();
    };

    let mut clip_ids: Vec<SourceClipId> = schedule
        .instances
        .iter()
        .map(|inst| inst.source_id)
        .collect();
    clip_ids.sort_unstable();
    clip_ids.dedup();
    clip_ids
}

pub fn clip_library_resample_selected_model(
    world: &mut crate::ecs::world::World,
    assets: &mut AssetStorage,
    fps: f32,
) -> Option<usize> {
    use crate::ecs::resource::EditHistory;
    use thyllore_anim_core::editable::clip_resample_at_fps;

    let clip_ids = query_selected_model_clip_ids(world);
    if clip_ids.is_empty() {
        return None;
    }

    let mut new_sources: Vec<(SourceClipId, SourceClip)> = Vec::new();
    {
        let mut lib = world.resource_mut::<ClipLibrary>();
        for source_id in &clip_ids {
            let Some(mut clip) = lib.get(*source_id).cloned() else {
                continue;
            };
            let summary = clip_resample_at_fps(&mut clip, fps);
            clip.name = format!("{} ({}fps)", clip.name, fps as i64);
            clip.source_path = None;
            let new_id = clip_library_register_and_activate(&mut lib, assets, clip);

            log!(
                "Resampled clip (src {}) to new clip {} at {} fps: {} curves, {} keyframes",
                source_id,
                new_id,
                fps,
                summary.resampled_curve_count,
                summary.total_keyframe_count,
            );
            if let Some(source) = lib.source_clips.get(&new_id).cloned() {
                new_sources.push((new_id, source));
            }
        }
    }

    if world.contains_resource::<EditHistory>() {
        let mut edit_history = world.resource_mut::<EditHistory>();
        for (new_id, source) in &new_sources {
            edit_history.push_clip_added(*new_id, source.clone(), "resample clip");
        }
    }

    Some(new_sources.len())
}

pub fn clip_library_count_references(
    world: &crate::ecs::world::World,
) -> Vec<(SourceClipId, usize)> {
    use crate::ecs::component::ClipSchedule;

    let mut counts: HashMap<SourceClipId, usize> = HashMap::new();

    let entities = world.component_entities::<ClipSchedule>();
    for entity in entities {
        if let Some(schedule) = world.get_component::<ClipSchedule>(entity) {
            for inst in &schedule.instances {
                *counts.entry(inst.source_id).or_insert(0) += 1;
            }
        }
    }

    counts.into_iter().collect()
}

fn deserialize_clip(content: &str) -> Result<EditableAnimationClip> {
    match crate::scene::parse_animation_clip(content) {
        Ok(clip) => Ok(clip),
        Err(SceneError::Parse(_)) => Ok(ron::from_str::<EditableAnimationClip>(content)?),
        Err(error) => Err(error.into()),
    }
}

/// The library's first animated clip by id, see `select_first_animated_clip`.
pub fn find_best_clip(world: &World) -> Option<SourceClipId> {
    let clip_library = world.get_resource::<ClipLibrary>()?;
    let mut clip_ids: Vec<_> = clip_library.all_clip_ids().copied().collect();
    clip_ids.sort_unstable();

    select_first_animated_clip(&clip_library, &clip_ids)
}

/// The first clip with bone or morph tracks, else the first clip: an empty default take is skipped.
pub fn select_first_animated_clip(
    clip_library: &ClipLibrary,
    clip_ids: &[SourceClipId],
) -> Option<SourceClipId> {
    clip_ids
        .iter()
        .copied()
        .find(|&id| {
            clip_library
                .get(id)
                .is_some_and(|clip| clip.has_pose_tracks())
        })
        .or_else(|| clip_ids.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_a_clip_moves_role_tracks_to_model_bones() {
        use thyllore_anim_core::editable::curve_add_keyframe;
        use thyllore_avatar_core::humanoid::HumanoidRole;

        let mut clip = crate::ecs::systems::new_empty_clip("bow");
        let track = clip.add_track(
            HumanoidRole::Head.index() as BoneId,
            HumanoidRole::Head.unity_name().to_string(),
        );
        curve_add_keyframe(&mut track.rotation_x, 0.0, 10.0);
        let path = std::env::temp_dir().join(format!("clip_load_{}.anim.ron", std::process::id()));
        crate::scene::save_animation_clip(&path, &clip).unwrap();

        let model_head_bone: BoneId = 181;
        let mut model_bones = HashMap::new();
        model_bones.insert("Head".to_string(), model_head_bone);
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();
        let id =
            clip_library_load_from_file(&mut lib, &mut assets, &path, Some(&model_bones)).unwrap();
        fs::remove_file(&path).ok();

        let loaded = lib.get(id).unwrap();
        assert_eq!(loaded.tracks.len(), 1);
        let head_track = loaded.get_track(model_head_bone).unwrap();
        assert_eq!(head_track.bone_name, "Head");
        assert_eq!(head_track.bone_id, model_head_bone);
        assert_eq!(head_track.rotation_x.keyframes.len(), 1);
    }

    #[test]
    fn loading_a_clip_binds_its_unbound_tracks_to_model_bones() {
        use thyllore_anim_core::editable::{curve_add_keyframe, BoneTrack};

        let mut clip = crate::ecs::systems::new_empty_clip("tail wag");
        let mut tail_track = BoneTrack::new(7, "Tail".to_string(), 1);
        curve_add_keyframe(&mut tail_track.rotation_x, 0.0, 10.0);
        clip.unbound_tracks.push(tail_track);
        let path =
            std::env::temp_dir().join(format!("clip_unbound_{}.anim.ron", std::process::id()));
        crate::scene::save_animation_clip(&path, &clip).unwrap();

        let model_tail_bone: BoneId = 42;
        let mut model_bones = HashMap::new();
        model_bones.insert("Tail".to_string(), model_tail_bone);
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();
        let id =
            clip_library_load_from_file(&mut lib, &mut assets, &path, Some(&model_bones)).unwrap();
        fs::remove_file(&path).ok();

        let loaded = lib.get(id).unwrap();
        assert!(loaded.unbound_tracks.is_empty());
        assert_eq!(loaded.get_track(model_tail_bone).unwrap().bone_name, "Tail");
    }

    #[test]
    fn clip_registers_with_a_playable_asset() {
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();

        let clip = EditableAnimationClip::new(0, "clip".to_string());

        let source_id = clip_library_register_and_activate(&mut lib, &mut assets, clip);

        assert!(lib.source_clips.contains_key(&source_id));
        assert!(lib.source_to_asset_id.contains_key(&source_id));
        assert_eq!(assets.animation_clips.len(), 1);
    }

    #[test]
    fn resampled_copy_does_not_own_the_file_of_its_source_clip() {
        use crate::ecs::resource::HierarchyState;
        use crate::ecs::systems::schedule_entity_clip;

        let mut world = crate::ecs::world::World::new();
        let mut assets = AssetStorage::default();
        let mut lib = ClipLibrary::default();
        let mut saved_clip = EditableAnimationClip::new(0, "saved".to_string());
        saved_clip.source_path = Some("animations/saved.anim.ron".to_string());
        let saved_id = clip_library_register_and_activate(&mut lib, &mut assets, saved_clip);
        world.insert_resource(lib);
        let owner = world.entity().with_name("owner").build();
        schedule_entity_clip(&mut world, owner, saved_id);
        let mut hierarchy = HierarchyState::default();
        hierarchy.selected_entity = Some(owner);
        world.insert_resource(hierarchy);

        let resampled_count = clip_library_resample_selected_model(&mut world, &mut assets, 30.0);

        let lib = world.resource::<ClipLibrary>();
        let resampled_id = lib
            .find_source_by_name("saved (30fps)")
            .expect("resampled copy");
        assert_eq!(resampled_count, Some(1));
        assert_eq!(lib.get(resampled_id).expect("copy").source_path, None);
        assert!(lib.get(saved_id).expect("source").source_path.is_some());
    }

    #[test]
    fn rebuild_restores_the_playable_asset_of_every_clip() {
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();
        let clip = EditableAnimationClip::new(0, "kept".to_string());
        let source_id = clip_library_register_and_activate(&mut lib, &mut assets, clip);
        assets.clear();

        clip_library_rebuild_playable_assets(&mut lib, &mut assets);

        let asset_id = lib.get_asset_id_for_source(source_id).expect("asset id");
        assert!(assets.animation_clips.contains_key(&asset_id));
        assert_eq!(assets.animation_clips.len(), 1);
    }

    #[test]
    fn sync_releases_the_playable_asset_of_a_removed_clip() {
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();
        let clip = EditableAnimationClip::new(0, "removed".to_string());
        let source_id = clip_library_register_and_activate(&mut lib, &mut assets, clip);

        lib.remove(source_id);
        clip_library_sync_playable_assets(&mut lib, &mut assets);

        assert!(assets.animation_clips.is_empty());
    }

    #[test]
    fn sync_gives_a_restored_clip_a_playable_asset() {
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();
        let clip = EditableAnimationClip::new(0, "restored".to_string());
        let source_id = clip_library_register_and_activate(&mut lib, &mut assets, clip);
        let removed = lib.remove(source_id).expect("removed clip");

        lib.restore(source_id, removed);
        let synced_ids = clip_library_sync_playable_assets(&mut lib, &mut assets);

        let asset_id = lib.get_asset_id_for_source(source_id).expect("asset id");
        assert_eq!(synced_ids, vec![source_id]);
        assert!(assets.animation_clips.contains_key(&asset_id));
        assert_eq!(assets.animation_clips.len(), 1);
    }

    #[test]
    fn sync_dirty_returns_all_synced_ids() {
        let mut lib = ClipLibrary::default();
        let mut assets = AssetStorage::default();

        let clip_a = EditableAnimationClip::new(0, "a".to_string());
        let source_id_a = clip_library_register_and_activate(&mut lib, &mut assets, clip_a);

        let clip_b = EditableAnimationClip::new(0, "b".to_string());
        let source_id_b = clip_library_register_and_activate(&mut lib, &mut assets, clip_b);

        lib.dirty_sources.insert(source_id_a);
        lib.dirty_sources.insert(source_id_b);

        let returned = clip_library_sync_playable_assets(&mut lib, &mut assets);

        assert_eq!(returned.len(), 2);
        assert!(returned.contains(&source_id_a));
        assert!(returned.contains(&source_id_b));
    }
}
