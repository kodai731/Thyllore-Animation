use crate::wind::analytic::WindShellParams;
use crate::wind::{build_wind_model_matrix, WindTornadoEffect, WindUBO};
use cgmath::{Matrix4, SquareMatrix};

pub fn build_wind_ubo(effect: &WindTornadoEffect) -> WindUBO {
    let model = build_wind_model_matrix(effect);
    let inverse_model = model.invert().unwrap_or(Matrix4::identity());
    let params = WindShellParams::from_effect(effect);

    WindUBO {
        model,
        inverse_model,
        shape: [
            params.height,
            params.wall_radius_base,
            params.wall_radius_slope,
            params.wall_width_q,
        ],
        core: [
            params.core_radius_sq,
            params.core_strength,
            params.wall_strength,
            params.top_fade,
        ],
        optics: [
            params.sigma_t,
            effect.ambient_brightness,
            effect.time,
            params.h_top,
        ],
        albedo: [
            effect.albedo[0],
            effect.albedo[1],
            effect.albedo[2],
            params.spread_offset,
        ],
        ring: [
            params.ring_height,
            params.ring_radius_sq,
            params.ring_width_q,
            params.ring_strength,
        ],
        lighting: [effect.phase_g, effect.sun_intensity, 0.0, 0.0],
        streak: [
            params.streak_order,
            params.streak_twist,
            params.streak_rise_speed,
            params.streak_amplitude,
        ],
        streak2: [params.streak_phase, params.streak_rise_time, 0.0, 0.0],
        eddy: [
            params.eddy_amplitude,
            params.eddy_cell_theta,
            params.eddy_cell_height,
            params.eddy_cell_radial,
        ],
        eddy2: [
            params.eddy_shear,
            params.eddy_rise_speed,
            params.eddy_reseed_period,
            0.0,
        ],
        layers: [
            params.layer_count as f32,
            params.layer_spacing_q,
            params.layer_decay,
            0.0,
        ],
        puff_params: [params.puff_count as f32, effect.puff_strength, 0.0, 0.0],
        puffs: params.puffs,
        inv_view_proj: Matrix4::identity(),
    }
}

impl Default for WindUBO {
    fn default() -> Self {
        build_wind_ubo(&WindTornadoEffect::default())
    }
}
