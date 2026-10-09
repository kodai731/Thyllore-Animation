use crate::asset::AssetStorage;
use crate::ecs::component::{CameraAimTarget, CameraParam, CAMERA_DOMAIN};
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{ActiveCamera, Camera, HelmState, TimelineState};
use crate::ecs::systems::{
    compute_camera_direction, compute_camera_position, ensure_entity_clip, plan_camera_shot,
    resolve_transform_entity, scalar_clip_insert_key,
};
use crate::ecs::world::{Entity, World};
use crate::helm::components::tool_call::{ShotPreset, SpeedPreset};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use cgmath::Vector3;

#[derive(Clone, Debug)]
pub enum CameraRigEvent {
    Shot {
        preset: ShotPreset,
        speed: SpeedPreset,
        target: Option<Entity>,
    },
    Direction {
        utterance: String,
        target: Option<Entity>,
    },
    SetActive(Option<Entity>),
}

impl UiCommand for CameraRigEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            CameraRigEvent::Shot {
                preset,
                speed,
                target,
            } => handle_camera_shot(world, assets, preset, speed, target),
            CameraRigEvent::Direction { utterance, target } => {
                handle_camera_direction(world, assets, &utterance, target)
            }
            CameraRigEvent::SetActive(target) => {
                if let Some(mut active_camera) = world.get_resource_mut::<ActiveCamera>() {
                    active_camera.0 = target;
                }
            }
        }
    }
}

fn handle_camera_shot(
    world: &mut World,
    assets: &mut AssetStorage,
    preset: ShotPreset,
    speed: SpeedPreset,
    target: Option<Entity>,
) {
    let is_look_at_orbit = matches!(
        preset,
        ShotPreset::LookAtSelection | ShotPreset::OrbitAroundSelection
    );

    if is_look_at_orbit {
        let active_camera = world.resource::<ActiveCamera>();
        if let (Some(camera_entity), Some(target_entity)) = (active_camera.0, target) {
            drop(active_camera);
            let resolved_target = resolve_transform_entity(world, target_entity);
            world.insert_component(camera_entity, CameraAimTarget::look_at(resolved_target));
            return;
        }
    }

    let is_dolly_crane = matches!(
        preset,
        ShotPreset::DollyIn | ShotPreset::DollyOut | ShotPreset::CraneUp | ShotPreset::CraneDown
    );

    if is_dolly_crane {
        let active_camera = world.resource::<ActiveCamera>();
        if let Some(camera_entity) = active_camera.0 {
            drop(active_camera);

            let camera = world.resource::<Camera>();
            let forward = compute_camera_direction(&camera);
            let current_pos = compute_camera_position(&camera);
            drop(camera);

            let duration = match speed {
                SpeedPreset::Slow => 2.0,
                SpeedPreset::Normal => 1.0,
                SpeedPreset::Fast => 0.5,
            };

            let target_pos = match preset {
                ShotPreset::DollyIn => current_pos + forward * 3.0,
                ShotPreset::DollyOut => current_pos - forward * 3.0,
                ShotPreset::CraneUp => {
                    Vector3::new(current_pos.x, current_pos.y + 3.0, current_pos.z)
                }
                ShotPreset::CraneDown => {
                    Vector3::new(current_pos.x, current_pos.y - 3.0, current_pos.z)
                }
                _ => unreachable!(),
            };

            let timeline = world.resource::<TimelineState>();
            let current_time = timeline.current_time;
            let end_time = current_time + duration;
            drop(timeline);

            let clip_id = ensure_entity_clip(world, assets, camera_entity, &CAMERA_DOMAIN);

            for param in [
                CameraParam::TranslationX,
                CameraParam::TranslationY,
                CameraParam::TranslationZ,
            ] {
                let property_type = param.property_type();
                let current_value = (CAMERA_DOMAIN.read)(world, camera_entity, property_type)
                    .unwrap_or(match param {
                        CameraParam::TranslationX => current_pos.x,
                        CameraParam::TranslationY => current_pos.y,
                        CameraParam::TranslationZ => current_pos.z,
                        _ => 0.0,
                    });
                let target_value = match param {
                    CameraParam::TranslationX => target_pos.x,
                    CameraParam::TranslationY => target_pos.y,
                    CameraParam::TranslationZ => target_pos.z,
                    _ => 0.0,
                };

                super::scalar_curve::edit_clip(world, clip_id, "Camera shot", |clip| {
                    scalar_clip_insert_key(clip, property_type, current_time, current_value);
                    scalar_clip_insert_key(clip, property_type, end_time, target_value);
                });
            }

            return;
        }
    }

    let target_pos = if let Some(entity) = target {
        let transform_entity = resolve_transform_entity(world, entity);
        world
            .get_component::<crate::ecs::world::Transform>(transform_entity)
            .map(|t| t.translation)
    } else {
        None
    };

    let camera = world.resource::<Camera>();
    let tween = plan_camera_shot(&camera, preset, speed, target_pos);
    drop(camera);

    let mut motion = world.resource_mut::<crate::ecs::systems::CameraShotMotion>();
    motion.active = Some(tween);
}

fn handle_camera_direction(
    world: &mut World,
    assets: &mut AssetStorage,
    utterance: &str,
    target: Option<Entity>,
) {
    let active_camera = world.resource::<ActiveCamera>();
    let camera_entity = match active_camera.0 {
        Some(e) => e,
        None => {
            log_warn!("camera_direction: no active camera for '{}'", utterance);
            let mut state = world.resource_mut::<HelmState>();
            state.feedback = Some(crate::ecs::resource::CommandFeedback::Report(
                "camera_direction: no active camera".to_string(),
            ));
            return;
        }
    };
    drop(active_camera);

    let paths = match thyllore_ml_core::model_path::resolve_camera_direction_model_paths() {
        Some(p) => p,
        None => {
            log_error!("camera_direction: model not found for '{}'", utterance);
            let mut state = world.resource_mut::<HelmState>();
            state.feedback = Some(crate::ecs::resource::CommandFeedback::Report(
                "camera_direction: model not found".to_string(),
            ));
            return;
        }
    };

    let Some(caption) =
        thyllore_ml_core::copilot::camera_direction::caption::build_movement_caption(utterance)
    else {
        log_warn!(
            "camera_direction: no movement keyword recognized in '{}'",
            utterance
        );
        let mut state = world.resource_mut::<HelmState>();
        state.feedback = Some(crate::ecs::resource::CommandFeedback::Report(format!(
            "camera_direction: no movement keyword recognized in '{}'",
            utterance
        )));
        return;
    };

    let poses = match thyllore_ml_core::copilot::camera_direction::generate_camera_poses(
        &paths, &caption,
    ) {
        Ok(p) => p,
        Err(e) => {
            log_error!(
                "camera_direction: generation failed for '{}' (caption '{}'): {}",
                utterance,
                caption,
                e
            );
            let mut state = world.resource_mut::<HelmState>();
            state.feedback = Some(crate::ecs::resource::CommandFeedback::DispatchError(
                format!("camera_direction: {}", e),
            ));
            return;
        }
    };

    let camera_to_world = {
        let camera = world.resource::<Camera>();
        camera_to_world_matrix(&camera)
    };
    let world_poses =
        thyllore_ml_core::copilot::camera_direction::keyframes::transform_poses_to_world(
            &camera_to_world,
            &poses,
        );
    let tuples = thyllore_ml_core::copilot::camera_direction::keyframes::poses_to_keyframe_tuples(
        &world_poses,
        30.0,
        5,
    );

    let current_time = world.resource::<TimelineState>().current_time;

    let clip_id = ensure_entity_clip(world, assets, camera_entity, &CAMERA_DOMAIN);

    let n = tuples.len();

    super::scalar_curve::edit_clip(world, clip_id, "Camera direction", |clip| {
        for (time, param, value) in tuples {
            scalar_clip_insert_key(
                clip,
                camera_param_from_key(param).property_type(),
                current_time + time,
                value,
            );
        }
    });

    log!(
        "camera_direction: {} keys from '{}' (caption '{}')",
        n,
        utterance,
        caption
    );
    let mut state = world.resource_mut::<HelmState>();
    state.feedback = Some(crate::ecs::resource::CommandFeedback::Executed(format!(
        "camera_direction: {} keys from '{}'",
        n, caption
    )));
}

fn camera_to_world_matrix(camera: &Camera) -> [[f32; 4]; 4] {
    let right = crate::ecs::systems::compute_camera_right(camera);
    let up = crate::ecs::systems::compute_camera_up(camera);
    let backward = -compute_camera_direction(camera);
    let position = compute_camera_position(camera);
    [
        [right.x, up.x, backward.x, position.x],
        [right.y, up.y, backward.y, position.y],
        [right.z, up.z, backward.z, position.z],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn camera_param_from_key(
    key: thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam,
) -> CameraParam {
    match key {
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::TranslationX => {
            CameraParam::TranslationX
        }
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::TranslationY => {
            CameraParam::TranslationY
        }
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::TranslationZ => {
            CameraParam::TranslationZ
        }
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::RotationX => {
            CameraParam::RotationX
        }
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::RotationY => {
            CameraParam::RotationY
        }
        thyllore_ml_core::copilot::camera_direction::keyframes::CameraKeyParam::RotationZ => {
            CameraParam::RotationZ
        }
    }
}
