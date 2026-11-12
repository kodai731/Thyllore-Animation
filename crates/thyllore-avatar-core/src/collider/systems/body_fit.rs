use glam::Vec3;
use thyllore_anim_core::spring_bone::{
    bone_local_points, bone_segment_local, fit_bone_capsule, fit_bone_capsules, fit_capsule_along,
    radius_around_axis, weighted_positions, CapsuleFitParams, FittedCapsule,
};
use thyllore_math_core::{extent_centre_across, PrincipalAxis};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

use crate::collider::systems::torso_volume::fitted_torso_fallback;
use crate::humanoid::components::body_collider_roles::body_collider_bone_indices;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::HumanoidRole;

/// The body colliders of a rig: the skin fit on the humanoid body bones, the head fit along its
/// bone, completed by the torso capsules the skeleton implies where the skin fit left a torso
/// bone without one.
pub fn fit_body_capsules(
    skins: &[&SkinData],
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    params: &CapsuleFitParams,
) -> Vec<FittedCapsule> {
    let allowed = body_collider_bone_indices(mapping);
    let head_bone = mapping
        .by_role
        .get(&HumanoidRole::Head)
        .map(|&bone| bone as BoneId);
    let mut fitted: Vec<FittedCapsule> = fit_bone_capsules(skins, skeleton, params)
        .into_iter()
        .filter(|capsule| allowed.contains(&(capsule.bone_id as usize)))
        .filter(|capsule| Some(capsule.bone_id) != head_bone)
        .collect();
    fitted.extend(head_bone.and_then(|bone| fit_head_capsule(skins, skeleton, bone, params)));

    let own_radius = |bone: BoneId, head: Vec3, tail: Vec3| {
        let points = weighted_positions(skins, bone, params.min_weight);
        if points.len() < params.min_vertex_count as usize {
            return None;
        }
        let axis = tail - head;
        if axis.length_squared() <= f32::EPSILON {
            return None;
        }
        radius_around_axis(&points, head, axis.normalize(), params)
    };
    fitted.extend(fitted_torso_fallback(
        skeleton,
        mapping,
        &fitted,
        &own_radius,
    ));
    fitted
}

/// The head's skin is a face and a skull: its capsule runs along the head bone, centred on the
/// skull's extents rather than on the face where the vertices are dense, so chin, crown and back
/// of the head are all covered.
fn fit_head_capsule(
    skins: &[&SkinData],
    skeleton: &Skeleton,
    bone_id: BoneId,
    params: &CapsuleFitParams,
) -> Option<FittedCapsule> {
    let bone = skeleton.get_bone(bone_id)?;
    let (head, tail) = bone_segment_local(skeleton, bone_id);
    let axis = tail - head;
    if axis.length_squared() <= f32::EPSILON {
        return fit_bone_capsule(skins, bone, params);
    }

    let points = bone_local_points(skins, bone, params);
    let direction = axis.normalize();
    let centre = extent_centre_across(&points, direction)?;
    fit_capsule_along(
        bone_id,
        &points,
        PrincipalAxis {
            mean: centre,
            axis: direction,
        },
        params,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, UVec4, Vec3, Vec4};
    use thyllore_model_core::Bone;

    fn bone_at(id: u32, head: Vec3) -> Bone {
        Bone {
            id,
            name: format!("bone{id}"),
            inverse_bind_pose: Mat4::from_translation(head).try_inverse().unwrap(),
            ..Bone::default()
        }
    }

    fn ring_around(skin: &mut SkinData, bone: u32, head: Vec3, radius: f32) {
        for step in 0..16 {
            let angle = step as f32 * std::f32::consts::TAU / 16.0;
            let height = (step % 4) as f32 * 0.1;
            skin.base_positions
                .push(head + Vec3::new(radius * angle.cos(), -height, radius * angle.sin()));
            skin.bone_indices.push(UVec4::new(bone, 0, 0, 0));
            skin.bone_weights.push(Vec4::new(1.0, 0.0, 0.0, 0.0));
        }
    }

    #[test]
    fn a_neck_known_only_by_a_throat_patch_becomes_a_capsule_on_its_axis() {
        let chest = Vec3::new(0.0, 1.3, 0.0);
        let neck = Vec3::new(0.0, 1.5, 0.0);
        let head = Vec3::new(0.0, 1.6, 0.0);
        let mut skeleton = Skeleton::default();
        for (id, position) in [(0, chest), (1, neck), (2, head)] {
            skeleton.bones.push(bone_at(id, position));
        }
        skeleton.bones[1].children = vec![2];
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Chest, 0);
        mapping.by_role.insert(HumanoidRole::Neck, 1);
        mapping.by_role.insert(HumanoidRole::Head, 2);
        let mut skin = SkinData::default();
        for step in 0..16 {
            let patch = Vec3::new(
                -0.03 + 0.01 * (step % 4) as f32,
                1.5 + 0.02 * (step / 4) as f32,
                0.07,
            );
            skin.base_positions.push(patch);
            skin.bone_indices.push(UVec4::new(1, 0, 0, 0));
            skin.bone_weights.push(Vec4::new(1.0, 0.0, 0.0, 0.0));
        }

        let fitted = fit_body_capsules(&[&skin], &skeleton, &mapping, &CapsuleFitParams::default());

        let neck_capsule = fitted.iter().find(|capsule| capsule.bone_id == 1).unwrap();
        assert!(neck_capsule.head.length() < 1e-5);
        assert!((neck_capsule.tail - Vec3::new(0.0, 0.1, 0.0)).length() < 1e-5);
        assert!(neck_capsule.radius > 0.05 && neck_capsule.radius < 0.08);
    }

    #[test]
    fn the_head_capsule_runs_along_its_bone_from_the_chin_to_the_crown() {
        let neck = Vec3::new(0.0, 1.5, 0.0);
        let head = Vec3::new(0.0, 1.6, 0.0);
        let crown = Vec3::new(0.0, 1.8, 0.0);
        let mut skeleton = Skeleton::default();
        for (id, position) in [(0, neck), (1, head), (2, crown)] {
            skeleton.bones.push(bone_at(id, position));
        }
        skeleton.bones[1].children = vec![2];
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Neck, 0);
        mapping.by_role.insert(HumanoidRole::Head, 1);
        let mut skin = SkinData::default();
        for level in 0..6 {
            let height = 1.55 + 0.05 * level as f32;
            for step in 0..16 {
                let angle = step as f32 * std::f32::consts::TAU / 16.0;
                let on_the_face = angle.sin() < 0.0;
                let copies = if on_the_face { 4 } else { 1 };
                for _ in 0..copies {
                    skin.base_positions.push(Vec3::new(
                        0.07 * angle.cos(),
                        height,
                        0.07 * angle.sin(),
                    ));
                    skin.bone_indices.push(UVec4::new(1, 0, 0, 0));
                    skin.bone_weights.push(Vec4::new(1.0, 0.0, 0.0, 0.0));
                }
            }
        }

        let fitted = fit_body_capsules(&[&skin], &skeleton, &mapping, &CapsuleFitParams::default());

        let head_capsule = fitted.iter().find(|capsule| capsule.bone_id == 1).unwrap();
        let axis = head_capsule.tail - head_capsule.head;
        assert!(axis.y > 0.05, "capsule has length along the bone: {axis:?}");
        assert!(axis.x.abs() < 1e-5 && axis.z.abs() < 1e-5);
        assert!(head_capsule.head.x.abs() < 1e-5 && head_capsule.head.z.abs() < 1e-5);
        assert!(head_capsule.head.y - 1.6 < -0.05 + head_capsule.radius + 1e-3);
    }

    #[test]
    fn body_roles_keep_their_fit_and_the_torso_fallback_completes_them() {
        let hips = Vec3::new(0.0, 1.0, 0.0);
        let left_leg = Vec3::new(0.1, 0.95, 0.0);
        let right_leg = Vec3::new(-0.1, 0.95, 0.0);
        let thumb = Vec3::new(0.5, 1.2, 0.0);
        let mut skeleton = Skeleton::default();
        for (id, head) in [(0, hips), (1, left_leg), (2, right_leg), (3, thumb)] {
            skeleton.bones.push(bone_at(id, head));
        }
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::LeftUpperLeg, 1);
        mapping.by_role.insert(HumanoidRole::RightUpperLeg, 2);
        mapping.by_role.insert(HumanoidRole::LeftThumbProximal, 3);
        let mut skin = SkinData::default();
        ring_around(&mut skin, 1, left_leg, 0.08);
        ring_around(&mut skin, 2, right_leg, 0.08);
        ring_around(&mut skin, 3, thumb, 0.01);

        let fitted = fit_body_capsules(&[&skin], &skeleton, &mapping, &CapsuleFitParams::default());

        let bones: Vec<u32> = fitted.iter().map(|capsule| capsule.bone_id).collect();
        assert_eq!(bones, vec![1, 2, 0]);
        let pelvis = &fitted[2];
        assert!((pelvis.radius - fitted[0].radius.max(fitted[1].radius)).abs() < 0.02);
    }
}
