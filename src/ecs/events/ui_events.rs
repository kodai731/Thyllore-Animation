use crate::ecs::component::{FlameEffect, LightningEffect, WaterTorusEffect, WindTornadoEffect};
use crate::ecs::resource::{
    FlameRenderSettings, LightningRenderSettings, WaterRenderSettings, WindRenderSettings,
};
use crate::ecs::world::Entity;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugPrimitiveKind {
    Cube,
    Sphere,
    Floor,
}

impl DebugPrimitiveKind {
    pub const ALL: [DebugPrimitiveKind; 3] = [
        DebugPrimitiveKind::Cube,
        DebugPrimitiveKind::Sphere,
        DebugPrimitiveKind::Floor,
    ];

    /// Stable name persisted in scene files.
    pub fn name(self) -> &'static str {
        match self {
            DebugPrimitiveKind::Cube => "cube",
            DebugPrimitiveKind::Sphere => "sphere",
            DebugPrimitiveKind::Floor => "floor",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

#[derive(Clone, Debug)]
pub enum UIEvent {
    DumpFlameWallProbe {
        viewport_size: [f32; 2],
    },

    UpdateFlameEffect {
        entity: Entity,
        effect: Box<FlameEffect>,
    },
    UpdateFlameBaked(Box<thyllore_effect_core::FlameBaked>),
    ApplyFlamePreset(String),
    ApplyFlameTextureFit {
        path: String,
        blend: f32,
        groups: [bool; 4],
        profile: bool,
    },
    ApplyFlameStyle {
        path: String,
        groups: [bool; 3],
    },
    SaveFlameStyle {
        name: String,
    },
    UpdateFlameRenderSettings(FlameRenderSettings),
    UpdateFlameTrailEnabled(bool),
    UpdateFlameTrailFade(f32),
    UpdateWaterEffect {
        entity: Entity,
        effect: Box<WaterTorusEffect>,
    },
    ApplyWaterPreset(String),
    UpdateWaterRenderSettings(WaterRenderSettings),
    UpdateWindEffect {
        entity: Entity,
        effect: Box<WindTornadoEffect>,
    },
    ApplyWindPreset(String),
    UpdateWindRenderSettings(WindRenderSettings),
    UpdateLightningEffect {
        entity: Entity,
        effect: Box<LightningEffect>,
    },
    ApplyLightningPreset(String),
    AddLightningTarget,
    ClearLightningTarget,
    AddLightningWaypoint,
    RemoveLightningWaypoint(usize),
    UpdateLightningRenderSettings(LightningRenderSettings),
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
