use glam::{Mat3, Quat, Vec3};

use crate::humanoid::components::humanoid_frame::HumanoidFrame;

pub fn role_rotation_to_thyllore(frame: &HumanoidFrame, euler_degrees: [f32; 3]) -> Quat {
    let [x_degrees, y_degrees, z_degrees] = euler_degrees;
    let unity_rotation = Mat3::from_rotation_y(f32::to_radians(y_degrees))
        * Mat3::from_rotation_x(f32::to_radians(x_degrees))
        * Mat3::from_rotation_z(f32::to_radians(z_degrees));

    let unity_to_thyllore = Mat3::from_cols(
        Vec3::from(frame.right),
        Vec3::from(frame.up),
        Vec3::from(frame.forward),
    );

    let thyllore_rotation = unity_to_thyllore * unity_rotation * unity_to_thyllore.transpose();
    Quat::from_mat3(&thyllore_rotation)
}

pub fn thyllore_rotation_to_role(frame: &HumanoidFrame, rotation: Quat) -> [f32; 3] {
    let unity_to_thyllore = Mat3::from_cols(
        Vec3::from(frame.right),
        Vec3::from(frame.up),
        Vec3::from(frame.forward),
    );
    let unity_rotation =
        unity_to_thyllore.transpose() * Mat3::from_quat(rotation) * unity_to_thyllore;

    let x_radians = (-unity_rotation.col(2)[1]).clamp(-1.0, 1.0).asin();
    let y_radians = unity_rotation.col(2)[0].atan2(unity_rotation.col(2)[2]);
    let z_radians = unity_rotation.col(0)[1].atan2(unity_rotation.col(1)[1]);

    [
        x_radians.to_degrees(),
        y_radians.to_degrees(),
        z_radians.to_degrees(),
    ]
}

#[cfg(test)]
mod tests {
    use glam::{Mat3, Vec3};

    use crate::humanoid::components::humanoid_frame::HumanoidFrame;

    use super::*;

    const CANONICAL: HumanoidFrame = HumanoidFrame {
        right: [-1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        forward: [0.0, 0.0, 1.0],
    };

    fn rotated_frame() -> HumanoidFrame {
        let angle = std::f32::consts::PI / 4.0;
        let c = angle.cos();
        let s = angle.sin();

        let ry: Mat3 = Mat3::from_cols_array(&[c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c]);

        let base_right = Vec3::new(-1.0, 0.0, 0.0);
        let base_up = Vec3::new(0.0, 1.0, 0.0);
        let base_forward = Vec3::new(0.0, 0.0, 1.0);

        let right = ry * base_right;
        let up = ry * base_up;
        let forward = ry * base_forward;

        HumanoidFrame {
            right: [right.x, right.y, right.z],
            up: [up.x, up.y, up.z],
            forward: [forward.x, forward.y, forward.z],
        }
    }

    fn apply_quaternion(q: Quat, v: Vec3) -> Vec3 {
        let q_inv = q.inverse();
        let v_q = Quat::from_xyzw(v.x, v.y, v.z, 0.0);
        let result = q * v_q * q_inv;
        Vec3::new(result.x, result.y, result.z)
    }

    fn almost_equal(a: Vec3, b: Vec3) -> bool {
        a.dot(b) > 0.999
    }

    #[test]
    fn test_right_upper_arm_z_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, 0.0, 90.0]);
            let right = Vec3::new(frame.right[0], frame.right[1], frame.right[2]);
            let up = Vec3::new(frame.up[0], frame.up[1], frame.up[2]);
            let rotated_right = apply_quaternion(q, right);
            assert!(almost_equal(rotated_right, up));
        }
    }

    #[test]
    fn test_left_upper_arm_z_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, 0.0, -90.0]);
            let right = Vec3::new(frame.right[0], frame.right[1], frame.right[2]);
            let up = Vec3::new(frame.up[0], frame.up[1], frame.up[2]);
            let neg_right = -right;
            let rotated = apply_quaternion(q, neg_right);
            assert!(almost_equal(rotated, up));
        }
    }

    #[test]
    fn test_right_upper_arm_y_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, -90.0, 0.0]);
            let right = Vec3::new(frame.right[0], frame.right[1], frame.right[2]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let rotated_right = apply_quaternion(q, right);
            assert!(almost_equal(rotated_right, forward));
        }
    }

    #[test]
    fn test_left_upper_arm_y_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, 90.0, 0.0]);
            let right = Vec3::new(frame.right[0], frame.right[1], frame.right[2]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_right = -right;
            let rotated = apply_quaternion(q, neg_right);
            assert!(almost_equal(rotated, forward));
        }
    }

    #[test]
    fn test_head_x_plus_30() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [30.0, 0.0, 0.0]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let up = Vec3::new(frame.up[0], frame.up[1], frame.up[2]);
            let rotated_forward = apply_quaternion(q, forward);
            assert!(rotated_forward.dot(up) < 0.0);
        }
    }

    #[test]
    fn test_head_y_plus_30() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, 30.0, 0.0]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let right = Vec3::new(frame.right[0], frame.right[1], frame.right[2]);
            let rotated_forward = apply_quaternion(q, forward);
            assert!(rotated_forward.dot(right) > 0.0);
        }
    }

    #[test]
    fn test_right_upper_leg_x_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [-90.0, 0.0, 0.0]);
            let up = Vec3::new(frame.up[0], frame.up[1], frame.up[2]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_up = -up;
            let rotated = apply_quaternion(q, neg_up);
            assert!(almost_equal(rotated, forward));
        }
    }

    #[test]
    fn test_right_lower_leg_x_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [90.0, 0.0, 0.0]);
            let up = Vec3::new(frame.up[0], frame.up[1], frame.up[2]);
            let forward = Vec3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_up = -up;
            let rotated = apply_quaternion(q, neg_up);
            assert!(almost_equal(rotated, -forward));
        }
    }

    #[test]
    fn test_thyllore_rotation_to_role_inverts_role_rotation_to_thyllore() {
        let angle_sets = [
            [25.0, -40.0, 60.0],
            [-10.0, 75.0, -120.0],
            [5.0, 170.0, 30.0],
        ];
        for frame in [CANONICAL, rotated_frame()] {
            for euler in angle_sets {
                let q = role_rotation_to_thyllore(&frame, euler);
                let recovered = thyllore_rotation_to_role(&frame, q);
                for axis in 0..3 {
                    assert!(
                        (recovered[axis] - euler[axis]).abs() < 1e-3,
                        "euler {:?} recovered {:?}",
                        euler,
                        recovered
                    );
                }
            }
        }
    }

    #[test]
    fn test_identity() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = role_rotation_to_thyllore(&frame, [0.0, 0.0, 0.0]);
            let diff = q.xyz().dot(q.xyz());
            assert!(q.w.abs() > 1.0 - 1e-6 && diff < 1e-6);
        }
    }
}
