use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use thyllore_anim_core::editable::{EditableAnimationClip, PropertyCurve};
use thyllore_anim_core::BoneId;

use crate::ecs::component::{
    scalar_channel_domains, scalar_channel_for_property, AnimationMeta, ClipSchedule,
};
use crate::ecs::resource::{AnimationType, ClipLibrary, HumanoidRigState, TimelineState};
use crate::ecs::world::{Entity, World};

fn schedule_instances_json(world: &World, entity: Entity) -> Vec<serde_json::Value> {
    world
        .get_component::<ClipSchedule>(entity)
        .map(|s| {
            s.instances
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "instance_id": i.instance_id,
                        "source_id": i.source_id,
                        "start_time": i.start_time,
                        "clip_in": i.clip_in,
                        "clip_out": i.clip_out,
                        "speed": i.speed,
                        "muted": i.muted,
                    })
                })
                .collect::<Vec<_>>()
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

fn bone_tracks_json(world: &World, clip: &EditableAnimationClip) -> Vec<serde_json::Value> {
    let role_names = collect_role_names_by_bone(world);

    let mut tracks: Vec<_> = clip.tracks.values().collect();
    tracks.sort_unstable_by_key(|track| track.bone_id);

    tracks
        .into_iter()
        .map(|track| {
            let role = role_names.get(&track.bone_id);

            let curves = serde_json::json!({
                "rot_x": build_curve_array(&track.rotation_x),
                "rot_y": build_curve_array(&track.rotation_y),
                "rot_z": build_curve_array(&track.rotation_z),
                "pos_x": build_curve_array(&track.translation_x),
                "pos_y": build_curve_array(&track.translation_y),
                "pos_z": build_curve_array(&track.translation_z),
            });

            serde_json::json!({
                "bone_id": track.bone_id,
                "bone_name": track.bone_name,
                "role": role,
                "curves": curves,
            })
        })
        .collect()
}

fn build_curve_array(curve: &PropertyCurve) -> Vec<serde_json::Value> {
    curve
        .keyframes
        .iter()
        .map(|k| serde_json::json!({"time": k.time, "value": k.value}))
        .collect()
}

/// Serialize the animation-facing world state (effects, their scheduled clips,
/// every clip's scalar curves, timeline) so agents can inspect edits without a
/// window. Written once at engine exit; the file is the access surface.
pub fn batch_anim_dump_json(world: &World, include_tracks: bool) -> serde_json::Value {
    use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;

    let entities: Vec<serde_json::Value> = scalar_channel_domains()
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
                serde_json::json!({
                    "entity": entity,
                    "domain": domain.name,
                    "time": (domain.local_time)(world, entity),
                    "clip_id": find_entity_clip_id(world, entity),
                    "params": params,
                    "schedule": schedule,
                })
            })
        })
        .collect();

    let clips: Vec<serde_json::Value> = world
        .get_resource::<ClipLibrary>()
        .map(|library| {
            let mut ids: Vec<_> = library.all_clip_ids().copied().collect();
            ids.sort_unstable();
            ids.iter()
                .filter_map(|&id| library.get(id))
                .map(|clip| {
                    let curves: Vec<serde_json::Value> = clip
                        .scalar_curves
                        .iter()
                        .map(|curve| {
                            let property = scalar_channel_for_property(curve.property_type)
                                .map(|(_, c)| c.cli_name.to_string())
                                .unwrap_or_else(|| format!("{:?}", curve.property_type));
                            let keyframes: Vec<serde_json::Value> = curve
                                .keyframes
                                .iter()
                                .map(|k| serde_json::json!({"time": k.time, "value": k.value}))
                                .collect();
                            serde_json::json!({"property": property, "keyframes": keyframes})
                        })
                        .collect();
                    let mut clip_obj = serde_json::json!({
                        "id": clip.id,
                        "name": clip.name,
                        "duration": clip.duration,
                        "bone_track_count": clip.tracks.len(),
                        "scalar_curves": curves,
                    });
                    if include_tracks {
                        let bone_tracks = bone_tracks_json(world, clip);
                        clip_obj["bone_tracks"] = serde_json::json!(bone_tracks);
                    }
                    clip_obj
                })
                .collect()
        })
        .unwrap_or_default();

    let drag_preview = world
        .get_resource::<crate::ecs::resource::TimelineInteractionState>()
        .and_then(|s| s.drag_preview)
        .map(|p| {
            serde_json::json!({
                "entity": p.entity,
                "instance_id": p.instance_id,
                "start_time": p.start_time,
                "end_time": p.end_time,
            })
        })
        .unwrap_or(serde_json::Value::Null);

    let timeline = world
        .get_resource::<TimelineState>()
        .map(|t| {
            serde_json::json!({
                "current_time": t.current_time,
                "playing": t.playing,
                "looping": t.looping,
                "current_clip_id": t.current_clip_id,
                "drag_preview": drag_preview,
            })
        })
        .unwrap_or(serde_json::Value::Null);

    let models: Vec<serde_json::Value> = world
        .component_entities::<ClipSchedule>()
        .into_iter()
        .filter(|&entity| {
            world
                .get_component::<AnimationMeta>(entity)
                .is_some_and(|meta| meta.animation_type == AnimationType::Skeletal)
        })
        .map(|entity| {
            serde_json::json!({
                "entity": entity,
                "schedule": schedule_instances_json(world, entity),
            })
        })
        .collect();

    serde_json::json!({"entities": entities, "clips": clips, "timeline": timeline, "models": models})
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
