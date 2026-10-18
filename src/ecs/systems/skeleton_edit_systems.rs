use cgmath::{Vector3, Vector4};
use thyllore_model_core::{BoneRemap, SkinData};

use crate::animation::{BoneId, ConstraintType, Skeleton, SkeletonId};
use crate::asset::AssetStorage;
use crate::ecs::component::{ConstraintSet, SpringBoneSetup};
use crate::ecs::resource::{
    AvatarSetupState, BoneGizmoData, BonePoseOverride, BoneSelectionState, ClipLibrary,
    CurveEditorState, CurveEditorTarget, EditHistory, FbxModelCache, HierarchyState,
    SkeletonSnapshot, SkinSnapshot, SpringBoneState, TimelineState, TransformGizmoData,
};
use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
use crate::ecs::systems::humanoid_rig_systems::invalidate_humanoid_rig;
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::world::World;
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

    let (skeleton_id, skeleton) = edited_skeleton_mut(assets)?;
    let bone_id = resolve_bone(skeleton, bone_name)?;
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

    refresh_after_skeleton_edit(world, assets, graphics);
    record_skeleton_edit(world, assets, graphics, before, "Remove bone");
    msg_info!(
        "Removed bone \"{}\" and {} descendants",
        bone_name,
        removed.removed.len() - 1
    );
    Ok(())
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

fn record_skeleton_edit(
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
    for (entity, setup) in snapshot.spring_setups {
        if let Some(current) = world.get_component_mut::<SpringBoneSetup>(entity) {
            *current = setup;
        }
    }
    for (entity, set) in snapshot.constraint_sets {
        if let Some(current) = world.get_component_mut::<ConstraintSet>(entity) {
            *current = set;
        }
    }
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
mod tests {
    use super::*;
    use cgmath::Matrix4;

    fn build_skin(influences: &[[(u32, f32); 4]]) -> SkinData {
        SkinData {
            skeleton_id: 0,
            bone_indices: influences
                .iter()
                .map(|v| Vector4::new(v[0].0, v[1].0, v[2].0, v[3].0))
                .collect(),
            bone_weights: influences
                .iter()
                .map(|v| Vector4::new(v[0].1, v[1].1, v[2].1, v[3].1))
                .collect(),
            base_positions: vec![Vector3::new(0.0, 0.0, 0.0); influences.len()],
            base_normals: vec![Vector3::new(0.0, 1.0, 0.0); influences.len()],
        }
    }

    fn remove_middle_of_three() -> (Vec<BoneId>, BoneRemap, Option<BoneId>) {
        let mut skeleton = Skeleton::new("s");
        let root = skeleton.add_bone("root", None);
        let middle = skeleton.add_bone("middle", Some(root));
        skeleton.add_bone("tip", Some(middle));
        skeleton.add_bone("side", Some(root));
        for bone in &mut skeleton.bones {
            bone.inverse_bind_pose = Matrix4::from_scale(1.0);
        }
        let removed = skeleton.remove_bone(middle).unwrap();
        (removed.removed, removed.remap, removed.parent)
    }

    #[test]
    fn removed_subtree_weight_moves_to_its_parent_and_merges() {
        let (removed, remap, parent) = remove_middle_of_three();
        let mut skin = build_skin(&[
            [(0, 0.5), (1, 0.3), (2, 0.2), (0, 0.0)],
            [(2, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)],
            [(3, 0.5), (1, 0.5), (0, 0.0), (0, 0.0)],
        ]);

        merge_removed_bone_weights(&mut skin, &removed, parent, &remap);

        assert_eq!(skin.bone_indices[0], Vector4::new(0, 0, 0, 0));
        assert!((skin.bone_weights[0].x - 1.0).abs() < 1e-6);
        assert_eq!(skin.bone_indices[1], Vector4::new(0, 0, 0, 0));
        assert!((skin.bone_weights[1].x - 1.0).abs() < 1e-6);
        assert_eq!(skin.bone_indices[2], Vector4::new(0, 1, 0, 0));
        assert!((skin.bone_weights[2].x - 0.5).abs() < 1e-6);
        assert!((skin.bone_weights[2].y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn removed_root_weight_is_dropped_and_the_rest_renormalized() {
        let mut skeleton = Skeleton::new("s");
        let root = skeleton.add_bone("root", None);
        skeleton.add_bone("child", Some(root));
        skeleton.add_bone("other", None);
        for bone in &mut skeleton.bones {
            bone.inverse_bind_pose = Matrix4::from_scale(1.0);
        }
        let removed = skeleton.remove_bone(root).unwrap();
        let mut skin = build_skin(&[[(0, 0.25), (1, 0.25), (2, 0.5), (0, 0.0)]]);

        merge_removed_bone_weights(&mut skin, &removed.removed, removed.parent, &removed.remap);

        assert_eq!(skin.bone_indices[0], Vector4::new(0, 0, 0, 0));
        assert!((skin.bone_weights[0].x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn restore_skin_weights_writes_the_snapshot_back_when_sizes_match() {
        let mut skin = build_skin(&[[(0, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)]]);
        let snapshot = SkinSnapshot {
            mesh_index: 0,
            bone_indices: vec![Vector4::new(2, 1, 0, 0)],
            bone_weights: vec![Vector4::new(0.5, 0.5, 0.0, 0.0)],
        };

        restore_skin_weights(&mut skin, &snapshot);

        assert_eq!(skin.bone_indices[0], Vector4::new(2, 1, 0, 0));
        assert_eq!(skin.bone_weights[0], Vector4::new(0.5, 0.5, 0.0, 0.0));
    }

    #[test]
    fn constraints_that_lose_a_bone_are_dropped_and_others_renumbered() {
        let (_, remap, _) = remove_middle_of_three();
        let mut kept = ConstraintType::Position(crate::animation::PositionConstraintData {
            constrained_bone: 3,
            target_bone: 0,
            ..Default::default()
        });
        let mut dropped = ConstraintType::Aim(crate::animation::AimConstraintData {
            source_bone: 1,
            target_bone: 2,
            ..Default::default()
        });

        assert!(remap_constraint(&mut kept, &remap));
        assert!(!remap_constraint(&mut dropped, &remap));
        match kept {
            ConstraintType::Position(data) => assert_eq!(data.constrained_bone, 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn spring_chains_drop_removed_joints_and_empty_chains() {
        let (_, remap, _) = remove_middle_of_three();
        let mut setup = SpringBoneSetup::default();
        setup.chains.push(crate::ecs::component::SpringChain {
            joints: vec![
                crate::ecs::component::SpringJointParam {
                    bone_id: 1,
                    ..Default::default()
                },
                crate::ecs::component::SpringJointParam {
                    bone_id: 3,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        setup.chains.push(crate::ecs::component::SpringChain {
            joints: vec![crate::ecs::component::SpringJointParam {
                bone_id: 1,
                ..Default::default()
            }],
            ..Default::default()
        });

        remap_spring_setup(&mut setup, &remap);

        assert_eq!(setup.chains.len(), 1);
        assert_eq!(setup.chains[0].joints.len(), 1);
        assert_eq!(setup.chains[0].joints[0].bone_id, 1);
    }
}
