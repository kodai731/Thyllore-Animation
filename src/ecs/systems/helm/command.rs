use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::camera_rig::CameraRigEvent;
use crate::ecs::systems::phases::event_dispatch::edit_history::EditHistoryEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::phases::event_dispatch::scene::SceneEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::{Entity, Visibility, World};
use crate::helm::components::tool_call::{ShotPreset, SpeedPreset};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug, PartialEq)]
pub enum HelmCommand {
    TakeScreenshot,
    Play,
    Pause,
    Stop,
    ToggleLoop,
    SetSpeed(f32),
    SetTime(f32),
    SelectEntity(Entity),
    Undo,
    Redo,
    SaveScene,
    SetEntityVisible(Entity, Visibility),
    MoveCameraToModel,
    ResetCamera,
    FocusOnEntity(Entity),
    CameraShot {
        preset: ShotPreset,
        speed: SpeedPreset,
        target: Option<Entity>,
    },
}

impl UiCommand for HelmCommand {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        match *self {
            HelmCommand::TakeScreenshot => {
                Box::new(CameraEvent::TakeScreenshot).apply(world, assets, graphics);
            }
            HelmCommand::Play => {
                Box::new(TimelineEvent::Play).apply(world, assets, graphics);
            }
            HelmCommand::Pause => {
                Box::new(TimelineEvent::Pause).apply(world, assets, graphics);
            }
            HelmCommand::Stop => {
                Box::new(TimelineEvent::Stop).apply(world, assets, graphics);
            }
            HelmCommand::ToggleLoop => {
                Box::new(TimelineEvent::ToggleLoop).apply(world, assets, graphics);
            }
            HelmCommand::SetSpeed(speed) => {
                Box::new(TimelineEvent::SetSpeed(speed)).apply(world, assets, graphics);
            }
            HelmCommand::SetTime(time) => {
                Box::new(TimelineEvent::SetTime(time)).apply(world, assets, graphics);
            }
            HelmCommand::SelectEntity(entity) => {
                Box::new(HierarchyEvent::SelectEntity(entity)).apply(world, assets, graphics);
            }
            HelmCommand::Undo => {
                Box::new(EditHistoryEvent::Undo).apply(world, assets, graphics);
            }
            HelmCommand::Redo => {
                Box::new(EditHistoryEvent::Redo).apply(world, assets, graphics);
            }
            HelmCommand::SaveScene => {
                Box::new(SceneEvent::SaveScene).apply(world, assets, graphics);
            }
            HelmCommand::SetEntityVisible(entity, visibility) => {
                Box::new(HierarchyEvent::SetEntityVisible(entity, visibility))
                    .apply(world, assets, graphics);
            }
            HelmCommand::MoveCameraToModel => {
                Box::new(CameraEvent::MoveCameraToModel).apply(world, assets, graphics);
            }
            HelmCommand::ResetCamera => {
                Box::new(CameraEvent::ResetCamera).apply(world, assets, graphics);
            }
            HelmCommand::FocusOnEntity(entity) => {
                Box::new(HierarchyEvent::FocusOnEntity(entity)).apply(world, assets, graphics);
            }
            HelmCommand::CameraShot {
                preset,
                speed,
                target,
            } => {
                Box::new(CameraRigEvent::Shot {
                    preset,
                    speed,
                    target,
                })
                .apply(world, assets, graphics);
            }
        }
    }
}
