use thyllore_effect_core::{ScalarParam, UiParam, WATER_SCALAR_PARAMS, WATER_UI_PARAMS};

use super::effect::WaterTorusEffect;
use crate::ecs::component::{effect_scalar_domain, ScalarChannelDomain, ScalarDomainSource};

pub struct WaterScalars;

impl ScalarDomainSource for WaterScalars {
    type Component = WaterTorusEffect;
    const NAME: &'static str = "Water";
    const SCALARS: &'static [ScalarParam<WaterTorusEffect>] = WATER_SCALAR_PARAMS;
    const UI: &'static [UiParam] = WATER_UI_PARAMS;

    fn local_time(effect: &WaterTorusEffect) -> f32 {
        effect.time
    }
}

pub static WATER_DOMAIN: ScalarChannelDomain = effect_scalar_domain::<WaterScalars>();

crate::scalar_channel_domain!(WATER_DOMAIN);
