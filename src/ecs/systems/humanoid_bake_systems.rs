use std::collections::HashSet;

use cgmath::{Quaternion, Vector3};
use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::systems::clip_convert::clip_to_animation;
use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::components::baked_motion::BakedMotion;
use thyllore_avatar_core::motion::components::retarget_context::RetargetContext;
use thyllore_avatar_core::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};
use thyllore_math_core::{continuous_euler, decompose, quaternion_to_euler_degrees};

use crate::animation::editable::SourceClipId;
use crate::animation::{BoneId, Skeleton};
use crate::asset::{AnimationClipAsset, AssetId, AssetStorage};
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::resource::{
    AnimationType, BakedHumanoidClip, BakedHumanoidClips, ClipLibrary, ClipPreview, HumanoidRig,
    HumanoidRigState, TimelineState,
};
use crate::ecs::systems::avatar_setup_systems::{
    compute_bone_global_transform, find_first_skeleton, skeleton_to_bone_inputs,
};
use crate::ecs::systems::clip_schedule_systems::find_preview_owner;
use crate::ecs::world::{Entity, World};

pub fn request_bake_scan(world: &mut World) {
    if let Some(mut baked_clips) = world.get_resource_mut::<BakedHumanoidClips>() {
        baked_clips.scan_requested = true;
    }
}

pub fn skeleton_to_retarget_skeleton(skeleton: &Skeleton) -> RetargetSkeleton {
    let bones = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(bone_index, bone)| {
            let world_matrix = compute_bone_global_transform(skeleton, bone_index);
            RetargetBone {
                parent: bone.parent_id.map(|id| id as usize),
                world_position: world_matrix.w.truncate(),
                world_rotation: decompose(&world_matrix).1,
            }
        })
        .collect();
    RetargetSkeleton { bones }
}

pub fn baked_motion_to_clip(
    baked: &BakedMotion,
    skeleton: &Skeleton,
    hips_bone: usize,
    clip_name: &str,
) -> EditableAnimationClip {
    let mut clip = EditableAnimationClip::new(0, clip_name.to_string());

    for (&bone_index, rotations) in &baked.bone_rotations {
        let bone_name = skeleton.bones[bone_index].name.clone();
        let track = clip.add_track(bone_index as BoneId, bone_name);

        let mut prev_euler: Option<Vector3<f32>> = None;
        for (&time, quat) in baked.frame_times.iter().zip(rotations) {
            let continuous = continuous_euler(quaternion_to_euler_degrees(quat), prev_euler);
            curve_add_keyframe(&mut track.rotation_x, time, continuous.x);
            curve_add_keyframe(&mut track.rotation_y, time, continuous.y);
            curve_add_keyframe(&mut track.rotation_z, time, continuous.z);
            prev_euler = Some(continuous);
        }

        if bone_index == hips_bone {
            let hips = &skeleton.bones[hips_bone];
            let bind_translation = hips.local_transform.w.truncate();
            let parent_world_rotation_inv = match hips.parent_id {
                Some(parent_id) => {
                    decompose(&compute_bone_global_transform(skeleton, parent_id as usize))
                        .1
                        .conjugate()
                }
                None => Quaternion::new(1.0, 0.0, 0.0, 0.0),
            };

            for (&time, offset) in baked.frame_times.iter().zip(&baked.hips_offsets) {
                let translation = bind_translation + parent_world_rotation_inv * *offset;
                curve_add_keyframe(&mut track.translation_x, time, translation.x);
                curve_add_keyframe(&mut track.translation_y, time, translation.y);
                curve_add_keyframe(&mut track.translation_z, time, translation.z);
            }
        }
    }

    if let Some(last_time) = baked.frame_times.last() {
        clip.duration = *last_time;
    }

    clip
}

pub fn build_role_retarget_context(
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
) -> anyhow::Result<RetargetContext> {
    let bones = skeleton_to_bone_inputs(skeleton);
    let frame = thyllore_avatar_core::humanoid::systems::humanoid_frame::derive_humanoid_frame(
        mapping, &bones,
    )
    .ok_or_else(|| anyhow::anyhow!("failed to derive character frame"))?;

    let rest_pose =
        thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose(mapping, &bones);

    let retarget_skeleton = skeleton_to_retarget_skeleton(skeleton);
    let ctx = thyllore_avatar_core::motion::systems::retarget_pose::build_retarget_context(
        &retarget_skeleton,
        mapping,
        &frame,
        rest_pose,
    )
    .ok_or_else(|| anyhow::anyhow!("failed to build retarget context"))?;

    Ok(ctx)
}

pub fn bake_to_bone_clip(
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    rig: &HumanoidRig,
    fps: u32,
) -> anyhow::Result<EditableAnimationClip> {
    let baked =
        thyllore_avatar_core::motion::systems::bake::bake_humanoid_clip(&rig.context, clip, fps);
    let hips_bone = *rig
        .mapping
        .by_role
        .get(&HumanoidRole::Hips)
        .ok_or_else(|| anyhow::anyhow!("mapping has no Hips bone"))?;
    let mut baked_clip = baked_motion_to_clip(&baked, skeleton, hips_bone, &clip.name);

    for track in clip.tracks.values() {
        if baked_clip.tracks.contains_key(&track.bone_id) {
            continue;
        }
        if !rig
            .mapping
            .by_role
            .values()
            .any(|&v| v as BoneId == track.bone_id)
        {
            baked_clip.tracks.insert(track.bone_id, track.clone());
        }
    }

    Ok(baked_clip)
}

pub fn new_empty_clip(name: &str) -> EditableAnimationClip {
    let mut clip = EditableAnimationClip::new(0, name.to_string());
    clip.duration = 2.0;
    clip.min_duration = 2.0;
    clip
}

pub fn unresolved_clip_roles(
    clip: &EditableAnimationClip,
    mapping: Option<&HumanoidMapping>,
) -> Vec<HumanoidRole> {
    let mut seen = HashSet::new();
    for track in clip.tracks.values() {
        if !track.has_rotation_keyframes() && !track.has_translation_keyframes() {
            continue;
        }
        let Some(role) = HumanoidRole::from_unity_name(&track.bone_name) else {
            continue;
        };
        if !seen.insert(role) {
            continue;
        }
        if let Some(m) = mapping {
            if m.by_role.contains_key(&role) {
                seen.remove(&role);
                continue;
            }
        }
    }
    let roles: Vec<HumanoidRole> = HumanoidRole::ALL
        .iter()
        .filter(|r| seen.contains(r))
        .copied()
        .collect();
    roles
}

fn collect_needed_bake_keys(world: &World) -> HashSet<(SourceClipId, Entity)> {
    let clip_library = world.resource::<ClipLibrary>();
    let mut needed = HashSet::new();

    for (entity, schedule) in world.iter_components::<ClipSchedule>() {
        let is_skeletal = world
            .get_component::<AnimationMeta>(entity)
            .is_some_and(|meta| meta.animation_type == AnimationType::Skeletal);
        if !is_skeletal {
            continue;
        }
        for instance in &schedule.instances {
            let Some(_clip) = clip_library.get(instance.source_id) else {
                continue;
            };
            needed.insert((instance.source_id, entity));
        }
    }

    let timeline = world.resource::<TimelineState>();
    if timeline.preview == ClipPreview::Solo {
        if let Some(current_clip_id) = timeline.current_clip_id {
            let Some(_clip) = clip_library.get(current_clip_id) else {
                return needed;
            };
            if let Some(preview_owner) = find_preview_owner(world) {
                needed.insert((current_clip_id, preview_owner));
            }
        }
    }

    needed
}

fn collect_keys_to_bake(world: &World, fps: u32) -> Vec<(SourceClipId, Entity)> {
    let rig = world.resource::<HumanoidRigState>();
    if rig.rig.is_none() {
        return Vec::new();
    }

    let baked_clips = world.resource::<BakedHumanoidClips>();
    collect_needed_bake_keys(world)
        .into_iter()
        .filter(|key| !baked_clips.failed.contains(key))
        .filter(|key| {
            baked_clips
                .by_key
                .get(key)
                .is_none_or(|baked| baked.fps != fps)
        })
        .collect()
}

pub fn refresh_baked_humanoid_clips(world: &mut World, assets: &mut AssetStorage) {
    let mut baked_clips = world.resource_mut::<BakedHumanoidClips>();

    let rig_state = world.resource::<HumanoidRigState>();
    let current_revision = rig_state.revision;
    let has_rig = rig_state.rig.is_some();
    drop(rig_state);

    if !has_rig {
        if !baked_clips.by_key.is_empty() {
            let to_remove: Vec<AssetId> = baked_clips
                .by_key
                .values()
                .map(|entry| entry.asset_id)
                .collect();
            baked_clips.by_key.clear();
            baked_clips.failed.clear();
            for asset_id in to_remove {
                assets.animation_clips.remove(&asset_id);
            }
        }
        return;
    }

    if baked_clips.rig_revision != current_revision {
        let to_remove: Vec<AssetId> = baked_clips
            .by_key
            .values()
            .map(|entry| entry.asset_id)
            .collect();
        baked_clips.by_key.clear();
        baked_clips.failed.clear();
        baked_clips.rig_revision = current_revision;
        for asset_id in to_remove {
            assets.animation_clips.remove(&asset_id);
        }
    }

    let timeline = world.resource::<TimelineState>();
    let fps = timeline.snap_settings.frame_rate.round() as u32;
    let current_view = (timeline.current_clip_id, timeline.preview, fps);
    drop(timeline);

    if baked_clips.last_view != Some(current_view) {
        baked_clips.last_view = Some(current_view);
        baked_clips.scan_requested = true;
    }

    if !baked_clips.scan_requested {
        return;
    }
    baked_clips.scan_requested = false;
    drop(baked_clips);

    let keys_to_bake = collect_keys_to_bake(world, fps);
    if keys_to_bake.is_empty() {
        return;
    }

    let skeleton = find_first_skeleton(assets).cloned();
    let rig = world.resource::<HumanoidRigState>().rig.clone();
    let clip_library = world.resource::<ClipLibrary>();
    let mut baked_clips = world.resource_mut::<BakedHumanoidClips>();

    let is_inferred_rig = rig.as_ref().is_some_and(|rig| !rig.confirmed);
    if is_inferred_rig && baked_clips.inferred_warned_revision != Some(current_revision) {
        msg_warn!(
            "humanoid mapping of this model is inferred, not confirmed; save it in Avatar Setup"
        );
        baked_clips.inferred_warned_revision = Some(current_revision);
    }

    for key in keys_to_bake {
        let (source_id, entity) = key;
        let Some(source_clip) = clip_library.get(source_id) else {
            continue;
        };
        let baked = match (&skeleton, &rig) {
            (Some(skeleton), Some(rig)) => bake_to_bone_clip(source_clip, skeleton, rig, fps),
            _ => Err(anyhow::anyhow!("no model rig to bake onto")),
        };

        match baked {
            Ok(clip) => {
                let asset_id = assets.add_animation_clip(AnimationClipAsset {
                    id: 0,
                    clip: clip_to_animation(&clip),
                });
                let replaced = baked_clips.by_key.insert(
                    key,
                    BakedHumanoidClip {
                        fps,
                        asset_id,
                        clip,
                    },
                );
                if let Some(replaced) = replaced {
                    assets.animation_clips.remove(&replaced.asset_id);
                }
            }
            Err(error) => {
                log_warn!(
                    "bake clip {} for entity {} failed: {}",
                    source_id,
                    entity,
                    error
                );
                baked_clips.failed.insert(key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use cgmath::{InnerSpace, Rad, Rotation3};
    use thyllore_math_core::euler_degrees_to_quaternion;

    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };

    fn make_chain_skeleton(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        for i in 0..bone_count {
            let parent_id = if i == 0 {
                None
            } else {
                Some((i - 1) as BoneId)
            };
            skeleton.add_bone(&format!("bone_{}", i), parent_id);
        }
        skeleton
    }

    #[test]
    fn test_quaternion_roundtrip() {
        let skeleton = make_chain_skeleton(3);
        let quaternions: Vec<Quaternion<f32>> = vec![
            Quaternion::new(1.0, 0.0, 0.0, 0.0),
            Quaternion::from_angle_x(Rad(90.0_f32.to_radians())),
            Quaternion::from_angle_y(Rad(45.0_f32.to_radians())),
        ];
        let fps = 30;
        let frame_times: Vec<f32> = (0..quaternions.len())
            .map(|i| i as f32 / fps as f32)
            .collect();
        let mut bone_rotations = BTreeMap::new();
        bone_rotations.insert(0, quaternions.clone());
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![],
            morph_weights: BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for bone 0");

        for (i, quat) in quaternions.iter().enumerate() {
            let rx = track.rotation_x.keyframes[i].value;
            let ry = track.rotation_y.keyframes[i].value;
            let rz = track.rotation_z.keyframes[i].value;
            let reconstructed = euler_degrees_to_quaternion(&Vector3::new(rx, ry, rz));
            let dot = quat.dot(reconstructed);
            assert!(
                dot.abs() > 0.9999,
                "frame {}: dot={:.6}, quats not close",
                i,
                dot
            );
        }
    }

    #[test]
    fn test_baked_motion_to_clip_keeps_euler_continuous() {
        let skeleton = make_chain_skeleton(1);
        let q1 = Quaternion::from_angle_z(Rad(170.0_f32.to_radians()));
        let q2 = Quaternion::from_angle_z(Rad(190.0_f32.to_radians()));
        let quaternions = vec![q1, q2];
        let fps = 30;
        let frame_times: Vec<f32> = (0..quaternions.len())
            .map(|i| i as f32 / fps as f32)
            .collect();
        let mut bone_rotations = BTreeMap::new();
        bone_rotations.insert(0, quaternions);
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![],
            morph_weights: BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for bone 0");

        let z0 = track.rotation_z.keyframes[0].value;
        let z1 = track.rotation_z.keyframes[1].value;
        assert!(
            (z0 - 170.0).abs() < 0.01,
            "first key z={:.2}, expected ~170",
            z0
        );
        assert!(
            (z1 - 190.0).abs() < 0.01,
            "second key z={:.2}, expected ~190 (not -170)",
            z1
        );
    }

    #[test]
    fn test_hips_offset_translation() {
        let mut skeleton = Skeleton::new("test");
        skeleton.add_bone("Hips", None);
        let baked_offset = Vector3::new(1.0, 2.0, 3.0);
        let quaternions = vec![Quaternion::new(1.0, 0.0, 0.0, 0.0)];
        let fps = 30;
        let frame_times = vec![0.0];
        let mut bone_rotations = BTreeMap::new();
        bone_rotations.insert(0, quaternions);
        let baked = BakedMotion {
            fps,
            frame_times,
            bone_rotations,
            hips_offsets: vec![baked_offset],
            morph_weights: BTreeMap::new(),
        };
        let clip = baked_motion_to_clip(&baked, &skeleton, 0, "test");
        let track = clip.get_track(0).expect("track for Hips");

        let tx = track.translation_x.keyframes[0].value;
        let ty = track.translation_y.keyframes[0].value;
        let tz = track.translation_z.keyframes[0].value;
        assert!((tx - 1.0).abs() < 0.001, "tx={:.3}, expected 1.0", tx);
        assert!((ty - 2.0).abs() < 0.001, "ty={:.3}, expected 2.0", ty);
        assert!((tz - 3.0).abs() < 0.001, "tz={:.3}, expected 3.0", tz);
    }

    #[test]
    fn bake_to_bone_clip_keys_the_mapped_bone() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (_, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");

        let mut clip = EditableAnimationClip::new(0, "test_role".to_string());

        let right_lower_arm_bone = rig.track_bones["RightLowerArm"];
        let track = clip.add_track(right_lower_arm_bone, "RightLowerArm".to_string());
        curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_z, 2.0, 90.0);
        clip.duration = 2.0;

        let baked = bake_to_bone_clip(&clip, &skeleton, &rig, 30).unwrap();

        let baked_track = baked
            .get_track(right_lower_arm_bone)
            .expect("baked track for RightLowerArm bone not found");

        assert_eq!(
            baked_track.rotation_x.keyframes.len(),
            61,
            "expected 61 keys (2.0 * 30 + 1), got {}",
            baked_track.rotation_x.keyframes.len()
        );
    }

    #[test]
    fn refresh_bakes_a_scheduled_clip_once() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip = EditableAnimationClip::new(0, "test_role".to_string());
        let track = bone_clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        bone_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            bone_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        let asset_id = world
            .resource::<BakedHumanoidClips>()
            .by_key
            .get(&(source_id, entity))
            .expect("scheduled bone clip was not baked")
            .asset_id;
        assert!(assets.animation_clips.contains_key(&asset_id));
        let asset_count = assets.animation_clips.len();

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        assert_eq!(assets.animation_clips.len(), asset_count);
    }

    #[test]
    fn refresh_does_not_rescan_when_nothing_changed() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip = EditableAnimationClip::new(0, "test_role".to_string());
        let track = bone_clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        bone_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            bone_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        let baked_count = world.resource::<BakedHumanoidClips>().by_key.len();
        assert_eq!(baked_count, 1);

        let mut baked = world.resource_mut::<BakedHumanoidClips>();
        baked.by_key.clear();
        drop(baked);

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        let baked_count = world.resource::<BakedHumanoidClips>().by_key.len();
        assert_eq!(baked_count, 0);

        request_bake_scan(&mut world);
        refresh_baked_humanoid_clips(&mut world, &mut assets);
        let baked_count = world.resource::<BakedHumanoidClips>().by_key.len();
        assert_eq!(baked_count, 1);
    }

    #[test]
    fn refresh_without_rig_drops_baked_clips() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip = EditableAnimationClip::new(0, "test_role".to_string());
        let track = bone_clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        bone_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            bone_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        assert_eq!(world.resource::<BakedHumanoidClips>().by_key.len(), 1);

        world.resource_mut::<HumanoidRigState>().rig = None;
        refresh_baked_humanoid_clips(&mut world, &mut assets);
        assert!(world.resource::<BakedHumanoidClips>().by_key.is_empty());
    }

    #[test]
    fn unresolved_clip_roles_without_mapping_lists_every_role() {
        let mut clip = EditableAnimationClip::new(0, "test".to_string());

        let hips_idx = HumanoidRole::Hips.index();
        let spine_idx = HumanoidRole::Spine.index();

        let hips_track = clip.add_track(hips_idx as BoneId, "Hips".to_string());
        curve_add_keyframe(&mut hips_track.rotation_x, 0.0, 0.0);
        let spine_track = clip.add_track(spine_idx as BoneId, "Spine".to_string());
        curve_add_keyframe(&mut spine_track.rotation_x, 0.0, 0.0);

        let roles = unresolved_clip_roles(&clip, None);
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&HumanoidRole::Hips));
        assert!(roles.contains(&HumanoidRole::Spine));
    }

    #[test]
    fn unresolved_clip_roles_lists_unmapped_roles() {
        let mut clip = EditableAnimationClip::new(0, "test".to_string());

        let hips_idx = HumanoidRole::Hips.index();
        let spine_idx = HumanoidRole::Spine.index();

        let hips_track = clip.add_track(hips_idx as BoneId, "Hips".to_string());
        curve_add_keyframe(&mut hips_track.rotation_x, 0.0, 0.0);
        let spine_track = clip.add_track(spine_idx as BoneId, "Spine".to_string());
        curve_add_keyframe(&mut spine_track.rotation_x, 0.0, 0.0);

        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);

        let roles = unresolved_clip_roles(&clip, Some(&mapping));
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], HumanoidRole::Spine);
    }

    #[test]
    fn new_empty_clip_starts_without_tracks() {
        let clip = new_empty_clip("test");
        assert!((clip.duration - 2.0).abs() < f32::EPSILON);
        assert!((clip.min_duration - 2.0).abs() < f32::EPSILON);
        assert!(clip.tracks.is_empty());
    }

    #[test]
    fn unresolved_clip_roles_ignores_unkeyed_tracks() {
        let clip = new_empty_clip("test");
        let roles = unresolved_clip_roles(&clip, None);
        assert!(roles.is_empty());
    }

    #[test]
    fn rig_revision_change_clears_failed_bakes() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip = EditableAnimationClip::new(0, "test_role".to_string());
        let track = bone_clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        bone_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            bone_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        {
            let baked = world.resource::<BakedHumanoidClips>();
            assert_eq!(baked.by_key.len(), 1);
            let baked_revision = baked.rig_revision;
            drop(baked);
            let rig_revision = world.resource::<HumanoidRigState>().revision;
            assert_eq!(baked_revision, rig_revision);
        }

        {
            let mut rig_state = world.resource_mut::<HumanoidRigState>();
            rig_state.revision += 1;
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        let baked = world.resource::<BakedHumanoidClips>();
        assert!(
            baked.failed.is_empty(),
            "failed should be empty after rig revision change"
        );
    }

    #[test]
    fn bake_passes_unmapped_tracks_through() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (_, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");

        let mut clip = EditableAnimationClip::new(0, "test_unmapped".to_string());

        let head_bone = rig.track_bones["Head"];
        let head_track = clip.add_track(head_bone, "Head".to_string());
        curve_add_keyframe(&mut head_track.rotation_x, 0.0, 0.0);
        curve_add_keyframe(&mut head_track.rotation_x, 2.0, 30.0);

        let skirt_bone = rig.track_bones["Skirt_Front_1"];
        let skirt_track = clip.add_track(skirt_bone, "Skirt_Front_1".to_string());
        curve_add_keyframe(&mut skirt_track.rotation_x, 0.0, 0.0);
        curve_add_keyframe(&mut skirt_track.rotation_x, 2.0, 45.0);

        clip.duration = 2.0;

        let baked = bake_to_bone_clip(&clip, &skeleton, &rig, 30).unwrap();

        let baked_skirt = baked
            .get_track(skirt_bone)
            .expect("baked track for Skirt_Front_1 not found");
        assert_eq!(baked_skirt.rotation_x.keyframes.len(), 2);
        assert!((baked_skirt.rotation_x.keyframes[0].value - 0.0).abs() < 0.001);
        assert!((baked_skirt.rotation_x.keyframes[1].value - 45.0).abs() < 0.001);

        let baked_head = baked
            .get_track(head_bone)
            .expect("baked track for Head not found");
        assert_eq!(
            baked_head.rotation_x.keyframes.len(),
            61,
            "expected 61 keys (2.0 * 30 + 1), got {}",
            baked_head.rotation_x.keyframes.len()
        );
    }

    #[test]
    fn bone_clip_is_baked_on_a_humanoid_model() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip = EditableAnimationClip::new(0, "test_bone".to_string());
        let track = bone_clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        bone_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            bone_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        assert!(
            world
                .resource::<BakedHumanoidClips>()
                .by_key
                .contains_key(&(source_id, entity)),
            "bone clip was not baked on humanoid model"
        );

        let (mut world2, mut assets2) = test_humanoid_world(&fbx_path);
        world2.insert_resource(ClipLibrary::default());
        world2.insert_resource(TimelineState::default());
        world2.insert_resource(BakedHumanoidClips::default());
        let entity2 = world2
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut bone_clip2 = EditableAnimationClip::new(0, "test_bone2".to_string());
        let track2 = bone_clip2.add_track(0, "Hips".to_string());
        curve_add_keyframe(&mut track2.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track2.rotation_y, 1.0, 45.0);
        bone_clip2.duration = 1.0;
        let source_id2 = clip_library_register_and_activate(
            &mut world2.resource_mut::<ClipLibrary>(),
            &mut assets2,
            bone_clip2,
        );
        if let Some(schedule) = world2.get_component_mut::<ClipSchedule>(entity2) {
            clip_schedule_add_instance(schedule, source_id2, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world2, &mut assets2);
        assert!(
            !world2
                .resource::<BakedHumanoidClips>()
                .by_key
                .contains_key(&(source_id2, entity2)),
            "bone clip was baked on non-humanoid model (rig is None)"
        );
    }

    #[test]
    fn clip_is_still_baked_on_a_humanoid_model() {
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (mut world, mut assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");
        let hips_bone = rig.track_bones["Hips"];
        world.resource_mut::<HumanoidRigState>().rig = Some(rig);

        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedHumanoidClips::default());
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut clip = new_empty_clip("test_role");
        let track = clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_humanoid_clips(&mut world, &mut assets);
        assert!(
            world
                .resource::<BakedHumanoidClips>()
                .by_key
                .contains_key(&(source_id, entity)),
            "clip was not baked on humanoid model"
        );
    }
}
