/// PCG-style integer hash over a key of u32 words; u32 arithmetic only so a shader mirror stays bit-identical.
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
        let values = [7u32, 42, 3, 2];
        assert_eq!(hash_u32(&values), hash_u32(&values));
        assert_eq!(hash_f32(&values).to_bits(), hash_f32(&values).to_bits());
    }

    #[test]
    fn test_hash_f32_stays_in_unit_range() {
        for node in 0u32..512 {
            let value = hash_f32(&[3, node, 0]);
            assert!((0.0..1.0).contains(&value), "node {node} gave {value}");
        }
    }

    #[test]
    fn test_neighbouring_inputs_and_orderings_decorrelate() {
        assert_ne!(hash_u32(&[1, 2, 0]), hash_u32(&[1, 2, 1]));
        assert_ne!(hash_u32(&[1, 2, 3]), hash_u32(&[1, 2, 4]));
        assert_ne!(hash_u32(&[1, 2, 3]), hash_u32(&[3, 2, 1]));
    }

    #[test]
    fn test_hash_distribution_covers_the_unit_range() {
        let mut buckets = [0u32; 8];
        for strike in 0u32..2048 {
            let value = hash_f32(&[11, strike, 5]);
            buckets[(value * 8.0) as usize] += 1;
        }
        assert!(buckets.iter().all(|count| *count > 128), "{buckets:?}");
    }
}
