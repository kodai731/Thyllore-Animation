/// Key word that separates the random streams drawn from `thyllore_math_core::hash_u32`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashChannel {
    DisplacementAngle,
    DisplacementDistance,
    Branch,
    BranchAngle,
    BranchAxis,
    Jitter,
    SourcePoint,
    Flicker,
    Reseed,
    EndVariance,
}

impl From<HashChannel> for u32 {
    fn from(channel: HashChannel) -> Self {
        match channel {
            HashChannel::DisplacementAngle => 0,
            HashChannel::DisplacementDistance => 1,
            HashChannel::Branch => 2,
            HashChannel::BranchAngle => 3,
            HashChannel::BranchAxis => 4,
            HashChannel::Jitter => 5,
            HashChannel::SourcePoint => 6,
            HashChannel::Flicker => 7,
            HashChannel::Reseed => 8,
            HashChannel::EndVariance => 9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_math_core::hash_u32;

    #[test]
    fn test_channels_decorrelate() {
        let displacement = hash_u32(&[1, 2, HashChannel::DisplacementAngle.into()]);
        let distance = hash_u32(&[1, 2, HashChannel::DisplacementDistance.into()]);
        assert_ne!(displacement, distance);
    }

    #[test]
    fn test_channel_ids_are_unique() {
        let ids: Vec<u32> = [
            HashChannel::DisplacementAngle,
            HashChannel::DisplacementDistance,
            HashChannel::Branch,
            HashChannel::BranchAngle,
            HashChannel::BranchAxis,
            HashChannel::Jitter,
            HashChannel::SourcePoint,
            HashChannel::Flicker,
            HashChannel::Reseed,
            HashChannel::EndVariance,
        ]
        .into_iter()
        .map(u32::from)
        .collect();

        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
    }
}
