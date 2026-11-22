use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::clip_library_clip_names;
use crate::ecs::systems::timeline::timeline_apply_fit_zoom;
use crate::ecs::world::World;
use crate::hooks::scene::{decode_scene_value, encode_scene_value, SceneValue};
use crate::hooks::scene_resource::SceneResourceHook;
use thyllore_anim_core::editable::SourceClipId;

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
    let named_clip = record
        .current_clip
        .as_deref()
        .and_then(|name| find_library_clip_named(world, name));

    let Some(mut timeline) = world.get_resource_mut::<TimelineState>() else {
        anyhow::bail!("TimelineState resource missing");
    };
    timeline.current_time = record.current_time;
    timeline.playing = record.playing;
    timeline.looping = record.looping;
    timeline.speed = record.speed;

    match named_clip {
        Some((clip_id, clip_duration)) => {
            timeline.current_clip_id = Some(clip_id);
            timeline.requested_clip_name = None;
            timeline_apply_fit_zoom(&mut timeline, clip_duration);
        }
        None => timeline.requested_clip_name = record.current_clip,
    }
    Ok(())
}

fn find_library_clip_named(world: &World, name: &str) -> Option<(SourceClipId, f32)> {
    let library = world.get_resource::<ClipLibrary>()?;
    let clip_id = clip_library_clip_names(&library)
        .into_iter()
        .filter(|(_, clip_name)| clip_name == name)
        .map(|(id, _)| id)
        .min()?;

    library.get(clip_id).map(|clip| (clip_id, clip.duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::resource::ZoomFit;
    use crate::ecs::systems::clip_library_register_and_activate;
    use thyllore_anim_core::editable::EditableAnimationClip;

    fn world_with_library_clip(clip_name: &str) -> (World, SourceClipId) {
        let mut world = World::new();
        let mut library = ClipLibrary::new();
        let mut clip = EditableAnimationClip::new(0, clip_name.to_string());
        clip.duration = 2.0;
        let clip_id =
            clip_library_register_and_activate(&mut library, &mut AssetStorage::new(), clip);
        world.insert_resource(library);
        world.insert_resource(TimelineState::new());
        (world, clip_id)
    }

    fn record_naming(current_clip: &str) -> SceneValue {
        encode_scene_value(&TimelineSceneRecord {
            current_time: 0.5,
            playing: false,
            looping: true,
            speed: 1.0,
            current_clip: Some(current_clip.to_string()),
        })
        .expect("encode")
    }

    #[test]
    fn library_clip_is_selected_and_waits_for_the_window_width_to_fit() {
        let (mut world, clip_id) = world_with_library_clip("user");

        apply_timeline_scene_record(&mut world, &record_naming("user")).expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, Some(clip_id));
        assert_eq!(timeline.requested_clip_name, None);
        assert_eq!(timeline.zoom_fit, ZoomFit::AwaitingVisibleWidth);
    }

    #[test]
    fn clip_missing_from_the_library_stays_requested_by_name() {
        let (mut world, _) = world_with_library_clip("user");

        apply_timeline_scene_record(&mut world, &record_naming("model take")).expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, None);
        assert_eq!(timeline.requested_clip_name.as_deref(), Some("model take"));
    }

    #[test]
    fn captured_record_names_the_current_clip() {
        let (mut world, clip_id) = world_with_library_clip("user");
        world.resource_mut::<TimelineState>().current_clip_id = Some(clip_id);

        let value = capture_timeline_scene_record(&world).expect("capture");
        let record: TimelineSceneRecord = decode_scene_value(&value).expect("decode");

        assert_eq!(record.current_clip.as_deref(), Some("user"));
    }
}
