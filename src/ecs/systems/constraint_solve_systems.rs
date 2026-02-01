use cgmath::{InnerSpace, Matrix4, Quaternion, SquareMatrix, Vector3};

use crate::animation::{
    decompose_transform, normalize_quat, slerp, AimConstraintData, BoneId,
    ConstraintType, IkConstraintData, ParentConstraintData,
    PositionConstraintData, RotationConstraintData, ScaleConstraintData,
    Skeleton, SkeletonPose,
};
use crate::ecs::component::ConstraintSet;
use crate::ecs::systems::compute_pose_global_transforms;

pub fn apply_constraints(
    constraint_set: &ConstraintSet,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
) {
    let entries = constraint_set.enabled_constraints();
    if entries.is_empty() {
        return;
    }

    let mut globals = compute_pose_global_transforms(skeleton, pose);
    let mut last_priority = entries[0].priority;

    for entry in &entries {
        if entry.priority != last_priority {
            globals = compute_pose_global_transforms(skeleton, pose);
            last_priority = entry.priority;
        }

        match &entry.constraint {
            ConstraintType::Parent(ref data) => {
                solve_parent_constraint(data, skeleton, pose, &globals);
            }
            ConstraintType::Position(ref data) => {
                solve_position_constraint(data, skeleton, pose, &globals);
            }
            ConstraintType::Rotation(ref data) => {
                solve_rotation_constraint(data, skeleton, pose, &globals);
            }
            ConstraintType::Scale(ref data) => {
                solve_scale_constraint(data, skeleton, pose, &globals);
            }
            ConstraintType::Aim(ref data) => {
                solve_aim_constraint(data, skeleton, pose, &globals);
            }
            ConstraintType::Ik(ref data) => {
                solve_ik_constraint(data, skeleton, pose, &mut globals);
            }
        }
    }
}

fn extract_translation(m: &Matrix4<f32>) -> Vector3<f32> {
    Vector3::new(m[3][0], m[3][1], m[3][2])
}

fn get_parent_global_transform(
    bone_id: BoneId,
    skeleton: &Skeleton,
    globals: &[Matrix4<f32>],
) -> Matrix4<f32> {
    skeleton
        .get_bone(bone_id)
        .and_then(|bone| bone.parent_id)
        .and_then(|pid| globals.get(pid as usize).copied())
        .unwrap_or(skeleton.root_transform)
}

fn lerp_vec3(
    a: Vector3<f32>,
    b: Vector3<f32>,
    t: f32,
) -> Vector3<f32> {
    a + (b - a) * t
}

fn conjugate_quat(q: Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(q.s, -q.v.x, -q.v.y, -q.v.z)
}

fn quat_mul(a: Quaternion<f32>, b: Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(
        a.s * b.s - a.v.x * b.v.x - a.v.y * b.v.y - a.v.z * b.v.z,
        a.s * b.v.x + a.v.x * b.s + a.v.y * b.v.z - a.v.z * b.v.y,
        a.s * b.v.y - a.v.x * b.v.z + a.v.y * b.s + a.v.z * b.v.x,
        a.s * b.v.z + a.v.x * b.v.y - a.v.y * b.v.x + a.v.z * b.s,
    )
}

fn rotation_between_vectors(
    from: Vector3<f32>,
    to: Vector3<f32>,
) -> Quaternion<f32> {
    let from_n = from.normalize();
    let to_n = to.normalize();
    let dot = from_n.dot(to_n);

    if dot > 0.9999 {
        return Quaternion::new(1.0, 0.0, 0.0, 0.0);
    }

    if dot < -0.9999 {
        let perp = if from_n.x.abs() < 0.9 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };
        let axis = from_n.cross(perp).normalize();
        return Quaternion::new(0.0, axis.x, axis.y, axis.z);
    }

    let axis = from_n.cross(to_n);
    let s = ((1.0 + dot) * 2.0).sqrt();
    let inv_s = 1.0 / s;
    normalize_quat(Quaternion::new(
        s * 0.5,
        axis.x * inv_s,
        axis.y * inv_s,
        axis.z * inv_s,
    ))
}

fn quaternion_from_axis_angle(
    axis: Vector3<f32>,
    angle: f32,
) -> Quaternion<f32> {
    let half = angle * 0.5;
    let s = half.sin();
    let c = half.cos();
    let a = axis.normalize();
    Quaternion::new(c, a.x * s, a.y * s, a.z * s)
}

fn solve_position_constraint(
    data: &PositionConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    if data.weight <= 0.0 {
        return;
    }

    let target_idx = data.target_bone as usize;
    let bone_idx = data.constrained_bone as usize;
    if target_idx >= globals.len() || bone_idx >= pose.bone_poses.len() {
        return;
    }

    let target_pos = extract_translation(&globals[target_idx]) + data.offset;

    let parent_global =
        get_parent_global_transform(data.constrained_bone, skeleton, globals);
    let Some(parent_inv) = parent_global.invert() else {
        return;
    };

    let local_target_h = parent_inv
        * cgmath::Vector4::new(target_pos.x, target_pos.y, target_pos.z, 1.0);
    let local_target =
        Vector3::new(local_target_h.x, local_target_h.y, local_target_h.z);

    let current = pose.bone_poses[bone_idx].translation;

    let masked = Vector3::new(
        if data.affect_axes[0] { local_target.x } else { current.x },
        if data.affect_axes[1] { local_target.y } else { current.y },
        if data.affect_axes[2] { local_target.z } else { current.z },
    );

    pose.bone_poses[bone_idx].translation =
        lerp_vec3(current, masked, data.weight);
}

fn solve_rotation_constraint(
    data: &RotationConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    if data.weight <= 0.0 {
        return;
    }

    let target_idx = data.target_bone as usize;
    let bone_idx = data.constrained_bone as usize;
    if target_idx >= globals.len() || bone_idx >= pose.bone_poses.len() {
        return;
    }

    let (_, target_rot, _) = decompose_transform(&globals[target_idx]);
    let target_rot = quat_mul(target_rot, data.offset);

    let parent_global =
        get_parent_global_transform(data.constrained_bone, skeleton, globals);
    let (_, parent_rot, _) = decompose_transform(&parent_global);
    let parent_rot_inv = conjugate_quat(parent_rot);

    let local_rot = normalize_quat(quat_mul(parent_rot_inv, target_rot));

    let effective_weight = if data.affect_axes.iter().all(|&a| a) {
        data.weight
    } else {
        let active_count =
            data.affect_axes.iter().filter(|&&a| a).count() as f32;
        data.weight * (active_count / 3.0)
    };

    let current = pose.bone_poses[bone_idx].rotation;
    pose.bone_poses[bone_idx].rotation =
        slerp(current, local_rot, effective_weight);
}

fn solve_scale_constraint(
    data: &ScaleConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    if data.weight <= 0.0 {
        return;
    }

    let target_idx = data.target_bone as usize;
    let bone_idx = data.constrained_bone as usize;
    if target_idx >= globals.len() || bone_idx >= pose.bone_poses.len() {
        return;
    }

    let (_, _, target_scale) = decompose_transform(&globals[target_idx]);
    let target_scale = Vector3::new(
        target_scale.x * data.offset.x,
        target_scale.y * data.offset.y,
        target_scale.z * data.offset.z,
    );

    let parent_global =
        get_parent_global_transform(data.constrained_bone, skeleton, globals);
    let (_, _, parent_scale) = decompose_transform(&parent_global);

    let local_scale = Vector3::new(
        target_scale.x / parent_scale.x.max(0.0001),
        target_scale.y / parent_scale.y.max(0.0001),
        target_scale.z / parent_scale.z.max(0.0001),
    );

    let current = pose.bone_poses[bone_idx].scale;

    let masked = Vector3::new(
        if data.affect_axes[0] { local_scale.x } else { current.x },
        if data.affect_axes[1] { local_scale.y } else { current.y },
        if data.affect_axes[2] { local_scale.z } else { current.z },
    );

    pose.bone_poses[bone_idx].scale =
        lerp_vec3(current, masked, data.weight);
}

fn solve_parent_constraint(
    data: &ParentConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    if data.weight <= 0.0 || data.sources.is_empty() {
        return;
    }

    let bone_idx = data.constrained_bone as usize;
    if bone_idx >= pose.bone_poses.len() {
        return;
    }

    let mut total_weight = 0.0f32;
    let mut blended_translation = Vector3::new(0.0, 0.0, 0.0);
    let mut blended_rotation = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let mut first_rotation = true;

    for &(source_bone, source_weight) in &data.sources {
        let src_idx = source_bone as usize;
        if src_idx >= globals.len() || source_weight <= 0.0 {
            continue;
        }

        let (t, r, _) = decompose_transform(&globals[src_idx]);
        blended_translation += t * source_weight;

        if first_rotation {
            blended_rotation = r;
            first_rotation = false;
        } else {
            let normalized_t =
                source_weight / (total_weight + source_weight);
            blended_rotation = slerp(blended_rotation, r, normalized_t);
        }

        total_weight += source_weight;
    }

    if total_weight <= 0.0 {
        return;
    }

    blended_translation /= total_weight;

    let parent_global = get_parent_global_transform(
        data.constrained_bone,
        skeleton,
        globals,
    );
    let Some(parent_inv) = parent_global.invert() else {
        return;
    };

    let current = &pose.bone_poses[bone_idx];
    let current_t = current.translation;
    let current_r = current.rotation;

    if data.affect_translation {
        let local_h = parent_inv
            * cgmath::Vector4::new(
                blended_translation.x,
                blended_translation.y,
                blended_translation.z,
                1.0,
            );
        let local_t = Vector3::new(local_h.x, local_h.y, local_h.z);
        pose.bone_poses[bone_idx].translation =
            lerp_vec3(current_t, local_t, data.weight);
    }

    if data.affect_rotation {
        let (_, parent_rot, _) = decompose_transform(&parent_global);
        let parent_rot_inv = conjugate_quat(parent_rot);
        let local_r =
            normalize_quat(quat_mul(parent_rot_inv, blended_rotation));
        pose.bone_poses[bone_idx].rotation =
            slerp(current_r, local_r, data.weight);
    }
}

fn solve_aim_constraint(
    data: &AimConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    if data.weight <= 0.0 {
        return;
    }

    let source_idx = data.source_bone as usize;
    let target_idx = data.target_bone as usize;
    if source_idx >= globals.len()
        || target_idx >= globals.len()
        || source_idx >= pose.bone_poses.len()
    {
        return;
    }

    let source_pos = extract_translation(&globals[source_idx]);
    let target_pos = extract_translation(&globals[target_idx]);
    let direction = target_pos - source_pos;

    if direction.magnitude2() < 1e-8 {
        return;
    }
    let direction = direction.normalize();

    let (_, source_rot, _) = decompose_transform(&globals[source_idx]);
    let current_aim = rotate_vector_by_quat(source_rot, data.aim_axis);

    let aim_rotation = rotation_between_vectors(current_aim, direction);

    let up_world = if let Some(up_bone) = data.up_target {
        let up_idx = up_bone as usize;
        if up_idx < globals.len() {
            let up_pos = extract_translation(&globals[up_idx]);
            (up_pos - source_pos).normalize()
        } else {
            data.up_axis
        }
    } else {
        data.up_axis
    };

    let rotated_up =
        rotate_vector_by_quat(aim_rotation, rotate_vector_by_quat(source_rot, data.up_axis));
    let desired_up = up_world - direction * direction.dot(up_world);
    let actual_up = rotated_up - direction * direction.dot(rotated_up);

    let final_rot = if desired_up.magnitude2() > 1e-8
        && actual_up.magnitude2() > 1e-8
    {
        let twist = rotation_between_vectors(
            actual_up.normalize(),
            desired_up.normalize(),
        );
        normalize_quat(quat_mul(twist, quat_mul(aim_rotation, source_rot)))
    } else {
        normalize_quat(quat_mul(aim_rotation, source_rot))
    };

    let parent_global =
        get_parent_global_transform(data.source_bone, skeleton, globals);
    let (_, parent_rot, _) = decompose_transform(&parent_global);
    let parent_rot_inv = conjugate_quat(parent_rot);
    let local_rot = normalize_quat(quat_mul(parent_rot_inv, final_rot));

    let current = pose.bone_poses[source_idx].rotation;
    pose.bone_poses[source_idx].rotation =
        slerp(current, local_rot, data.weight);
}

fn solve_ik_constraint(
    data: &IkConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &mut Vec<Matrix4<f32>>,
) {
    if data.weight <= 0.0 {
        return;
    }

    let effector_idx = data.effector_bone as usize;
    let target_idx = data.target_bone as usize;
    if effector_idx >= globals.len() || target_idx >= globals.len() {
        return;
    }

    if data.chain_length == 1 {
        solve_ik_single_bone(data, skeleton, pose, globals);
        return;
    }

    let Some(mid_bone_id) =
        skeleton.get_bone(data.effector_bone).and_then(|b| b.parent_id)
    else {
        return;
    };
    let Some(root_bone_id) =
        skeleton.get_bone(mid_bone_id).and_then(|b| b.parent_id)
    else {
        return;
    };

    let root_idx = root_bone_id as usize;
    let mid_idx = mid_bone_id as usize;
    if root_idx >= globals.len() || mid_idx >= globals.len() {
        return;
    }

    let root_pos = extract_translation(&globals[root_idx]);
    let mid_pos = extract_translation(&globals[mid_idx]);
    let effector_pos = extract_translation(&globals[effector_idx]);
    let target_pos = extract_translation(&globals[target_idx]);

    let upper_len = (mid_pos - root_pos).magnitude();
    let lower_len = (effector_pos - mid_pos).magnitude();

    if upper_len < 1e-6 || lower_len < 1e-6 {
        return;
    }

    let max_reach = upper_len + lower_len - 0.001;
    let target_vec = target_pos - root_pos;
    let target_dist = target_vec.magnitude().clamp(0.001, max_reach);

    let cos_root = ((upper_len * upper_len + target_dist * target_dist
        - lower_len * lower_len)
        / (2.0 * upper_len * target_dist))
        .clamp(-1.0, 1.0);
    let root_angle = cos_root.acos();

    let target_dir = if target_vec.magnitude2() > 1e-8 {
        target_vec.normalize()
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };

    let current_upper_dir = (mid_pos - root_pos).normalize();

    let pole_normal = compute_bend_plane_normal(
        data,
        root_pos,
        mid_pos,
        effector_pos,
        target_dir,
        globals,
    );

    let base_rotation =
        rotation_between_vectors(current_upper_dir, target_dir);
    let angle_offset = quaternion_from_axis_angle(pole_normal, -root_angle);
    let root_world_rot =
        normalize_quat(quat_mul(angle_offset, base_rotation));

    let (_, old_root_rot, _) = decompose_transform(&globals[root_idx]);
    let root_correction =
        normalize_quat(quat_mul(root_world_rot, conjugate_quat(old_root_rot)));

    let root_parent_global =
        get_parent_global_transform(root_bone_id, skeleton, globals);
    let (_, root_parent_rot, _) = decompose_transform(&root_parent_global);
    let root_parent_inv = conjugate_quat(root_parent_rot);

    let new_root_global_rot =
        normalize_quat(quat_mul(root_correction, old_root_rot));
    let new_root_local_rot =
        normalize_quat(quat_mul(root_parent_inv, new_root_global_rot));

    let current_root_local = pose.bone_poses[root_idx].rotation;
    pose.bone_poses[root_idx].rotation =
        slerp(current_root_local, new_root_local_rot, data.weight);

    *globals = compute_pose_global_transforms(skeleton, pose);

    let (_, mid_global_rot, _) = decompose_transform(&globals[mid_idx]);

    let new_mid_pos = extract_translation(&globals[mid_idx]);
    let new_effector_pos = extract_translation(&globals[effector_idx]);
    let current_lower_dir = (new_effector_pos - new_mid_pos).normalize();

    let desired_effector_pos = extract_translation(&globals[target_idx]);
    let desired_lower_dir = if (desired_effector_pos - new_mid_pos)
        .magnitude2()
        > 1e-8
    {
        (desired_effector_pos - new_mid_pos).normalize()
    } else {
        current_lower_dir
    };

    let mid_correction =
        rotation_between_vectors(current_lower_dir, desired_lower_dir);
    let new_mid_global_rot =
        normalize_quat(quat_mul(mid_correction, mid_global_rot));

    let mid_parent_global =
        get_parent_global_transform(mid_bone_id, skeleton, globals);
    let (_, mid_parent_rot, _) = decompose_transform(&mid_parent_global);
    let mid_parent_inv = conjugate_quat(mid_parent_rot);

    let new_mid_local_rot =
        normalize_quat(quat_mul(mid_parent_inv, new_mid_global_rot));
    let current_mid_local = pose.bone_poses[mid_idx].rotation;
    pose.bone_poses[mid_idx].rotation =
        slerp(current_mid_local, new_mid_local_rot, data.weight);

    if data.twist.abs() > 1e-6 {
        let twist_rot =
            quaternion_from_axis_angle(target_dir, data.twist);
        let twisted = normalize_quat(quat_mul(
            twist_rot,
            pose.bone_poses[root_idx].rotation,
        ));
        pose.bone_poses[root_idx].rotation = twisted;
    }

    *globals = compute_pose_global_transforms(skeleton, pose);
}

fn solve_ik_single_bone(
    data: &IkConstraintData,
    skeleton: &Skeleton,
    pose: &mut SkeletonPose,
    globals: &[Matrix4<f32>],
) {
    let effector_idx = data.effector_bone as usize;
    let target_idx = data.target_bone as usize;
    if effector_idx >= globals.len() || target_idx >= globals.len() {
        return;
    }

    let Some(parent_id) = skeleton
        .get_bone(data.effector_bone)
        .and_then(|b| b.parent_id)
    else {
        return;
    };

    let parent_idx = parent_id as usize;
    if parent_idx >= globals.len() || parent_idx >= pose.bone_poses.len() {
        return;
    }

    let parent_pos = extract_translation(&globals[parent_idx]);
    let effector_pos = extract_translation(&globals[effector_idx]);
    let target_pos = extract_translation(&globals[target_idx]);

    let current_dir = effector_pos - parent_pos;
    let desired_dir = target_pos - parent_pos;

    if current_dir.magnitude2() < 1e-8 || desired_dir.magnitude2() < 1e-8 {
        return;
    }

    let correction =
        rotation_between_vectors(current_dir.normalize(), desired_dir.normalize());

    let (_, parent_global_rot, _) =
        decompose_transform(&globals[parent_idx]);
    let new_global_rot =
        normalize_quat(quat_mul(correction, parent_global_rot));

    let grandparent_global =
        get_parent_global_transform(parent_id, skeleton, globals);
    let (_, grandparent_rot, _) = decompose_transform(&grandparent_global);
    let grandparent_inv = conjugate_quat(grandparent_rot);

    let new_local_rot =
        normalize_quat(quat_mul(grandparent_inv, new_global_rot));
    let current_local = pose.bone_poses[parent_idx].rotation;
    pose.bone_poses[parent_idx].rotation =
        slerp(current_local, new_local_rot, data.weight);
}

fn compute_bend_plane_normal(
    data: &IkConstraintData,
    root_pos: Vector3<f32>,
    mid_pos: Vector3<f32>,
    _effector_pos: Vector3<f32>,
    target_dir: Vector3<f32>,
    globals: &[Matrix4<f32>],
) -> Vector3<f32> {
    if let Some(pole_bone) = data.pole_target {
        let pole_idx = pole_bone as usize;
        if pole_idx < globals.len() {
            let pole_pos = extract_translation(&globals[pole_idx]);
            let to_pole = (pole_pos - root_pos).normalize();
            let normal = target_dir.cross(to_pole);
            if normal.magnitude2() > 1e-8 {
                return normal.normalize();
            }
        }
    }

    if let Some(pole_vec) = data.pole_vector {
        let normal = target_dir.cross(pole_vec);
        if normal.magnitude2() > 1e-8 {
            return normal.normalize();
        }
    }

    let mid_dir = (mid_pos - root_pos).normalize();
    let normal = target_dir.cross(mid_dir);
    if normal.magnitude2() > 1e-8 {
        return normal.normalize();
    }

    let fallback = if target_dir.x.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    target_dir.cross(fallback).normalize()
}

fn rotate_vector_by_quat(
    q: Quaternion<f32>,
    v: Vector3<f32>,
) -> Vector3<f32> {
    let qv = Vector3::new(q.v.x, q.v.y, q.v.z);
    let uv = qv.cross(v);
    let uuv = qv.cross(uv);
    v + (uv * q.s + uuv) * 2.0
}
