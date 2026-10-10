use cgmath::{Quaternion, Rad, Rotation3, Vector3};

pub fn quaternion_to_euler_degrees(q: &Quaternion<f32>) -> Vector3<f32> {
    let sinr_cosp = 2.0 * (q.s * q.v.x + q.v.y * q.v.z);
    let cosr_cosp = 1.0 - 2.0 * (q.v.x * q.v.x + q.v.y * q.v.y);
    let x_rot = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (q.s * q.v.y - q.v.z * q.v.x);
    let y_rot = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (q.s * q.v.z + q.v.x * q.v.y);
    let cosy_cosp = 1.0 - 2.0 * (q.v.y * q.v.y + q.v.z * q.v.z);
    let z_rot = siny_cosp.atan2(cosy_cosp);

    Vector3::new(x_rot.to_degrees(), y_rot.to_degrees(), z_rot.to_degrees())
}

pub fn euler_degrees_to_quaternion(euler_degrees: &Vector3<f32>) -> Quaternion<f32> {
    let qx = Quaternion::from_angle_x(Rad(euler_degrees.x.to_radians()));
    let qy = Quaternion::from_angle_y(Rad(euler_degrees.y.to_radians()));
    let qz = Quaternion::from_angle_z(Rad(euler_degrees.z.to_radians()));

    qz * qy * qx
}

pub fn unwrap_degrees(angle: f32, previous: f32) -> f32 {
    previous + (angle - previous + 180.0).rem_euclid(360.0) - 180.0
}

pub fn continuous_euler(euler: Vector3<f32>, prev: Option<Vector3<f32>>) -> Vector3<f32> {
    match prev {
        None => euler,
        Some(p) => Vector3::new(
            unwrap_degrees(euler.x, p.x),
            unwrap_degrees(euler.y, p.y),
            unwrap_degrees(euler.z, p.z),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::AbsDiffEq;

    #[test]
    fn roundtrip_identity() {
        let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let euler = quaternion_to_euler_degrees(&q);
        let q2 = euler_degrees_to_quaternion(&euler);
        assert!(q.abs_diff_eq(&q2, 1e-5));
    }

    #[test]
    fn roundtrip_90_degree_x() {
        let q = Quaternion::from_angle_x(Rad(90.0_f32.to_radians()));
        let euler = quaternion_to_euler_degrees(&q);
        assert!((euler.x - 90.0).abs() < 0.01);
        assert!(euler.y.abs() < 0.01);
        assert!(euler.z.abs() < 0.01);

        let q2 = euler_degrees_to_quaternion(&euler);
        assert!(q.abs_diff_eq(&q2, 1e-5));
    }

    #[test]
    fn roundtrip_combined_rotation() {
        let euler = Vector3::new(30.0, 45.0, 60.0);
        let q = euler_degrees_to_quaternion(&euler);
        let euler2 = quaternion_to_euler_degrees(&q);
        assert!((euler.x - euler2.x).abs() < 0.01);
        assert!((euler.y - euler2.y).abs() < 0.01);
        assert!((euler.z - euler2.z).abs() < 0.01);
    }

    #[test]
    fn test_continuous_euler_z_170_to_190() {
        let q1 = Quaternion::from_angle_z(Rad(170.0_f32.to_radians()));
        let q2 = Quaternion::from_angle_z(Rad(190.0_f32.to_radians()));

        let euler1 = quaternion_to_euler_degrees(&q1);
        let continuous1 = continuous_euler(euler1, None);

        let euler2 = quaternion_to_euler_degrees(&q2);
        let continuous2 = continuous_euler(euler2, Some(continuous1));

        assert!(
            (continuous1.z - 170.0).abs() < 0.01,
            "first key z={:.2}, expected ~170",
            continuous1.z
        );
        assert!(
            (continuous2.z - 190.0).abs() < 0.01,
            "second key z={:.2}, expected ~190 (not -170)",
            continuous2.z
        );
    }
}
