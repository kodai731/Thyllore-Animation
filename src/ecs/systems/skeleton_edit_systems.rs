use cgmath::{Vector3, Vector4};
use thyllore_model_core::{BoneRemap, SkinData};

use crate::animation::{BoneId, ConstraintType, Skeleton, SkeletonId};
use crate::asset::AssetStorage;
use crate::ecs::component::{Constrained, ConstraintSet, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{
    AvatarSetupState, BoneGizmoData, BonePoseOverride, BoneSelectionState, ClipLibrary,
    CurveEditorState, CurveEditorTarget, EditHistory, FbxModelCache, HierarchyState,
    SkeletonSnapshot, SkinSnapshot, SpringBoneState, TimelineState, TransformGizmoData,
};
use crate::ecs::storage::Component;
use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
use crate::ecs::systems::humanoid_rig_systems::invalidate_humanoid_rig;
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::world::{Entity, World};
use crate::exporter::fbx_exporter::clusters_from_skin_data;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

fn edited_skeleton_mut(assets: &mut AssetStorage) -> anyhow::Result<(SkeletonId, &mut Skeleton)> {
    assets
        .skeletons
        .values_mut()
        .next()
        .map(|asset| (asset.skeleton_id, &mut asset.skeleton))
        .ok_or_else(|| anyhow::anyhow!("no skeleton is loaded"))
}

fn resolve_bone(skeleton: &Skeleton, name: &str) -> anyhow::Result<BoneId> {
    skeleton
        .bone_name_to_id
        .get(name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("bone \"{name}\" not found"))
}

pub fn add_skeleton_bone(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
    parent_name: &str,
    name: &str,
    local_offset: Vector3<f32>,
) -> anyhow::Result<BoneId> {
    let before = capture_skeleton_snapshot(world, assets, graphics);

    let (_, skeleton) = edited_skeleton_mut(assets)?;
    let parent = resolve_bone(skeleton, parent_name)?;
    let bone_id = skeleton.add_child_bone(parent, name, local_offset)?;

    refresh_after_skeleton_edit(world, assets, graphics);
    record_skeleton_edit(world, assets, graphics, before, "Add bone");
    msg_info!("Added bone \"{}\" under \"{}\"", name, parent_name);
    Ok(bone_id)
}

pub fn set_skeleton_bone_offset(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
    bone_name: &str,
    local_offset: Vector3<f32>,
) -> anyhow::Result<()> {
    let (_, skeleton) = edited_skeleton_mut(assets)?;
    let bone_id = resolve_bone(skeleton, bone_name)?;
    skeleton.set_bone_local_offset(bone_id, local_offset)?;

    refresh_after_skeleton_edit(world, assets, graphics);
    Ok(())
}

pub fn remove_skeleton_bone(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    bone_name: &str,
) -> anyhow::Result<()> {
    let before = capture_skeleton_snapshot(world, assets, graphics);

    let (_, skeleton) = edited_skeleton_mut(assets)?;
    let bone_id = resolve_bone(skeleton, bone_name)?;
    let removed_count = remove_bone_subtree(world, assets, graphics, bone_id)?;

    refresh_after_skeleton_edit(world, assets, graphics);
    record_skeleton_edit(world, assets, graphics, before, "Remove bone");
    msg_info!(
        "Removed bone \"{}\" and {} descendants",
        bone_name,
        removed_count - 1
    );
    Ok(())
}

/// Removes a bone with its descendants, folds their skin weights into the parent and renumbers every
/// bone reference in the world; returns the number of bones removed. The caller refreshes and records.
pub fn remove_bone_subtree(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    bone_id: BoneId,
) -> anyhow::Result<usize> {
    let (skeleton_id, skeleton) = edited_skeleton_mut(assets)?;
    let removed = skeleton.remove_bone(bone_id)?;

    for mesh in graphics.meshes.iter_mut() {
        if let Some(skin) = mesh
            .skin_data
            .as_mut()
            .filter(|s| s.skeleton_id == skeleton_id)
        {
            merge_removed_bone_weights(skin, &removed.removed, removed.parent, &removed.remap);
        }
    }
    remap_bone_references(world, &removed.remap);
    Ok(removed.removed.len())
}

/// The state a skeleton edit can touch, or `None` when no skeleton is loaded.
pub fn capture_skeleton_snapshot(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Option<SkeletonSnapshot> {
    let skeleton_asset = assets.skeletons.values().next()?;
    let skeleton_id = skeleton_asset.skeleton_id;

    let skins = graphics
        .meshes
        .iter()
        .enumerate()
        .filter_map(|(mesh_index, mesh)| {
            let skin = mesh
                .skin_data
                .as_ref()
                .filter(|s| s.skeleton_id == skeleton_id)?;
            Some(SkinSnapshot {
                mesh_index,
                bone_indices: skin.bone_indices.clone(),
                bone_weights: skin.bone_weights.clone(),
            })
        })
        .collect();
    let clip_tracks = world
        .resource::<ClipLibrary>()
        .source_clips
        .iter()
        .map(|(id, source)| (*id, source.editable_clip.tracks.clone()))
        .collect();

    Some(SkeletonSnapshot {
        skeleton: skeleton_asset.skeleton.clone(),
        skins,
        spring_setups: world
            .iter_components::<SpringBoneSetup>()
            .map(|(entity, setup)| (entity, setup.clone()))
            .collect(),
        constraint_sets: world
            .iter_components::<ConstraintSet>()
            .map(|(entity, set)| (entity, set.clone()))
            .collect(),
        clip_tracks,
    })
}

pub(crate) fn record_skeleton_edit(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    before: Option<SkeletonSnapshot>,
    description: &'static str,
) {
    let (Some(before), Some(after)) = (before, capture_skeleton_snapshot(world, assets, graphics))
    else {
        return;
    };
    if let Some(mut history) = world.get_resource_mut::<EditHistory>() {
        history.push_skeleton_edit(before, after, description);
    }
}

/// Writes a snapshot back over the skeleton, its skin weights and every bone reference, then refreshes.
pub fn restore_skeleton_snapshot(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    snapshot: SkeletonSnapshot,
) {
    let Some(skeleton_asset) = assets.skeletons.values_mut().next() else {
        msg_error!("Cannot restore a skeleton snapshot: no skeleton is loaded");
        return;
    };
    let skeleton_id = skeleton_asset.skeleton_id;
    let bone_count = snapshot.skeleton.bone_count();
    skeleton_asset.skeleton = snapshot.skeleton;

    for skin_snapshot in snapshot.skins {
        let Some(skin) = graphics
            .meshes
            .get_mut(skin_snapshot.mesh_index)
            .and_then(|mesh| mesh.skin_data.as_mut())
            .filter(|s| s.skeleton_id == skeleton_id)
        else {
            msg_error!(
                "Skeleton snapshot names graphics mesh {} which has no skin for this skeleton",
                skin_snapshot.mesh_index
            );
            continue;
        };
        restore_skin_weights(skin, &skin_snapshot);
    }
    restore_owned_components::<SpringBoneSetup, WithSpringBone>(world, snapshot.spring_setups);
    restore_owned_components::<ConstraintSet, Constrained>(world, snapshot.constraint_sets);
    {
        let mut clips = world.resource_mut::<ClipLibrary>();
        for (id, tracks) in snapshot.clip_tracks {
            if let Some(source) = clips.source_clips.get_mut(&id) {
                source.editable_clip.tracks = tracks;
            }
        }
    }
    drop_bone_state_beyond(world, bone_count as BoneId);
    invalidate_humanoid_rig(world);

    refresh_after_skeleton_edit(world, assets, graphics);
}

/// Makes the set of entities carrying `T` (and its marker `M`) equal to the snapshot's: entities the
/// snapshot does not name lose the component, since an edit may have created it.
fn restore_owned_components<T: Component, M: Component + Default>(
    world: &mut World,
    entries: Vec<(Entity, T)>,
) {
    let stale: Vec<Entity> = world
        .iter_components::<T>()
        .map(|(entity, _)| entity)
        .filter(|entity| !entries.iter().any(|(kept, _)| kept == entity))
        .collect();
    for entity in stale {
        world.remove_component::<T>(entity);
        world.remove_component::<M>(entity);
    }

    for (entity, component) in entries {
        world.insert_component(entity, component);
        world.insert_component(entity, M::default());
    }
}

pub fn restore_skin_weights(skin: &mut SkinData, snapshot: &SkinSnapshot) {
    if snapshot.bone_indices.len() != skin.bone_indices.len() {
        msg_error!(
            "Skeleton snapshot has {} skinned vertices for mesh {}, the mesh has {}",
            snapshot.bone_indices.len(),
            snapshot.mesh_index,
            skin.bone_indices.len()
        );
        return;
    }
    skin.bone_indices.clone_from(&snapshot.bone_indices);
    skin.bone_weights.clone_from(&snapshot.bone_weights);
}

/// Editor state that names a bone the restored skeleton does not have is cleared.
fn drop_bone_state_beyond(world: &mut World, bone_count: BoneId) {
    let keep = |bone: BoneId| bone < bone_count;
    if let Some(mut hierarchy) = world.get_resource_mut::<HierarchyState>() {
        hierarchy.selected_bone_id = hierarchy.selected_bone_id.filter(|&b| keep(b));
        hierarchy.expanded_bone_ids.retain(|&b| keep(b));
    }
    if let Some(mut selection) = world.get_resource_mut::<BoneSelectionState>() {
        selection
            .selected_bone_indices
            .retain(|&b| keep(b as BoneId));
        selection.active_bone_index = selection.active_bone_index.filter(|&b| keep(b as BoneId));
    }
    if let Some(mut timeline) = world.get_resource_mut::<TimelineState>() {
        timeline.expanded_tracks.retain(|&b| keep(b));
        timeline.baked_bone_ids.retain(|&b| keep(b));
    }
    if let Some(mut spring) = world.get_resource_mut::<SpringBoneState>() {
        spring.baked_bone_ids.retain(|&b| keep(b));
    }
    if let Some(mut overrides) = world.get_resource_mut::<BonePoseOverride>() {
        overrides.overrides.retain(|&b, _| keep(b));
    }
    if let Some(mut curve) = world.get_resource_mut::<CurveEditorState>() {
        if let Some(CurveEditorTarget::Bone(bone)) = curve.selected_target {
            if !keep(bone) {
                curve.selected_target = None;
            }
        }
    }
    if let Some(mut gizmo) = world.get_resource_mut::<TransformGizmoData>() {
        gizmo.target_bone_id = gizmo.target_bone_id.filter(|&b| keep(b));
    }
}

/// Gives the removed subtree's weight to the parent it hung from and renumbers the surviving bones.
pub fn merge_removed_bone_weights(
    skin: &mut SkinData,
    removed: &[BoneId],
    parent: Option<BoneId>,
    remap: &BoneRemap,
) {
    for (indices, weights) in skin.bone_indices.iter_mut().zip(&mut skin.bone_weights) {
        let mut influences: Vec<(BoneId, f32)> = (0..4)
            .map(|slot| (indices[slot], weights[slot]))
            .filter(|(_, weight)| *weight > 0.0)
            .filter_map(|(bone, weight)| match (removed.contains(&bone), parent) {
                (false, _) => remap.map(bone).map(|new| (new, weight)),
                (true, Some(parent)) => Some((parent, weight)),
                (true, None) => None,
            })
            .collect();
        merge_duplicate_influences(&mut influences);
        normalize_influences(&mut influences);

        *indices = Vector4::new(0, 0, 0, 0);
        *weights = Vector4::new(0.0, 0.0, 0.0, 0.0);
        for (slot, (bone, weight)) in influences.into_iter().take(4).enumerate() {
            indices[slot] = bone;
            weights[slot] = weight;
        }
    }
}

fn merge_duplicate_influences(influences: &mut Vec<(BoneId, f32)>) {
    influences.sort_by_key(|(bone, _)| *bone);
    influences.dedup_by(|later, earlier| {
        if later.0 == earlier.0 {
            earlier.1 += later.1;
            true
        } else {
            false
        }
    });
    influences.sort_by(|a, b| b.1.total_cmp(&a.1));
}

fn normalize_influences(influences: &mut [(BoneId, f32)]) {
    let total: f32 = influences.iter().map(|(_, weight)| weight).sum();
    if total > 0.0 {
        for influence in influences.iter_mut() {
            influence.1 /= total;
        }
    }
}

fn remap_bone_references(world: &mut World, remap: &BoneRemap) {
    for (_, setup) in world.iter_components_mut::<SpringBoneSetup>() {
        remap_spring_setup(setup, remap);
    }
    for (_, constraints) in world.iter_components_mut::<ConstraintSet>() {
        constraints
            .constraints
            .retain_mut(|entry| remap_constraint(&mut entry.constraint, remap));
    }

    let mut clips = world.resource_mut::<ClipLibrary>();
    for source in clips.source_clips.values_mut() {
        let tracks = std::mem::take(&mut source.editable_clip.tracks);
        source.editable_clip.tracks = tracks
            .into_iter()
            .filter_map(|(bone, mut track)| {
                let new = remap.map(bone)?;
                track.bone_id = new;
                Some((new, track))
            })
            .collect();
    }
    drop(clips);

    remap_editor_bone_state(world, remap);
    invalidate_humanoid_rig(world);
}

fn remap_spring_setup(setup: &mut SpringBoneSetup, remap: &BoneRemap) {
    for chain in &mut setup.chains {
        chain
            .joints
            .retain_mut(|joint| match remap.map(joint.bone_id) {
                Some(new) => {
                    joint.bone_id = new;
                    true
                }
                None => false,
            });
        chain.center_bone_id = chain.center_bone_id.and_then(|bone| remap.map(bone));
    }
    setup.chains.retain(|chain| !chain.joints.is_empty());
    setup
        .colliders
        .retain_mut(|collider| match remap.map(collider.bone_id) {
            Some(new) => {
                collider.bone_id = new;
                true
            }
            None => false,
        });
    let collider_ids: Vec<_> = setup.colliders.iter().map(|c| c.id).collect();
    for group in &mut setup.collider_groups {
        group.collider_ids.retain(|id| collider_ids.contains(id));
    }
}

/// Renumbers every bone a constraint names; a constraint that loses a bone is dropped.
fn remap_constraint(constraint: &mut ConstraintType, remap: &BoneRemap) -> bool {
    let map = |bone: &mut BoneId| -> bool {
        match remap.map(*bone) {
            Some(new) => {
                *bone = new;
                true
            }
            None => false,
        }
    };
    let map_optional = |bone: &mut Option<BoneId>| -> bool {
        match bone {
            Some(id) => map(id),
            None => true,
        }
    };
    match constraint {
        ConstraintType::Ik(data) => {
            map(&mut data.target_bone)
                && map(&mut data.effector_bone)
                && map_optional(&mut data.pole_target)
        }
        ConstraintType::Aim(data) => {
            map(&mut data.source_bone)
                && map(&mut data.target_bone)
                && map_optional(&mut data.up_target)
        }
        ConstraintType::Parent(data) => {
            map(&mut data.constrained_bone)
                && data.sources.iter_mut().all(|(source, _)| map(source))
        }
        ConstraintType::Position(data) => {
            map(&mut data.constrained_bone) && map(&mut data.target_bone)
        }
        ConstraintType::Rotation(data) => {
            map(&mut data.constrained_bone) && map(&mut data.target_bone)
        }
        ConstraintType::Scale(data) => {
            map(&mut data.constrained_bone) && map(&mut data.target_bone)
        }
    }
}

fn remap_editor_bone_state(world: &mut World, remap: &BoneRemap) {
    if let Some(mut hierarchy) = world.get_resource_mut::<HierarchyState>() {
        hierarchy.selected_bone_id = hierarchy.selected_bone_id.and_then(|b| remap.map(b));
        hierarchy.expanded_bone_ids = hierarchy
            .expanded_bone_ids
            .iter()
            .filter_map(|&b| remap.map(b))
            .collect();
    }
    if let Some(mut selection) = world.get_resource_mut::<BoneSelectionState>() {
        selection.selected_bone_indices = selection
            .selected_bone_indices
            .iter()
            .filter_map(|&b| remap.map(b as BoneId).map(|n| n as usize))
            .collect();
        selection.active_bone_index = selection
            .active_bone_index
            .and_then(|b| remap.map(b as BoneId).map(|n| n as usize));
    }
    if let Some(mut timeline) = world.get_resource_mut::<TimelineState>() {
        timeline.expanded_tracks = timeline
            .expanded_tracks
            .iter()
            .filter_map(|&b| remap.map(b))
            .collect();
        timeline.baked_bone_ids = timeline
            .baked_bone_ids
            .iter()
            .filter_map(|&b| remap.map(b))
            .collect();
    }
    if let Some(mut spring) = world.get_resource_mut::<SpringBoneState>() {
        spring.baked_bone_ids = spring
            .baked_bone_ids
            .iter()
            .filter_map(|&b| remap.map(b))
            .collect();
    }
    if let Some(mut overrides) = world.get_resource_mut::<BonePoseOverride>() {
        let remapped = std::mem::take(&mut overrides.overrides);
        overrides.overrides = remapped
            .into_iter()
            .filter_map(|(bone, pose)| remap.map(bone).map(|n| (n, pose)))
            .collect();
    }
    if let Some(mut curve) = world.get_resource_mut::<CurveEditorState>() {
        if let Some(CurveEditorTarget::Bone(bone)) = curve.selected_target {
            curve.selected_target = remap.map(bone).map(CurveEditorTarget::Bone);
        }
    }
    if let Some(mut gizmo) = world.get_resource_mut::<TransformGizmoData>() {
        gizmo.target_bone_id = gizmo.target_bone_id.and_then(|b| remap.map(b));
    }
}

/// Rebuilds everything derived from the skeleton: spring state, bone gizmo, setup bone list, FBX clusters.
pub fn refresh_after_skeleton_edit(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    reset_spring_bone_state(world);

    let Some(skeleton_asset) = assets.skeletons.values().next() else {
        return;
    };
    let skeleton = &skeleton_asset.skeleton;
    let rest_globals = crate::ecs::compute_pose_global_transforms(
        skeleton,
        &crate::ecs::create_pose_from_rest(skeleton),
    );
    if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
        bone_gizmo.cached_skeleton_id = Some(skeleton_asset.skeleton_id);
        bone_gizmo.bone_local_offsets =
            crate::ecs::systems::bone_gizmo_systems::compute_bone_local_offsets(
                skeleton,
                &rest_globals,
            );
        bone_gizmo.cached_global_transforms = rest_globals;
    }
    world.resource_mut::<AvatarSetupState>().bones = skeleton_to_bone_inputs(skeleton);

    sync_fbx_clusters(world, assets, graphics, skeleton_asset.skeleton_id);
}

fn sync_fbx_clusters(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    skeleton_id: SkeletonId,
) {
    let Some(skeleton) = assets.get_skeleton_by_skeleton_id(skeleton_id) else {
        return;
    };
    let mut fbx_cache = world.resource_mut::<FbxModelCache>();

    for (mesh_index, mesh) in graphics.meshes.iter().enumerate() {
        let Some(skin) = mesh
            .skin_data
            .as_ref()
            .filter(|s| s.skeleton_id == skeleton_id)
        else {
            continue;
        };
        let Some(fbx_data_index) = fbx_cache.fbx_data_index(mesh_index) else {
            msg_error!(
                "graphics mesh {} is not part of the loaded FBX model; its skin weights will not reach the FBX export",
                mesh_index
            );
            continue;
        };
        let clusters = clusters_from_skin_data(skin, skeleton);
        if let Some(fbx_model) = fbx_cache.fbx_model_mut() {
            fbx_model.fbx_data[fbx_data_index].clusters = clusters;
        }
    }
}

#[cfg(test)]
mod tests;
