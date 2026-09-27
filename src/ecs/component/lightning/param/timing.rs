use super::domain::lightning_channel;
use crate::ecs::component::ScalarChannel;

/// Channels of `LightningTiming`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningTimingParam {
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
    GrowthTime,
}

impl LightningTimingParam {
    pub const ALL: [LightningTimingParam; 16] = [
        LightningTimingParam::BurstStart,
        LightningTimingParam::BurstInterval,
        LightningTimingParam::BurstJitter,
        LightningTimingParam::BurstCount,
        LightningTimingParam::AttackTime,
        LightningTimingParam::SustainTime,
        LightningTimingParam::ReleaseTime,
        LightningTimingParam::StrokeCount,
        LightningTimingParam::StrokeInterval,
        LightningTimingParam::StrokeDecay,
        LightningTimingParam::FlickerAmplitude,
        LightningTimingParam::FlickerPeriod,
        LightningTimingParam::ReseedLevel,
        LightningTimingParam::ReseedPeriod,
        LightningTimingParam::ChargeRamp,
        LightningTimingParam::GrowthTime,
    ];

    pub const fn channel(self) -> ScalarChannel {
        match self {
            LightningTimingParam::BurstStart => lightning_channel(
                797,
                "Burst Start",
                "timing_burst_start",
                "BurstStart",
                (0.0, 10.0),
            ),
            LightningTimingParam::BurstInterval => lightning_channel(
                798,
                "Burst Interval",
                "timing_burst_interval",
                "BurstInterval",
                (0.01, 10.0),
            ),
            LightningTimingParam::BurstJitter => lightning_channel(
                799,
                "Burst Jitter",
                "timing_burst_jitter",
                "BurstJitter",
                (0.0, 1.0),
            ),
            LightningTimingParam::BurstCount => lightning_channel(
                800,
                "Burst Count",
                "timing_burst_count",
                "BurstCount",
                (0.0, 16.0),
            ),
            LightningTimingParam::AttackTime => lightning_channel(
                801,
                "Attack Time",
                "timing_attack_time",
                "AttackTime",
                (0.0, 1.0),
            ),
            LightningTimingParam::SustainTime => lightning_channel(
                802,
                "Sustain Time",
                "timing_sustain_time",
                "SustainTime",
                (0.0, 1.0),
            ),
            LightningTimingParam::ReleaseTime => lightning_channel(
                803,
                "Release Time",
                "timing_release_time",
                "ReleaseTime",
                (0.0, 2.0),
            ),
            LightningTimingParam::StrokeCount => lightning_channel(
                804,
                "Stroke Count",
                "timing_stroke_count",
                "StrokeCount",
                (1.0, 8.0),
            ),
            LightningTimingParam::StrokeInterval => lightning_channel(
                805,
                "Stroke Interval",
                "timing_stroke_interval",
                "StrokeInterval",
                (0.001, 1.0),
            ),
            LightningTimingParam::StrokeDecay => lightning_channel(
                806,
                "Stroke Decay",
                "timing_stroke_decay",
                "StrokeDecay",
                (0.0, 1.0),
            ),
            LightningTimingParam::FlickerAmplitude => lightning_channel(
                807,
                "Flicker Amplitude",
                "timing_flicker_amplitude",
                "FlickerAmplitude",
                (0.0, 1.0),
            ),
            LightningTimingParam::FlickerPeriod => lightning_channel(
                808,
                "Flicker Period",
                "timing_flicker_period",
                "FlickerPeriod",
                (0.001, 1.0),
            ),
            LightningTimingParam::ReseedLevel => lightning_channel(
                809,
                "Reseed Level",
                "timing_reseed_level",
                "ReseedLevel",
                (0.0, 8.0),
            ),
            LightningTimingParam::ReseedPeriod => lightning_channel(
                810,
                "Reseed Period",
                "timing_reseed_period",
                "ReseedPeriod",
                (0.01, 10.0),
            ),
            LightningTimingParam::ChargeRamp => lightning_channel(
                811,
                "Charge Ramp",
                "timing_charge_ramp",
                "ChargeRamp",
                (0.0, 2.0),
            ),
            LightningTimingParam::GrowthTime => lightning_channel(
                816,
                "Growth Time",
                "timing_growth_time",
                "GrowthTime",
                (0.0, 1.0),
            ),
        }
    }
}
