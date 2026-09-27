use thyllore_effect_core::{ScalarParam, UiParam, FLAME_SCALAR_PARAMS, FLAME_UI_PARAMS};

use super::effect::FlameEffect;
use crate::ecs::component::{effect_scalar_domain, ScalarChannelDomain, ScalarDomainSource};

pub struct FlameScalars;

impl ScalarDomainSource for FlameScalars {
    type Component = FlameEffect;
    const NAME: &'static str = "Flame";
    const SCALARS: &'static [ScalarParam<FlameEffect>] = FLAME_SCALAR_PARAMS;
    const UI: &'static [UiParam] = FLAME_UI_PARAMS;

    fn local_time(effect: &FlameEffect) -> f32 {
        effect.time
    }
}

pub static FLAME_DOMAIN: ScalarChannelDomain = effect_scalar_domain::<FlameScalars>();

crate::scalar_channel_domain!(FLAME_DOMAIN);
