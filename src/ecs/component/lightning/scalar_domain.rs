use thyllore_effect_core::{ScalarParam, UiParam, LIGHTNING_SCALAR_PARAMS, LIGHTNING_UI_PARAMS};

use super::effect::LightningEffect;
use crate::ecs::component::{effect_scalar_domain, ScalarChannelDomain, ScalarDomainSource};

pub struct LightningScalars;

impl ScalarDomainSource for LightningScalars {
    type Component = LightningEffect;
    const NAME: &'static str = "Lightning";
    const SCALARS: &'static [ScalarParam<LightningEffect>] = LIGHTNING_SCALAR_PARAMS;
    const UI: &'static [UiParam] = LIGHTNING_UI_PARAMS;

    fn local_time(effect: &LightningEffect) -> f32 {
        effect.time
    }
}

pub static LIGHTNING_DOMAIN: ScalarChannelDomain = effect_scalar_domain::<LightningScalars>();

crate::scalar_channel_domain!(LIGHTNING_DOMAIN);
