use cgmath::{InnerSpace, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleClosestPoint {
    pub position: Vector3<f32>,
    pub barycentric: Vector3<f32>,
}

// Ericson, Real-Time Collision Detection 5.1.5.
pub fn closest_point_on_triangle(
    point: Vector3<f32>,
    a: Vector3<f32>,
    b: Vector3<f32>,
    c: Vector3<f32>,
) -> TriangleClosestPoint {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);

    if d1 <= 0.0 && d2 <= 0.0 {
        return TriangleClosestPoint {
            position: a,
            barycentric: Vector3::new(1.0, 0.0, 0.0),
        };
    }

    let bp = point - b;
    let cp = point - c;

    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);

    if d3 >= 0.0 && d4 <= d3 {
        return TriangleClosestPoint {
            position: b,
            barycentric: Vector3::new(0.0, 1.0, 0.0),
        };
    }

    let vc = d1 * d4 - d3 * d2;

    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return TriangleClosestPoint {
            position: a + ab * v,
            barycentric: Vector3::new(1.0 - v, v, 0.0),
        };
    }

    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);

    if d6 >= 0.0 && d5 <= d6 {
        return TriangleClosestPoint {
            position: c,
            barycentric: Vector3::new(0.0, 0.0, 1.0),
        };
    }

    let vb = d5 * d2 - d6 * d1;

    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return TriangleClosestPoint {
            position: a + ac * w,
            barycentric: Vector3::new(1.0 - w, 0.0, w),
        };
    }

    let va = d3 * d6 - d5 * d4;

    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return TriangleClosestPoint {
            position: b + (c - b) * w,
            barycentric: Vector3::new(0.0, 1.0 - w, w),
        };
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let u = 1.0 - v - w;

    TriangleClosestPoint {
        position: a + ab * v + ac * w,
        barycentric: Vector3::new(u, v, w),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::MetricSpace;

    #[test]
    fn test_interior_projection() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(1.0, 1.732_050_8, 0.0);
        let point = Vector3::new(1.0, 0.5, 1.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!((result.position.x - 1.0).abs() < 1e-6);
        assert!((result.position.y - 0.5).abs() < 1e-6);
        assert!(result.position.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6, "barycentric sum = {}", sum);

        assert!(result.barycentric.x > 0.0);
        assert!(result.barycentric.y > 0.0);
        assert!(result.barycentric.z > 0.0);
    }

    #[test]
    fn test_vertex_a_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(0.0, 2.0, 0.0);
        let point = Vector3::new(-1.0, -1.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!(
            result.position.distance(a) < 1e-6,
            "position = {:?}",
            result.position
        );
        assert!((result.barycentric.x - 1.0).abs() < 1e-6);
        assert!(result.barycentric.y.abs() < 1e-6);
        assert!(result.barycentric.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_vertex_b_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(0.0, 2.0, 0.0);
        let point = Vector3::new(3.0, -1.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!(
            result.position.distance(b) < 1e-6,
            "position = {:?}",
            result.position
        );
        assert!(result.barycentric.x.abs() < 1e-6);
        assert!((result.barycentric.y - 1.0).abs() < 1e-6);
        assert!(result.barycentric.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_vertex_c_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(0.0, 2.0, 0.0);
        let point = Vector3::new(-1.0, 3.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!(
            result.position.distance(c) < 1e-6,
            "position = {:?}",
            result.position
        );
        assert!(result.barycentric.x.abs() < 1e-6);
        assert!(result.barycentric.y.abs() < 1e-6);
        assert!((result.barycentric.z - 1.0).abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_edge_ab_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(1.0, 2.0, 0.0);
        let point = Vector3::new(1.0, -1.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!((result.position.x - 1.0).abs() < 1e-6);
        assert!(result.position.y.abs() < 1e-6);
        assert!(result.position.z.abs() < 1e-6);

        assert!((result.barycentric.x - 0.5).abs() < 1e-6);
        assert!((result.barycentric.y - 0.5).abs() < 1e-6);
        assert!(result.barycentric.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_edge_bc_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(1.0, 2.0, 0.0);
        let point = Vector3::new(2.5, 1.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!((result.position.x - 1.7).abs() < 1e-5);
        assert!((result.position.y - 0.6).abs() < 1e-5);
        assert!(result.position.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);

        assert!(result.barycentric.x.abs() < 1e-5);
    }

    #[test]
    fn test_edge_ca_region() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(1.0, 2.0, 0.0);
        let point = Vector3::new(-0.5, 1.0, 0.0);

        let result = closest_point_on_triangle(point, a, b, c);

        assert!((result.position.x - 0.3).abs() < 1e-5);
        assert!((result.position.y - 0.6).abs() < 1e-5);
        assert!(result.position.z.abs() < 1e-6);

        let sum: f32 = result.barycentric.x + result.barycentric.y + result.barycentric.z;
        assert!((sum - 1.0).abs() < 1e-6);

        assert!(result.barycentric.y.abs() < 1e-5);
    }
}
