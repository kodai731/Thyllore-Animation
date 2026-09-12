//! Hash and lattice noise mirrored by `shaders/include/noise.glsl`.

use crate::smooth_step::mix;

/// Jarzynski & Olano, "Hash Functions for GPU Rendering" (2020).
pub fn pcg3d(v: [u32; 3]) -> [u32; 3] {
    let mut v = v.map(|component| component.wrapping_mul(1664525).wrapping_add(1013904223));
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v = v.map(|component| component ^ (component >> 16));
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v
}

/// Uniform value in [0, 1) from the bit pattern of `p`.
pub fn hash13(p: [f32; 3]) -> f32 {
    let h = pcg3d([p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]);
    h[0] as f32 * (1.0 / 4294967296.0)
}

/// Perlin's improved fade 6t^5 - 15t^4 + 10t^3.
pub fn quintic_fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lattice_gradient(cell: [f32; 3]) -> [f32; 3] {
    let g = [
        2.0 * hash13(cell) - 1.0,
        2.0 * hash13([cell[0] + 17.1, cell[1] + 9.3, cell[2] + 4.7]) - 1.0,
        2.0 * hash13([cell[0] + 31.7, cell[1] + 2.9, cell[2] + 12.3]) - 1.0,
    ];
    let inv_len = 1.0 / (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt().max(1e-4);
    [g[0] * inv_len, g[1] * inv_len, g[2] * inv_len]
}

fn corner_dot(cell: [f32; 3], f: [f32; 3], dx: f32, dy: f32, dz: f32) -> f32 {
    let g = lattice_gradient([cell[0] + dx, cell[1] + dy, cell[2] + dz]);
    g[0] * (f[0] - dx) + g[1] * (f[1] - dy) + g[2] * (f[2] - dz)
}

/// Perlin gradient noise on the unit lattice, zero at every lattice point.
pub fn gradient_noise3(p: [f32; 3]) -> f32 {
    let cell = [p[0].floor(), p[1].floor(), p[2].floor()];
    let f = [p[0] - cell[0], p[1] - cell[1], p[2] - cell[2]];
    let w = [quintic_fade(f[0]), quintic_fade(f[1]), quintic_fade(f[2])];

    let nx00 = mix(
        corner_dot(cell, f, 0.0, 0.0, 0.0),
        corner_dot(cell, f, 1.0, 0.0, 0.0),
        w[0],
    );
    let nx10 = mix(
        corner_dot(cell, f, 0.0, 1.0, 0.0),
        corner_dot(cell, f, 1.0, 1.0, 0.0),
        w[0],
    );
    let nx01 = mix(
        corner_dot(cell, f, 0.0, 0.0, 1.0),
        corner_dot(cell, f, 1.0, 0.0, 1.0),
        w[0],
    );
    let nx11 = mix(
        corner_dot(cell, f, 0.0, 1.0, 1.0),
        corner_dot(cell, f, 1.0, 1.0, 1.0),
        w[0],
    );
    let nxy0 = mix(nx00, nx10, w[1]);
    let nxy1 = mix(nx01, nx11, w[1]);
    mix(nxy0, nxy1, w[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stays_in_unit_interval_and_is_deterministic() {
        for i in 0..256 {
            let p = [i as f32 * 0.37, -(i as f32) * 1.3, 4.2];
            let value = hash13(p);
            assert!((0.0..1.0).contains(&value), "{value} out of range");
            assert_eq!(value, hash13(p));
        }
    }

    #[test]
    fn quintic_fade_is_a_smooth_ramp() {
        assert_eq!(quintic_fade(0.0), 0.0);
        assert_eq!(quintic_fade(1.0), 1.0);
        assert!((quintic_fade(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gradient_noise_vanishes_on_lattice_points() {
        for x in -3..3 {
            for z in -3..3 {
                let value = gradient_noise3([x as f32, 1.0, z as f32]);
                assert!(value.abs() < 1e-6, "lattice point noise {value}");
            }
        }
    }

    #[test]
    fn gradient_noise_stays_bounded_off_lattice() {
        for i in 0..512 {
            let p = [
                i as f32 * 0.113,
                i as f32 * 0.071 + 0.5,
                -(i as f32) * 0.029,
            ];
            let value = gradient_noise3(p);
            assert!(value.abs() <= 1.0, "{value} outside [-1, 1]");
        }
    }
}
