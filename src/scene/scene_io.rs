use std::fs;
use std::path::{Path, PathBuf};

use super::clip_io::{load_animation_clip, save_animation_clip};
use super::error::{SceneError, SceneResult};
use super::format::{
    AnimationClipRef, AutoExposureState, BloomState, CameraState, DepthOfFieldState, EditorState,
    ExposureState, LensEffectsState, PanelLayoutState, PhysicalCameraState, SceneFile,
    SceneMetadata, TimelineConfig, ToneMappingState, SCENE_FORMAT_VERSION,
};
use crate::animation::editable::SourceClipId;
use crate::ecs::resource::{
    AutoExposure, BloomSettings, Camera, ClipLibrary, DepthOfField, Exposure, LensEffects,
    ModelState, PanelLayout, PhysicalCameraParameters, SceneState, TimelineState, ToneMapOperator,
    ToneMapping,
};
use crate::ecs::world::World;
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
    panel_layout: Option<PanelLayoutState>,
}

impl CollectedSceneState {
    fn from_world(world: &World) -> Self {
        let model_path = world
            .get_resource::<ModelState>()
            .map(|s| s.model_path.clone())
            .unwrap_or_default();

        let physical_camera =
            world
                .get_resource::<PhysicalCameraParameters>()
                .map(|p| PhysicalCameraState {
                    focal_length_mm: p.focal_length_mm,
                    sensor_height_mm: p.sensor_height_mm,
                    aperture_f_stops: p.aperture_f_stops,
                    shutter_speed_s: p.shutter_speed_s,
                    sensitivity_iso: p.sensitivity_iso,
                });

        let exposure = world.get_resource::<Exposure>().map(|e| ExposureState {
            ev100: e.ev100,
            exposure_value: e.exposure_value,
        });

        let depth_of_field = world
            .get_resource::<DepthOfField>()
            .map(|d| DepthOfFieldState {
                enabled: d.enabled,
                focus_distance: d.focus_distance,
                max_blur_radius: d.max_blur_radius,
            });

        let tone_mapping = world.get_resource::<ToneMapping>().map(|tm| {
            let operator_str = match tm.operator {
                ToneMapOperator::None => "None",
                ToneMapOperator::AcesFilmic => "AcesFilmic",
                ToneMapOperator::Reinhard => "Reinhard",
            };
            ToneMappingState {
                enabled: tm.enabled,
                operator: operator_str.to_string(),
                gamma: tm.gamma,
            }
        });

        let lens_effects = world
            .get_resource::<LensEffects>()
            .map(|le| LensEffectsState {
                vignette_enabled: le.vignette_enabled,
                vignette_intensity: le.vignette_intensity,
                chromatic_aberration_enabled: le.chromatic_aberration_enabled,
                chromatic_aberration_intensity: le.chromatic_aberration_intensity,
            });

        let bloom = world.get_resource::<BloomSettings>().map(|bs| BloomState {
            enabled: bs.enabled,
            intensity: bs.intensity,
            threshold: bs.threshold,
            knee: bs.knee,
            mip_count: bs.mip_count,
        });

        let auto_exposure = world
            .get_resource::<AutoExposure>()
            .map(|ae| AutoExposureState {
                enabled: ae.enabled,
                min_ev: ae.min_ev,
                max_ev: ae.max_ev,
                adaptation_speed_up: ae.adaptation_speed_up,
                adaptation_speed_down: ae.adaptation_speed_down,
                low_percent: ae.low_percent,
                high_percent: ae.high_percent,
            });

        let camera = world
            .get_resource::<Camera>()
            .map(|c| CameraState {
                pivot: [c.pivot.x, c.pivot.y, c.pivot.z],
                yaw: c.yaw,
                pitch: c.pitch,
                distance: c.distance,
                fov_y: c.fov_y.0,
                position: None,
                direction: None,
                up: None,
                physical_camera: physical_camera.clone(),
                exposure: exposure.clone(),
                depth_of_field: depth_of_field.clone(),
                tone_mapping,
                lens_effects,
                bloom,
                auto_exposure,
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

        let panel_layout = world
            .get_resource::<PanelLayout>()
            .map(|l| PanelLayoutState {
                hierarchy_width: l.hierarchy_width,
                inspector_width: l.inspector_width,
                timeline_height: l.timeline_height,
                debug_height: l.debug_height,
            });

        Self {
            model_path,
            camera,
            timeline,
            editor,
            current_clip_name,
            panel_layout,
        }
    }
}

fn collect_timeline_and_clip(world: &World) -> (TimelineConfig, Option<String>) {
    let timeline_state = world.get_resource::<TimelineState>();
    let clip_library = world.get_resource::<ClipLibrary>();

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
    let current_clip_name = current_clip_id
        .and_then(|id| clip_library.and_then(|cm| cm.get(id).map(|c| c.name.clone())));

    (timeline, current_clip_name)
}

fn collect_previous_metadata(world: &World) -> Option<SceneMetadata> {
    world
        .get_resource::<SceneState>()
        .and_then(|s| s.previous_metadata.clone())
}

fn save_animation_clips(
    world: &World,
    animations_dir: &Path,
) -> SceneResult<Vec<AnimationClipRef>> {
    let clip_library = match world.get_resource::<ClipLibrary>() {
        Some(cm) => cm,
        None => return Ok(Vec::new()),
    };

    let mut animation_clips = Vec::new();
    let mut saved_paths = std::collections::HashSet::new();

    for (clip_id, clip_name) in
        crate::ecs::systems::clip_library_systems::clip_library_clip_names(&clip_library)
    {
        if let Some(clip) = clip_library.get(clip_id) {
            let clip_filename = sanitize_filename(&clip_name);
            let relative_path = format!("animations/{}.anim.ron", clip_filename);

            let clip_path = animations_dir.join(format!("{}.anim.ron", clip_filename));
            save_animation_clip(&clip_path, clip)?;

            if saved_paths.insert(relative_path.clone()) {
                animation_clips.push(AnimationClipRef::new(&relative_path));
            }
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
    scene.panel_layout = collected.panel_layout;

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

    if scene.version != SCENE_FORMAT_VERSION
        && scene.version != 1
        && scene.version != 2
        && scene.version != 3
    {
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
    let mut loaded_paths = std::collections::HashSet::new();
    for clip_ref in &scene.animation_clips {
        if !loaded_paths.insert(clip_ref.path.clone()) {
            continue;
        }
        let clip_path = assets_dir.join(&clip_ref.path);
        let mut clip = load_animation_clip(&clip_path)?;
        clip.source_path = Some(clip_path.to_string_lossy().to_string());
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
    clips_with_ids: &[(SourceClipId, String)],
) {
    if let Some(mut camera) = world.get_resource_mut::<Camera>() {
        if let Some(pos) = loaded.scene.camera.position {
            use crate::ecs::systems::camera_systems::create_camera;
            let position = cgmath::Vector3::new(pos[0], pos[1], pos[2]);
            let target = cgmath::Vector3::new(0.0, 0.0, 0.0);
            *camera = create_camera(position, target);
        } else {
            camera.pivot = cgmath::Vector3::new(
                loaded.scene.camera.pivot[0],
                loaded.scene.camera.pivot[1],
                loaded.scene.camera.pivot[2],
            );
            camera.yaw = loaded.scene.camera.yaw;
            camera.pitch = loaded.scene.camera.pitch;
            camera.distance = loaded.scene.camera.distance;
            camera.fov_y = cgmath::Deg(loaded.scene.camera.fov_y);

            camera.initial_pivot = camera.pivot;
            camera.initial_yaw = camera.yaw;
            camera.initial_pitch = camera.pitch;
            camera.initial_distance = camera.distance;
        }
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

    if let Some(ref phys) = loaded.scene.camera.physical_camera {
        if let Some(mut params) = world.get_resource_mut::<PhysicalCameraParameters>() {
            params.focal_length_mm = phys.focal_length_mm;
            params.sensor_height_mm = phys.sensor_height_mm;
            params.aperture_f_stops = phys.aperture_f_stops;
            params.shutter_speed_s = phys.shutter_speed_s;
            params.sensitivity_iso = phys.sensitivity_iso;
        }
    }

    if let Some(ref exp) = loaded.scene.camera.exposure {
        if let Some(mut exposure) = world.get_resource_mut::<Exposure>() {
            exposure.ev100 = exp.ev100;
            exposure.exposure_value = exp.exposure_value;
        }
    }

    if let Some(ref dof) = loaded.scene.camera.depth_of_field {
        if let Some(mut depth_of_field) = world.get_resource_mut::<DepthOfField>() {
            depth_of_field.enabled = dof.enabled;
            depth_of_field.focus_distance = dof.focus_distance;
            depth_of_field.max_blur_radius = dof.max_blur_radius;
        }
    }

    if let Some(ref tm) = loaded.scene.camera.tone_mapping {
        if let Some(mut tone_mapping) = world.get_resource_mut::<ToneMapping>() {
            tone_mapping.enabled = tm.enabled;
            tone_mapping.operator = match tm.operator.as_str() {
                "AcesFilmic" => ToneMapOperator::AcesFilmic,
                "Reinhard" => ToneMapOperator::Reinhard,
                _ => ToneMapOperator::None,
            };
            tone_mapping.gamma = tm.gamma;
        }
    }

    if let Some(ref le) = loaded.scene.camera.lens_effects {
        if let Some(mut lens_effects) = world.get_resource_mut::<LensEffects>() {
            lens_effects.vignette_enabled = le.vignette_enabled;
            lens_effects.vignette_intensity = le.vignette_intensity;
            lens_effects.chromatic_aberration_enabled = le.chromatic_aberration_enabled;
            lens_effects.chromatic_aberration_intensity = le.chromatic_aberration_intensity;
        }
    }

    if let Some(ref bs) = loaded.scene.camera.bloom {
        if let Some(mut bloom_settings) = world.get_resource_mut::<BloomSettings>() {
            bloom_settings.enabled = bs.enabled;
            bloom_settings.intensity = bs.intensity;
            bloom_settings.threshold = bs.threshold;
            bloom_settings.knee = bs.knee;
            bloom_settings.mip_count = bs.mip_count;
        }
    }

    if let Some(ref ae) = loaded.scene.camera.auto_exposure {
        if let Some(mut auto_exposure) = world.get_resource_mut::<AutoExposure>() {
            auto_exposure.enabled = ae.enabled;
            auto_exposure.min_ev = ae.min_ev;
            auto_exposure.max_ev = ae.max_ev;
            auto_exposure.adaptation_speed_up = ae.adaptation_speed_up;
            auto_exposure.adaptation_speed_down = ae.adaptation_speed_down;
            auto_exposure.low_percent = ae.low_percent;
            auto_exposure.high_percent = ae.high_percent;
        }
    }

    if let Some(ref pl) = loaded.scene.panel_layout {
        if let Some(mut layout) = world.get_resource_mut::<PanelLayout>() {
            layout.hierarchy_width = pl.hierarchy_width;
            layout.inspector_width = pl.inspector_width;
            layout.timeline_height = pl.timeline_height;
            layout.debug_height = pl.debug_height;
            crate::log!(
                "Restored panel layout: hierarchy={:.0}, inspector={:.0}, timeline={:.0}, debug={:.0}",
                pl.hierarchy_width,
                pl.inspector_width,
                pl.timeline_height,
                pl.debug_height,
            );
        }
    }
}
