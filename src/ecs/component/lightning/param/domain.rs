use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::{find_scalar_param, ScalarParam, LIGHTNING_SCALAR_PARAMS};

use super::{LightningBranchParam, LightningLookParam, LightningShapeParam, LightningTimingParam};
use crate::ecs::component::lightning::effect::LightningEffect;
use crate::ecs::component::{ScalarChannel, ScalarChannelDomain};
use crate::ecs::world::{Entity, World};

/// Lightning's animatable channels, grouped like the sub structs of `LightningEffect`.
/// Codes are persisted in clip files (`PropertyType::Custom(code)`): never reorder or reuse them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningParam {
    Shape(LightningShapeParam),
    Branch(LightningBranchParam),
    Look(LightningLookParam),
    Timing(LightningTimingParam),
}

const LIGHTNING_PARAM_COUNT: usize = LightningShapeParam::ALL.len()
    + LightningBranchParam::ALL.len()
    + LightningLookParam::ALL.len()
    + LightningTimingParam::ALL.len();

pub(super) const fn lightning_channel(
    code: u16,
    display_name: &'static str,
    cli_name: &'static str,
    scene_name: &'static str,
    debug_value_range: (f32, f32),
) -> ScalarChannel {
    ScalarChannel {
        code,
        display_name,
        cli_name,
        scene_name,
        debug_value_range,
    }
}

impl LightningParam {
    pub const ALL: [LightningParam; LIGHTNING_PARAM_COUNT] = collect_lightning_params();

    pub const fn channel(self) -> ScalarChannel {
        match self {
            LightningParam::Shape(param) => param.channel(),
            LightningParam::Branch(param) => param.channel(),
            LightningParam::Look(param) => param.channel(),
            LightningParam::Timing(param) => param.channel(),
        }
    }

    pub const fn code(self) -> u16 {
        self.channel().code
    }

    pub const fn cli_name(self) -> &'static str {
        self.channel().cli_name
    }

    pub fn from_code(code: u16) -> Option<LightningParam> {
        LightningParam::ALL
            .iter()
            .copied()
            .find(|p| p.code() == code)
    }

    pub const fn property_type(self) -> PropertyType {
        PropertyType::Custom(self.code())
    }

    pub fn from_property_type(property_type: PropertyType) -> Option<LightningParam> {
        match property_type {
            PropertyType::Custom(code) => LightningParam::from_code(code),
            _ => None,
        }
    }

    pub fn from_cli_name(name: &str) -> Option<LightningParam> {
        LightningParam::ALL
            .iter()
            .copied()
            .find(|p| p.cli_name() == name)
    }
}

const fn collect_lightning_params() -> [LightningParam; LIGHTNING_PARAM_COUNT] {
    let mut all = [LightningParam::Shape(LightningShapeParam::ALL[0]); LIGHTNING_PARAM_COUNT];
    let mut next = 0;

    let mut i = 0;
    while i < LightningShapeParam::ALL.len() {
        all[next] = LightningParam::Shape(LightningShapeParam::ALL[i]);
        next += 1;
        i += 1;
    }

    let mut i = 0;
    while i < LightningBranchParam::ALL.len() {
        all[next] = LightningParam::Branch(LightningBranchParam::ALL[i]);
        next += 1;
        i += 1;
    }

    let mut i = 0;
    while i < LightningLookParam::ALL.len() {
        all[next] = LightningParam::Look(LightningLookParam::ALL[i]);
        next += 1;
        i += 1;
    }

    let mut i = 0;
    while i < LightningTimingParam::ALL.len() {
        all[next] = LightningParam::Timing(LightningTimingParam::ALL[i]);
        next += 1;
        i += 1;
    }

    all
}

pub static LIGHTNING_CHANNELS: [ScalarChannel; LIGHTNING_PARAM_COUNT] = {
    let mut channels = [LightningParam::ALL[0].channel(); LIGHTNING_PARAM_COUNT];
    let mut i = 0;
    while i < LIGHTNING_PARAM_COUNT {
        channels[i] = LightningParam::ALL[i].channel();
        i += 1;
    }
    channels
};

pub static LIGHTNING_DOMAIN: ScalarChannelDomain = ScalarChannelDomain {
    name: "Lightning",
    channels: &LIGHTNING_CHANNELS,
    has_component: lightning_has_component,
    entities: lightning_entities,
    read: lightning_channel_read,
    local_time: lightning_local_time,
};

crate::scalar_channel_domain!(LIGHTNING_DOMAIN);

fn lightning_has_component(world: &World, entity: Entity) -> bool {
    world.get_component::<LightningEffect>(entity).is_some()
}

fn lightning_entities(world: &World) -> Vec<Entity> {
    world.entities_with::<LightningEffect>()
}

fn lightning_channel_read(
    world: &World,
    entity: Entity,
    property_type: PropertyType,
) -> Option<f32> {
    let param = LightningParam::from_property_type(property_type)?;
    world
        .get_component::<LightningEffect>(entity)
        .map(|effect| lightning_param_value(&effect, param))
}

fn lightning_local_time(world: &World, entity: Entity) -> Option<f32> {
    world
        .get_component::<LightningEffect>(entity)
        .map(|effect| effect.time)
}

fn scalar_param(param: LightningParam) -> &'static ScalarParam<LightningEffect> {
    find_scalar_param(LIGHTNING_SCALAR_PARAMS, param.cli_name())
        .expect("every LightningParam cli_name is registered in LIGHTNING_SCALAR_PARAMS")
}

pub fn apply_lightning_param_value(
    effect: &mut LightningEffect,
    param: LightningParam,
    value: f32,
) {
    (scalar_param(param).set)(effect, value)
}

pub fn lightning_param_value(effect: &LightningEffect, param: LightningParam) -> f32 {
    (scalar_param(param).get)(effect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_and_cli_name_roundtrip_all_params() {
        for param in LightningParam::ALL {
            assert_eq!(LightningParam::from_code(param.code()), Some(param));
            assert_eq!(
                LightningParam::from_property_type(param.property_type()),
                Some(param)
            );
            assert_eq!(LightningParam::from_cli_name(param.cli_name()), Some(param));
        }
        assert_eq!(LightningParam::from_code(1), None);
        assert_eq!(LightningParam::from_cli_name("no_such_param"), None);
    }

    #[test]
    fn test_every_cli_name_is_in_the_scalar_registry() {
        for param in LightningParam::ALL {
            assert!(
                find_scalar_param(LIGHTNING_SCALAR_PARAMS, param.cli_name()).is_some(),
                "{:?}",
                param
            );
        }
    }

    #[test]
    fn test_codes_are_unique_and_start_at_768() {
        let mut codes: Vec<u16> = LightningParam::ALL.iter().map(|p| p.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), LightningParam::ALL.len());
        assert!(codes.iter().all(|code| *code >= 768));
    }

    #[test]
    fn test_channel_table_mirrors_enum() {
        assert_eq!(LIGHTNING_CHANNELS.len(), LightningParam::ALL.len());
        for (channel, param) in LIGHTNING_CHANNELS.iter().zip(LightningParam::ALL) {
            assert_eq!(channel.code, param.code());
            assert_eq!(channel.cli_name, param.cli_name());
            assert_eq!(channel.property_type(), param.property_type());
        }
    }

    #[test]
    fn test_param_value_mirrors_apply_for_all_params() {
        let mut effect = LightningEffect::default();
        for (i, param) in LightningParam::ALL.into_iter().enumerate() {
            let value = 10.0 + i as f32;
            apply_lightning_param_value(&mut effect, param, value);
            assert!(
                (lightning_param_value(&effect, param) - value).abs() < 1e-6,
                "{param:?}"
            );
        }
    }
}
