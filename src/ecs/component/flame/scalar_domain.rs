use std::sync::OnceLock;

use thyllore_effect_core::{
    ScalarCodeBlock, ScalarParam, UiParam, FLAME_SCALAR_CODES, FLAME_SCALAR_PARAMS, FLAME_UI_PARAMS,
};

use super::effect::FlameEffect;
use crate::ecs::component::{
    build_effect_scalar_channels, effect_scalar_domain, ScalarChannel, ScalarChannelDomain,
    ScalarDomainSource,
};

pub struct FlameScalarSource;

impl ScalarDomainSource for FlameScalarSource {
    type Component = FlameEffect;

    const NAME: &'static str = "Flame";
    const CODE_BLOCK: ScalarCodeBlock = FLAME_SCALAR_CODES;

    fn scalars() -> &'static [ScalarParam<FlameEffect>] {
        &FLAME_SCALAR_PARAMS
    }

    fn ui() -> &'static [UiParam] {
        &FLAME_UI_PARAMS
    }

    fn local_time(component: &FlameEffect) -> f32 {
        component.time
    }
}

fn flame_channels() -> &'static [ScalarChannel] {
    static CHANNELS: OnceLock<Vec<ScalarChannel>> = OnceLock::new();
    CHANNELS.get_or_init(build_effect_scalar_channels::<FlameScalarSource>)
}

pub static FLAME_DOMAIN: ScalarChannelDomain =
    effect_scalar_domain::<FlameScalarSource>(flame_channels);

crate::scalar_channel_domain!(FLAME_DOMAIN);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::{find_scalar_param_for_property, scalar_channel_for_scene_name};

    #[test]
    fn test_former_scene_names_resolve_to_their_channels() {
        for (former_name, cli_name) in [
            ("WarpAmp", "warp_amp"),
            ("WarpFreq", "warp_freq"),
            ("NoiseAmplitude", "noise_amplitude"),
            ("WindX", "wind_direction_x"),
            ("WindZ", "wind_direction_y"),
            ("EdgeLow", "edge_low"),
            ("EdgeHigh", "edge_high"),
        ] {
            let (domain, channel) = scalar_channel_for_scene_name(former_name)
                .unwrap_or_else(|| panic!("{former_name} resolves to a channel"));
            assert_eq!(domain.name, FLAME_DOMAIN.name);
            assert_eq!(channel.cli_name, cli_name);
        }
    }

    #[test]
    fn test_every_channel_writes_and_reads_its_own_field() {
        let mut effect = FlameEffect::default();
        for index in 0..FLAME_DOMAIN.channels().len() {
            let property_type = FLAME_DOMAIN.property_type_at(index);
            let scalar = find_scalar_param_for_property::<FlameScalarSource>(property_type)
                .expect("every flame channel resolves to a scalar");
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
