use crate::wind::analytic::shell_integral::WindShellParams;

// Mirror of the eddy field in shaders/wind/include/wind_shell_field.glsl and of pcg3d / hash13
// in shaders/include/noise.glsl; expressions, constants and operation order are kept identical.

const PI: f32 = 3.14159265;

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn pcg3d(v: [u32; 3]) -> [u32; 3] {
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

pub fn hash13(p: [f32; 3]) -> f32 {
    let h = pcg3d([p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]);
    h[0] as f32 * (1.0 / 4294967296.0)
}

/// Value noise whose x lattice wraps every `period_theta` cells, closing around the column.
pub fn periodic_noise(u: [f32; 3], period_theta: f32) -> f32 {
    let cell = [u[0].floor(), u[1].floor(), u[2].floor()];
    let f = [u[0] - cell[0], u[1] - cell[1], u[2] - cell[2]];
    let w = [
        f[0] * f[0] * (3.0 - 2.0 * f[0]),
        f[1] * f[1] * (3.0 - 2.0 * f[1]),
        f[2] * f[2] * (3.0 - 2.0 * f[2]),
    ];

    let period = period_theta as i32;
    let cx = (cell[0] as i32).rem_euclid(period) as f32;
    let cx1 = (cell[0] as i32 + 1).rem_euclid(period) as f32;

    let n000 = hash13([cx, cell[1], cell[2]]);
    let n100 = hash13([cx1, cell[1], cell[2]]);
    let n010 = hash13([cx, cell[1] + 1.0, cell[2]]);
    let n110 = hash13([cx1, cell[1] + 1.0, cell[2]]);
    let n001 = hash13([cx, cell[1], cell[2] + 1.0]);
    let n101 = hash13([cx1, cell[1], cell[2] + 1.0]);
    let n011 = hash13([cx, cell[1] + 1.0, cell[2] + 1.0]);
    let n111 = hash13([cx1, cell[1] + 1.0, cell[2] + 1.0]);

    let nx00 = mix(n000, n100, w[0]);
    let nx10 = mix(n010, n110, w[0]);
    let nx01 = mix(n001, n101, w[0]);
    let nx11 = mix(n011, n111, w[0]);
    let nxy0 = mix(nx00, nx10, w[1]);
    let nxy1 = mix(nx01, nx11, w[1]);
    mix(nxy0, nxy1, w[2])
}

fn periodic_noise_octave(u: [f32; 3], period_theta: f32, freq: f32) -> f32 {
    periodic_noise([freq * u[0], freq * u[1], freq * u[2]], period_theta * freq)
}

pub fn periodic_noise_fbm(u: [f32; 3], period_theta: f32) -> f32 {
    let mut sum = 0.5 * periodic_noise_octave(u, period_theta, 1.0);
    sum += 0.25 * periodic_noise_octave(u, period_theta, 2.0);
    sum += 0.125 * periodic_noise_octave(u, period_theta, 4.0);
    sum * (1.0 / 0.875)
}

/// Noise coordinates of `local` and the angular cell count used as their x period.
pub fn eddy_coords(
    params: &WindShellParams,
    local: [f32; 3],
    age: f32,
    seed: f32,
) -> ([f32; 3], f32) {
    let r = (local[0] * local[0] + local[2] * local[2]).sqrt();
    let theta = local[2].atan2(local[0]);
    let h = local[1];

    let omega = (params.streak_phase / params.time.max(1e-3))
        * (params.wall_radius_sq(h) / (r * r).max(params.core_radius_sq));
    let phi = mix(params.streak_phase, omega * age, params.eddy_shear);

    let n_theta = (2.0 * PI * params.wall_radius(h) / params.eddy_cell_theta)
        .round()
        .max(1.0);
    let u_theta = n_theta * (theta - phi) / (2.0 * PI);
    let u_h = (h - params.eddy_rise_speed * age) / params.eddy_cell_height;
    let u_r = (r - params.wall_radius(h)) / params.eddy_cell_radial;

    (
        [u_theta + seed, u_h + seed * 0.37, u_r + seed * 0.61],
        n_theta,
    )
}

/// Density multiplier of the eddy field: two noise layers crossfaded so neither pops on reseed.
pub fn eddy_sigma(params: &WindShellParams, local: [f32; 3]) -> f32 {
    let reseed_period = params.eddy_reseed_period;
    let t = params.time;

    let k_a = (t / reseed_period).floor();
    let age_a = t - k_a * reseed_period;
    let w_a = 1.0 - (2.0 * age_a / reseed_period - 1.0).abs();

    let k_b = (t / reseed_period + 0.5).floor();
    let age_b = t + 0.5 * reseed_period - k_b * reseed_period;
    let w_b = 1.0 - w_a;

    let (coords_a, period) = eddy_coords(params, local, age_a, 17.0 * k_a + 3.0);
    let noise_a = periodic_noise_fbm(coords_a, period);
    let (coords_b, period) = eddy_coords(params, local, age_b, 17.0 * k_b + 3.0);
    let noise_b = periodic_noise_fbm(coords_b, period);

    let noise = w_a * noise_a + w_b * noise_b;
    1.0 + params.eddy_amplitude * (2.0 * noise - 1.0)
}
