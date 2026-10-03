use std::sync::OnceLock;

use thyllore_effect_core::{ScalarParam, UiParam, LIGHTNING_SCALAR_PARAMS, LIGHTNING_UI_PARAMS};

use super::effect::LightningEffect;
use crate::ecs::component::{
    build_effect_scalar_channels, effect_scalar_domain, ScalarChannel, ScalarChannelDomain,
    ScalarDomainSource,
};

pub struct LightningScalarSource;

impl ScalarDomainSource for LightningScalarSource {
    type Component = LightningEffect;

    const NAME: &'static str = "Lightning";
    const CHANNEL_ORDER: &'static [&'static str] = &[
        "shape_end_offset_x",
        "shape_end_offset_y",
        "shape_end_offset_z",
        "shape_strikes_per_burst",
        "shape_detail_levels",
        "shape_tortuosity",
        "shape_roughness",
        "shape_core_radius",
        "shape_tip_radius_ratio",
        "shape_edge_fraction",
        "shape_end_variance",
        "branch_depth",
        "branch_probability",
        "branch_count",
        "branch_zone_start",
        "branch_zone_end",
        "branch_angle",
        "branch_length_ratio",
        "branch_radius_ratio",
        "branch_intensity_ratio",
        "look_core_intensity",
        "look_core_color_r",
        "look_core_color_g",
        "look_core_color_b",
        "look_rim_ratio",
        "look_rim_intensity",
        "look_rim_color_r",
        "look_rim_color_g",
        "look_rim_color_b",
        "look_beam_radius",
        "look_beam_arc_count",
        "look_flash_gain",
        "look_flash_radius",
        "timing_burst_start",
        "timing_burst_interval",
        "timing_burst_jitter",
        "timing_burst_count",
        "timing_attack_time",
        "timing_sustain_time",
        "timing_release_time",
        "timing_stroke_count",
        "timing_stroke_interval",
        "timing_stroke_decay",
        "timing_flicker_amplitude",
        "timing_flicker_period",
        "timing_reseed_level",
        "timing_reseed_period",
        "timing_charge_ramp",
        "timing_growth_time",
    ];

    fn scalars() -> &'static [ScalarParam<LightningEffect>] {
        &LIGHTNING_SCALAR_PARAMS
    }

    fn ui() -> &'static [UiParam] {
        &LIGHTNING_UI_PARAMS
    }

    fn local_time(component: &LightningEffect) -> f32 {
        component.time
    }
}

fn lightning_channels() -> &'static [ScalarChannel] {
    static CHANNELS: OnceLock<Vec<ScalarChannel>> = OnceLock::new();
    CHANNELS.get_or_init(build_effect_scalar_channels::<LightningScalarSource>)
}

pub static LIGHTNING_DOMAIN: ScalarChannelDomain =
    effect_scalar_domain::<LightningScalarSource>(lightning_channels);

crate::scalar_channel_domain!(LIGHTNING_DOMAIN);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::find_scalar_param_for_property;

    #[test]
    fn test_every_channel_writes_and_reads_its_own_field() {
        let mut effect = LightningEffect::default();
        for index in 0..LIGHTNING_DOMAIN.channels().len() {
            let property_type = LIGHTNING_DOMAIN.property_type_at(index);
            let scalar = find_scalar_param_for_property::<LightningScalarSource>(property_type)
                .expect("every lightning channel resolves to a scalar");
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
