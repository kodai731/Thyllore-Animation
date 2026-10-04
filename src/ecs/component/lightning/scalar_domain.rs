use std::sync::OnceLock;

use thyllore_effect_core::{
    ScalarCodeBlock, ScalarParam, UiParam, LIGHTNING_SCALAR_CODES, LIGHTNING_SCALAR_PARAMS,
    LIGHTNING_UI_PARAMS,
};

use super::effect::LightningEffect;
use crate::ecs::component::{
    build_effect_scalar_channels, effect_scalar_domain, ScalarChannel, ScalarChannelDomain,
    ScalarDomainSource,
};

pub struct LightningScalarSource;

impl ScalarDomainSource for LightningScalarSource {
    type Component = LightningEffect;

    const NAME: &'static str = "Lightning";
    const CODE_BLOCK: ScalarCodeBlock = LIGHTNING_SCALAR_CODES;

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
