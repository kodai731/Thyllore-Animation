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
                784,
                "Core Intensity",
                "core_intensity",
                "CoreIntensity",
                (0.0, 200.0),
            ),
            LightningLookParam::CoreColorR => lightning_channel(
                785,
                "Core Color R",
                "core_color_r",
                "CoreColorR",
                (0.0, 1.0),
            ),
            LightningLookParam::CoreColorG => lightning_channel(
                786,
                "Core Color G",
                "core_color_g",
                "CoreColorG",
                (0.0, 1.0),
            ),
            LightningLookParam::CoreColorB => lightning_channel(
                787,
                "Core Color B",
                "core_color_b",
                "CoreColorB",
                (0.0, 1.0),
            ),
            LightningLookParam::RimRatio => {
                lightning_channel(788, "Rim Ratio", "rim_ratio", "RimRatio", (1.0, 20.0))
            }
            LightningLookParam::RimIntensity => lightning_channel(
                789,
                "Rim Intensity",
                "rim_intensity",
                "RimIntensity",
                (0.0, 100.0),
            ),
            LightningLookParam::RimColorR => {
                lightning_channel(790, "Rim Color R", "rim_color_r", "RimColorR", (0.0, 1.0))
            }
            LightningLookParam::RimColorG => {
                lightning_channel(791, "Rim Color G", "rim_color_g", "RimColorG", (0.0, 1.0))
            }
            LightningLookParam::RimColorB => {
                lightning_channel(792, "Rim Color B", "rim_color_b", "RimColorB", (0.0, 1.0))
            }
            LightningLookParam::BeamRadius => {
                lightning_channel(793, "Beam Radius", "beam_radius", "BeamRadius", (0.0, 10.0))
            }
            LightningLookParam::BeamArcCount => lightning_channel(
                794,
                "Beam Arc Count",
                "beam_arc_count",
                "BeamArcCount",
                (0.0, 64.0),
            ),
            LightningLookParam::FlashGain => {
                lightning_channel(795, "Flash Gain", "flash_gain", "FlashGain", (0.0, 10.0))
            }
            LightningLookParam::FlashRadius => lightning_channel(
                796,
                "Flash Radius",
                "flash_radius",
                "FlashRadius",
                (0.0, 100.0),
            ),
        }
    }
}
