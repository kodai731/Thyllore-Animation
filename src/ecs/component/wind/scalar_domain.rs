use std::sync::OnceLock;

use thyllore_effect_core::{ScalarParam, UiParam, WIND_SCALAR_PARAMS, WIND_UI_PARAMS};

use super::effect::WindTornadoEffect;
use crate::ecs::component::{
    build_effect_scalar_channels, effect_scalar_domain, ScalarChannel, ScalarChannelDomain,
    ScalarDomainSource,
};

pub struct WindScalarSource;

impl ScalarDomainSource for WindScalarSource {
    type Component = WindTornadoEffect;

    const NAME: &'static str = "Wind";
    const CHANNEL_ORDER: &'static [&'static str] = &[
        "column_height",
        "wall_radius_base",
        "wall_radius_top",
        "wall_width_q",
        "wall_strength",
        "top_fade",
        "density",
        "albedo_r",
        "albedo_g",
        "albedo_b",
        "ambient_brightness",
        "phase_g",
        "sun_intensity",
        "rise_initial_height",
        "rise_duration",
        "spread_start",
        "spread_rate",
        "dissipate_start",
        "dissipate_time",
        "circulation",
        "streak_order",
        "streak_twist",
        "streak_rise_speed",
        "streak_amplitude",
        "eddy_amplitude",
        "eddy_cell_theta",
        "eddy_cell_height",
        "eddy_cell_radial",
        "eddy_shear",
        "eddy_speed_spread",
        "eddy_rise_speed",
        "eddy_reseed_period",
        "eddy_erosion",
        "puff_count_theta",
        "puff_count_height",
        "puff_radius",
        "puff_radius_jitter",
        "puff_offset_q",
        "puff_strength",
        "puff_rise_speed",
    ];

    fn scalars() -> &'static [ScalarParam<WindTornadoEffect>] {
        &WIND_SCALAR_PARAMS
    }

    fn ui() -> &'static [UiParam] {
        &WIND_UI_PARAMS
    }

    fn local_time(component: &WindTornadoEffect) -> f32 {
        component.time
    }
}

fn wind_channels() -> &'static [ScalarChannel] {
    static CHANNELS: OnceLock<Vec<ScalarChannel>> = OnceLock::new();
    CHANNELS.get_or_init(build_effect_scalar_channels::<WindScalarSource>)
}

pub static WIND_DOMAIN: ScalarChannelDomain =
    effect_scalar_domain::<WindScalarSource>(wind_channels);

crate::scalar_channel_domain!(WIND_DOMAIN);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::find_scalar_param_for_property;

    #[test]
    fn test_every_channel_writes_and_reads_its_own_field() {
        let mut effect = WindTornadoEffect::default();
        for index in 0..WIND_DOMAIN.channels().len() {
            let property_type = WIND_DOMAIN.property_type_at(index);
            let scalar = find_scalar_param_for_property::<WindScalarSource>(property_type)
                .expect("every wind channel resolves to a scalar");
            let value = 10.0 + index as f32;
            (scalar.set)(&mut effect, value);
            assert!(
                ((scalar.get)(&effect) - value).abs() < 1e-6,
                "{}",
                scalar.name
            );
        }
    }
}
