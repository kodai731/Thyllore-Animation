use std::collections::{HashMap, HashSet};
use std::path::Path;

use thyllore_anim_core::BoneId;
use thyllore_avatar_core::humanoid::components::avatar_rig::AvatarRig;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::systems::mapping_io::load_or_infer_rig;

use crate::animation::Skeleton;
use crate::ecs::resource::HumanoidRig;
use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
use crate::ecs::systems::role_clip_systems::build_role_retarget_context;

pub fn build_humanoid_rig(model_path: &Path, skeleton: &Skeleton) -> Option<HumanoidRig> {
    let bones = skeleton_to_bone_inputs(skeleton);
    let (avatar_rig, _) = match load_or_infer_rig(model_path, &bones) {
        Ok(result) => result,
        Err(error) => {
            log_warn!(
                "Failed to load or infer humanoid rig {}: {}",
                model_path.display(),
                error
            );
            return None;
        }
    };

    let confirmed = matches!(avatar_rig, AvatarRig::Confirmed(_));

    let mapping = match avatar_rig {
        AvatarRig::NotHumanoid => return None,
        AvatarRig::Inferred(mapping) => mapping,
        AvatarRig::Confirmed(mapping) => mapping,
    };

    let context = match build_role_retarget_context(skeleton, &mapping) {
        Ok(ctx) => ctx,
        Err(error) => {
            log_warn!(
                "Failed to build retarget context for {}: {}",
                model_path.display(),
                error
            );
            return None;
        }
    };

    let mut track_bones: HashMap<String, BoneId> = HashMap::new();
    let mut track_names: HashMap<BoneId, String> = HashMap::new();

    for (i, bone) in skeleton.bones.iter().enumerate() {
        let bone_id = i as BoneId;
        let name = if let Some((role, _)) = mapping.by_role.iter().find(|(_, &b)| b == i) {
            role.unity_name().to_string()
        } else {
            let name = &bone.name;
            if HumanoidRole::from_unity_name(name.as_str()).is_some() {
                log_warn!(
                    "Bone '{}' is not in mapping but matches role name, renamed to '{}#bone'",
                    name,
                    name
                );
                format!("{}#bone", name)
            } else {
                name.clone()
            }
        };
        track_bones.insert(name.clone(), bone_id);
        track_names.insert(bone_id, name);
    }

    Some(HumanoidRig {
        confirmed,
        mapping,
        context,
        track_bones,
        track_names,
    })
}

use thyllore_anim_core::editable::systems::clip_ops::clip_remap_bone_ids;

use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::asset::AssetStorage;
use crate::ecs::resource::{ClipLibrary, HumanoidRigState};
use crate::ecs::systems::avatar_setup_systems::{find_first_skeleton, find_model_path};
use crate::ecs::world::World;

pub fn sync_humanoid_rig(world: &mut World, assets: &AssetStorage) {
    let Some(model_path) = find_model_path(world) else {
        return;
    };
    let is_synced = world
        .get_resource::<HumanoidRigState>()
        .is_none_or(|state| state.source_model_path == model_path);
    if is_synced {
        return;
    }
    let Some(skeleton) = find_first_skeleton(assets) else {
        return;
    };

    let rig = build_humanoid_rig(Path::new(&model_path), skeleton);

    {
        let mut state = world.resource_mut::<HumanoidRigState>();
        state.rig = rig;
        state.source_model_path = model_path;
        state.revision += 1;
    }

    remap_user_clips_to_model(world, assets);
}

fn remap_user_clips_to_model(world: &mut World, assets: &AssetStorage) {
    let Some(table) = engine_bone_name_to_id(world, assets) else {
        return;
    };
    let Some(mut library) = world.get_resource_mut::<ClipLibrary>() else {
        return;
    };

    let user_clip_ids: Vec<SourceClipId> = library
        .source_clips
        .keys()
        .filter(|id| !library.model_clip_ids.contains(id))
        .copied()
        .collect();

    for id in user_clip_ids {
        let Some(clip) = library.get_mut(id) else {
            continue;
        };

        let names_before = collect_track_names(clip);
        clip_remap_bone_ids(clip, &table);
        let names_after = collect_track_names(clip);

        let dropped: Vec<&String> = names_before.difference(&names_after).collect();
        if !dropped.is_empty() {
            log_warn!(
                "clip '{}' dropped tracks not on this model: {:?}",
                clip.name,
                dropped
            );
        }

        library.mark_dirty(id);
    }
}

fn collect_track_names(clip: &EditableAnimationClip) -> HashSet<String> {
    clip.tracks
        .values()
        .map(|track| track.bone_name.clone())
        .collect()
}

pub fn invalidate_humanoid_rig(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<HumanoidRigState>() {
        state.source_model_path.clear();
    }
}

pub fn engine_bone_name_to_id(
    world: &World,
    assets: &AssetStorage,
) -> Option<HashMap<String, BoneId>> {
    if let Some(state) = world.get_resource::<HumanoidRigState>() {
        if let Some(ref rig) = state.rig {
            return Some(rig.track_bones.clone());
        }
    }
    find_first_skeleton(assets).map(|skeleton| skeleton.bone_name_to_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::SourceClip;
    use crate::asset::SkeletonAsset;
    use crate::ecs::resource::ModelState;
    use thyllore_avatar_core::humanoid::systems::mapping_io::save_not_humanoid;

    fn load_skeleton(fbx_path: &Path) -> Skeleton {
        let load_result = thyllore_importer_core::fbx::loader::load_fbx_to_graphics_resources(
            fbx_path.to_str().unwrap(),
        );
        let (fbx_result, _) = load_result.expect("Failed to load FBX");
        fbx_result
            .animation_system
            .skeletons
            .first()
            .expect("No skeleton in loaded model")
            .clone()
    }

    #[test]
    fn test_humanoid_builds_a_confirmed_rig_with_standard_names() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let skeleton = load_skeleton(&fbx_path);

        let rig =
            build_humanoid_rig(&fbx_path, &skeleton).expect("build_humanoid_rig returned None");

        assert!(rig.confirmed, "expected confirmed rig");
        assert_eq!(
            rig.track_bones.len(),
            skeleton.bones.len(),
            "expected track_bones to cover all {} bones",
            skeleton.bones.len()
        );

        let head_bone_id = *rig
            .track_bones
            .get("Head")
            .expect("missing Head in track_bones");
        let head_name = &skeleton.bones[head_bone_id as usize].name;
        assert_eq!(head_name, "Head", "Head bone name mismatch");

        assert!(
            rig.track_bones.contains_key("Skirt_Front_1"),
            "missing Skirt_Front_1 in track_bones"
        );

        for name in rig.track_bones.keys() {
            assert!(
                !name.contains("#bone"),
                "found #bone suffix in track name: {}",
                name
            );
        }
    }

    #[test]
    fn not_humanoid_sidecar_builds_no_rig() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, sidecar_path) = copy_test_humanoid_fixture(temp_dir.path());
        let skeleton = load_skeleton(&fbx_path);

        save_not_humanoid(&sidecar_path).expect("failed to save not-humanoid sidecar");

        let rig = build_humanoid_rig(&fbx_path, &skeleton);
        assert!(rig.is_none(), "expected None for not-humanoid sidecar");
    }

    #[test]
    fn sync_is_idempotent_revision_increases_only_once() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);

        sync_humanoid_rig(&mut world, &assets);
        let revision_after_first = world.resource::<HumanoidRigState>().revision;
        assert_eq!(
            revision_after_first, 1,
            "first sync should increment revision to 1"
        );

        sync_humanoid_rig(&mut world, &assets);
        let revision_after_second = world.resource::<HumanoidRigState>().revision;
        assert_eq!(
            revision_after_second, 1,
            "second sync should not increment revision (idempotent)"
        );
    }

    #[test]
    fn invalidate_then_sync_increments_revision_again() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);

        sync_humanoid_rig(&mut world, &assets);
        assert_eq!(world.resource::<HumanoidRigState>().revision, 1);

        invalidate_humanoid_rig(&mut world);
        assert!(
            world
                .resource::<HumanoidRigState>()
                .source_model_path
                .is_empty(),
            "invalidate should clear source_model_path"
        );

        sync_humanoid_rig(&mut world, &assets);
        let revision_after_rebuild = world.resource::<HumanoidRigState>().revision;
        assert_eq!(
            revision_after_rebuild, 2,
            "sync after invalidate should increment revision again"
        );
    }

    #[test]
    fn sync_remaps_user_clips_to_the_new_model() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, assets) = test_humanoid_world(&fbx_path);
        world.insert_resource(ClipLibrary::new());

        let source_id: SourceClipId = 1;
        {
            let mut library = world.resource_mut::<ClipLibrary>();
            let mut clip = EditableAnimationClip::new(source_id, "user".to_string());
            let track = clip.add_track(999, "Head".to_string());
            let curve = track.get_curve_mut(thyllore_anim_core::editable::PropertyType::RotationX);
            thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe(curve, 0.0, 0.5);
            library
                .source_clips
                .insert(source_id, SourceClip::new(source_id, clip));
        }

        sync_humanoid_rig(&mut world, &assets);

        let state = world.resource::<HumanoidRigState>();
        let rig = state.rig.as_ref().expect("rig should be Some");
        let expected_bone_id = *rig.track_bones.get("Head").expect("Head in track_bones");

        let library = world.resource::<ClipLibrary>();
        let clip = library.get(source_id).expect("clip should exist");
        let track = clip
            .get_track(expected_bone_id)
            .expect("track should be at remapped bone id");
        assert_eq!(track.bone_id, expected_bone_id);
    }

    #[test]
    fn not_humanoid_model_is_synced_once() {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("test skipped: cannot create temp dir: {}", e);
                return;
            }
        };

        let (fbx_path, sidecar_path) = copy_test_humanoid_fixture(temp_dir.path());
        save_not_humanoid(&sidecar_path).expect("failed to save not-humanoid sidecar");

        let (mut world, mut assets) = test_humanoid_world(&fbx_path);

        sync_humanoid_rig(&mut world, &assets);
        {
            let state = world.resource::<HumanoidRigState>();
            assert!(
                state.rig.is_none(),
                "rig should be None for not-humanoid model"
            );
            assert_eq!(state.revision, 1, "revision should be 1 after first sync");
        }

        sync_humanoid_rig(&mut world, &assets);
        {
            let state = world.resource::<HumanoidRigState>();
            assert!(
                state.rig.is_none(),
                "rig should still be None after second sync"
            );
            assert_eq!(state.revision, 1, "revision should stay 1 (idempotent)");
        }
    }
}

#[cfg(test)]
pub(crate) fn test_humanoid_world(fbx_path: &Path) -> (World, AssetStorage) {
    let skeleton = {
        let load_result = thyllore_importer_core::fbx::loader::load_fbx_to_graphics_resources(
            fbx_path.to_str().unwrap(),
        );
        let (fbx_result, _) = load_result.expect("Failed to load FBX");
        fbx_result
            .animation_system
            .skeletons
            .first()
            .expect("No skeleton in loaded model")
            .clone()
    };

    let model_path = fbx_path.to_string_lossy().to_string();
    let mut assets = AssetStorage::new();
    assets.add_skeleton(crate::asset::SkeletonAsset {
        id: 0,
        skeleton_id: 0,
        skeleton,
    });

    let mut world = World::new();
    world.insert_resource(HumanoidRigState::default());
    world.insert_resource(crate::ecs::resource::ModelState {
        model_path,
        ..Default::default()
    });

    (world, assets)
}

#[cfg(test)]
pub(crate) fn copy_test_humanoid_fixture(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/test_humanoid");
    let fbx_source = source_dir.join("test_humanoid.fbx");
    let sidecar_source = source_dir.join("test_humanoid.humanoid.ron");

    if !fbx_source.exists() || !sidecar_source.exists() {
        panic!(
            "test fixture not found at {:?} (fbx or sidecar missing)",
            source_dir
        );
    }

    let fbx_dest = dir.join("test_humanoid.fbx");
    let sidecar_dest = dir.join("test_humanoid.humanoid.ron");

    std::fs::copy(&fbx_source, &fbx_dest).unwrap();
    std::fs::copy(&sidecar_source, &sidecar_dest).unwrap();

    (fbx_dest, sidecar_dest)
}
