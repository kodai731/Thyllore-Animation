use super::*;

const RAY_COUNT: usize = 2000;

fn sample_shells() -> [VolumeShell; 3] {
    [
        VolumeShell {
            height: 4.0,
            radius_base: 1.0,
            radius_slope: 0.6,
            radius_offset_q: 0.0,
            width_q: 0.4,
            strength: 1.2,
            h_top: 0.9,
            top_fade: 0.3,
            sigma_t: 0.8,
        },
        VolumeShell {
            height: 2.5,
            radius_base: 0.5,
            radius_slope: -0.2,
            radius_offset_q: 0.1,
            width_q: 0.15,
            strength: 2.0,
            h_top: 1.0,
            top_fade: 0.05,
            sigma_t: 1.5,
        },
        VolumeShell {
            height: 6.0,
            radius_base: 2.0,
            radius_slope: 1.5,
            radius_offset_q: -0.3,
            width_q: 1.0,
            strength: 0.7,
            h_top: 0.6,
            top_fade: 0.6,
            sigma_t: 0.3,
        },
    ]
}

fn random_ray(rng: &mut fastrand::Rng, shell: &VolumeShell) -> (Vector3<f32>, Vector3<f32>) {
    let reach = shell
        .envelope_radius(shell.h_top)
        .max(shell.envelope_radius(0.0))
        * 2.0;
    let origin = Vector3::new(
        (rng.f32() * 2.0 - 1.0) * reach,
        (rng.f32() * 1.4 - 0.2) * shell.height,
        (rng.f32() * 2.0 - 1.0) * reach,
    );
    let direction = Vector3::new(
        rng.f32() * 2.0 - 1.0,
        rng.f32() * 2.0 - 1.0,
        rng.f32() * 2.0 - 1.0,
    );
    (origin, direction)
}

struct Mismatch {
    count: usize,
    worst_abs: f32,
    worst_rel: f32,
}

fn random_wall_point(rng: &mut fastrand::Rng, shell: &VolumeShell) -> (Vector3<f32>, Vector3<f32>) {
    let h = rng.f32() * 1.2 * shell.h_top;
    let half_width = shell.width_q.sqrt();
    let radius = shell.wall_radius(h) + (rng.f32() * 2.0 - 1.0) * 1.5 * half_width;
    let angle = rng.f32() * std::f32::consts::TAU;
    let point = Vector3::new(radius * angle.cos(), h * shell.height, radius * angle.sin());
    (point, Vector3::new(0.0, 0.0, 0.0))
}

fn compare<S, F, G>(name: &str, sample: S, rust: F, slang: G) -> Mismatch
where
    S: Fn(&mut fastrand::Rng, &VolumeShell) -> (Vector3<f32>, Vector3<f32>),
    F: Fn(&VolumeShell, Vector3<f32>, Vector3<f32>) -> f32,
    G: Fn(&VolumeShell, Vector3<f32>, Vector3<f32>) -> f32,
{
    let mut rng = fastrand::Rng::with_seed(0x5148);
    let mut mismatch = Mismatch {
        count: 0,
        worst_abs: 0.0,
        worst_rel: 0.0,
    };
    let mut evaluated = 0usize;
    let mut nonzero = 0usize;
    for shell in sample_shells() {
        for _ in 0..RAY_COUNT {
            let (origin, direction) = sample(&mut rng, &shell);
            let expected = rust(&shell, origin, direction);
            let actual = slang(&shell, origin, direction);
            evaluated += 1;
            if expected != 0.0 {
                nonzero += 1;
            }
            if expected.to_bits() != actual.to_bits() {
                mismatch.count += 1;
                let abs = (expected - actual).abs();
                mismatch.worst_abs = mismatch.worst_abs.max(abs);
                mismatch.worst_rel = mismatch
                    .worst_rel
                    .max(abs / expected.abs().max(f32::MIN_POSITIVE));
            }
        }
    }
    println!(
        "{name}: {evaluated} rays ({nonzero} non-zero), {} not bit-identical, worst abs {:e}, worst rel {:e}",
        mismatch.count, mismatch.worst_abs, mismatch.worst_rel
    );
    assert!(
        nonzero > evaluated / 10,
        "{name}: too few rays hit the shell"
    );
    mismatch
}

#[test]
fn optical_depth_matches_rust_mirror() {
    let mismatch = compare(
        "shellOpticalDepth",
        random_ray,
        |shell, o, d| shell.optical_depth(o, d, 0.0, 50.0),
        |shell, o, d| shell_optical_depth(shell, o, d, 0.0, 50.0),
    );
    assert!(
        mismatch.worst_rel <= 1e-4,
        "relative error {}",
        mismatch.worst_rel
    );
}

#[test]
fn optical_depth_toward_matches_rust_mirror() {
    let mismatch = compare(
        "shellOpticalDepthToward",
        random_ray,
        |shell, o, d| shell.optical_depth_toward(o, d, 1e4),
        |shell, o, d| shell_optical_depth_toward(shell, o, d, 1e4),
    );
    assert!(
        mismatch.worst_rel <= 1e-4,
        "relative error {}",
        mismatch.worst_rel
    );
}

#[test]
fn density_at_matches_rust_mirror() {
    let mismatch = compare(
        "shellDensityAt",
        random_wall_point,
        |shell, o, _| shell.density_at(o),
        |shell, o, _| shell_density_at(shell, o),
    );
    assert!(
        mismatch.worst_rel <= 1e-5,
        "relative error {}",
        mismatch.worst_rel
    );
}
