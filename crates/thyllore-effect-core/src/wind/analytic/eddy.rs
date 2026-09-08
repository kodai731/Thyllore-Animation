use crate::wind::analytic::shell_integral::WindShellParams;

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

pub struct EddyGeometry {
    pub theta: f32,
    pub height: f32,
    pub shear_rate: f32,
    pub radial_coord: f32,
    pub period_theta: f32,
}

const EDDY_MIN_RADIUS_SQ: f32 = 1e-4;

pub fn eddy_geometry(params: &WindShellParams, local: [f32; 3]) -> EddyGeometry {
    let r = (local[0] * local[0] + local[2] * local[2]).sqrt();
    let theta = local[2].atan2(local[0]);
    let h = local[1];
    let wall_radius = params.wall_radius(h);

    let omega = (params.streak_phase / params.time.max(1e-3))
        * (params.wall_radius_sq(h) / (r * r).max(EDDY_MIN_RADIUS_SQ));
    let n_theta = (2.0 * PI * wall_radius / params.eddy_cell_theta)
        .round()
        .max(1.0);

    EddyGeometry {
        theta,
        height: h,
        shear_rate: omega,
        radial_coord: (r - wall_radius) / params.eddy_cell_radial,
        period_theta: n_theta,
    }
}

pub fn eddy_layer_coords(
    params: &WindShellParams,
    geometry: &EddyGeometry,
    age: f32,
    seed: f32,
) -> [f32; 3] {
    let phi = mix(
        params.streak_phase,
        geometry.shear_rate * age,
        params.eddy_shear,
    );

    let u_theta = geometry.period_theta * (geometry.theta - phi) / (2.0 * PI);
    let u_h = (geometry.height - params.eddy_rise_speed * age) / params.eddy_cell_height;

    [
        u_theta + seed,
        u_h + seed * 0.37,
        geometry.radial_coord + seed * 0.61,
    ]
}

pub fn eddy_sigma(params: &WindShellParams, local: [f32; 3]) -> f32 {
    let reseed_period = params.eddy_reseed_period;
    let t = params.time;

    let k_a = (t / reseed_period).floor();
    let age_a = t - k_a * reseed_period;
    let w_a = 1.0 - (2.0 * age_a / reseed_period - 1.0).abs();

    let k_b = (t / reseed_period + 0.5).floor();
    let age_b = t + 0.5 * reseed_period - k_b * reseed_period;
    let w_b = 1.0 - w_a;

    let geometry = eddy_geometry(params, local);

    let coords_a = eddy_layer_coords(params, &geometry, age_a, 17.0 * k_a + 3.0);
    let noise_a = periodic_noise_fbm(coords_a, geometry.period_theta);
    let coords_b = eddy_layer_coords(params, &geometry, age_b, 17.0 * k_b + 3.0);
    let noise_b = periodic_noise_fbm(coords_b, geometry.period_theta);

    let noise = w_a * noise_a + w_b * noise_b;
    let eroded = ((noise - params.eddy_erosion) / (1.0 - params.eddy_erosion)).clamp(0.0, 1.0);
    1.0 + params.eddy_amplitude * (2.0 * eroded - 1.0)
}
