use std::fs;
use std::path::{Path, PathBuf};

use super::clip_io::{load_animation_clip, save_animation_clip};
use super::error::{SceneError, SceneResult};
use super::format::{
    AnimationClipRef, CameraState, EditorState, SceneFile, SceneMetadata, TimelineConfig,
    SCENE_FORMAT_VERSION,
};
use super::Camera;

use crate::animation::editable::{EditableClipId, EditableClipManager};
use crate::ecs::resource::{SceneState, TimelineState};
use crate::ecs::world::World;
use crate::ecs::AnimationPlayback;
use crate::platform::CurveEditorState;

pub fn save_scene(scene_path: &Path, world: &World) -> SceneResult<()> {
    let collected = CollectedSceneState::from_world(world);
    let previous_metadata = collect_previous_metadata(world);

    let animations_dir = scene_path
        .parent()
        .unwrap_or(Path::new("."))
        .parent()
        .unwrap_or(Path::new("."))
        .join("animations");

    fs::create_dir_all(&animations_dir)?;

    let animation_clips = save_animation_clips(world, &animations_dir)?;

    let scene = build_scene_file(collected, previous_metadata, animation_clips);

    write_scene_file(scene_path, &scene)?;

    crate::log!("Saved scene to: {}", scene_path.display());
    Ok(())
}

struct CollectedSceneState {
    model_path: String,
    camera: CameraState,
    timeline: TimelineConfig,
    editor: EditorState,
    current_clip_name: Option<String>,
}

impl CollectedSceneState {
    fn from_world(world: &World) -> Self {
        let model_path = world
            .get_resource::<AnimationPlayback>()
            .map(|p| p.model_path.clone())
            .unwrap_or_default();

        let camera = world
            .get_resource::<Camera>()
            .map(|c| CameraState {
                position: [c.position.x, c.position.y, c.position.z],
                direction: [c.direction.x, c.direction.y, c.direction.z],
                up: [c.up.x, c.up.y, c.up.z],
            })
            .unwrap_or_default();

        let (timeline, current_clip_name) = collect_timeline_and_clip(world);

        let editor = world
            .get_resource::<CurveEditorState>()
            .map(|e| EditorState {
                selected_bone_id: e.selected_bone_id,
                curve_editor_open: e.is_open,
            })
            .unwrap_or_default();

        Self {
            model_path,
            camera,
            timeline,
            editor,
            current_clip_name,
        }
    }
}

fn collect_timeline_and_clip(world: &World) -> (TimelineConfig, Option<String>) {
    let timeline_state = world.get_resource::<TimelineState>();
    let clip_manager = world.get_resource::<EditableClipManager>();

    let timeline = timeline_state
        .as_ref()
        .map(|t| TimelineConfig {
            current_time: t.current_time,
            playing: t.playing,
            looping: t.looping,
            speed: t.speed,
        })
        .unwrap_or_default();

    let current_clip_id = timeline_state.as_ref().and_then(|t| t.current_clip_id);
    let current_clip_name =
        current_clip_id.and_then(|id| clip_manager.and_then(|cm| cm.get(id).map(|c| c.name.clone())));

    (timeline, current_clip_name)
}

fn collect_previous_metadata(world: &World) -> Option<SceneMetadata> {
    world
        .get_resource::<SceneState>()
        .and_then(|s| s.previous_metadata.clone())
}

fn save_animation_clips(world: &World, animations_dir: &Path) -> SceneResult<Vec<AnimationClipRef>> {
    let clip_manager = match world.get_resource::<EditableClipManager>() {
        Some(cm) => cm,
        None => return Ok(Vec::new()),
    };

    let mut animation_clips = Vec::new();

    for (clip_id, clip_name) in clip_manager.clip_names() {
        if let Some(clip) = clip_manager.get(clip_id) {
            let clip_filename = sanitize_filename(&clip_name);
            let clip_path = animations_dir.join(format!("{}.anim.ron", clip_filename));

            save_animation_clip(&clip_path, clip)?;

            let relative_path = format!("animations/{}.anim.ron", clip_filename);
            animation_clips.push(AnimationClipRef::new(&relative_path));
        }
    }

    Ok(animation_clips)
}

fn build_scene_file(
    collected: CollectedSceneState,
    previous_metadata: Option<SceneMetadata>,
    animation_clips: Vec<AnimationClipRef>,
) -> SceneFile {
    let scene_name = previous_metadata
        .as_ref()
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "Untitled Scene".to_string());

    let mut scene = SceneFile::new(&scene_name, &collected.model_path);
    scene.animation_clips = animation_clips;
    scene.current_clip = collected.current_clip_name;
    scene.camera = collected.camera;
    scene.timeline = collected.timeline;
    scene.editor = collected.editor;

    if let Some(prev) = previous_metadata {
        scene.metadata.created_at = prev.created_at;
    }
    scene.metadata.update_modified();

    scene
}

fn write_scene_file(scene_path: &Path, scene: &SceneFile) -> SceneResult<()> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(8)
        .separate_tuple_members(true)
        .enumerate_arrays(false);

    let content = ron::ser::to_string_pretty(scene, config)?;

    if let Some(parent) = scene_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(scene_path, content)?;
    Ok(())
}

pub fn load_scene(scene_path: &Path) -> SceneResult<LoadedScene> {
    if !scene_path.exists() {
        return Err(SceneError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Scene file not found: {}", scene_path.display()),
        )));
    }

    let content = fs::read_to_string(scene_path)?;
    let scene: SceneFile = ron::from_str(&content)?;

    if scene.version != SCENE_FORMAT_VERSION {
        return Err(SceneError::VersionMismatch {
            expected: SCENE_FORMAT_VERSION,
            found: scene.version,
        });
    }

    let assets_dir = scene_path
        .parent()
        .unwrap_or(Path::new("."))
        .parent()
        .unwrap_or(Path::new("."));

    let model_path = assets_dir.join(&scene.model.path);
    if !model_path.exists() {
        return Err(SceneError::ModelNotFound(model_path));
    }

    let mut clips = Vec::new();
    for clip_ref in &scene.animation_clips {
        let clip_path = assets_dir.join(&clip_ref.path);
        let clip = load_animation_clip(&clip_path)?;
        clips.push(clip);
    }

    crate::log!("Loaded scene from: {}", scene_path.display());

    Ok(LoadedScene {
        scene,
        model_path,
        clips,
    })
}

pub fn find_default_scene() -> Option<PathBuf> {
    let path = PathBuf::from("assets/scenes/default.scene.ron");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub struct LoadedScene {
    pub scene: SceneFile,
    pub model_path: PathBuf,
    pub clips: Vec<crate::animation::editable::EditableAnimationClip>,
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn apply_loaded_scene_to_world(
    loaded: &LoadedScene,
    world: &mut World,
    clips_with_ids: &[(EditableClipId, String)],
) {
    if let Some(mut camera) = world.get_resource_mut::<Camera>() {
        camera.position = cgmath::Vector3::new(
            loaded.scene.camera.position[0],
            loaded.scene.camera.position[1],
            loaded.scene.camera.position[2],
        );
        camera.direction = cgmath::Vector3::new(
            loaded.scene.camera.direction[0],
            loaded.scene.camera.direction[1],
            loaded.scene.camera.direction[2],
        );
        camera.up = cgmath::Vector3::new(
            loaded.scene.camera.up[0],
            loaded.scene.camera.up[1],
            loaded.scene.camera.up[2],
        );
    }

    if let Some(mut timeline) = world.get_resource_mut::<TimelineState>() {
        timeline.current_time = loaded.scene.timeline.current_time;
        timeline.playing = loaded.scene.timeline.playing;
        timeline.looping = loaded.scene.timeline.looping;
        timeline.speed = loaded.scene.timeline.speed;

        if let Some(ref clip_name) = loaded.scene.current_clip {
            for (id, name) in clips_with_ids {
                if name == clip_name {
                    timeline.current_clip_id = Some(*id);
                    break;
                }
            }
        }
    }

    if let Some(mut curve_editor) = world.get_resource_mut::<CurveEditorState>() {
        curve_editor.selected_bone_id = loaded.scene.editor.selected_bone_id;
        curve_editor.is_open = loaded.scene.editor.curve_editor_open;
    }
}
