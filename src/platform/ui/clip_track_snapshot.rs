use crate::animation::editable::{
    BlendMode, ClipGroupId, ClipInstanceId, SourceClipId,
};
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::world::{Entity, Name, World};

pub struct ClipTrackSnapshot {
    pub entries: Vec<ClipTrackEntry>,
}

pub struct ClipTrackEntry {
    pub entity: Entity,
    pub entity_name: String,
    pub instances: Vec<ClipInstanceSnapshot>,
    pub groups: Vec<ClipGroupSnapshot>,
}

pub struct ClipInstanceSnapshot {
    pub instance_id: ClipInstanceId,
    pub source_id: SourceClipId,
    pub clip_name: String,
    pub start_time: f32,
    pub end_time: f32,
    pub clip_in: f32,
    pub clip_out: f32,
    pub muted: bool,
    pub weight: f32,
    pub blend_mode: BlendMode,
    pub group_id: Option<ClipGroupId>,
}

pub struct ClipGroupSnapshot {
    pub id: ClipGroupId,
    pub name: String,
    pub muted: bool,
    pub weight: f32,
    pub instance_ids: Vec<ClipInstanceId>,
}

pub fn collect_clip_track_snapshot(
    world: &World,
    clip_library: &ClipLibrary,
) -> ClipTrackSnapshot {
    let mut entries = Vec::new();

    for (entity, schedule) in world.iter_components::<ClipSchedule>() {
        let entity_name = world
            .get_component::<Name>(entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| format!("Entity {}", entity));

        let instances: Vec<ClipInstanceSnapshot> = schedule
            .instances
            .iter()
            .map(|inst| {
                let clip_name = clip_library
                    .get_source(inst.source_id)
                    .map(|s| s.name().to_string())
                    .unwrap_or_else(|| format!("Clip {}", inst.source_id));

                let group_id = schedule
                    .find_group_for_instance(inst.instance_id)
                    .map(|g| g.id);

                ClipInstanceSnapshot {
                    instance_id: inst.instance_id,
                    source_id: inst.source_id,
                    clip_name,
                    start_time: inst.start_time,
                    end_time: inst.end_time(),
                    clip_in: inst.clip_in,
                    clip_out: inst.clip_out,
                    muted: inst.muted,
                    weight: inst.weight,
                    blend_mode: inst.blend_mode,
                    group_id,
                }
            })
            .collect();

        let groups: Vec<ClipGroupSnapshot> = schedule
            .groups
            .iter()
            .map(|g| ClipGroupSnapshot {
                id: g.id,
                name: g.name.clone(),
                muted: g.muted,
                weight: g.weight,
                instance_ids: g.instance_ids.clone(),
            })
            .collect();

        if !instances.is_empty() {
            entries.push(ClipTrackEntry {
                entity,
                entity_name,
                instances,
                groups,
            });
        }
    }

    ClipTrackSnapshot { entries }
}
