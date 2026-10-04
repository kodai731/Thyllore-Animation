use cgmath::{Matrix4, SquareMatrix, Vector2};
use thyllore_effect_core::burst_start_time;

use crate::ecs::component::LightningEffect;
use crate::ecs::resource::ProjectionData;

pub(super) fn lightning_inside_first_burst() -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.time =
        burst_start_time(&effect, 0) + effect.timing.attack_time + effect.timing.sustain_time * 0.5;
    effect
}

pub(super) fn orthographic_projection(half_size: f32) -> ProjectionData {
    ProjectionData {
        view: Matrix4::identity(),
        proj: cgmath::ortho(-half_size, half_size, -half_size, half_size, -100.0, 100.0),
        screen_size: Vector2::new(200.0, 200.0),
        aspect: 1.0,
    }
}
