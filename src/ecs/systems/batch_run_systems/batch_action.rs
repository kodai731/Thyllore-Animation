use anyhow::Result;

use crate::ecs::events::{DebugPrimitiveKind, UIEvent, UIEventQueue};
use crate::ecs::resource::{DebugViewMode, DebugViewState};
use crate::ecs::world::World;

/// A headless `--batch-debug-action`; implementations register with `batch_action!` from their domain.
pub trait BatchAction: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn apply(&self, world: &World);
}

pub type BatchActionParseFn = fn(&str) -> Option<Result<Box<dyn BatchAction>>>;

pub struct BatchActionDescriptor {
    pub name: &'static str,
    pub parse: BatchActionParseFn,
}

inventory::collect!(BatchActionDescriptor);

/// Registers a `BatchAction` parser under `name` at link time.
#[macro_export]
macro_rules! batch_action {
    ($name:literal, $parse:expr) => {
        inventory::submit! {
            $crate::ecs::systems::BatchActionDescriptor {
                name: $name,
                parse: $parse,
            }
        }
    };
}

/// Parses an action that takes no value: the text must equal the action's name.
pub fn unit_action_parse<A: BatchAction + Default + 'static>(
    text: &str,
) -> Option<Result<Box<dyn BatchAction>>> {
    let action = A::default();
    (text == action.name()).then(|| Ok(Box::new(action) as Box<dyn BatchAction>))
}

/// Every registered action, sorted by name.
pub fn batch_action_registry() -> Vec<&'static BatchActionDescriptor> {
    let mut descriptors: Vec<_> = inventory::iter::<BatchActionDescriptor>
        .into_iter()
        .collect();
    descriptors.sort_by_key(|descriptor| descriptor.name);
    descriptors
}

#[derive(Debug, Default)]
pub struct ResetCamera;

#[derive(Debug, Default)]
pub struct ResetCameraUp;

#[derive(Debug, Default)]
pub struct CameraToModel;

impl BatchAction for ResetCamera {
    fn name(&self) -> &'static str {
        "reset_camera"
    }
    fn apply(&self, world: &World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::ResetCamera);
    }
}

impl BatchAction for ResetCameraUp {
    fn name(&self) -> &'static str {
        "reset_camera_up"
    }
    fn apply(&self, world: &World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::ResetCameraUp);
    }
}

impl BatchAction for CameraToModel {
    fn name(&self) -> &'static str {
        "camera_to_model"
    }
    fn apply(&self, world: &World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::MoveCameraToModel);
    }
}

crate::batch_action!("reset_camera", unit_action_parse::<ResetCamera>);
crate::batch_action!("reset_camera_up", unit_action_parse::<ResetCameraUp>);
crate::batch_action!("camera_to_model", unit_action_parse::<CameraToModel>);

#[derive(Debug)]
pub struct ViewMode(pub DebugViewMode);

#[derive(Debug, Default)]
pub struct BlackBackground;

impl BatchAction for ViewMode {
    fn name(&self) -> &'static str {
        "view_mode"
    }
    fn apply(&self, world: &World) {
        world.resource_mut::<DebugViewState>().debug_view_mode = self.0;
    }
}

impl BatchAction for BlackBackground {
    fn name(&self) -> &'static str {
        "black_background"
    }
    fn apply(&self, world: &World) {
        world.resource_mut::<DebugViewState>().black_background = true;
    }
}

fn debug_view_mode_parse(name: &str) -> Option<DebugViewMode> {
    match name {
        "final" => Some(DebugViewMode::Final),
        "position" => Some(DebugViewMode::Position),
        "normal" => Some(DebugViewMode::Normal),
        "shadow_mask" => Some(DebugViewMode::ShadowMask),
        "ndotl" => Some(DebugViewMode::NdotL),
        "light_direction" => Some(DebugViewMode::LightDirection),
        "view_depth" => Some(DebugViewMode::ViewDepth),
        "object_id" => Some(DebugViewMode::ObjectID),
        "selection_view" => Some(DebugViewMode::SelectionView),
        "selection_ubo" => Some(DebugViewMode::SelectionUBO),
        _ => None,
    }
}

fn view_mode_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let mode_name = text.strip_prefix("view_mode=")?.trim();
    Some(
        debug_view_mode_parse(mode_name)
            .map(|mode| Box::new(ViewMode(mode)) as Box<dyn BatchAction>)
            .ok_or_else(|| anyhow::anyhow!("unknown view_mode '{mode_name}'")),
    )
}

crate::batch_action!("view_mode", view_mode_parse);
crate::batch_action!("black_background", unit_action_parse::<BlackBackground>);

#[derive(Debug)]
pub struct SpawnDebugPrimitive(pub DebugPrimitiveKind);

impl BatchAction for SpawnDebugPrimitive {
    fn name(&self) -> &'static str {
        match self.0 {
            DebugPrimitiveKind::Cube => "spawn_cube",
            DebugPrimitiveKind::Sphere => "spawn_sphere",
            DebugPrimitiveKind::Floor => "spawn_floor",
        }
    }
    fn apply(&self, world: &World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::SpawnDebugPrimitive { kind: self.0 });
    }
}

fn spawn_primitive_parse(
    text: &str,
    kind: DebugPrimitiveKind,
) -> Option<Result<Box<dyn BatchAction>>> {
    let action = SpawnDebugPrimitive(kind);
    (text == action.name()).then(|| Ok(Box::new(action) as Box<dyn BatchAction>))
}

crate::batch_action!("spawn_cube", |text| spawn_primitive_parse(
    text,
    DebugPrimitiveKind::Cube
));
crate::batch_action!("spawn_sphere", |text| spawn_primitive_parse(
    text,
    DebugPrimitiveKind::Sphere
));
crate::batch_action!("spawn_floor", |text| spawn_primitive_parse(
    text,
    DebugPrimitiveKind::Floor
));
