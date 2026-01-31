use crate::animation::editable::{ClipInstanceId, SourceClipId};
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
}

pub fn collect_clip_track_snapshot(world: &World, clip_library: &ClipLibrary) -> ClipTrackSnapshot {
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

                ClipInstanceSnapshot {
                    instance_id: inst.instance_id,
                    source_id: inst.source_id,
                    clip_name,
                    start_time: inst.start_time,
                    end_time: inst.end_time(),
                    clip_in: inst.clip_in,
                    clip_out: inst.clip_out,
                    muted: inst.muted,
                }
            })
            .collect();

        if !instances.is_empty() {
            entries.push(ClipTrackEntry {
                entity,
                entity_name,
                instances,
            });
        }
    }

    ClipTrackSnapshot { entries }
}
