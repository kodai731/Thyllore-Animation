use cgmath::{Deg, Matrix, Matrix3, Quaternion, Vector3};

use crate::humanoid::components::character_frame::CharacterFrame;

pub fn recipe_rotation_to_engine(
    frame: &CharacterFrame,
    euler_degrees: [f32; 3],
) -> Quaternion<f32> {
    let [x_degrees, y_degrees, z_degrees] = euler_degrees;
    let unity_rotation = Matrix3::from_angle_y(Deg(y_degrees))
        * Matrix3::from_angle_x(Deg(x_degrees))
        * Matrix3::from_angle_z(Deg(z_degrees));

    let unity_to_engine = Matrix3::from_cols(
        Vector3::from(frame.right),
        Vector3::from(frame.up),
        Vector3::from(frame.forward),
    );

    let engine_rotation = unity_to_engine * unity_rotation * unity_to_engine.transpose();
    Quaternion::from(engine_rotation)
}

#[cfg(test)]
mod tests {
    use cgmath::{InnerSpace, Matrix3, Rotation, Vector3};

    use crate::humanoid::components::character_frame::CharacterFrame;

    use super::*;

    const CANONICAL: CharacterFrame = CharacterFrame {
        right: [-1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        forward: [0.0, 0.0, 1.0],
    };

    fn rotated_frame() -> CharacterFrame {
        let angle = std::f32::consts::PI / 4.0;
        let c = angle.cos();
        let s = angle.sin();

        let ry: Matrix3<f32> = Matrix3::new(c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c);

        let base_right = Vector3::new(-1.0, 0.0, 0.0);
        let base_up = Vector3::new(0.0, 1.0, 0.0);
        let base_forward = Vector3::new(0.0, 0.0, 1.0);

        let right = ry * base_right;
        let up = ry * base_up;
        let forward = ry * base_forward;

        CharacterFrame {
            right: [right.x, right.y, right.z],
            up: [up.x, up.y, up.z],
            forward: [forward.x, forward.y, forward.z],
        }
    }

    fn apply_quaternion(q: Quaternion<f32>, v: Vector3<f32>) -> Vector3<f32> {
        let q_inv = q.invert();
        let v_q = cgmath::Quaternion::new(0.0, v.x, v.y, v.z);
        let result = q * v_q * q_inv;
        Vector3::new(result.v.x, result.v.y, result.v.z)
    }

    fn almost_equal(a: Vector3<f32>, b: Vector3<f32>) -> bool {
        a.dot(b) > 0.999
    }

    #[test]
    fn test_right_upper_arm_z_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, 0.0, 90.0]);
            let right = Vector3::new(frame.right[0], frame.right[1], frame.right[2]);
            let up = Vector3::new(frame.up[0], frame.up[1], frame.up[2]);
            let rotated_right = apply_quaternion(q, right);
            assert!(almost_equal(rotated_right, up));
        }
    }

    #[test]
    fn test_left_upper_arm_z_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, 0.0, -90.0]);
            let right = Vector3::new(frame.right[0], frame.right[1], frame.right[2]);
            let up = Vector3::new(frame.up[0], frame.up[1], frame.up[2]);
            let neg_right = -right;
            let rotated = apply_quaternion(q, neg_right);
            assert!(almost_equal(rotated, up));
        }
    }

    #[test]
    fn test_right_upper_arm_y_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, -90.0, 0.0]);
            let right = Vector3::new(frame.right[0], frame.right[1], frame.right[2]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let rotated_right = apply_quaternion(q, right);
            assert!(almost_equal(rotated_right, forward));
        }
    }

    #[test]
    fn test_left_upper_arm_y_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, 90.0, 0.0]);
            let right = Vector3::new(frame.right[0], frame.right[1], frame.right[2]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_right = -right;
            let rotated = apply_quaternion(q, neg_right);
            assert!(almost_equal(rotated, forward));
        }
    }

    #[test]
    fn test_head_x_plus_30() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [30.0, 0.0, 0.0]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let up = Vector3::new(frame.up[0], frame.up[1], frame.up[2]);
            let rotated_forward = apply_quaternion(q, forward);
            assert!(rotated_forward.dot(up) < 0.0);
        }
    }

    #[test]
    fn test_head_y_plus_30() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, 30.0, 0.0]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let right = Vector3::new(frame.right[0], frame.right[1], frame.right[2]);
            let rotated_forward = apply_quaternion(q, forward);
            assert!(rotated_forward.dot(right) > 0.0);
        }
    }

    #[test]
    fn test_right_upper_leg_x_minus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [-90.0, 0.0, 0.0]);
            let up = Vector3::new(frame.up[0], frame.up[1], frame.up[2]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_up = -up;
            let rotated = apply_quaternion(q, neg_up);
            assert!(almost_equal(rotated, forward));
        }
    }

    #[test]
    fn test_right_lower_leg_x_plus_90() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [90.0, 0.0, 0.0]);
            let up = Vector3::new(frame.up[0], frame.up[1], frame.up[2]);
            let forward = Vector3::new(frame.forward[0], frame.forward[1], frame.forward[2]);
            let neg_up = -up;
            let rotated = apply_quaternion(q, neg_up);
            assert!(almost_equal(rotated, -forward));
        }
    }

    #[test]
    fn test_identity() {
        for frame in [CANONICAL, rotated_frame()] {
            let q = recipe_rotation_to_engine(&frame, [0.0, 0.0, 0.0]);
            let diff = q.v.dot(q.v);
            assert!(q.s.abs() > 1.0 - 1e-6 && diff < 1e-6);
        }
    }
}
