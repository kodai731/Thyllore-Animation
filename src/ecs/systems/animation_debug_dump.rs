use std::collections::HashMap;

use cgmath::Matrix4;
use serde::Serialize;

use crate::animation::{decompose_transform, AnimationClip, BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::skeleton_pose_systems::{
    compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use crate::ecs::World;

#[derive(Serialize)]
struct DebugDump {
    export_info: ExportInfo,
    skeleton: SkeletonDump,
    pose_at_current_time: PoseDump,
    clip_channels: ClipChannelsDump,
}

#[derive(Serialize)]
struct ExportInfo {
    date: String,
    clip_name: String,
    clip_duration: f32,
}

#[derive(Serialize)]
struct SkeletonDump {
    bone_count: usize,
    bones: Vec<BoneDump>,
}

#[derive(Serialize)]
struct BoneDump {
    id: u32,
    name: String,
    parent_id: Option<u32>,
    rest_local_matrix: [[f32; 4]; 4],
    rest_translation: [f32; 3],
    rest_rotation_quaternion: [f32; 4],
    rest_scale: [f32; 3],
}

#[derive(Serialize)]
struct PoseDump {
    time: f32,
    bones: Vec<PoseBoneDump>,
}

#[derive(Serialize)]
struct PoseBoneDump {
    id: u32,
    name: String,
    local_translation: [f32; 3],
    local_rotation_quaternion: [f32; 4],
    local_scale: [f32; 3],
    global_matrix: [[f32; 4]; 4],
}

#[derive(Serialize)]
struct ClipChannelsDump {
    channel_count: usize,
    channels: Vec<ChannelDump>,
}

#[derive(Serialize)]
struct ChannelDump {
    bone_id: u32,
    bone_name: String,
    translation_keyframes: usize,
    rotation_keyframes: usize,
    scale_keyframes: usize,
    rotation_at_0: Option<[f32; 4]>,
    translation_at_0: Option<[f32; 3]>,
}

pub fn dump_animation_debug(
    world: &World,
    assets: &AssetStorage,
    clip_library: &ClipLibrary,
) -> anyhow::Result<()> {
    let skeleton = assets
        .skeletons
        .values()
        .next()
        .map(|a| &a.skeleton)
        .ok_or_else(|| anyhow::anyhow!("No skeleton found"))?;

    let timeline_state = world.resource::<TimelineState>();
    let current_time = timeline_state.current_time;
    let current_clip_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let (clip_name, clip_duration, anim_clip) = resolve_current_clip(
        current_clip_id,
        clip_library,
        assets,
    );

    let skeleton_dump = build_skeleton_dump(skeleton);
    let pose_dump = build_pose_dump(skeleton, anim_clip.as_ref(), current_time, looping);
    let clip_channels_dump = build_clip_channels_dump(skeleton, anim_clip.as_ref());

    let now = chrono::Local::now();
    let dump = DebugDump {
        export_info: ExportInfo {
            date: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            clip_name,
            clip_duration,
        },
        skeleton: skeleton_dump,
        pose_at_current_time: pose_dump,
        clip_channels: clip_channels_dump,
    };

    let filename = format!("log/animation_debug_{}.json", now.format("%Y%m%d_%H%M%S"));
    std::fs::create_dir_all("log")?;
    let json = serde_json::to_string_pretty(&dump)?;
    std::fs::write(&filename, &json)?;

    crate::log!("Animation debug dumped to {}", filename);
    Ok(())
}

fn resolve_current_clip(
    current_clip_id: Option<u64>,
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
) -> (String, f32, Option<AnimationClip>) {
    let Some(source_id) = current_clip_id else {
        return ("(none)".to_string(), 0.0, None);
    };

    let editable = clip_library.get(source_id);
    let clip_name = editable
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "(unknown)".to_string());
    let clip_duration = editable.map(|e| e.duration).unwrap_or(0.0);

    let anim_clip = clip_library
        .get_anim_clip_id_for_source(source_id)
        .and_then(|anim_id| {
            assets
                .animation_clips
                .values()
                .find(|a| a.clip_id == anim_id)
                .map(|a| a.clip.clone())
        });

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
    anim_clip: Option<&AnimationClip>,
    current_time: f32,
    looping: bool,
) -> PoseDump {
    let mut pose = create_pose_from_rest(skeleton);
    if let Some(clip) = anim_clip {
        sample_clip_to_pose(clip, current_time, skeleton, &mut pose, looping);
    }

    let global_transforms = compute_pose_global_transforms(skeleton, &pose);

    let bones = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(idx, bone)| {
            let bp = &pose.bone_poses[idx];
            PoseBoneDump {
                id: bone.id,
                name: bone.name.clone(),
                local_translation: [bp.translation.x, bp.translation.y, bp.translation.z],
                local_rotation_quaternion: [bp.rotation.s, bp.rotation.v.x, bp.rotation.v.y, bp.rotation.v.z],
                local_scale: [bp.scale.x, bp.scale.y, bp.scale.z],
                global_matrix: matrix4_to_arrays(&global_transforms[idx]),
            }
        })
        .collect();

    PoseDump {
        time: current_time,
        bones,
    }
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
            let bone_name = bone_name_map
                .get(&bone_id)
                .unwrap_or(&"?")
                .to_string();

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
