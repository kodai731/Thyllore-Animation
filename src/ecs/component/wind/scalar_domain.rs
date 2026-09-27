use thyllore_effect_core::{ScalarParam, UiParam, WIND_SCALAR_PARAMS, WIND_UI_PARAMS};

use super::effect::WindTornadoEffect;
use crate::ecs::component::{effect_scalar_domain, ScalarChannelDomain, ScalarDomainSource};

pub struct WindScalars;

impl ScalarDomainSource for WindScalars {
    type Component = WindTornadoEffect;
    const NAME: &'static str = "Wind";
    const SCALARS: &'static [ScalarParam<WindTornadoEffect>] = WIND_SCALAR_PARAMS;
    const UI: &'static [UiParam] = WIND_UI_PARAMS;

    fn local_time(effect: &WindTornadoEffect) -> f32 {
        effect.time
    }
}

pub static WIND_DOMAIN: ScalarChannelDomain = effect_scalar_domain::<WindScalars>();

crate::scalar_channel_domain!(WIND_DOMAIN);
