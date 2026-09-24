use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::{find_scalar_param, ScalarParam, LIGHTNING_SCALAR_PARAMS};

use super::effect::LightningEffect;
use crate::ecs::component::{ScalarChannel, ScalarChannelDomain};
use crate::ecs::world::{Entity, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningParam {
    EndOffsetX,
    EndOffsetY,
    EndOffsetZ,
    StrikesPerBurst,
    DetailLevels,
    Tortuosity,
    Roughness,
    CoreRadius,
    TipRadiusRatio,
    EdgeFraction,
    BranchDepth,
    BranchProbability,
    BranchCount,
    BranchZoneStart,
    BranchZoneEnd,
    BranchAngle,
    BranchLengthRatio,
    BranchRadiusRatio,
    BranchIntensityRatio,
    CoreIntensity,
    CoreColorR,
    CoreColorG,
    CoreColorB,
    RimRatio,
    RimIntensity,
    RimColorR,
    RimColorG,
    RimColorB,
    BeamRadius,
    BeamArcCount,
    FlashGain,
    FlashRadius,
    BurstStart,
    BurstInterval,
    BurstJitter,
    BurstCount,
    AttackTime,
    SustainTime,
    ReleaseTime,
    StrokeCount,
    StrokeInterval,
    StrokeDecay,
    FlickerAmplitude,
    FlickerPeriod,
    ReseedLevel,
    ReseedPeriod,
    ChargeRamp,
}

impl LightningParam {
    pub const ALL: [LightningParam; 47] = [
        LightningParam::EndOffsetX,
        LightningParam::EndOffsetY,
        LightningParam::EndOffsetZ,
        LightningParam::StrikesPerBurst,
        LightningParam::DetailLevels,
        LightningParam::Tortuosity,
        LightningParam::Roughness,
        LightningParam::CoreRadius,
        LightningParam::TipRadiusRatio,
        LightningParam::EdgeFraction,
        LightningParam::BranchDepth,
        LightningParam::BranchProbability,
        LightningParam::BranchCount,
        LightningParam::BranchZoneStart,
        LightningParam::BranchZoneEnd,
        LightningParam::BranchAngle,
        LightningParam::BranchLengthRatio,
        LightningParam::BranchRadiusRatio,
        LightningParam::BranchIntensityRatio,
        LightningParam::CoreIntensity,
        LightningParam::CoreColorR,
        LightningParam::CoreColorG,
        LightningParam::CoreColorB,
        LightningParam::RimRatio,
        LightningParam::RimIntensity,
        LightningParam::RimColorR,
        LightningParam::RimColorG,
        LightningParam::RimColorB,
        LightningParam::BeamRadius,
        LightningParam::BeamArcCount,
        LightningParam::FlashGain,
        LightningParam::FlashRadius,
        LightningParam::BurstStart,
        LightningParam::BurstInterval,
        LightningParam::BurstJitter,
        LightningParam::BurstCount,
        LightningParam::AttackTime,
        LightningParam::SustainTime,
        LightningParam::ReleaseTime,
        LightningParam::StrokeCount,
        LightningParam::StrokeInterval,
        LightningParam::StrokeDecay,
        LightningParam::FlickerAmplitude,
        LightningParam::FlickerPeriod,
        LightningParam::ReseedLevel,
        LightningParam::ReseedPeriod,
        LightningParam::ChargeRamp,
    ];

    pub const fn code(self) -> u16 {
        match self {
            LightningParam::EndOffsetX => 768,
            LightningParam::EndOffsetY => 769,
            LightningParam::EndOffsetZ => 770,
            LightningParam::StrikesPerBurst => 771,
            LightningParam::DetailLevels => 772,
            LightningParam::Tortuosity => 773,
            LightningParam::Roughness => 774,
            LightningParam::CoreRadius => 775,
            LightningParam::TipRadiusRatio => 776,
            LightningParam::EdgeFraction => 777,
            LightningParam::BranchDepth => 778,
            LightningParam::BranchProbability => 779,
            LightningParam::BranchCount => 812,
            LightningParam::BranchZoneStart => 813,
            LightningParam::BranchZoneEnd => 814,
            LightningParam::BranchAngle => 780,
            LightningParam::BranchLengthRatio => 781,
            LightningParam::BranchRadiusRatio => 782,
            LightningParam::BranchIntensityRatio => 783,
            LightningParam::CoreIntensity => 784,
            LightningParam::CoreColorR => 785,
            LightningParam::CoreColorG => 786,
            LightningParam::CoreColorB => 787,
            LightningParam::RimRatio => 788,
            LightningParam::RimIntensity => 789,
            LightningParam::RimColorR => 790,
            LightningParam::RimColorG => 791,
            LightningParam::RimColorB => 792,
            LightningParam::BeamRadius => 793,
            LightningParam::BeamArcCount => 794,
            LightningParam::FlashGain => 795,
            LightningParam::FlashRadius => 796,
            LightningParam::BurstStart => 797,
            LightningParam::BurstInterval => 798,
            LightningParam::BurstJitter => 799,
            LightningParam::BurstCount => 800,
            LightningParam::AttackTime => 801,
            LightningParam::SustainTime => 802,
            LightningParam::ReleaseTime => 803,
            LightningParam::StrokeCount => 804,
            LightningParam::StrokeInterval => 805,
            LightningParam::StrokeDecay => 806,
            LightningParam::FlickerAmplitude => 807,
            LightningParam::FlickerPeriod => 808,
            LightningParam::ReseedLevel => 809,
            LightningParam::ReseedPeriod => 810,
            LightningParam::ChargeRamp => 811,
        }
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

    pub const fn display_name(self) -> &'static str {
        match self {
            LightningParam::EndOffsetX => "End Offset X",
            LightningParam::EndOffsetY => "End Offset Y",
            LightningParam::EndOffsetZ => "End Offset Z",
            LightningParam::StrikesPerBurst => "Strikes Per Burst",
            LightningParam::DetailLevels => "Detail Levels",
            LightningParam::Tortuosity => "Tortuosity",
            LightningParam::Roughness => "Roughness",
            LightningParam::CoreRadius => "Core Radius",
            LightningParam::TipRadiusRatio => "Tip Radius Ratio",
            LightningParam::EdgeFraction => "Edge Fraction",
            LightningParam::BranchDepth => "Branch Depth",
            LightningParam::BranchProbability => "Branch Probability",
            LightningParam::BranchCount => "Branch Count",
            LightningParam::BranchZoneStart => "Branch Zone Start",
            LightningParam::BranchZoneEnd => "Branch Zone End",
            LightningParam::BranchAngle => "Branch Angle",
            LightningParam::BranchLengthRatio => "Branch Length Ratio",
            LightningParam::BranchRadiusRatio => "Branch Radius Ratio",
            LightningParam::BranchIntensityRatio => "Branch Intensity Ratio",
            LightningParam::CoreIntensity => "Core Intensity",
            LightningParam::CoreColorR => "Core Color R",
            LightningParam::CoreColorG => "Core Color G",
            LightningParam::CoreColorB => "Core Color B",
            LightningParam::RimRatio => "Rim Ratio",
            LightningParam::RimIntensity => "Rim Intensity",
            LightningParam::RimColorR => "Rim Color R",
            LightningParam::RimColorG => "Rim Color G",
            LightningParam::RimColorB => "Rim Color B",
            LightningParam::BeamRadius => "Beam Radius",
            LightningParam::BeamArcCount => "Beam Arc Count",
            LightningParam::FlashGain => "Flash Gain",
            LightningParam::FlashRadius => "Flash Radius",
            LightningParam::BurstStart => "Burst Start",
            LightningParam::BurstInterval => "Burst Interval",
            LightningParam::BurstJitter => "Burst Jitter",
            LightningParam::BurstCount => "Burst Count",
            LightningParam::AttackTime => "Attack Time",
            LightningParam::SustainTime => "Sustain Time",
            LightningParam::ReleaseTime => "Release Time",
            LightningParam::StrokeCount => "Stroke Count",
            LightningParam::StrokeInterval => "Stroke Interval",
            LightningParam::StrokeDecay => "Stroke Decay",
            LightningParam::FlickerAmplitude => "Flicker Amplitude",
            LightningParam::FlickerPeriod => "Flicker Period",
            LightningParam::ReseedLevel => "Reseed Level",
            LightningParam::ReseedPeriod => "Reseed Period",
            LightningParam::ChargeRamp => "Charge Ramp",
        }
    }

    pub const fn cli_name(self) -> &'static str {
        match self {
            LightningParam::EndOffsetX => "end_offset_x",
            LightningParam::EndOffsetY => "end_offset_y",
            LightningParam::EndOffsetZ => "end_offset_z",
            LightningParam::StrikesPerBurst => "strikes_per_burst",
            LightningParam::DetailLevels => "detail_levels",
            LightningParam::Tortuosity => "tortuosity",
            LightningParam::Roughness => "roughness",
            LightningParam::CoreRadius => "core_radius",
            LightningParam::TipRadiusRatio => "tip_radius_ratio",
            LightningParam::EdgeFraction => "edge_fraction",
            LightningParam::BranchDepth => "branch_depth",
            LightningParam::BranchProbability => "branch_probability",
            LightningParam::BranchCount => "branch_count",
            LightningParam::BranchZoneStart => "branch_zone_start",
            LightningParam::BranchZoneEnd => "branch_zone_end",
            LightningParam::BranchAngle => "branch_angle",
            LightningParam::BranchLengthRatio => "branch_length_ratio",
            LightningParam::BranchRadiusRatio => "branch_radius_ratio",
            LightningParam::BranchIntensityRatio => "branch_intensity_ratio",
            LightningParam::CoreIntensity => "core_intensity",
            LightningParam::CoreColorR => "core_color_r",
            LightningParam::CoreColorG => "core_color_g",
            LightningParam::CoreColorB => "core_color_b",
            LightningParam::RimRatio => "rim_ratio",
            LightningParam::RimIntensity => "rim_intensity",
            LightningParam::RimColorR => "rim_color_r",
            LightningParam::RimColorG => "rim_color_g",
            LightningParam::RimColorB => "rim_color_b",
            LightningParam::BeamRadius => "beam_radius",
            LightningParam::BeamArcCount => "beam_arc_count",
            LightningParam::FlashGain => "flash_gain",
            LightningParam::FlashRadius => "flash_radius",
            LightningParam::BurstStart => "burst_start",
            LightningParam::BurstInterval => "burst_interval",
            LightningParam::BurstJitter => "burst_jitter",
            LightningParam::BurstCount => "burst_count",
            LightningParam::AttackTime => "attack_time",
            LightningParam::SustainTime => "sustain_time",
            LightningParam::ReleaseTime => "release_time",
            LightningParam::StrokeCount => "stroke_count",
            LightningParam::StrokeInterval => "stroke_interval",
            LightningParam::StrokeDecay => "stroke_decay",
            LightningParam::FlickerAmplitude => "flicker_amplitude",
            LightningParam::FlickerPeriod => "flicker_period",
            LightningParam::ReseedLevel => "reseed_level",
            LightningParam::ReseedPeriod => "reseed_period",
            LightningParam::ChargeRamp => "charge_ramp",
        }
    }

    pub fn from_cli_name(name: &str) -> Option<LightningParam> {
        LightningParam::ALL
            .iter()
            .copied()
            .find(|p| p.cli_name() == name)
    }

    pub const fn scene_name(self) -> &'static str {
        match self {
            LightningParam::EndOffsetX => "EndOffsetX",
            LightningParam::EndOffsetY => "EndOffsetY",
            LightningParam::EndOffsetZ => "EndOffsetZ",
            LightningParam::StrikesPerBurst => "StrikesPerBurst",
            LightningParam::DetailLevels => "DetailLevels",
            LightningParam::Tortuosity => "Tortuosity",
            LightningParam::Roughness => "Roughness",
            LightningParam::CoreRadius => "CoreRadius",
            LightningParam::TipRadiusRatio => "TipRadiusRatio",
            LightningParam::EdgeFraction => "EdgeFraction",
            LightningParam::BranchDepth => "BranchDepth",
            LightningParam::BranchProbability => "BranchProbability",
            LightningParam::BranchCount => "BranchCount",
            LightningParam::BranchZoneStart => "BranchZoneStart",
            LightningParam::BranchZoneEnd => "BranchZoneEnd",
            LightningParam::BranchAngle => "BranchAngle",
            LightningParam::BranchLengthRatio => "BranchLengthRatio",
            LightningParam::BranchRadiusRatio => "BranchRadiusRatio",
            LightningParam::BranchIntensityRatio => "BranchIntensityRatio",
            LightningParam::CoreIntensity => "CoreIntensity",
            LightningParam::CoreColorR => "CoreColorR",
            LightningParam::CoreColorG => "CoreColorG",
            LightningParam::CoreColorB => "CoreColorB",
            LightningParam::RimRatio => "RimRatio",
            LightningParam::RimIntensity => "RimIntensity",
            LightningParam::RimColorR => "RimColorR",
            LightningParam::RimColorG => "RimColorG",
            LightningParam::RimColorB => "RimColorB",
            LightningParam::BeamRadius => "BeamRadius",
            LightningParam::BeamArcCount => "BeamArcCount",
            LightningParam::FlashGain => "FlashGain",
            LightningParam::FlashRadius => "FlashRadius",
            LightningParam::BurstStart => "BurstStart",
            LightningParam::BurstInterval => "BurstInterval",
            LightningParam::BurstJitter => "BurstJitter",
            LightningParam::BurstCount => "BurstCount",
            LightningParam::AttackTime => "AttackTime",
            LightningParam::SustainTime => "SustainTime",
            LightningParam::ReleaseTime => "ReleaseTime",
            LightningParam::StrokeCount => "StrokeCount",
            LightningParam::StrokeInterval => "StrokeInterval",
            LightningParam::StrokeDecay => "StrokeDecay",
            LightningParam::FlickerAmplitude => "FlickerAmplitude",
            LightningParam::FlickerPeriod => "FlickerPeriod",
            LightningParam::ReseedLevel => "ReseedLevel",
            LightningParam::ReseedPeriod => "ReseedPeriod",
            LightningParam::ChargeRamp => "ChargeRamp",
        }
    }

    pub const fn debug_value_range(self) -> (f32, f32) {
        match self {
            LightningParam::EndOffsetX => (-20.0, 20.0),
            LightningParam::EndOffsetY => (-20.0, 20.0),
            LightningParam::EndOffsetZ => (-20.0, 20.0),
            LightningParam::StrikesPerBurst => (1.0, 8.0),
            LightningParam::DetailLevels => (1.0, 8.0),
            LightningParam::Tortuosity => (0.0, 2.0),
            LightningParam::Roughness => (0.0, 1.0),
            LightningParam::CoreRadius => (0.001, 1.0),
            LightningParam::TipRadiusRatio => (0.0, 1.0),
            LightningParam::EdgeFraction => (0.0, 1.0),
            LightningParam::BranchDepth => (0.0, 5.0),
            LightningParam::BranchProbability => (0.0, 1.0),
            LightningParam::BranchCount => (0.0, 32.0),
            LightningParam::BranchZoneStart => (0.0, 1.0),
            LightningParam::BranchZoneEnd => (0.0, 1.0),
            LightningParam::BranchAngle => (0.0, 1.57),
            LightningParam::BranchLengthRatio => (0.0, 1.0),
            LightningParam::BranchRadiusRatio => (0.0, 1.0),
            LightningParam::BranchIntensityRatio => (0.0, 1.0),
            LightningParam::CoreIntensity => (0.0, 200.0),
            LightningParam::CoreColorR => (0.0, 1.0),
            LightningParam::CoreColorG => (0.0, 1.0),
            LightningParam::CoreColorB => (0.0, 1.0),
            LightningParam::RimRatio => (1.0, 20.0),
            LightningParam::RimIntensity => (0.0, 100.0),
            LightningParam::RimColorR => (0.0, 1.0),
            LightningParam::RimColorG => (0.0, 1.0),
            LightningParam::RimColorB => (0.0, 1.0),
            LightningParam::BeamRadius => (0.0, 10.0),
            LightningParam::BeamArcCount => (0.0, 64.0),
            LightningParam::FlashGain => (0.0, 10.0),
            LightningParam::FlashRadius => (0.0, 100.0),
            LightningParam::BurstStart => (0.0, 10.0),
            LightningParam::BurstInterval => (0.01, 10.0),
            LightningParam::BurstJitter => (0.0, 1.0),
            LightningParam::BurstCount => (0.0, 16.0),
            LightningParam::AttackTime => (0.0, 1.0),
            LightningParam::SustainTime => (0.0, 1.0),
            LightningParam::ReleaseTime => (0.0, 2.0),
            LightningParam::StrokeCount => (1.0, 8.0),
            LightningParam::StrokeInterval => (0.001, 1.0),
            LightningParam::StrokeDecay => (0.0, 1.0),
            LightningParam::FlickerAmplitude => (0.0, 1.0),
            LightningParam::FlickerPeriod => (0.001, 1.0),
            LightningParam::ReseedLevel => (0.0, 8.0),
            LightningParam::ReseedPeriod => (0.01, 10.0),
            LightningParam::ChargeRamp => (0.0, 2.0),
        }
    }

    const fn channel(self) -> ScalarChannel {
        ScalarChannel {
            code: self.code(),
            display_name: self.display_name(),
            cli_name: self.cli_name(),
            scene_name: self.scene_name(),
            debug_value_range: self.debug_value_range(),
        }
    }
}

pub static LIGHTNING_CHANNELS: [ScalarChannel; LightningParam::ALL.len()] = {
    let mut channels = [LightningParam::EndOffsetX.channel(); LightningParam::ALL.len()];
    let mut i = 0;
    while i < LightningParam::ALL.len() {
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
