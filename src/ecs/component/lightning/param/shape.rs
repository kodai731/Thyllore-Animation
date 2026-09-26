use super::domain::lightning_channel;
use crate::ecs::component::ScalarChannel;

/// Channels of `LightningShape`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningShapeParam {
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
    EndVariance,
}

impl LightningShapeParam {
    pub const ALL: [LightningShapeParam; 11] = [
        LightningShapeParam::EndOffsetX,
        LightningShapeParam::EndOffsetY,
        LightningShapeParam::EndOffsetZ,
        LightningShapeParam::StrikesPerBurst,
        LightningShapeParam::DetailLevels,
        LightningShapeParam::Tortuosity,
        LightningShapeParam::Roughness,
        LightningShapeParam::CoreRadius,
        LightningShapeParam::TipRadiusRatio,
        LightningShapeParam::EdgeFraction,
        LightningShapeParam::EndVariance,
    ];

    pub const fn channel(self) -> ScalarChannel {
        match self {
            LightningShapeParam::EndOffsetX => lightning_channel(
                768,
                "End Offset X",
                "end_offset_x",
                "EndOffsetX",
                (-20.0, 20.0),
            ),
            LightningShapeParam::EndOffsetY => lightning_channel(
                769,
                "End Offset Y",
                "end_offset_y",
                "EndOffsetY",
                (-20.0, 20.0),
            ),
            LightningShapeParam::EndOffsetZ => lightning_channel(
                770,
                "End Offset Z",
                "end_offset_z",
                "EndOffsetZ",
                (-20.0, 20.0),
            ),
            LightningShapeParam::StrikesPerBurst => lightning_channel(
                771,
                "Strikes Per Burst",
                "strikes_per_burst",
                "StrikesPerBurst",
                (1.0, 8.0),
            ),
            LightningShapeParam::DetailLevels => lightning_channel(
                772,
                "Detail Levels",
                "detail_levels",
                "DetailLevels",
                (1.0, 8.0),
            ),
            LightningShapeParam::Tortuosity => {
                lightning_channel(773, "Tortuosity", "tortuosity", "Tortuosity", (0.0, 2.0))
            }
            LightningShapeParam::Roughness => {
                lightning_channel(774, "Roughness", "roughness", "Roughness", (0.0, 1.0))
            }
            LightningShapeParam::CoreRadius => lightning_channel(
                775,
                "Core Radius",
                "core_radius",
                "CoreRadius",
                (0.001, 1.0),
            ),
            LightningShapeParam::TipRadiusRatio => lightning_channel(
                776,
                "Tip Radius Ratio",
                "tip_radius_ratio",
                "TipRadiusRatio",
                (0.0, 1.0),
            ),
            LightningShapeParam::EdgeFraction => lightning_channel(
                777,
                "Edge Fraction",
                "edge_fraction",
                "EdgeFraction",
                (0.0, 1.0),
            ),
            LightningShapeParam::EndVariance => lightning_channel(
                815,
                "End Variance",
                "end_variance",
                "EndVariance",
                (0.0, 4.0),
            ),
        }
    }
}
