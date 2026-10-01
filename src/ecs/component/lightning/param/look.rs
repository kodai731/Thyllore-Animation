use super::domain::lightning_channel;
use crate::ecs::component::ScalarChannel;

/// Channels of `LightningLook`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningLookParam {
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
}

impl LightningLookParam {
    pub const ALL: [LightningLookParam; 13] = [
        LightningLookParam::CoreIntensity,
        LightningLookParam::CoreColorR,
        LightningLookParam::CoreColorG,
        LightningLookParam::CoreColorB,
        LightningLookParam::RimRatio,
        LightningLookParam::RimIntensity,
        LightningLookParam::RimColorR,
        LightningLookParam::RimColorG,
        LightningLookParam::RimColorB,
        LightningLookParam::BeamRadius,
        LightningLookParam::BeamArcCount,
        LightningLookParam::FlashGain,
        LightningLookParam::FlashRadius,
    ];

    pub const fn channel(self) -> ScalarChannel {
        match self {
            LightningLookParam::CoreIntensity => lightning_channel(
                "Core Intensity",
                "look_core_intensity",
                "CoreIntensity",
                (0.0, 200.0),
            ),
            LightningLookParam::CoreColorR => lightning_channel(
                "Core Color R",
                "look_core_color_r",
                "CoreColorR",
                (0.0, 1.0),
            ),
            LightningLookParam::CoreColorG => lightning_channel(
                "Core Color G",
                "look_core_color_g",
                "CoreColorG",
                (0.0, 1.0),
            ),
            LightningLookParam::CoreColorB => lightning_channel(
                "Core Color B",
                "look_core_color_b",
                "CoreColorB",
                (0.0, 1.0),
            ),
            LightningLookParam::RimRatio => {
                lightning_channel("Rim Ratio", "look_rim_ratio", "RimRatio", (1.0, 20.0))
            }
            LightningLookParam::RimIntensity => lightning_channel(
                "Rim Intensity",
                "look_rim_intensity",
                "RimIntensity",
                (0.0, 100.0),
            ),
            LightningLookParam::RimColorR => {
                lightning_channel("Rim Color R", "look_rim_color_r", "RimColorR", (0.0, 1.0))
            }
            LightningLookParam::RimColorG => {
                lightning_channel("Rim Color G", "look_rim_color_g", "RimColorG", (0.0, 1.0))
            }
            LightningLookParam::RimColorB => {
                lightning_channel("Rim Color B", "look_rim_color_b", "RimColorB", (0.0, 1.0))
            }
            LightningLookParam::BeamRadius => {
                lightning_channel("Beam Radius", "look_beam_radius", "BeamRadius", (0.0, 10.0))
            }
            LightningLookParam::BeamArcCount => lightning_channel(
                "Beam Arc Count",
                "look_beam_arc_count",
                "BeamArcCount",
                (0.0, 64.0),
            ),
            LightningLookParam::FlashGain => {
                lightning_channel("Flash Gain", "look_flash_gain", "FlashGain", (0.0, 10.0))
            }
            LightningLookParam::FlashRadius => lightning_channel(
                "Flash Radius",
                "look_flash_radius",
                "FlashRadius",
                (0.0, 100.0),
            ),
        }
    }
}
