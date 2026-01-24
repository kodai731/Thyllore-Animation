use cgmath::{Quaternion, Vector3};

use crate::app::data::LightMoveTarget;
use crate::ecs::world::Entity;

#[derive(Clone, Debug)]
pub enum UIEvent {
    LoadModel { path: String },

    ResetCamera,
    ResetCameraUp,
    MoveCameraToModel,
    MoveCameraToLightGizmo,

    SetLightPosition(Vector3<f32>),
    MoveLightToBounds(LightMoveTarget),

    TakeScreenshot,

    DebugShadowInfo,
    DebugBillboardDepth,
    DumpDebugInfo,

    SelectEntity(Entity),
    DeselectAll,
    ToggleEntitySelection(Entity),
    ExpandEntity(Entity),
    CollapseEntity(Entity),
    SetSearchFilter(String),

    SetEntityVisible(Entity, bool),
    SetEntityTranslation(Entity, Vector3<f32>),
    SetEntityRotation(Entity, Quaternion<f32>),
    SetEntityScale(Entity, Vector3<f32>),
    RenameEntity(Entity, String),
    FocusOnEntity(Entity),
}

#[derive(Default)]
pub struct UIEventQueue {
    events: Vec<UIEvent>,
}

impl UIEventQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn send(&mut self, event: UIEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = UIEvent> + '_ {
        self.events.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}
