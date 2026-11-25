use crate::flame::*;
use thyllore_scene_core::UiCondition;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlameEmitterKind {
    #[default]
    Cylinder,
    Ring,
    MeshSdf,
}

impl FlameEmitterKind {
    pub const ALL: [FlameEmitterKind; 3] = [
        FlameEmitterKind::Cylinder,
        FlameEmitterKind::Ring,
        FlameEmitterKind::MeshSdf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlameEmitterKind::Cylinder => "Cylinder",
            FlameEmitterKind::Ring => "Ring",
            FlameEmitterKind::MeshSdf => "Mesh SDF",
        }
    }

    pub fn as_shader_value(self) -> u32 {
        match self {
            FlameEmitterKind::Cylinder => 0,
            FlameEmitterKind::Ring => 1,
            FlameEmitterKind::MeshSdf => 2,
        }
    }

    pub fn from_shader_value(value: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_shader_value() == value)
    }
}

pub static EMITTER_IS_NOT_RING: UiCondition = UiCondition {
    description: "the emitter is not a ring, so the ring parameters have no effect",
};

pub fn emitter_ui_conditions(emitter: &FlameEmitter) -> Vec<&'static UiCondition> {
    match emitter.kind {
        FlameEmitterKind::Ring => Vec::new(),
        FlameEmitterKind::Cylinder | FlameEmitterKind::MeshSdf => vec![&EMITTER_IS_NOT_RING],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner)]
pub struct FlameEmitter {
    #[runtime]
    pub kind: FlameEmitterKind,
    #[runtime(ui(
        primary,
        min = 0.2,
        max = 5.0,
        label = "Ring Radius",
        hidden_when = EMITTER_IS_NOT_RING
    ))]
    pub ring_major_radius: f32,
    #[runtime(ui(
        primary,
        min = 0.0,
        max = std::f32::consts::TAU,
        label = "Ring Speed",
        hidden_when = EMITTER_IS_NOT_RING
    ))]
    pub ring_angular_speed: f32,
}

impl Default for FlameEmitter {
    fn default() -> Self {
        Self {
            kind: FlameEmitterKind::default(),
            ring_major_radius: 1.0,
            ring_angular_speed: 0.6,
        }
    }
}

const SDF_SLAB_DEPTH: f32 = 0.15;

pub fn emitter_bounding_radius(emitter: &FlameEmitter, radius: f32) -> f32 {
    match emitter.kind {
        FlameEmitterKind::Ring => emitter.ring_major_radius + radius,
        FlameEmitterKind::Cylinder | FlameEmitterKind::MeshSdf => radius,
    }
}

pub fn build_emitter_params(emitter: &FlameEmitter, radius: f32) -> FlameEmitterParams {
    FlameEmitterParams {
        kind: emitter.kind.as_shader_value() as f32,
        ring_major_ratio: match emitter.kind {
            FlameEmitterKind::Ring => {
                emitter.ring_major_radius / emitter_bounding_radius(emitter, radius)
            }
            FlameEmitterKind::Cylinder | FlameEmitterKind::MeshSdf => 0.0,
        },
        ring_angular_speed: emitter.ring_angular_speed,
        sdf_slab_depth: match emitter.kind {
            FlameEmitterKind::MeshSdf => SDF_SLAB_DEPTH,
            FlameEmitterKind::Cylinder | FlameEmitterKind::Ring => 0.0,
        },
    }
}
