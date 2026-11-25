use glam::{Mat4, Quat, Vec3};
use thyllore_math_core::{tangent_frame_around, world_y_axis};

use super::force::SpringForceModel;
use crate::{compose_transform, decompose_transform, normalize_quat};

/// The rest frame of a joint: where its bone points when the chain is undisturbed.
#[derive(Clone, Copy, Debug)]
pub struct JointRestFrame {
    pub head: Vec3,
    pub parent_world_rotation: Quat,
    pub initial_local_rotation: Quat,
    pub bone_axis: Vec3,
    pub bone_length: f32,
    /// The gravity direction expressed in the parent's frame at initialization.
    pub gravity_in_parent_rest: Vec3,
}

/// Joint forces in solver units: `stiffness` is a return rate per second, `gravity` the per-step magnitude the model expects.
#[derive(Clone, Copy, Debug)]
pub struct JointForces {
    pub drag_force: f32,
    pub stiffness: f32,
    pub gravity_dir: Vec3,
    pub gravity: f32,
}

pub fn integrate_joint(
    current_tail: Vec3,
    prev_tail: Vec3,
    frame: &JointRestFrame,
    forces: &JointForces,
    model: SpringForceModel,
    dt: f32,
) -> Vec3 {
    let velocity = (current_tail - prev_tail) * (1.0 - forces.drag_force);
    let rest_rotation = frame.parent_world_rotation * frame.initial_local_rotation;
    let rest_dir = unit_or_up(rest_rotation * frame.bone_axis);

    match model {
        SpringForceModel::Vrm => {
            current_tail
                + velocity
                + rest_dir * forces.stiffness * dt
                + forces.gravity_dir * forces.gravity * dt
        }
        SpringForceModel::RestRelative => {
            let rest_gravity = frame.parent_world_rotation * frame.gravity_in_parent_rest;
            let rest_tail = frame.head + rest_dir * frame.bone_length;
            let return_share = (forces.stiffness * dt).min(1.0);
            current_tail
                + velocity * (1.0 - return_share)
                + (rest_tail - current_tail) * return_share
                + (forces.gravity_dir - rest_gravity) * forces.gravity * dt * dt
        }
    }
}

fn unit_or_up(v: Vec3) -> Vec3 {
    let len = v.length();
    if len < 1e-8 {
        world_y_axis()
    } else {
        v / len
    }
}

pub fn apply_length_constraint(head: Vec3, tail: Vec3, bone_length: f32) -> Vec3 {
    let dir = tail - head;
    let len = dir.length();

    if len < 1e-8 {
        return head + Vec3::new(0.0, bone_length, 0.0);
    }

    head + (dir / len) * bone_length
}

pub fn compute_joint_rotation(
    head_position: Vec3,
    parent_world_rotation: Quat,
    initial_local_rotation: Quat,
    bone_axis: Vec3,
    current_tail: Vec3,
) -> Quat {
    let rest_dir = (parent_world_rotation * initial_local_rotation) * bone_axis;

    let actual_dir = current_tail - head_position;
    let actual_len = actual_dir.length();
    if actual_len < 1e-8 {
        return initial_local_rotation;
    }
    let actual_dir = actual_dir / actual_len;

    let rotation_diff = rotation_between(rest_dir, actual_dir);

    let parent_rot_inv = parent_world_rotation.conjugate();
    normalize_quat(parent_rot_inv * rotation_diff * parent_world_rotation * initial_local_rotation)
}

pub fn extract_world_position(transform: &Mat4) -> Vec3 {
    transform.w_axis.truncate()
}

pub fn compute_tail_position(
    head_position: Vec3,
    parent_world_rotation: Quat,
    initial_local_rotation: Quat,
    bone_axis: Vec3,
    bone_length: f32,
) -> Vec3 {
    let dir = (parent_world_rotation * initial_local_rotation) * bone_axis;
    head_position + dir * bone_length
}

pub fn recompute_global_transform(
    parent_global: &Mat4,
    local_rotation: Quat,
    original_local_transform: &Mat4,
) -> Mat4 {
    let (t, _, s) = decompose_transform(original_local_transform);
    let local = compose_transform(t, local_rotation, s);
    parent_global * local
}

fn rotation_between(from: Vec3, to: Vec3) -> Quat {
    let from_len = from.length();
    let to_len = to.length();
    if from_len < 1e-8 || to_len < 1e-8 {
        return Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
    }

    let from_n = from / from_len;
    let to_n = to / to_len;

    let dot = from_n.x * to_n.x + from_n.y * to_n.y + from_n.z * to_n.z;

    if dot > 0.9999 {
        return Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
    }

    if dot < -0.9999 {
        let (axis, _) = tangent_frame_around(from_n);
        return Quat::from_xyzw(axis.x, axis.y, axis.z, 0.0);
    }

    let cross = from_n.cross(to_n);
    let w = 1.0 + dot;
    normalize_quat(Quat::from_xyzw(cross.x, cross.y, cross.z, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Quat = Quat::from_xyzw(0.0, 0.0, 0.0, 1.0);
    const DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);

    fn frame(bone_axis: Vec3, bone_length: f32) -> JointRestFrame {
        JointRestFrame {
            head: Vec3::new(0.0, 0.0, 0.0),
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
        let head = Vec3::new(0.0, 0.0, 0.0);
        let tail = Vec3::new(3.0, 4.0, 0.0);
        let bone_length = 2.0;

        let result = apply_length_constraint(head, tail, bone_length);
        let dist =
            ((result - head).x.powi(2) + (result - head).y.powi(2) + (result - head).z.powi(2))
                .sqrt();

        assert!((dist - bone_length).abs() < 1e-5);
    }

    #[test]
    fn test_integrate_joint_with_no_forces() {
        let current = Vec3::new(0.0, 1.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        for model in [SpringForceModel::Vrm, SpringForceModel::RestRelative] {
            let result = integrate_joint(
                current,
                current,
                &frame(up, 1.0),
                &forces(1.0, 0.0, 0.0),
                model,
                0.016,
            );
            assert!((result - current).length() < 1e-5);
        }
    }

    #[test]
    fn test_rotation_between_same_direction() {
        let dir = Vec3::new(0.0, 1.0, 0.0);
        let q = rotation_between(dir, dir);
        assert!((q.w - 1.0).abs() < 1e-4);
    }

    #[test]
    fn vrm_stiffness_is_a_position_step_along_the_rest_direction() {
        let origin = Vec3::new(0.0, 0.0, 0.0);
        let result = integrate_joint(
            origin,
            origin,
            &frame(DOWN, 1.0),
            &forces(0.0, 2.0, 0.0),
            SpringForceModel::Vrm,
            0.5,
        );
        assert!((result - DOWN).length() < 1e-5);
    }

    #[test]
    fn rest_relative_gravity_vanishes_in_the_rest_pose_for_any_strength() {
        let sideways = Vec3::new(1.0, 0.0, 0.0);
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
            assert!((result - current).length() < 1e-6, "gravity {gravity}");
        }
    }

    #[test]
    fn rest_relative_gravity_acts_once_the_parent_tilts() {
        let sideways = Vec3::new(1.0, 0.0, 0.0);
        let current = sideways * 0.1;
        let tilted = JointRestFrame {
            parent_world_rotation: Quat::from_rotation_z(f32::to_radians(90.0)),
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
        let expected_gravity = DOWN - tilted.parent_world_rotation * DOWN;
        assert!((result - current - expected_gravity).length() < 1e-5);
    }

    #[test]
    fn rest_relative_spring_snaps_to_rest_once_the_rate_saturates() {
        let rest = Vec3::new(0.0, -1.0, 0.0);
        let current = Vec3::new(0.03, 0.0, 0.0);
        let result = integrate_joint(
            current,
            Vec3::new(0.0, 0.0, 0.0),
            &frame(rest, 0.03),
            &forces(0.0, 1000.0, 0.0),
            SpringForceModel::RestRelative,
            1.0 / 30.0,
        );
        assert!((result - rest * 0.03).length() < 1e-6);
    }

    #[test]
    fn rest_relative_stiffness_turns_short_and_long_bones_by_the_same_angle() {
        let rest = Vec3::new(0.0, -1.0, 0.0);
        let displaced = Vec3::new(1.0, 0.0, 0.0);
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
            let constrained = apply_length_constraint(Vec3::new(0.0, 0.0, 0.0), result, length);
            constrained.normalize().dot(rest).acos()
        };
        assert!((angle_after(0.02) - angle_after(0.12)).abs() < 1e-5);
    }
}
