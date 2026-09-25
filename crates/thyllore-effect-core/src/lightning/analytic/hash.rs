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

/// PCG-style integer hash; u32 arithmetic only so the shader mirror stays bit-identical.
pub fn hash_u32(values: &[u32]) -> u32 {
    let mut state = 0x9e37_79b9u32;
    for &value in values {
        state = state.wrapping_add(value).wrapping_mul(0x85eb_ca6b);
        state ^= state >> 15;
        state = state.wrapping_mul(0xc2b2_ae35);
        state ^= state >> 13;
    }

    state ^= state >> 16;
    state = state.wrapping_mul(0x7feb_352d);
    state ^= state >> 15;
    state = state.wrapping_mul(0x846c_a68b);
    state ^ (state >> 16)
}

/// Hash mapped to [0, 1) through the 24 bits an f32 represents exactly.
pub fn hash_f32(values: &[u32]) -> f32 {
    (hash_u32(values) >> 8) as f32 * (1.0 / 16_777_216.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_is_deterministic() {
        let values = [7u32, 42, 3, HashChannel::Branch.into()];
        assert_eq!(hash_u32(&values), hash_u32(&values));
        assert_eq!(hash_f32(&values).to_bits(), hash_f32(&values).to_bits());
    }

    #[test]
    fn test_hash_f32_stays_in_unit_range() {
        for node in 0u32..512 {
            let value = hash_f32(&[3, node, HashChannel::DisplacementAngle.into()]);
            assert!((0.0..1.0).contains(&value), "node {node} gave {value}");
        }
    }

    #[test]
    fn test_channels_and_neighbouring_inputs_decorrelate() {
        let displacement = hash_u32(&[1, 2, HashChannel::DisplacementAngle.into()]);
        let distance = hash_u32(&[1, 2, HashChannel::DisplacementDistance.into()]);
        assert_ne!(displacement, distance);
        assert_ne!(hash_u32(&[1, 2, 3]), hash_u32(&[1, 2, 4]));
        assert_ne!(hash_u32(&[1, 2, 3]), hash_u32(&[3, 2, 1]));
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

    #[test]
    fn test_hash_distribution_covers_the_unit_range() {
        let mut buckets = [0u32; 8];
        for strike in 0u32..2048 {
            let value = hash_f32(&[11, strike, HashChannel::Jitter.into()]);
            buckets[(value * 8.0) as usize] += 1;
        }
        assert!(buckets.iter().all(|count| *count > 128), "{buckets:?}");
    }
}
