use super::domain::lightning_channel;
use crate::ecs::component::ScalarChannel;

/// Channels of `LightningBranching`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightningBranchParam {
    BranchDepth,
    BranchProbability,
    BranchCount,
    BranchZoneStart,
    BranchZoneEnd,
    BranchAngle,
    BranchLengthRatio,
    BranchRadiusRatio,
    BranchIntensityRatio,
}

impl LightningBranchParam {
    pub const ALL: [LightningBranchParam; 9] = [
        LightningBranchParam::BranchDepth,
        LightningBranchParam::BranchProbability,
        LightningBranchParam::BranchCount,
        LightningBranchParam::BranchZoneStart,
        LightningBranchParam::BranchZoneEnd,
        LightningBranchParam::BranchAngle,
        LightningBranchParam::BranchLengthRatio,
        LightningBranchParam::BranchRadiusRatio,
        LightningBranchParam::BranchIntensityRatio,
    ];

    pub const fn channel(self) -> ScalarChannel {
        match self {
            LightningBranchParam::BranchDepth => lightning_channel(
                778,
                "Branch Depth",
                "branch_depth",
                "BranchDepth",
                (0.0, 5.0),
            ),
            LightningBranchParam::BranchProbability => lightning_channel(
                779,
                "Branch Probability",
                "branch_probability",
                "BranchProbability",
                (0.0, 1.0),
            ),
            LightningBranchParam::BranchCount => lightning_channel(
                812,
                "Branch Count",
                "branch_count",
                "BranchCount",
                (0.0, 32.0),
            ),
            LightningBranchParam::BranchZoneStart => lightning_channel(
                813,
                "Branch Zone Start",
                "branch_zone_start",
                "BranchZoneStart",
                (0.0, 1.0),
            ),
            LightningBranchParam::BranchZoneEnd => lightning_channel(
                814,
                "Branch Zone End",
                "branch_zone_end",
                "BranchZoneEnd",
                (0.0, 1.0),
            ),
            LightningBranchParam::BranchAngle => lightning_channel(
                780,
                "Branch Angle",
                "branch_angle",
                "BranchAngle",
                (0.0, 1.57),
            ),
            LightningBranchParam::BranchLengthRatio => lightning_channel(
                781,
                "Branch Length Ratio",
                "branch_length_ratio",
                "BranchLengthRatio",
                (0.0, 1.0),
            ),
            LightningBranchParam::BranchRadiusRatio => lightning_channel(
                782,
                "Branch Radius Ratio",
                "branch_radius_ratio",
                "BranchRadiusRatio",
                (0.0, 1.0),
            ),
            LightningBranchParam::BranchIntensityRatio => lightning_channel(
                783,
                "Branch Intensity Ratio",
                "branch_intensity_ratio",
                "BranchIntensityRatio",
                (0.0, 1.0),
            ),
        }
    }
}
