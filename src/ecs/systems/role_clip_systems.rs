use std::collections::HashSet;
use std::path::Path;

use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, Vector3};
use thyllore_anim_core::editable::components::clip::{ClipSpace, EditableAnimationClip};
use thyllore_anim_core::editable::systems::clip_convert::clip_to_animation;
use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::components::baked_motion::BakedMotion;
use thyllore_avatar_core::motion::components::retarget_context::RetargetContext;
use thyllore_avatar_core::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};
use thyllore_math_core::quaternion_to_euler_degrees;

use crate::animation::editable::SourceClipId;
use crate::animation::{BoneId, Skeleton};
use crate::asset::{AnimationClipAsset, AssetStorage};
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::resource::{
    AnimationType, AvatarSetupState, BakedRoleClip, BakedRoleClips, ClipLibrary, ClipPreview,
    TimelineState,
};
use crate::ecs::systems::avatar_setup_systems::{
    compute_bone_global_transform, find_first_skeleton, find_model_path, load_or_infer_mapping,
    skeleton_to_bone_inputs,
};
use crate::ecs::systems::clip_schedule_systems::find_preview_owner;
use crate::ecs::world::{Entity, World};

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
                world_rotation: extract_rotation(&world_matrix),
            }
        })
        .collect();
    RetargetSkeleton { bones }
}

fn extract_rotation(matrix: &Matrix4<f32>) -> Quaternion<f32> {
    Quaternion::from(Matrix3::from_cols(
        matrix.x.truncate().normalize(),
        matrix.y.truncate().normalize(),
        matrix.z.truncate().normalize(),
    ))
}

fn unwrap_degrees(angle: f32, previous: f32) -> f32 {
    previous + (angle - previous + 180.0).rem_euclid(360.0) - 180.0
}

fn continuous_euler(euler: Vector3<f32>, prev: Option<Vector3<f32>>) -> Vector3<f32> {
    match prev {
        None => euler,
        Some(p) => Vector3::new(
            unwrap_degrees(euler.x, p.x),
            unwrap_degrees(euler.y, p.y),
            unwrap_degrees(euler.z, p.z),
        ),
    }
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
                    extract_rotation(&compute_bone_global_transform(skeleton, parent_id as usize))
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
    let frame = thyllore_avatar_core::humanoid::systems::character_frame::derive_character_frame(
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

pub fn bake_role_clip_to_bone_clip(
    role_clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    fps: u32,
) -> anyhow::Result<EditableAnimationClip> {
    let ctx = build_role_retarget_context(skeleton, mapping)?;
    let baked = thyllore_avatar_core::motion::systems::bake::bake_role_clip(&ctx, role_clip, fps);
    let hips_bone = *mapping
        .by_role
        .get(&HumanoidRole::Hips)
        .ok_or_else(|| anyhow::anyhow!("mapping has no Hips bone"))?;
    Ok(baked_motion_to_clip(
        &baked,
        skeleton,
        hips_bone,
        &role_clip.name,
    ))
}

pub fn resolve_model_rig(
    world: &World,
    assets: &AssetStorage,
) -> Option<(Skeleton, HumanoidMapping)> {
    let skeleton = find_first_skeleton(assets)?.clone();
    let model_path = find_model_path(world)?;
    let mapping = match world.get_resource::<AvatarSetupState>() {
        Some(state) if state.source_model_path == model_path => state.mapping.clone(),
        _ => load_or_infer_mapping(Path::new(&model_path), &skeleton_to_bone_inputs(&skeleton)).0,
    };
    Some((skeleton, mapping))
}

pub fn unresolved_clip_roles(
    clip: &EditableAnimationClip,
    mapping: Option<&HumanoidMapping>,
) -> Vec<HumanoidRole> {
    let mut seen = HashSet::new();
    let mut roles = Vec::new();
    for bone_id in clip.tracks.keys() {
        let idx = *bone_id as usize;
        if idx >= HumanoidRole::ALL.len() {
            continue;
        }
        let role = HumanoidRole::ALL[idx];
        if !seen.insert(role) {
            continue;
        }
        if let Some(m) = mapping {
            if m.by_role.contains_key(&role) {
                continue;
            }
        }
        roles.push(role);
    }
    roles
}

fn is_role_clip(clip_library: &ClipLibrary, source_id: SourceClipId) -> bool {
    clip_library
        .get(source_id)
        .is_some_and(|clip| clip.space == ClipSpace::HumanoidRole)
}

fn collect_needed_role_clip_keys(world: &World) -> HashSet<(SourceClipId, Entity)> {
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
            if is_role_clip(&clip_library, instance.source_id) {
                needed.insert((instance.source_id, entity));
            }
        }
    }

    let timeline = world.resource::<TimelineState>();
    if timeline.preview == ClipPreview::Solo {
        if let Some(current_clip_id) = timeline.current_clip_id {
            if is_role_clip(&clip_library, current_clip_id) {
                if let Some(preview_owner) = find_preview_owner(world) {
                    needed.insert((current_clip_id, preview_owner));
                }
            }
        }
    }

    needed
}

fn collect_keys_to_bake(world: &World, fps: u32) -> Vec<(SourceClipId, Entity)> {
    let baked_role_clips = world.resource::<BakedRoleClips>();
    collect_needed_role_clip_keys(world)
        .into_iter()
        .filter(|key| !baked_role_clips.failed.contains(key))
        .filter(|key| {
            baked_role_clips
                .by_key
                .get(key)
                .is_none_or(|baked| baked.fps != fps)
        })
        .collect()
}

pub fn refresh_baked_role_clips(world: &mut World, assets: &mut AssetStorage) {
    let fps = world
        .resource::<TimelineState>()
        .snap_settings
        .frame_rate
        .round() as u32;
    let keys_to_bake = collect_keys_to_bake(world, fps);
    if keys_to_bake.is_empty() {
        return;
    }

    let rig = resolve_model_rig(world, assets);
    let clip_library = world.resource::<ClipLibrary>();
    let mut baked_role_clips = world.resource_mut::<BakedRoleClips>();

    for key in keys_to_bake {
        let (source_id, entity) = key;
        let Some(role_clip) = clip_library.get(source_id) else {
            continue;
        };
        let baked = match &rig {
            Some((skeleton, mapping)) => {
                bake_role_clip_to_bone_clip(role_clip, skeleton, mapping, fps)
            }
            None => Err(anyhow::anyhow!("no model rig to bake onto")),
        };

        match baked {
            Ok(clip) => {
                let asset_id = assets.add_animation_clip(AnimationClipAsset {
                    id: 0,
                    clip: clip_to_animation(&clip),
                });
                let replaced = baked_role_clips.by_key.insert(
                    key,
                    BakedRoleClip {
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
                    "bake role clip {} for entity {} failed: {}",
                    source_id,
                    entity,
                    error
                );
                baked_role_clips.failed.insert(key);
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn load_mixamo_fixture_skeleton() -> Skeleton {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/thyllore-avatar-core/tests/data");
    let fbx_txt_path = test_data_dir.join("rigs/mixamo.fbx.txt");
    let counter = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let fbx_path = std::env::temp_dir().join(format!(
        "motion_recipe_mixamo_{}_{}.fbx",
        std::process::id(),
        counter
    ));
    std::fs::copy(&fbx_txt_path, &fbx_path).unwrap();
    let load_result = thyllore_importer_core::fbx::loader::load_fbx_to_graphics_resources(
        fbx_path.to_str().unwrap(),
    );
    std::fs::remove_file(&fbx_path).ok();
    let (fbx_result, _) = load_result.expect("Failed to load FBX");
    fbx_result
        .animation_system
        .skeletons
        .first()
        .expect("No skeleton in loaded model")
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use cgmath::{Rad, Rotation3};
    use thyllore_anim_core::editable::components::clip::ClipSpace;
    use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
    use thyllore_math_core::euler_degrees_to_quaternion;

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
    fn test_continuous_euler_z_170_to_190() {
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
    fn bake_role_clip_to_bone_clip_keys_the_mapped_bone() {
        let skeleton = load_mixamo_fixture_skeleton();
        let bones = skeleton_to_bone_inputs(&skeleton);
        let (mapping, _) = infer_mapping(&bones);

        let mut role_clip = EditableAnimationClip::new(0, "test_role".to_string());
        role_clip.space = ClipSpace::HumanoidRole;

        let right_lower_arm_role_index = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::RightLowerArm)
            .unwrap();
        let fake_bone_id: BoneId = right_lower_arm_role_index as BoneId;

        let track = role_clip.add_track(fake_bone_id, "RightLowerArm".to_string());
        curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_z, 2.0, 90.0);
        role_clip.duration = 2.0;

        let baked = bake_role_clip_to_bone_clip(&role_clip, &skeleton, &mapping, 30).unwrap();

        let right_lower_arm_bone_idx = *mapping
            .by_role
            .get(&HumanoidRole::RightLowerArm)
            .expect("RightLowerArm not found in mapping");
        let baked_track = baked
            .get_track(right_lower_arm_bone_idx as BoneId)
            .expect("baked track for RightLowerArm bone not found");

        assert_eq!(
            baked_track.rotation_x.keyframes.len(),
            61,
            "expected 61 keys (2.0 * 30 + 1), got {}",
            baked_track.rotation_x.keyframes.len()
        );
    }

    #[test]
    fn refresh_bakes_a_scheduled_role_clip_once() {
        use crate::asset::SkeletonAsset;
        use crate::ecs::resource::ModelState;
        use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
        use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

        let skeleton = load_mixamo_fixture_skeleton();
        let (mapping, _) = infer_mapping(&skeleton_to_bone_inputs(&skeleton));
        let mut assets = AssetStorage::new();
        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton,
        });

        let model_path = std::env::temp_dir()
            .join("role_clip_refresh_test_model.fbx")
            .to_string_lossy()
            .to_string();
        let mut world = World::new();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        world.insert_resource(BakedRoleClips::default());
        world.insert_resource(ModelState {
            model_path: model_path.clone(),
            ..Default::default()
        });
        world.insert_resource(AvatarSetupState {
            source_model_path: model_path,
            mapping,
            ..Default::default()
        });
        let entity = world
            .entity()
            .with_clip_schedule(ClipSchedule::new())
            .with_animation_meta(AnimationMeta {
                animation_type: AnimationType::Skeletal,
                node_animation_scale: 1.0,
            })
            .build();

        let mut role_clip = EditableAnimationClip::new(0, "test_role".to_string());
        role_clip.space = ClipSpace::HumanoidRole;
        let hips_role_index = HumanoidRole::ALL
            .iter()
            .position(|role| *role == HumanoidRole::Hips)
            .unwrap();
        let track = role_clip.add_track(hips_role_index as BoneId, "Hips".to_string());
        curve_add_keyframe(&mut track.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_y, 1.0, 45.0);
        role_clip.duration = 1.0;
        let source_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            role_clip,
        );
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
            clip_schedule_add_instance(schedule, source_id, 1.0);
        }

        refresh_baked_role_clips(&mut world, &mut assets);
        let asset_id = world
            .resource::<BakedRoleClips>()
            .by_key
            .get(&(source_id, entity))
            .expect("scheduled role clip was not baked")
            .asset_id;
        assert!(assets.animation_clips.contains_key(&asset_id));
        let asset_count = assets.animation_clips.len();

        refresh_baked_role_clips(&mut world, &mut assets);
        assert_eq!(assets.animation_clips.len(), asset_count);
    }

    #[test]
    fn unresolved_clip_roles_without_mapping_lists_every_role() {
        let mut clip = EditableAnimationClip::new(0, "test".to_string());
        clip.space = ClipSpace::HumanoidRole;

        let hips_idx = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::Hips)
            .unwrap();
        let spine_idx = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::Spine)
            .unwrap();

        clip.add_track(hips_idx as BoneId, "Hips".to_string());
        clip.add_track(spine_idx as BoneId, "Spine".to_string());

        let roles = unresolved_clip_roles(&clip, None);
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&HumanoidRole::Hips));
        assert!(roles.contains(&HumanoidRole::Spine));
    }

    #[test]
    fn unresolved_clip_roles_lists_unmapped_roles() {
        let mut clip = EditableAnimationClip::new(0, "test".to_string());
        clip.space = ClipSpace::HumanoidRole;

        let hips_idx = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::Hips)
            .unwrap();
        let spine_idx = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::Spine)
            .unwrap();

        clip.add_track(hips_idx as BoneId, "Hips".to_string());
        clip.add_track(spine_idx as BoneId, "Spine".to_string());

        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);

        let roles = unresolved_clip_roles(&clip, Some(&mapping));
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], HumanoidRole::Spine);
    }
}
