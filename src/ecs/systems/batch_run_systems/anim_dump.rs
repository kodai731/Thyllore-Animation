use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use thyllore_anim_core::editable::{EditableAnimationClip, EditableKeyframe};
use thyllore_anim_core::BoneId;

use crate::ecs::component::{
    scalar_channel_domains, scalar_channel_for_property, AnimationMeta, ClipSchedule,
};
use crate::ecs::resource::{AnimationType, ClipLibrary, HumanoidRigState, TimelineState};
use crate::ecs::world::{Entity, World};

#[derive(Serialize)]
struct ScheduleInstanceJson {
    instance_id: u64,
    source_id: u64,
    start_time: f32,
    clip_in: f32,
    clip_out: f32,
    speed: f32,
    muted: bool,
}

#[derive(Serialize)]
struct CurveKeyJson {
    time: f32,
    value: f32,
}

#[derive(Serialize)]
struct ScalarCurveJson {
    property: String,
    keyframes: Vec<CurveKeyJson>,
}

#[derive(Serialize)]
struct BoneCurvesJson {
    rot_x: Vec<CurveKeyJson>,
    rot_y: Vec<CurveKeyJson>,
    rot_z: Vec<CurveKeyJson>,
    pos_x: Vec<CurveKeyJson>,
    pos_y: Vec<CurveKeyJson>,
    pos_z: Vec<CurveKeyJson>,
}

#[derive(Serialize)]
struct BoneTrackJson {
    bone_id: BoneId,
    bone_name: String,
    role: Option<String>,
    curves: BoneCurvesJson,
}

#[derive(Serialize)]
struct ClipJson {
    id: u64,
    name: String,
    duration: f32,
    bone_track_count: usize,
    scalar_curves: Vec<ScalarCurveJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bone_tracks: Option<Vec<BoneTrackJson>>,
}

#[derive(Serialize)]
struct EntityJson {
    entity: Entity,
    domain: &'static str,
    time: Option<f32>,
    clip_id: Option<u64>,
    params: serde_json::Map<String, serde_json::Value>,
    schedule: Vec<ScheduleInstanceJson>,
}

#[derive(Serialize)]
struct DragPreviewJson {
    entity: Entity,
    instance_id: u64,
    start_time: f32,
    end_time: f32,
}

#[derive(Serialize)]
struct TimelineJson {
    current_time: f32,
    playing: bool,
    looping: bool,
    current_clip_id: Option<u64>,
    drag_preview: Option<DragPreviewJson>,
}

#[derive(Serialize)]
struct ModelJson {
    entity: Entity,
    schedule: Vec<ScheduleInstanceJson>,
}

#[derive(Serialize)]
struct AnimDumpJson {
    entities: Vec<EntityJson>,
    clips: Vec<ClipJson>,
    timeline: Option<TimelineJson>,
    models: Vec<ModelJson>,
}

fn schedule_instances_json(world: &World, entity: Entity) -> Vec<ScheduleInstanceJson> {
    world
        .get_component::<ClipSchedule>(entity)
        .map(|s| {
            s.instances
                .iter()
                .map(|i| ScheduleInstanceJson {
                    instance_id: i.instance_id,
                    source_id: i.source_id,
                    start_time: i.start_time,
                    clip_in: i.clip_in,
                    clip_out: i.clip_out,
                    speed: i.speed,
                    muted: i.muted,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn collect_role_names_by_bone(world: &World) -> HashMap<BoneId, String> {
    let mut role_names: HashMap<BoneId, String> = HashMap::new();

    let rig_state = world.get_resource::<HumanoidRigState>();
    if let Some(rig) = rig_state.as_ref().and_then(|state| state.rig.as_ref()) {
        for (role, &bone_index) in &rig.mapping.by_role {
            role_names.insert(bone_index as BoneId, role.unity_name().to_string());
        }
    }

    role_names
}

fn bone_tracks_json(world: &World, clip: &EditableAnimationClip) -> Vec<BoneTrackJson> {
    let role_names = collect_role_names_by_bone(world);

    let mut tracks: Vec<_> = clip.tracks.values().collect();
    tracks.sort_unstable_by_key(|track| track.bone_id);

    tracks
        .into_iter()
        .map(|track| BoneTrackJson {
            bone_id: track.bone_id,
            bone_name: track.bone_name.clone(),
            role: role_names.get(&track.bone_id).cloned(),
            curves: BoneCurvesJson {
                rot_x: curve_keyframes(&track.rotation_x.keyframes),
                rot_y: curve_keyframes(&track.rotation_y.keyframes),
                rot_z: curve_keyframes(&track.rotation_z.keyframes),
                pos_x: curve_keyframes(&track.translation_x.keyframes),
                pos_y: curve_keyframes(&track.translation_y.keyframes),
                pos_z: curve_keyframes(&track.translation_z.keyframes),
            },
        })
        .collect()
}

fn curve_keyframes(keyframes: &[EditableKeyframe]) -> Vec<CurveKeyJson> {
    keyframes
        .iter()
        .map(|k| CurveKeyJson {
            time: k.time,
            value: k.value,
        })
        .collect()
}

fn entity_dumps(world: &World) -> Vec<EntityJson> {
    use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;

    scalar_channel_domains()
        .iter()
        .flat_map(|domain| {
            (domain.entities)(world).into_iter().map(move |entity| {
                let params: serde_json::Map<String, serde_json::Value> = domain
                    .channels()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, channel)| {
                        (domain.read)(world, entity, domain.property_type_at(index))
                            .map(|value| (channel.cli_name.to_string(), value.into()))
                    })
                    .collect();
                let schedule = schedule_instances_json(world, entity);
                EntityJson {
                    entity,
                    domain: domain.name,
                    time: (domain.local_time)(world, entity),
                    clip_id: find_entity_clip_id(world, entity),
                    params,
                    schedule,
                }
            })
        })
        .collect()
}

fn clip_dump(world: &World, clip: &EditableAnimationClip, include_tracks: bool) -> ClipJson {
    let scalar_curves: Vec<ScalarCurveJson> = clip
        .scalar_curves
        .iter()
        .map(|curve| {
            let property = scalar_channel_for_property(curve.property_type)
                .map(|(_, c)| c.cli_name.to_string())
                .unwrap_or_else(|| format!("{:?}", curve.property_type));
            ScalarCurveJson {
                property,
                keyframes: curve_keyframes(&curve.keyframes),
            }
        })
        .collect();

    ClipJson {
        id: clip.id,
        name: clip.name.clone(),
        duration: clip.duration,
        bone_track_count: clip.tracks.len(),
        scalar_curves,
        bone_tracks: if include_tracks {
            Some(bone_tracks_json(world, clip))
        } else {
            None
        },
    }
}

fn clips_dumps(world: &World, include_tracks: bool) -> Vec<ClipJson> {
    world
        .get_resource::<ClipLibrary>()
        .map(|library| {
            let mut ids: Vec<_> = library.all_clip_ids().copied().collect();
            ids.sort_unstable();
            ids.iter()
                .filter_map(|&id| library.get(id))
                .map(|clip| clip_dump(world, clip, include_tracks))
                .collect()
        })
        .unwrap_or_default()
}

fn timeline_dump(world: &World) -> Option<TimelineJson> {
    world.get_resource::<TimelineState>().map(|t| {
        let drag_preview = world
            .get_resource::<crate::ecs::resource::TimelineInteractionState>()
            .and_then(|s| s.drag_preview)
            .map(|p| DragPreviewJson {
                entity: p.entity,
                instance_id: p.instance_id,
                start_time: p.start_time,
                end_time: p.end_time,
            });

        TimelineJson {
            current_time: t.current_time,
            playing: t.playing,
            looping: t.looping,
            current_clip_id: t.current_clip_id,
            drag_preview,
        }
    })
}

fn model_dumps(world: &World) -> Vec<ModelJson> {
    world
        .component_entities::<ClipSchedule>()
        .into_iter()
        .filter(|&entity| {
            world
                .get_component::<AnimationMeta>(entity)
                .is_some_and(|meta| meta.animation_type == AnimationType::Skeletal)
        })
        .map(|entity| ModelJson {
            entity,
            schedule: schedule_instances_json(world, entity),
        })
        .collect()
}

/// Serialize the animation-facing world state (effects, their scheduled clips,
/// every clip's scalar curves, timeline) so agents can inspect edits without a
/// window. Written once at engine exit; the file is the access surface.
pub fn batch_anim_dump_json(world: &World, include_tracks: bool) -> serde_json::Value {
    let dump = AnimDumpJson {
        entities: entity_dumps(world),
        clips: clips_dumps(world, include_tracks),
        timeline: timeline_dump(world),
        models: model_dumps(world),
    };
    serde_json::to_value(&dump).expect("AnimDumpJson serialization")
}

pub fn batch_anim_dump_write(world: &World, path: &str, include_tracks: bool) -> Result<()> {
    let json = batch_anim_dump_json(world, include_tracks);
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&json)?)
        .with_context(|| format!("failed to write anim dump to {path}"))?;
    Ok(())
}
