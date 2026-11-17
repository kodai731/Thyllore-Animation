use glam::{DVec3, Vec3};
use std::f64::consts::PI;

pub fn compute_triangle_solid_angle(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> f32 {
    compute_triangle_solid_angle_f64(a, b, c, p) as f32
}

pub fn compute_winding_number(triangles: &[[Vec3; 3]], p: Vec3) -> f32 {
    let total_solid_angle: f64 = triangles
        .iter()
        .map(|triangle| compute_triangle_solid_angle_f64(triangle[0], triangle[1], triangle[2], p))
        .sum();
    (total_solid_angle / (4.0 * PI)) as f32
}

pub fn verify_shell_is_closed(triangles: &[[Vec3; 3]], probes: &[Vec3]) -> bool {
    probes.iter().all(|&probe| {
        let winding = compute_winding_number(triangles, probe);
        (winding - winding.round()).abs() < 1e-3
    })
}

fn compute_triangle_solid_angle_f64(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> f64 {
    let ra = to_relative_f64(a, p);
    let rb = to_relative_f64(b, p);
    let rc = to_relative_f64(c, p);

    let la = ra.length();
    let lb = rb.length();
    let lc = rc.length();

    let numerator = ra.dot(rb.cross(rc));
    let denominator = la * lb * lc + ra.dot(rb) * lc + rb.dot(rc) * la + rc.dot(ra) * lb;
    2.0 * numerator.atan2(denominator)
}

fn to_relative_f64(v: Vec3, p: Vec3) -> DVec3 {
    (v - p).as_dvec3()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI as PI_F32;

    fn quad_to_triangles(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> [[Vec3; 3]; 2] {
        [[a, b, c], [a, c, d]]
    }

    fn build_cube_triangles() -> Vec<[Vec3; 3]> {
        let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        let faces = [
            quad_to_triangles(
                v(1.0, -1.0, -1.0),
                v(1.0, 1.0, -1.0),
                v(1.0, 1.0, 1.0),
                v(1.0, -1.0, 1.0),
            ),
            quad_to_triangles(
                v(-1.0, -1.0, -1.0),
                v(-1.0, -1.0, 1.0),
                v(-1.0, 1.0, 1.0),
                v(-1.0, 1.0, -1.0),
            ),
            quad_to_triangles(
                v(-1.0, 1.0, -1.0),
                v(-1.0, 1.0, 1.0),
                v(1.0, 1.0, 1.0),
                v(1.0, 1.0, -1.0),
            ),
            quad_to_triangles(
                v(-1.0, -1.0, -1.0),
                v(1.0, -1.0, -1.0),
                v(1.0, -1.0, 1.0),
                v(-1.0, -1.0, 1.0),
            ),
            quad_to_triangles(
                v(-1.0, -1.0, 1.0),
                v(1.0, -1.0, 1.0),
                v(1.0, 1.0, 1.0),
                v(-1.0, 1.0, 1.0),
            ),
            quad_to_triangles(
                v(-1.0, -1.0, -1.0),
                v(-1.0, 1.0, -1.0),
                v(1.0, 1.0, -1.0),
                v(1.0, -1.0, -1.0),
            ),
        ];
        faces.into_iter().flatten().collect()
    }

    fn flip_triangles(triangles: &[[Vec3; 3]]) -> Vec<[Vec3; 3]> {
        triangles.iter().map(|t| [t[0], t[2], t[1]]).collect()
    }

    #[test]
    fn test_compute_triangle_solid_angle_octant() {
        let solid_angle = compute_triangle_solid_angle(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
        );
        assert!((solid_angle - PI_F32 / 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_compute_winding_number_cube_interior() {
        let cube = build_cube_triangles();
        let winding = compute_winding_number(&cube, Vec3::new(0.2, -0.3, 0.4));
        assert!((winding - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_compute_winding_number_cube_exterior() {
        let cube = build_cube_triangles();
        let winding = compute_winding_number(&cube, Vec3::new(2.0, 0.5, -0.3));
        assert!(winding.abs() < 1e-4);
    }

    #[test]
    fn test_compute_winding_number_flipped_normals() {
        let flipped = flip_triangles(&build_cube_triangles());
        let winding = compute_winding_number(&flipped, Vec3::new(0.2, -0.3, 0.4));
        assert!((winding + 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_verify_shell_is_closed_for_cube() {
        let cube = build_cube_triangles();
        let probes = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        assert!(verify_shell_is_closed(&cube, &probes));
    }

    #[test]
    fn test_verify_shell_is_closed_detects_missing_face() {
        let mut cube = build_cube_triangles();
        cube.truncate(cube.len() - 2);
        let probes = [Vec3::new(0.0, 0.0, 0.0)];
        assert!(!verify_shell_is_closed(&cube, &probes));
    }
}
