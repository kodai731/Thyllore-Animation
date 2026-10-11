use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::clip_library_systems::clip_library_clip_names;
use crate::ecs::world::World;
use crate::hooks::scene::{decode_scene_value, encode_scene_value, SceneValue};
use crate::hooks::scene_resource::SceneResourceHook;

/// Persisted timeline state; the active clip is named because clip ids are not stable on disk.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimelineSceneRecord {
    pub current_time: f32,
    pub playing: bool,
    pub looping: bool,
    pub speed: f32,
    pub current_clip: Option<String>,
}

impl thyllore_scene_core::SceneComponent for TimelineSceneRecord {
    const TYPE_KEY: &'static str = "timeline";
    const PERSISTED_FIELDS: &'static [&'static str] = &[
        "current_time",
        "playing",
        "looping",
        "speed",
        "current_clip",
    ];
}

inventory::submit! { TIMELINE_SCENE_RESOURCE }

const TIMELINE_SCENE_RESOURCE: SceneResourceHook = SceneResourceHook {
    type_key: <TimelineSceneRecord as thyllore_scene_core::SceneComponent>::TYPE_KEY,
    capture: capture_timeline_scene_record,
    apply: apply_timeline_scene_record,
};

fn capture_timeline_scene_record(world: &World) -> Option<SceneValue> {
    let timeline = world.get_resource::<TimelineState>()?;
    let current_clip = timeline.current_clip_id.and_then(|id| {
        world
            .get_resource::<ClipLibrary>()
            .and_then(|library| library.get(id).map(|clip| clip.name.clone()))
    });
    encode_scene_value(&TimelineSceneRecord {
        current_time: timeline.current_time,
        playing: timeline.playing,
        looping: timeline.looping,
        speed: timeline.speed,
        current_clip,
    })
    .ok()
}

fn apply_timeline_scene_record(world: &mut World, value: &SceneValue) -> anyhow::Result<()> {
    let record: TimelineSceneRecord = decode_scene_value(value)?;
    let current_clip_id = record.current_clip.as_deref().and_then(|name| {
        world.get_resource::<ClipLibrary>().and_then(|library| {
            clip_library_clip_names(&library)
                .into_iter()
                .filter(|(_, clip_name)| clip_name == name)
                .map(|(id, _)| id)
                .min()
        })
    });

    let Some(mut timeline) = world.get_resource_mut::<TimelineState>() else {
        anyhow::bail!("TimelineState resource missing");
    };
    timeline.current_time = record.current_time;
    timeline.playing = record.playing;
    timeline.looping = record.looping;
    timeline.speed = record.speed;
    if current_clip_id.is_some() {
        timeline.current_clip_id = current_clip_id;
    }
    Ok(())
}
