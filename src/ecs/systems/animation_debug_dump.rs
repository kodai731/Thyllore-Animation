use std::collections::HashMap;

use cgmath::{Matrix4, Vector3};
use serde::Serialize;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::{decompose_transform, AnimationClip, BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::resource::{BakedRoleClips, ClipLibrary, TimelineState};
use crate::ecs::systems::clip_schedule_systems::find_preview_owner;
use crate::ecs::systems::role_clip_systems::resolve_model_rig;
use crate::ecs::systems::skeleton_pose_systems::{
    compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use crate::ecs::world::Entity;
use crate::ecs::World;

pub struct AnimationDebugTarget<'a> {
    pub skeleton: &'a Skeleton,
    pub role_by_bone: HashMap<BoneId, HumanoidRole>,
    pub clip: Option<&'a AnimationClip>,
    pub clip_name: String,
    pub clip_duration: f32,
}

#[derive(Serialize)]
pub struct AnimationDebugDump {
    pub schema_version: u32,
    pub export_info: ExportInfo,
    pub skeleton: SkeletonDump,
    pub poses: Vec<PoseDump>,
    pub clip_channels: ClipChannelsDump,
}

#[derive(Serialize)]
pub struct ExportInfo {
    pub date: String,
    pub clip_name: String,
    pub clip_duration: f32,
}

#[derive(Serialize)]
pub struct SkeletonDump {
    pub bone_count: usize,
    pub bones: Vec<BoneDump>,
}

#[derive(Serialize)]
pub struct BoneDump {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub rest_local_matrix: [[f32; 4]; 4],
    pub rest_translation: [f32; 3],
    pub rest_rotation_quaternion: [f32; 4],
    pub rest_scale: [f32; 3],
}

#[derive(Serialize)]
pub struct PoseDump {
    pub time: f32,
    pub bones: Vec<PoseBoneDump>,
}

#[derive(Serialize)]
pub struct PoseBoneDump {
    pub id: u32,
    pub name: String,
    pub local_translation: [f32; 3],
    pub local_rotation_quaternion: [f32; 4],
    pub local_scale: [f32; 3],
    pub global_matrix: [[f32; 4]; 4],
    pub role: Option<String>,
    pub world_position: [f32; 3],
}

#[derive(Serialize)]
pub struct ClipChannelsDump {
    pub channel_count: usize,
    pub channels: Vec<ChannelDump>,
}

#[derive(Serialize)]
pub struct ChannelDump {
    pub bone_id: u32,
    pub bone_name: String,
    pub translation_keyframes: usize,
    pub rotation_keyframes: usize,
    pub scale_keyframes: usize,
    pub rotation_at_0: Option<[f32; 4]>,
    pub translation_at_0: Option<[f32; 3]>,
}

pub fn dump_animation_debug(world: &World, assets: &AssetStorage) -> anyhow::Result<()> {
    let timeline_state = world.resource::<TimelineState>();
    let current_time = timeline_state.current_time;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let now = chrono::Local::now();
    let path = std::path::PathBuf::from(format!(
        "log/animation_debug_{}.json",
        now.format("%Y%m%d_%H%M%S")
    ));
    write_animation_debug_dump(world, assets, &path, &[current_time], looping)?;

    log!("Animation debug dumped to {}", path.display());
    Ok(())
}

pub fn write_animation_debug_dump(
    world: &World,
    assets: &AssetStorage,
    path: &std::path::Path,
    times: &[f32],
    looping: bool,
) -> anyhow::Result<()> {
    let target = resolve_animation_debug_target(world, assets).ok_or_else(|| {
        anyhow::anyhow!("animation debug target not found (no clip/skeleton/rig)")
    })?;

    let dump = build_animation_debug_dump(&target, times, looping);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&dump)?;
    std::fs::write(path, &json)?;

    Ok(())
}

pub fn resolve_animation_debug_target<'a>(
    world: &World,
    assets: &'a AssetStorage,
) -> Option<AnimationDebugTarget<'a>> {
    let owner = find_preview_owner(world)?;
    let skeleton = assets.skeletons.values().next().map(|a| &a.skeleton)?;
    let (_, mapping) = resolve_model_rig(world, assets)?;

    let role_by_bone: HashMap<BoneId, HumanoidRole> = mapping
        .by_role
        .iter()
        .map(|(role, &bone_index)| (bone_index as BoneId, *role))
        .collect();

    let timeline_state = world.resource::<TimelineState>();
    let current_clip_id = timeline_state.current_clip_id;
    drop(timeline_state);

    let (clip_name, clip_duration, anim_clip) =
        resolve_current_clip(world, current_clip_id, owner, assets);

    Some(AnimationDebugTarget {
        skeleton,
        role_by_bone,
        clip: anim_clip,
        clip_name,
        clip_duration,
    })
}

pub fn build_animation_debug_dump(
    target: &AnimationDebugTarget,
    times: &[f32],
    looping: bool,
) -> AnimationDebugDump {
    let now = chrono::Local::now();
    let skeleton_dump = build_skeleton_dump(target.skeleton);
    let poses: Vec<PoseDump> = times
        .iter()
        .copied()
        .map(|t| {
            build_pose_dump(
                target.skeleton,
                &target.role_by_bone,
                target.clip,
                t,
                looping,
            )
        })
        .collect();
    let clip_channels_dump = build_clip_channels_dump(target.skeleton, target.clip);

    AnimationDebugDump {
        schema_version: 2,
        export_info: ExportInfo {
            date: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            clip_name: target.clip_name.clone(),
            clip_duration: target.clip_duration,
        },
        skeleton: skeleton_dump,
        poses,
        clip_channels: clip_channels_dump,
    }
}

fn resolve_current_clip<'a>(
    world: &World,
    current_clip_id: Option<u64>,
    owner: Entity,
    assets: &'a AssetStorage,
) -> (String, f32, Option<&'a AnimationClip>) {
    let Some(source_id) = current_clip_id else {
        return ("(none)".to_string(), 0.0, None);
    };

    let clip_library = world.resource::<ClipLibrary>();
    let editable = clip_library.get(source_id);
    let clip_name = editable
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "(unknown)".to_string());
    let clip_duration = editable.map(|e| e.duration).unwrap_or(0.0);

    let baked_asset_id = world
        .get_resource::<BakedRoleClips>()
        .and_then(|baked| baked.by_key.get(&(source_id, owner)).map(|b| b.asset_id));
    let anim_clip = baked_asset_id
        .or_else(|| clip_library.get_asset_id_for_source(source_id))
        .and_then(|asset_id| assets.animation_clips.get(&asset_id))
        .map(|a| &a.clip);

    (clip_name, clip_duration, anim_clip)
}

fn build_skeleton_dump(skeleton: &Skeleton) -> SkeletonDump {
    let bones = skeleton
        .bones
        .iter()
        .map(|bone| {
            let (t, r, s) = decompose_transform(&bone.local_transform);
            BoneDump {
                id: bone.id,
                name: bone.name.clone(),
                parent_id: bone.parent_id,
                rest_local_matrix: matrix4_to_arrays(&bone.local_transform),
                rest_translation: [t.x, t.y, t.z],
                rest_rotation_quaternion: [r.s, r.v.x, r.v.y, r.v.z],
                rest_scale: [s.x, s.y, s.z],
            }
        })
        .collect();

    SkeletonDump {
        bone_count: skeleton.bones.len(),
        bones,
    }
}

fn build_pose_dump(
    skeleton: &Skeleton,
    role_by_bone: &HashMap<BoneId, HumanoidRole>,
    anim_clip: Option<&AnimationClip>,
    time: f32,
    looping: bool,
) -> PoseDump {
    let mut pose = create_pose_from_rest(skeleton);
    if let Some(clip) = anim_clip {
        sample_clip_to_pose(clip, time, skeleton, &mut pose, looping);
    }

    let global_transforms = compute_pose_global_transforms(skeleton, &pose);

    let bones = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(idx, bone)| {
            let bp = &pose.bone_poses[idx];
            let global = global_transforms[idx];
            let world_position: Vector3<f32> = global.w.truncate();
            PoseBoneDump {
                id: bone.id,
                name: bone.name.clone(),
                local_translation: [bp.translation.x, bp.translation.y, bp.translation.z],
                local_rotation_quaternion: [
                    bp.rotation.s,
                    bp.rotation.v.x,
                    bp.rotation.v.y,
                    bp.rotation.v.z,
                ],
                local_scale: [bp.scale.x, bp.scale.y, bp.scale.z],
                global_matrix: matrix4_to_arrays(&global),
                role: role_by_bone.get(&bone.id).map(|r| format!("{r:?}")),
                world_position: [world_position.x, world_position.y, world_position.z],
            }
        })
        .collect();

    PoseDump { time, bones }
}

fn build_clip_channels_dump(
    skeleton: &Skeleton,
    anim_clip: Option<&AnimationClip>,
) -> ClipChannelsDump {
    let Some(clip) = anim_clip else {
        return ClipChannelsDump {
            channel_count: 0,
            channels: Vec::new(),
        };
    };

    let bone_name_map: HashMap<BoneId, &str> = skeleton
        .bones
        .iter()
        .map(|b| (b.id, b.name.as_str()))
        .collect();

    let mut channels: Vec<ChannelDump> = clip
        .channels
        .iter()
        .map(|(&bone_id, ch)| {
            let bone_name = bone_name_map.get(&bone_id).unwrap_or(&"?").to_string();

            ChannelDump {
                bone_id,
                bone_name,
                translation_keyframes: ch.translation.len(),
                rotation_keyframes: ch.rotation.len(),
                scale_keyframes: ch.scale.len(),
                rotation_at_0: ch.sample_rotation(0.0).map(|q| [q.s, q.v.x, q.v.y, q.v.z]),
                translation_at_0: ch.sample_translation(0.0).map(|v| [v.x, v.y, v.z]),
            }
        })
        .collect();

    channels.sort_by_key(|c| c.bone_id);

    ClipChannelsDump {
        channel_count: channels.len(),
        channels,
    }
}

fn matrix4_to_arrays(m: &Matrix4<f32>) -> [[f32; 4]; 4] {
    [
        [m.x.x, m.x.y, m.x.z, m.x.w],
        [m.y.x, m.y.y, m.y.z, m.y.w],
        [m.z.x, m.z.y, m.z.z, m.z.w],
        [m.w.x, m.w.y, m.w.z, m.w.w],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chain_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("chain");
        let root = skeleton.add_bone("Hips", None);
        let spine = skeleton.add_bone("Spine", Some(root));
        let chest = skeleton.add_bone("Chest", Some(spine));
        skeleton.bones[root as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
        skeleton.bones[spine as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.5, 0.0));
        skeleton.bones[chest as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.25, 0.1));
        skeleton
    }

    #[test]
    fn build_dump_reports_roles_and_world_positions_per_time() {
        let skeleton = make_chain_skeleton();

        let mut role_by_bone = HashMap::new();
        role_by_bone.insert(0u32, HumanoidRole::Hips);

        let target = AnimationDebugTarget {
            skeleton: &skeleton,
            role_by_bone,
            clip: None,
            clip_name: "test".to_string(),
            clip_duration: 1.0,
        };

        let times = [0.0f32, 0.5];
        let dump = build_animation_debug_dump(&target, &times, false);

        assert_eq!(dump.poses.len(), 2);
        assert_eq!(dump.poses[0].time, 0.0);
        assert_eq!(dump.poses[1].time, 0.5);

        let bone_0 = dump.poses[0]
            .bones
            .iter()
            .find(|b| b.id == 0)
            .expect("bone 0 not found");

        assert_eq!(bone_0.role, Some("Hips".to_string()));

        for pose in &dump.poses {
            for bone in &pose.bones {
                let translation = bone.global_matrix[3];
                assert_eq!(
                    bone.world_position,
                    [translation[0], translation[1], translation[2]]
                );
            }
        }
        assert_eq!(dump.poses[0].bones[2].world_position, [0.0, 1.75, 0.1]);
    }
}
