use std::sync::OnceLock;

use thyllore_effect_core::{
    ScalarCodeBlock, ScalarParam, UiParam, WATER_SCALAR_CODES, WATER_SCALAR_PARAMS, WATER_UI_PARAMS,
};

use super::effect::WaterTorusEffect;
use crate::ecs::component::{
    build_effect_scalar_channels, effect_scalar_domain, ScalarChannel, ScalarChannelDomain,
    ScalarDomainSource,
};

pub struct WaterScalarSource;

impl ScalarDomainSource for WaterScalarSource {
    type Component = WaterTorusEffect;

    const NAME: &'static str = "Water";
    const CODE_BLOCK: ScalarCodeBlock = WATER_SCALAR_CODES;

    fn scalars() -> &'static [ScalarParam<WaterTorusEffect>] {
        &WATER_SCALAR_PARAMS
    }

    fn ui() -> &'static [UiParam] {
        &WATER_UI_PARAMS
    }

    fn local_time(component: &WaterTorusEffect) -> f32 {
        component.time
    }
}

fn water_channels() -> &'static [ScalarChannel] {
    static CHANNELS: OnceLock<Vec<ScalarChannel>> = OnceLock::new();
    CHANNELS.get_or_init(build_effect_scalar_channels::<WaterScalarSource>)
}

pub static WATER_DOMAIN: ScalarChannelDomain =
    effect_scalar_domain::<WaterScalarSource>(water_channels);

crate::scalar_channel_domain!(WATER_DOMAIN);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::find_scalar_param_for_property;

    #[test]
    fn test_every_channel_writes_and_reads_its_own_field() {
        let mut effect = WaterTorusEffect::default();
        for index in 0..WATER_DOMAIN.channels().len() {
            let property_type = WATER_DOMAIN.property_type_at(index);
            let scalar = find_scalar_param_for_property::<WaterScalarSource>(property_type)
                .expect("every water channel resolves to a scalar");
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
