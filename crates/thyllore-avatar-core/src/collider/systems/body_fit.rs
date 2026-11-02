use cgmath::{InnerSpace, Vector3};
use thyllore_anim_core::spring_bone::{
    fit_bone_capsules, radius_around_axis, weighted_positions, CapsuleFitParams, FittedCapsule,
};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

use crate::collider::systems::torso_volume::fitted_torso_fallback;
use crate::humanoid::components::body_collider_roles::body_collider_bone_indices;
use crate::humanoid::components::mapping::HumanoidMapping;

/// The body colliders of a rig: the skin fit on the humanoid body bones, completed by the torso
/// capsules the skeleton implies where the skin fit left a torso bone without one.
pub fn fit_body_capsules(
    skins: &[&SkinData],
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    params: &CapsuleFitParams,
) -> Vec<FittedCapsule> {
    let allowed = body_collider_bone_indices(mapping);
    let mut fitted: Vec<FittedCapsule> = fit_bone_capsules(skins, skeleton, params)
        .into_iter()
        .filter(|capsule| allowed.contains(&(capsule.bone_id as usize)))
        .collect();

    let own_radius = |bone: BoneId, head: Vector3<f32>, tail: Vector3<f32>| {
        let points = weighted_positions(skins, bone, params.min_weight);
        if points.len() < params.min_vertex_count as usize {
            return None;
        }
        let axis = tail - head;
        if axis.magnitude2() <= f32::EPSILON {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::HumanoidRole;
    use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3, Vector4};
    use thyllore_model_core::Bone;

    fn bone_at(id: u32, head: Vector3<f32>) -> Bone {
        Bone {
            id,
            name: format!("bone{id}"),
            inverse_bind_pose: Matrix4::from_translation(head).invert().unwrap(),
            ..Bone::default()
        }
    }

    fn ring_around(skin: &mut SkinData, bone: u32, head: Vector3<f32>, radius: f32) {
        for step in 0..16 {
            let angle = step as f32 * std::f32::consts::TAU / 16.0;
            let height = (step % 4) as f32 * 0.1;
            skin.base_positions
                .push(head + Vector3::new(radius * angle.cos(), -height, radius * angle.sin()));
            skin.bone_indices.push(Vector4::new(bone, 0, 0, 0));
            skin.bone_weights.push(Vector4::new(1.0, 0.0, 0.0, 0.0));
        }
    }

    #[test]
    fn a_neck_known_only_by_a_throat_patch_becomes_a_capsule_on_its_axis() {
        let chest = Vector3::new(0.0, 1.3, 0.0);
        let neck = Vector3::new(0.0, 1.5, 0.0);
        let head = Vector3::new(0.0, 1.6, 0.0);
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
            let patch = Vector3::new(
                -0.03 + 0.01 * (step % 4) as f32,
                1.5 + 0.02 * (step / 4) as f32,
                0.07,
            );
            skin.base_positions.push(patch);
            skin.bone_indices.push(Vector4::new(1, 0, 0, 0));
            skin.bone_weights.push(Vector4::new(1.0, 0.0, 0.0, 0.0));
        }

        let fitted = fit_body_capsules(&[&skin], &skeleton, &mapping, &CapsuleFitParams::default());

        let neck_capsule = fitted.iter().find(|capsule| capsule.bone_id == 1).unwrap();
        assert!(neck_capsule.head.magnitude() < 1e-5);
        assert!((neck_capsule.tail - Vector3::new(0.0, 0.1, 0.0)).magnitude() < 1e-5);
        assert!(neck_capsule.radius > 0.05 && neck_capsule.radius < 0.08);
    }

    #[test]
    fn body_roles_keep_their_fit_and_the_torso_fallback_completes_them() {
        let hips = Vector3::new(0.0, 1.0, 0.0);
        let left_leg = Vector3::new(0.1, 0.95, 0.0);
        let right_leg = Vector3::new(-0.1, 0.95, 0.0);
        let thumb = Vector3::new(0.5, 1.2, 0.0);
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
