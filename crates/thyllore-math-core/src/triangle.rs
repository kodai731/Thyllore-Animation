use cgmath::{InnerSpace, Vector3};

/// The point of triangle `abc` closest to `p`. Ericson, Real-Time Collision Detection 5.1.5.
pub fn closest_point_on_triangle(
    p: Vector3<f32>,
    a: Vector3<f32>,
    b: Vector3<f32>,
    c: Vector3<f32>,
) -> Vector3<f32> {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }

    let denominator = 1.0 / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    a + ab * v + ac * w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
        (
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        )
    }

    #[test]
    fn interior_point_projects_onto_the_plane() {
        let (a, b, c) = triangle();
        let closest = closest_point_on_triangle(Vector3::new(0.25, 0.25, 2.0), a, b, c);
        assert!((closest - Vector3::new(0.25, 0.25, 0.0)).magnitude() < 1e-6);
    }

    #[test]
    fn points_beyond_a_corner_or_an_edge_land_on_it() {
        let (a, b, c) = triangle();
        let corner = closest_point_on_triangle(Vector3::new(-1.0, -1.0, 0.5), a, b, c);
        assert!((corner - a).magnitude() < 1e-6);
        let edge = closest_point_on_triangle(Vector3::new(0.5, -1.0, 0.0), a, b, c);
        assert!((edge - Vector3::new(0.5, 0.0, 0.0)).magnitude() < 1e-6);
        let hypotenuse = closest_point_on_triangle(Vector3::new(1.0, 1.0, 0.0), a, b, c);
        assert!((hypotenuse - Vector3::new(0.5, 0.5, 0.0)).magnitude() < 1e-6);
    }
}
