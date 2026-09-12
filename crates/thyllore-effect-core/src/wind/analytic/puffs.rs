use crate::wind::analytic::shell_integral::{WindShellParams, WIND_MAX_PUFFS};
use crate::wind::WindTornadoEffect;
use std::f32::consts::TAU;
use thyllore_math_core::hash13;

const GOLDEN_ANGLE: f32 = 2.39996;

pub fn build_wind_puffs(
    effect: &WindTornadoEffect,
    params: &WindShellParams,
) -> ([[f32; 4]; WIND_MAX_PUFFS], usize) {
    let mut puffs = [[0.0f32; 4]; WIND_MAX_PUFFS];
    let count_theta = effect.puff_count_theta as usize;
    let count_height = effect.puff_count_height as usize;
    let count = (count_theta * count_height).min(WIND_MAX_PUFFS);
    if count == 0 {
        return (puffs, 0);
    }

    let time = params.time;
    let mut index = 0;
    for k in 0..count_height {
        for j in 0..count_theta {
            if index >= count {
                return (puffs, count);
            }

            let rise = (k as f32 + 0.5) / count_height as f32
                + effect.puff_rise_speed * time / params.height;
            let height = rise.fract() * params.height * params.h_top;

            let wall_radius_sq = params.wall_radius_sq(height / params.height).max(1e-6);
            let wall_radius = wall_radius_sq.sqrt();
            let angular_speed = effect.circulation / (TAU * wall_radius_sq);
            let theta = TAU * j as f32 / count_theta as f32
                + k as f32 * GOLDEN_ANGLE
                + angular_speed * time;

            let signed_hash = hash13([j as f32, k as f32, 0.0]) * 2.0 - 1.0;
            let radial = wall_radius
                + effect.puff_offset_q * signed_hash * params.wall_width_q / (2.0 * wall_radius);
            let radius = effect.puff_radius
                * (1.0 + effect.puff_radius_jitter * hash13([j as f32, k as f32, 1.0]));

            puffs[index] = [radial * theta.cos(), height, radial * theta.sin(), radius];
            index += 1;
        }
    }

    (puffs, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_effect(count_theta: u32, count_height: u32, time: f32) -> WindTornadoEffect {
        WindTornadoEffect {
            puff_count_theta: count_theta,
            puff_count_height: count_height,
            circulation: 2.0,
            spread_rate: 0.05,
            rise_initial_height: 0.3,
            rise_duration: 1.0,
            time,
            ..WindTornadoEffect::default()
        }
    }

    fn build(
        count_theta: u32,
        count_height: u32,
        time: f32,
    ) -> ([[f32; 4]; WIND_MAX_PUFFS], usize) {
        let effect = make_effect(count_theta, count_height, time);
        let params = WindShellParams::from_effect(&effect);
        build_wind_puffs(&effect, &params)
    }

    #[test]
    fn zero_grid_produces_no_puffs() {
        for (puffs, count) in [build(0, 5, 0.0), build(5, 0, 0.0)] {
            assert_eq!(count, 0);
            assert!(puffs.iter().all(|puff| puff == &[0.0; 4]));
        }
    }

    #[test]
    fn unused_slots_stay_zero() {
        let (puffs, count) = build(8, 4, 1.0);
        assert_eq!(count, 32);
        assert!(puffs[count..].iter().all(|puff| puff == &[0.0; 4]));
    }

    #[test]
    fn repeated_build_is_deterministic() {
        let (first, first_count) = build(8, 4, 1.5);
        let (second, second_count) = build(8, 4, 1.5);
        assert_eq!(first_count, second_count);
        assert_eq!(first[..first_count], second[..second_count]);
    }

    #[test]
    fn mass_and_radial_extent_are_conserved_over_time() {
        let (reference, reference_count) = build(8, 4, 0.0);
        assert_eq!(reference_count, 32);

        for time in [0.0, 0.5, 1.0, 2.0, 5.0] {
            let effect = make_effect(8, 4, time);
            let params = WindShellParams::from_effect(&effect);
            let (puffs, count) = build_wind_puffs(&effect, &params);
            assert_eq!(count, reference_count, "count changed at time {time}");

            for (index, puff) in puffs[..count].iter().enumerate() {
                assert!(
                    (puff[3] - reference[index][3]).abs() < 1e-6,
                    "radius of puff {index} changed at time {time}"
                );

                let radial = (puff[0] * puff[0] + puff[2] * puff[2]).sqrt();
                let wall_radius = params.wall_radius_sq(puff[1] / params.height).sqrt();
                let band = effect.puff_offset_q * params.wall_width_q / (2.0 * wall_radius);
                assert!(
                    (radial - wall_radius).abs() <= band + 1e-4,
                    "puff {index} radial {radial} left the wall band at time {time}"
                );
            }
        }
    }

    #[test]
    fn puffs_follow_the_spreading_and_rotating_wall() {
        let (early, _) = build(8, 4, 0.0);
        let (late, _) = build(8, 4, 5.0);

        let spread_params = WindShellParams::from_effect(&make_effect(8, 4, 5.0));
        assert!(spread_params.spread_offset > 0.0);

        let radial_of = |puff: &[f32; 4]| (puff[0] * puff[0] + puff[2] * puff[2]).sqrt();
        assert!(radial_of(&late[0]) > radial_of(&early[0]));
        assert!((late[0][2] - early[0][2]).abs() > 1e-4);
    }

    #[test]
    fn grid_larger_than_limit_is_truncated() {
        assert_eq!(build(16, 8, 0.0).1, WIND_MAX_PUFFS);
    }
}
