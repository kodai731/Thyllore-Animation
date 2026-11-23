use std::path::Path;

use crate::ecs::component::ModelSource;
use crate::ecs::resource::{ClipLibrary, ClipOrigin, TimelineState};
use crate::ecs::systems::clip_library_clip_names;
use crate::ecs::systems::timeline::timeline_apply_fit_zoom;
use crate::ecs::world::World;
use crate::hooks::scene::{decode_scene_value, encode_scene_value, SceneValue};
use crate::hooks::scene_resource::SceneResourceHook;
use crate::scene::AnimationClipRef;
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};

/// Persisted timeline state. Clip ids are not stable on disk and clip names are not unique between
/// the model and the user's files, so a user clip is named by its file and a model clip by its name.
/// Whether the timeline plays is not part of a scene: a scene opens stopped.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimelineSceneRecord {
    pub current_time: f32,
    pub looping: bool,
    pub speed: f32,
    pub current_clip: Option<String>,
    pub current_clip_file: Option<String>,
}

impl thyllore_scene_core::SceneComponent for TimelineSceneRecord {
    const TYPE_KEY: &'static str = "timeline";
    const PERSISTED_FIELDS: &'static [&'static str] = &[
        "current_time",
        "looping",
        "speed",
        "current_clip",
        "current_clip_file",
    ];
}

inventory::submit! { TIMELINE_SCENE_RESOURCE }

const TIMELINE_SCENE_RESOURCE: SceneResourceHook = SceneResourceHook {
    type_key: <TimelineSceneRecord as thyllore_scene_core::SceneComponent>::TYPE_KEY,
    capture: capture_timeline_scene_record,
    apply: apply_timeline_scene_record,
};

enum SavedClip {
    UserFile(String),
    ModelClip(String),
}

fn capture_timeline_scene_record(world: &World) -> Option<SceneValue> {
    let timeline = world.get_resource::<TimelineState>()?;
    let saved_clip = timeline
        .current_clip_id
        .and_then(|clip_id| describe_saved_clip(world, clip_id));
    let (current_clip, current_clip_file) = match saved_clip {
        Some(SavedClip::UserFile(scene_file)) => (None, Some(scene_file)),
        Some(SavedClip::ModelClip(clip_name)) => (Some(clip_name), None),
        None => (None, None),
    };

    encode_scene_value(&TimelineSceneRecord {
        current_time: timeline.current_time,
        looping: timeline.looping,
        speed: timeline.speed,
        current_clip,
        current_clip_file,
    })
    .ok()
}

fn describe_saved_clip(world: &World, clip_id: SourceClipId) -> Option<SavedClip> {
    let library = world.get_resource::<ClipLibrary>()?;
    let clip = library.get(clip_id)?;

    match library.origin(clip_id) {
        ClipOrigin::Model => {
            is_model_saved_in_scene(world).then(|| SavedClip::ModelClip(clip.name.clone()))
        }
        ClipOrigin::User => scene_file_of(clip).map(SavedClip::UserFile),
    }
}

/// A generated model has no file, so a scene holds neither it nor the clips it brought.
fn is_model_saved_in_scene(world: &World) -> bool {
    world.iter_components::<ModelSource>().next().is_some()
}

fn scene_file_of(clip: &EditableAnimationClip) -> Option<String> {
    let clip_file = Path::new(clip.source_path.as_deref()?);
    AnimationClipRef::for_clip_file(clip_file).map(|clip_ref| clip_ref.path)
}

fn apply_timeline_scene_record(world: &mut World, value: &SceneValue) -> anyhow::Result<()> {
    let record: TimelineSceneRecord = decode_scene_value(value)?;
    let saved_clip = match (record.current_clip_file, record.current_clip) {
        (Some(scene_file), _) => Some(SavedClip::UserFile(scene_file)),
        (None, Some(clip_name)) => Some(SavedClip::ModelClip(clip_name)),
        (None, None) => None,
    };
    let library_clip = saved_clip
        .as_ref()
        .and_then(|saved_clip| find_library_clip(world, saved_clip));

    let Some(mut timeline) = world.get_resource_mut::<TimelineState>() else {
        anyhow::bail!("TimelineState resource missing");
    };
    timeline.current_time = record.current_time;
    timeline.looping = record.looping;
    timeline.speed = record.speed;

    match (library_clip, saved_clip) {
        (Some((clip_id, clip_duration)), _) => {
            timeline.current_clip_id = Some(clip_id);
            timeline.requested_clip_name = None;
            timeline_apply_fit_zoom(&mut timeline, clip_duration);
        }
        (None, Some(SavedClip::ModelClip(clip_name))) => {
            timeline.requested_clip_name = Some(clip_name)
        }
        (None, Some(SavedClip::UserFile(scene_file))) => {
            log_warn!("scene selects clip file {scene_file}, which is not loaded");
            timeline.requested_clip_name = None;
        }
        (None, None) => timeline.requested_clip_name = None,
    }
    Ok(())
}

fn find_library_clip(world: &World, saved_clip: &SavedClip) -> Option<(SourceClipId, f32)> {
    let library = world.get_resource::<ClipLibrary>()?;
    let clip_id = clip_library_clip_names(&library)
        .into_iter()
        .map(|(clip_id, _)| clip_id)
        .find(|clip_id| is_saved_clip(&library, *clip_id, saved_clip))?;

    library.get(clip_id).map(|clip| (clip_id, clip.duration))
}

fn is_saved_clip(library: &ClipLibrary, clip_id: SourceClipId, saved_clip: &SavedClip) -> bool {
    let Some(clip) = library.get(clip_id) else {
        return false;
    };

    match (saved_clip, library.origin(clip_id)) {
        (SavedClip::UserFile(scene_file), ClipOrigin::User) => {
            scene_file_of(clip).as_ref() == Some(scene_file)
        }
        (SavedClip::ModelClip(clip_name), ClipOrigin::Model) => &clip.name == clip_name,
        (SavedClip::UserFile(_), ClipOrigin::Model)
        | (SavedClip::ModelClip(_), ClipOrigin::User) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::resource::ZoomFit;
    use crate::ecs::systems::clip_library_register_and_activate;
    use thyllore_anim_core::editable::EditableAnimationClip;

    const USER_CLIP_SCENE_FILE: &str = "animations/user.anim.ron";

    fn world_with_user_clip(clip_name: &str) -> (World, SourceClipId) {
        let mut world = World::new();
        let mut library = ClipLibrary::new();
        let mut clip = EditableAnimationClip::new(0, clip_name.to_string());
        clip.duration = 2.0;
        clip.source_path = Some(format!("assets/{USER_CLIP_SCENE_FILE}"));
        let clip_id =
            clip_library_register_and_activate(&mut library, &mut AssetStorage::new(), clip);
        world.insert_resource(library);
        world.insert_resource(TimelineState::new());
        (world, clip_id)
    }

    fn register_model_clip(world: &mut World, clip_name: &str) -> SourceClipId {
        let mut library = world.resource_mut::<ClipLibrary>();
        let clip = EditableAnimationClip::new(0, clip_name.to_string());
        let clip_id =
            clip_library_register_and_activate(&mut library, &mut AssetStorage::new(), clip);
        library.model_clip_ids.insert(clip_id);
        clip_id
    }

    fn record_selecting(saved_clip: SavedClip) -> SceneValue {
        let (current_clip, current_clip_file) = match saved_clip {
            SavedClip::UserFile(scene_file) => (None, Some(scene_file)),
            SavedClip::ModelClip(clip_name) => (Some(clip_name), None),
        };
        encode_scene_value(&TimelineSceneRecord {
            current_time: 0.5,
            looping: true,
            speed: 1.0,
            current_clip,
            current_clip_file,
        })
        .expect("encode")
    }

    fn record_selecting_user_file() -> SceneValue {
        record_selecting(SavedClip::UserFile(USER_CLIP_SCENE_FILE.to_string()))
    }

    fn record_selecting_model_clip(clip_name: &str) -> SceneValue {
        record_selecting(SavedClip::ModelClip(clip_name.to_string()))
    }

    fn captured_record(world: &World) -> TimelineSceneRecord {
        let value = capture_timeline_scene_record(world).expect("capture");
        decode_scene_value(&value).expect("decode")
    }

    #[test]
    fn user_clip_is_selected_by_its_file_and_waits_for_the_window_width_to_fit() {
        let (mut world, clip_id) = world_with_user_clip("user");

        apply_timeline_scene_record(&mut world, &record_selecting_user_file()).expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, Some(clip_id));
        assert_eq!(timeline.requested_clip_name, None);
        assert_eq!(timeline.zoom_fit, ZoomFit::AwaitingVisibleWidth);
    }

    #[test]
    fn model_clip_missing_from_the_library_stays_requested_by_name() {
        let (mut world, _) = world_with_user_clip("user");

        apply_timeline_scene_record(&mut world, &record_selecting_model_clip("model take"))
            .expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, None);
        assert_eq!(timeline.requested_clip_name.as_deref(), Some("model take"));
    }

    #[test]
    fn model_clip_is_not_confused_with_a_user_clip_of_the_same_name() {
        let (mut world, _) = world_with_user_clip("take");

        apply_timeline_scene_record(&mut world, &record_selecting_model_clip("take"))
            .expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, None);
        assert_eq!(timeline.requested_clip_name.as_deref(), Some("take"));
    }

    #[test]
    fn loaded_model_clip_is_selected_over_a_user_clip_of_the_same_name() {
        let (mut world, _) = world_with_user_clip("take");
        let model_clip_id = register_model_clip(&mut world, "take");

        apply_timeline_scene_record(&mut world, &record_selecting_model_clip("take"))
            .expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert_eq!(timeline.current_clip_id, Some(model_clip_id));
        assert_eq!(timeline.requested_clip_name, None);
    }

    #[test]
    fn captured_record_names_a_user_clip_by_its_file() {
        let (world, clip_id) = world_with_user_clip("user");
        world.resource_mut::<TimelineState>().current_clip_id = Some(clip_id);

        let record = captured_record(&world);

        assert_eq!(record.current_clip, None);
        assert_eq!(
            record.current_clip_file.as_deref(),
            Some(USER_CLIP_SCENE_FILE)
        );
    }

    fn spawn_file_model(world: &mut World) {
        let model_entity = world.entity().with_name("model").build();
        world.insert_component(model_entity, ModelSource::new("model.fbx"));
    }

    #[test]
    fn captured_record_does_not_select_a_clip_of_a_model_the_scene_does_not_hold() {
        let (mut world, _) = world_with_user_clip("take");
        let generated_model_clip_id = register_model_clip(&mut world, "take");
        world.resource_mut::<TimelineState>().current_clip_id = Some(generated_model_clip_id);

        let record = captured_record(&world);

        assert_eq!(record.current_clip, None);
        assert_eq!(record.current_clip_file, None);
    }

    #[test]
    fn captured_record_names_a_model_clip_by_its_name() {
        let (mut world, _) = world_with_user_clip("take");
        spawn_file_model(&mut world);
        let model_clip_id = register_model_clip(&mut world, "take");
        world.resource_mut::<TimelineState>().current_clip_id = Some(model_clip_id);

        let record = captured_record(&world);

        assert_eq!(record.current_clip.as_deref(), Some("take"));
        assert_eq!(record.current_clip_file, None);
    }

    #[derive(serde::Serialize)]
    struct RecordWithPlayingFlag {
        current_time: f32,
        playing: bool,
        looping: bool,
        speed: f32,
        current_clip: Option<String>,
    }

    #[test]
    fn scene_saved_while_playing_opens_stopped() {
        let (mut world, _) = world_with_user_clip("user");
        let saved_while_playing = encode_scene_value(&RecordWithPlayingFlag {
            current_time: 0.5,
            playing: true,
            looping: true,
            speed: 1.0,
            current_clip: None,
        })
        .expect("encode");

        apply_timeline_scene_record(&mut world, &saved_while_playing).expect("apply");

        let timeline = world.resource::<TimelineState>();
        assert!(!timeline.playing);
        assert_eq!(timeline.current_time, 0.5);
    }

    #[test]
    fn applying_a_record_keeps_the_running_playback() {
        let (mut world, _) = world_with_user_clip("user");
        world.resource_mut::<TimelineState>().playing = true;

        apply_timeline_scene_record(&mut world, &record_selecting_user_file()).expect("apply");

        assert!(world.resource::<TimelineState>().playing);
    }
}
