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
                "End Offset X",
                "shape_end_offset_x",
                "EndOffsetX",
                (-20.0, 20.0),
            ),
            LightningShapeParam::EndOffsetY => lightning_channel(
                "End Offset Y",
                "shape_end_offset_y",
                "EndOffsetY",
                (-20.0, 20.0),
            ),
            LightningShapeParam::EndOffsetZ => lightning_channel(
                "End Offset Z",
                "shape_end_offset_z",
                "EndOffsetZ",
                (-20.0, 20.0),
            ),
            LightningShapeParam::StrikesPerBurst => lightning_channel(
                "Strikes Per Burst",
                "shape_strikes_per_burst",
                "StrikesPerBurst",
                (1.0, 8.0),
            ),
            LightningShapeParam::DetailLevels => lightning_channel(
                "Detail Levels",
                "shape_detail_levels",
                "DetailLevels",
                (1.0, 8.0),
            ),
            LightningShapeParam::Tortuosity => {
                lightning_channel("Tortuosity", "shape_tortuosity", "Tortuosity", (0.0, 2.0))
            }
            LightningShapeParam::Roughness => {
                lightning_channel("Roughness", "shape_roughness", "Roughness", (0.0, 1.0))
            }
            LightningShapeParam::CoreRadius => lightning_channel(
                "Core Radius",
                "shape_core_radius",
                "CoreRadius",
                (0.001, 1.0),
            ),
            LightningShapeParam::TipRadiusRatio => lightning_channel(
                "Tip Radius Ratio",
                "shape_tip_radius_ratio",
                "TipRadiusRatio",
                (0.0, 1.0),
            ),
            LightningShapeParam::EdgeFraction => lightning_channel(
                "Edge Fraction",
                "shape_edge_fraction",
                "EdgeFraction",
                (0.0, 1.0),
            ),
            LightningShapeParam::EndVariance => lightning_channel(
                "End Variance",
                "shape_end_variance",
                "EndVariance",
                (0.0, 4.0),
            ),
        }
    }
}
