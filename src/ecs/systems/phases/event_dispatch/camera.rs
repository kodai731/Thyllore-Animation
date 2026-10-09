use cgmath::Vector3;

use crate::asset::AssetStorage;
use crate::ecs::events::{DebugPrimitiveKind, UiCommand};
use crate::ecs::resource::{Camera, OutputCommand, OutputQueue, SceneLoadCommand, SceneLoadQueue};
use crate::ecs::systems::{camera_move_to_look_at, camera_reset};
use crate::ecs::world::World;
use crate::hooks::batch_capture::BatchCapture;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum CameraEvent {
    LoadModel {
        path: String,
    },
    LoadModelAdditive {
        path: String,
    },
    SpawnDebugPrimitive {
        kind: DebugPrimitiveKind,
    },
    ResetCamera,
    ResetCameraUp,
    MoveCameraToModel,
    TakeScreenshot,
    #[cfg(debug_assertions)]
    DebugShadowInfo,
    #[cfg(debug_assertions)]
    DebugBillboardDepth,
    DumpDebugInfo,
    DumpAnimationDebug,
    /// Runs one readback right away, outside the batch schedule (a debug window button).
    CaptureNow(Rc<dyn BatchCapture>),
}

impl UiCommand for CameraEvent {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        _: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        dispatch_camera_events(&[*self], world, graphics);
    }
}

fn dispatch_camera_events(events: &[CameraEvent], world: &mut World, graphics: &GraphicsResources) {
    let mut camera = world.resource_mut::<Camera>();
    let mut scene_loads = world.resource_mut::<SceneLoadQueue>();
    let mut outputs = world.resource_mut::<OutputQueue>();

    for event in events {
        match event {
            CameraEvent::ResetCamera | CameraEvent::ResetCameraUp => {
                camera_reset(&mut camera);
            }

            CameraEvent::MoveCameraToModel => {
                if let Some((min, max, center)) = graphics.calculate_model_bounds() {
                    let size = max - min;
                    let max_dim = size.x.max(size.y).max(size.z);
                    let distance = max_dim * 2.0;
                    let offset = Vector3::new(0.0, 0.0, distance);
                    camera_move_to_look_at(&mut camera, center, offset);
                    log!(
                        "Moved camera to model: center=({:.2}, {:.2}, {:.2}), distance={:.2}",
                        center.x,
                        center.y,
                        center.z,
                        distance
                    );
                }
            }

            CameraEvent::LoadModel { path } => {
                scene_loads.push(SceneLoadCommand::LoadModel { path: path.clone() });
            }

            CameraEvent::LoadModelAdditive { path } => {
                scene_loads.push(SceneLoadCommand::LoadModelAdditive { path: path.clone() });
            }

            CameraEvent::SpawnDebugPrimitive { kind } => {
                scene_loads.push(SceneLoadCommand::SpawnDebugPrimitive { kind: *kind });
            }

            CameraEvent::TakeScreenshot => {
                outputs.push(OutputCommand::TakeScreenshot);
            }

            #[cfg(debug_assertions)]
            CameraEvent::DebugShadowInfo => {
                outputs.push(OutputCommand::DebugShadowInfo);
            }

            #[cfg(debug_assertions)]
            CameraEvent::DebugBillboardDepth => {
                outputs.push(OutputCommand::DebugBillboardDepth);
            }

            CameraEvent::DumpDebugInfo => {
                outputs.push(OutputCommand::DumpDebugInfo);
            }

            CameraEvent::DumpAnimationDebug => {
                outputs.push(OutputCommand::DumpAnimationDebug);
            }

            CameraEvent::CaptureNow(capture) => {
                outputs.push(OutputCommand::CaptureNow(capture.clone()));
            }
        }
    }
}
