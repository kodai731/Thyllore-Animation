use cgmath::{InnerSpace, Matrix4, Quaternion, Vector3};
use thyllore_math_core::{tangent_frame_around, world_y_axis};

use super::force::SpringForceModel;
use crate::{compose_transform, decompose_transform, normalize_quat};

/// The rest frame of a joint: where its bone points when the chain is undisturbed.
#[derive(Clone, Copy, Debug)]
pub struct JointRestFrame {
    pub head: Vector3<f32>,
    pub parent_world_rotation: Quaternion<f32>,
    pub initial_local_rotation: Quaternion<f32>,
    pub bone_axis: Vector3<f32>,
    pub bone_length: f32,
    /// The gravity direction expressed in the parent's frame at initialization.
    pub gravity_in_parent_rest: Vector3<f32>,
}

/// Joint forces in solver units: `stiffness` is a return rate per second, `gravity` the per-step magnitude the model expects.
#[derive(Clone, Copy, Debug)]
pub struct JointForces {
    pub drag_force: f32,
    pub stiffness: f32,
    pub gravity_dir: Vector3<f32>,
    pub gravity: f32,
}

pub fn integrate_joint(
    current_tail: Vector3<f32>,
    prev_tail: Vector3<f32>,
    frame: &JointRestFrame,
    forces: &JointForces,
    model: SpringForceModel,
    dt: f32,
) -> Vector3<f32> {
    let velocity = (current_tail - prev_tail) * (1.0 - forces.drag_force);
    let rest_rotation = frame.parent_world_rotation * frame.initial_local_rotation;
    let rest_dir = unit_or_up(rotate_vector(rest_rotation, frame.bone_axis));

    match model {
        SpringForceModel::Vrm => {
            current_tail
                + velocity
                + rest_dir * forces.stiffness * dt
                + forces.gravity_dir * forces.gravity * dt
        }
        SpringForceModel::RestRelative => {
            let rest_gravity =
                rotate_vector(frame.parent_world_rotation, frame.gravity_in_parent_rest);
            let rest_tail = frame.head + rest_dir * frame.bone_length;
            let return_share = (forces.stiffness * dt).min(1.0);
            current_tail
                + velocity * (1.0 - return_share)
                + (rest_tail - current_tail) * return_share
                + (forces.gravity_dir - rest_gravity) * forces.gravity * dt * dt
        }
    }
}

fn unit_or_up(v: Vector3<f32>) -> Vector3<f32> {
    let len = v.magnitude();
    if len < 1e-8 {
        world_y_axis()
    } else {
        v / len
    }
}

pub fn apply_length_constraint(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    bone_length: f32,
) -> Vector3<f32> {
    let dir = tail - head;
    let len = dir.magnitude();

    if len < 1e-8 {
        return head + Vector3::new(0.0, bone_length, 0.0);
    }

    head + (dir / len) * bone_length
}

pub fn compute_joint_rotation(
    head_position: Vector3<f32>,
    parent_world_rotation: Quaternion<f32>,
    initial_local_rotation: Quaternion<f32>,
    bone_axis: Vector3<f32>,
    current_tail: Vector3<f32>,
) -> Quaternion<f32> {
    let rest_dir = rotate_vector(parent_world_rotation * initial_local_rotation, bone_axis);

    let actual_dir = current_tail - head_position;
    let actual_len = actual_dir.magnitude();
    if actual_len < 1e-8 {
        return initial_local_rotation;
    }
    let actual_dir = actual_dir / actual_len;

    let rotation_diff = rotation_between(rest_dir, actual_dir);

    let parent_rot_inv = conjugate(parent_world_rotation);
    normalize_quat(parent_rot_inv * rotation_diff * parent_world_rotation * initial_local_rotation)
}

pub fn extract_world_position(transform: &Matrix4<f32>) -> Vector3<f32> {
    Vector3::new(transform[3][0], transform[3][1], transform[3][2])
}

pub fn compute_tail_position(
    head_position: Vector3<f32>,
    parent_world_rotation: Quaternion<f32>,
    initial_local_rotation: Quaternion<f32>,
    bone_axis: Vector3<f32>,
    bone_length: f32,
) -> Vector3<f32> {
    let dir = rotate_vector(parent_world_rotation * initial_local_rotation, bone_axis);
    head_position + dir * bone_length
}

pub fn recompute_global_transform(
    parent_global: &Matrix4<f32>,
    local_rotation: Quaternion<f32>,
    original_local_transform: &Matrix4<f32>,
) -> Matrix4<f32> {
    let (t, _, s) = decompose_transform(original_local_transform);
    let local = compose_transform(t, local_rotation, s);
    parent_global * local
}

pub fn rotate_vector(q: Quaternion<f32>, v: Vector3<f32>) -> Vector3<f32> {
    let qv = Quaternion::new(0.0, v.x, v.y, v.z);
    let result = q * qv * conjugate(q);
    Vector3::new(result.v.x, result.v.y, result.v.z)
}

pub fn conjugate(q: Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(q.s, -q.v.x, -q.v.y, -q.v.z)
}

fn rotation_between(from: Vector3<f32>, to: Vector3<f32>) -> Quaternion<f32> {
    let from_len = from.magnitude();
    let to_len = to.magnitude();
    if from_len < 1e-8 || to_len < 1e-8 {
        return Quaternion::new(1.0, 0.0, 0.0, 0.0);
    }

    let from_n = from / from_len;
    let to_n = to / to_len;

    let dot = from_n.x * to_n.x + from_n.y * to_n.y + from_n.z * to_n.z;

    if dot > 0.9999 {
        return Quaternion::new(1.0, 0.0, 0.0, 0.0);
    }

    if dot < -0.9999 {
        let (axis, _) = tangent_frame_around(from_n);
        return Quaternion::new(0.0, axis.x, axis.y, axis.z);
    }

    let cross = from_n.cross(to_n);
    let w = 1.0 + dot;
    normalize_quat(Quaternion::new(w, cross.x, cross.y, cross.z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Rotation3;

    const IDENTITY: Quaternion<f32> = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    const DOWN: Vector3<f32> = Vector3::new(0.0, -1.0, 0.0);

    fn frame(bone_axis: Vector3<f32>, bone_length: f32) -> JointRestFrame {
        JointRestFrame {
            head: Vector3::new(0.0, 0.0, 0.0),
            parent_world_rotation: IDENTITY,
            initial_local_rotation: IDENTITY,
            bone_axis,
            bone_length,
            gravity_in_parent_rest: DOWN,
        }
    }

    fn forces(drag_force: f32, stiffness: f32, gravity: f32) -> JointForces {
        JointForces {
            drag_force,
            stiffness,
            gravity_dir: DOWN,
            gravity,
        }
    }

    #[test]
    fn test_apply_length_constraint_preserves_length() {
        let head = Vector3::new(0.0, 0.0, 0.0);
        let tail = Vector3::new(3.0, 4.0, 0.0);
        let bone_length = 2.0;

        let result = apply_length_constraint(head, tail, bone_length);
        let dist =
            ((result - head).x.powi(2) + (result - head).y.powi(2) + (result - head).z.powi(2))
                .sqrt();

        assert!((dist - bone_length).abs() < 1e-5);
    }

    #[test]
    fn test_integrate_joint_with_no_forces() {
        let current = Vector3::new(0.0, 1.0, 0.0);
        let up = Vector3::new(0.0, 1.0, 0.0);
        for model in [SpringForceModel::Vrm, SpringForceModel::RestRelative] {
            let result = integrate_joint(
                current,
                current,
                &frame(up, 1.0),
                &forces(1.0, 0.0, 0.0),
                model,
                0.016,
            );
            assert!((result - current).magnitude() < 1e-5);
        }
    }

    #[test]
    fn test_rotation_between_same_direction() {
        let dir = Vector3::new(0.0, 1.0, 0.0);
        let q = rotation_between(dir, dir);
        assert!((q.s - 1.0).abs() < 1e-4);
    }

    #[test]
    fn vrm_stiffness_is_a_position_step_along_the_rest_direction() {
        let origin = Vector3::new(0.0, 0.0, 0.0);
        let result = integrate_joint(
            origin,
            origin,
            &frame(DOWN, 1.0),
            &forces(0.0, 2.0, 0.0),
            SpringForceModel::Vrm,
            0.5,
        );
        assert!((result - DOWN).magnitude() < 1e-5);
    }

    #[test]
    fn rest_relative_gravity_vanishes_in_the_rest_pose_for_any_strength() {
        let sideways = Vector3::new(1.0, 0.0, 0.0);
        let current = sideways * 0.1;
        for gravity in [0.0, 1.0, 50.0] {
            let result = integrate_joint(
                current,
                current,
                &frame(sideways, 0.1),
                &forces(0.4, 0.0, gravity),
                SpringForceModel::RestRelative,
                1.0 / 30.0,
            );
            assert!((result - current).magnitude() < 1e-6, "gravity {gravity}");
        }
    }

    #[test]
    fn rest_relative_gravity_acts_once_the_parent_tilts() {
        let sideways = Vector3::new(1.0, 0.0, 0.0);
        let current = sideways * 0.1;
        let tilted = JointRestFrame {
            parent_world_rotation: Quaternion::from_angle_z(cgmath::Deg(90.0)),
            ..frame(sideways, 0.1)
        };
        let result = integrate_joint(
            current,
            current,
            &tilted,
            &forces(0.4, 0.0, 1.0),
            SpringForceModel::RestRelative,
            1.0,
        );
        let expected_gravity = DOWN - rotate_vector(tilted.parent_world_rotation, DOWN);
        assert!((result - current - expected_gravity).magnitude() < 1e-5);
    }

    #[test]
    fn rest_relative_spring_snaps_to_rest_once_the_rate_saturates() {
        let rest = Vector3::new(0.0, -1.0, 0.0);
        let current = Vector3::new(0.03, 0.0, 0.0);
        let result = integrate_joint(
            current,
            Vector3::new(0.0, 0.0, 0.0),
            &frame(rest, 0.03),
            &forces(0.0, 1000.0, 0.0),
            SpringForceModel::RestRelative,
            1.0 / 30.0,
        );
        assert!((result - rest * 0.03).magnitude() < 1e-6);
    }

    #[test]
    fn rest_relative_stiffness_turns_short_and_long_bones_by_the_same_angle() {
        let rest = Vector3::new(0.0, -1.0, 0.0);
        let displaced = Vector3::new(1.0, 0.0, 0.0);
        let angle_after = |length: f32| {
            let current = displaced * length;
            let result = integrate_joint(
                current,
                current,
                &frame(rest, length),
                &forces(0.0, 1.0, 0.0),
                SpringForceModel::RestRelative,
                0.1,
            );
            let constrained = apply_length_constraint(Vector3::new(0.0, 0.0, 0.0), result, length);
            constrained.normalize().dot(rest).acos()
        };
        assert!((angle_after(0.02) - angle_after(0.12)).abs() < 1e-5);
    }
}
