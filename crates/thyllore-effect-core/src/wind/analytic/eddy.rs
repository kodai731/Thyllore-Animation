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

fn quintic_fade(t: f32) -> f32 {
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

pub fn gradient_noise(p: [f32; 3]) -> f32 {
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

const OCTAVE_ROTATION: [[f32; 3]; 3] = [
    [0.784750, -0.045714, 0.618124],
    [0.509329, 0.615862, -0.601081],
    [-0.353201, 0.786527, 0.506581],
];

fn rotate_and_double(p: [f32; 3]) -> [f32; 3] {
    let r = &OCTAVE_ROTATION;
    [
        2.0 * (r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2]),
        2.0 * (r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2]),
        2.0 * (r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2]),
    ]
}

fn rotate(p: [f32; 3]) -> [f32; 3] {
    let r = &OCTAVE_ROTATION;
    [
        r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2],
        r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2],
        r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2],
    ]
}

// Difference against the antipode (theta + pi) has an exact zero mean around every ring,
// so no height can become a uniformly dense or empty band.
fn antipodal_octave(p: [f32; 3], antipode: [f32; 3]) -> f32 {
    (gradient_noise(p) - gradient_noise(antipode)) * std::f32::consts::FRAC_1_SQRT_2
}

pub fn eddy_noise_fbm(p: [f32; 3], antipode: [f32; 3]) -> f32 {
    let p0 = rotate(p);
    let q0 = rotate(antipode);
    let p1 = rotate_and_double(p0);
    let q1 = rotate_and_double(q0);
    let p2 = rotate_and_double(p1);
    let q2 = rotate_and_double(q1);
    let sum = 0.5 * antipodal_octave(p0, q0)
        + 0.25 * antipodal_octave(p1, q1)
        + 0.125 * antipodal_octave(p2, q2);
    (0.5 + sum * (1.0 / 0.875)).clamp(0.0, 1.0)
}

pub struct EddyGeometry {
    pub theta: f32,
    pub height: f32,
    pub shear_rate: f32,
    pub radial_coord: f32,
    pub ring_radius: f32,
}

const EDDY_MIN_RADIUS_SQ: f32 = 1e-4;

pub fn eddy_geometry(params: &WindShellParams, local: [f32; 3]) -> EddyGeometry {
    let r = (local[0] * local[0] + local[2] * local[2]).sqrt();
    let theta = local[2].atan2(local[0]);
    let h = local[1];
    let wall_radius = params.wall_radius(h);

    let omega = (params.streak_phase / params.time.max(1e-3))
        * (params.wall_radius_sq(h) / (r * r).max(EDDY_MIN_RADIUS_SQ));

    EddyGeometry {
        theta,
        height: h,
        shear_rate: omega,
        radial_coord: (r - wall_radius) / params.eddy_cell_radial,
        ring_radius: wall_radius / params.eddy_cell_theta,
    }
}

pub fn eddy_layer_coords(
    params: &WindShellParams,
    geometry: &EddyGeometry,
    age: f32,
    seed: f32,
) -> [[f32; 3]; 2] {
    let phi = mix(
        params.streak_phase,
        geometry.shear_rate * age,
        params.eddy_shear,
    );

    let sheared_theta = geometry.theta - phi;
    let rho = geometry.ring_radius + geometry.radial_coord;
    let u_h = (geometry.height - params.eddy_rise_speed * age) / params.eddy_cell_height;

    [
        [
            rho * sheared_theta.cos() + seed,
            rho * sheared_theta.sin() + seed * 0.37,
            u_h + seed * 0.61,
        ],
        [
            -rho * sheared_theta.cos() + seed,
            -rho * sheared_theta.sin() + seed * 0.37,
            u_h + seed * 0.61,
        ],
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

    let [pa, qa] = eddy_layer_coords(params, &geometry, age_a, 17.0 * k_a + 3.0);
    let [pb, qb] = eddy_layer_coords(params, &geometry, age_b, 17.0 * k_b + 3.0);
    let noise_a = eddy_noise_fbm(pa, qa);
    let noise_b = eddy_noise_fbm(pb, qb);

    let noise = w_a * noise_a + w_b * noise_b;
    let eroded = ((noise - params.eddy_erosion) / (1.0 - params.eddy_erosion)).clamp(0.0, 1.0);
    1.0 + params.eddy_amplitude * (2.0 * eroded - 1.0)
}
