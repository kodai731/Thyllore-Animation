use crate::wind::analytic::{wind_shadow_radial_extent, WindShellParams};
use crate::wind::{build_wind_model_matrix, WindTornadoEffect, WindUBO};
use cgmath::{Matrix4, SquareMatrix};

/// Instance slot of the baked shadow volume, packed along its radius axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindShadowSlot(pub u32);

pub fn build_wind_ubo(effect: &WindTornadoEffect, shadow_slot: WindShadowSlot) -> WindUBO {
    let model = build_wind_model_matrix(effect);
    let inverse_model = model.invert().unwrap_or(Matrix4::identity());
    let params = WindShellParams::from_effect(effect);

    WindUBO {
        model,
        inverse_model,
        shape: [
            params.height,
            params.wall_radius_base,
            params.wall_radius_slope,
            params.wall_width_q,
        ],
        wall: [params.wall_strength, params.top_fade, 0.0, 0.0],
        optics: [
            params.sigma_t,
            effect.ambient_brightness,
            effect.time,
            params.h_top,
        ],
        albedo: [
            effect.albedo[0],
            effect.albedo[1],
            effect.albedo[2],
            params.spread_offset,
        ],
        lighting: [
            effect.phase_g,
            effect.sun_intensity,
            effect.circulation,
            effect.spread_rate,
        ],
        streak: [
            params.streak_order,
            params.streak_twist,
            params.streak_rise_speed,
            params.streak_amplitude,
        ],
        streak2: [
            params.streak_phase,
            params.streak_rise_time,
            effect.eddy_speed_spread,
            effect.spread_start,
        ],
        eddy: [
            params.eddy_amplitude,
            params.eddy_cell_theta,
            params.eddy_cell_height,
            params.eddy_cell_radial,
        ],
        eddy2: [
            params.eddy_shear,
            params.eddy_rise_speed,
            params.eddy_reseed_period,
            params.eddy_erosion,
        ],
        puff_params: [
            params.puff_count as f32,
            effect.puff_strength,
            shadow_slot.0 as f32,
            wind_shadow_radial_extent(&params),
        ],
        puffs: params.puffs,
        inv_view_proj: Matrix4::identity(),
    }
}

impl Default for WindUBO {
    fn default() -> Self {
        build_wind_ubo(&WindTornadoEffect::default(), WindShadowSlot(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GoldenCase {
        name: &'static str,
        effect: WindTornadoEffect,
        slots: [(&'static str, [u32; 4]); 10],
        puffs_sum_bits: u32,
    }

    fn to_slot_bits(slot: [f32; 4]) -> [u32; 4] {
        [
            slot[0].to_bits(),
            slot[1].to_bits(),
            slot[2].to_bits(),
            slot[3].to_bits(),
        ]
    }

    fn collect_slot_bits(ubo: &WindUBO) -> [(&'static str, [u32; 4]); 10] {
        [
            ("shape", to_slot_bits(ubo.shape)),
            ("wall", to_slot_bits(ubo.wall)),
            ("optics", to_slot_bits(ubo.optics)),
            ("albedo", to_slot_bits(ubo.albedo)),
            ("lighting", to_slot_bits(ubo.lighting)),
            ("streak", to_slot_bits(ubo.streak)),
            ("streak2", to_slot_bits(ubo.streak2)),
            ("eddy", to_slot_bits(ubo.eddy)),
            ("eddy2", to_slot_bits(ubo.eddy2)),
            ("puff_params", to_slot_bits(ubo.puff_params)),
        ]
    }

    fn build_modified_effect() -> WindTornadoEffect {
        WindTornadoEffect {
            time: 2.7,
            streak_amplitude: 0.6,
            eddy_amplitude: 0.5,
            spread_start: 0.25,
            spread_rate: 0.08,
            dissipate_start: 0.5,
            dissipate_time: 1.5,
            puff_count_theta: 5,
            puff_count_height: 3,
            albedo: [0.7, 0.6, 0.55],
            ..WindTornadoEffect::default()
        }
    }

    #[test]
    fn the_wind_ubo_slots_hold_the_packed_effect_and_shell_values() {
        let cases = [
            GoldenCase {
                name: "default",
                effect: WindTornadoEffect::default(),
                slots: [
                    ("shape", [0x40000000, 0x3eb33333, 0x3e800001, 0x3da3d70a]),
                    ("wall", [0x3f800000, 0x3e99999a, 0x00000000, 0x00000000]),
                    ("optics", [0x40800000, 0x3f800000, 0x00000000, 0x3f800000]),
                    ("albedo", [0x3f666666, 0x3f6e147b, 0x3f800000, 0x00000000]),
                    ("lighting", [0x3f19999a, 0x3f800000, 0x00000000, 0x00000000]),
                    ("streak", [0x40400000, 0x40800000, 0x3f800000, 0x00000000]),
                    ("streak2", [0x00000000, 0x00000000, 0x00000000, 0x00000000]),
                    ("eddy", [0x00000000, 0x3ecccccd, 0x3e99999a, 0x3dcccccd]),
                    ("eddy2", [0x00000000, 0x00000000, 0x3f800000, 0x00000000]),
                    (
                        "puff_params",
                        [0x00000000, 0x3f800000, 0x40000000, 0x3dffffff],
                    ),
                ],
                puffs_sum_bits: 0x00000000,
            },
            GoldenCase {
                name: "modified",
                effect: build_modified_effect(),
                slots: [
                    ("shape", [0x40000000, 0x3eb33333, 0x3e800001, 0x3da3d70a]),
                    ("wall", [0x3e6c3ad5, 0x3e99999a, 0x00000000, 0x00000000]),
                    ("optics", [0x40800000, 0x3f800000, 0x402ccccd, 0x3f800000]),
                    ("albedo", [0x3f333333, 0x3f19999a, 0x3f0ccccd, 0x3ec8b439]),
                    ("lighting", [0x3f19999a, 0x3f800000, 0x00000000, 0x3da3d70a]),
                    ("streak", [0x40400000, 0x40800000, 0x3f800000, 0x3f19999a]),
                    ("streak2", [0x00000000, 0x402ccccd, 0x00000000, 0x3e800000]),
                    ("eddy", [0x3f000000, 0x3ecccccd, 0x3e99999a, 0x3dcccccd]),
                    ("eddy2", [0x00000000, 0x00000000, 0x3f800000, 0x00000000]),
                    (
                        "puff_params",
                        [0x41700000, 0x3f800000, 0x40000000, 0x3e8cb55f],
                    ),
                ],
                puffs_sum_bits: 0x419e7d9c,
            },
        ];

        for case in &cases {
            let ubo = build_wind_ubo(&case.effect, WindShadowSlot(2));

            for (actual, expected) in collect_slot_bits(&ubo).iter().zip(case.slots.iter()) {
                assert_eq!(actual.0, expected.0);
                assert_eq!(
                    actual.1, expected.1,
                    "case {} slot {} mismatch",
                    case.name, expected.0
                );
            }

            let puffs_sum: f32 = ubo.puffs.iter().flatten().sum();
            assert_eq!(
                puffs_sum.to_bits(),
                case.puffs_sum_bits,
                "case {} puffs sum mismatch",
                case.name
            );
        }
    }
}
