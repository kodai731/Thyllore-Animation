use cgmath::SquareMatrix;

use crate::garment::components::chain_gen::{GeneratedChain, InfluenceTarget, VertexInfluences};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

pub fn append_chain_bones(
    skeleton: &mut Skeleton,
    root: BoneId,
    chains: &[GeneratedChain],
) -> anyhow::Result<Vec<Vec<BoneId>>> {
    let existing_names: std::collections::HashSet<&str> =
        skeleton.bones.iter().map(|b| b.name.as_str()).collect();

    for chain in chains {
        for joint in &chain.joints {
            if existing_names.contains(joint.name.as_str()) {
                return Err(anyhow::anyhow!("bone {} already exists", joint.name));
            }
        }
    }

    let root_bone = skeleton
        .get_bone(root)
        .ok_or_else(|| anyhow::anyhow!("root bone not found"))?;
    let b_root = root_bone
        .inverse_bind_pose
        .invert()
        .ok_or_else(|| anyhow::anyhow!("root inverse_bind_pose is singular"))?;

    let mut bone_ids: Vec<Vec<BoneId>> = Vec::with_capacity(chains.len());

    for chain in chains {
        let mut chain_bones: Vec<BoneId> = Vec::with_capacity(chain.joints.len());

        let mut parent_id: Option<BoneId> = Some(root);
        let mut b_parent = b_root;

        for joint in &chain.joints {
            let mut b_j = b_root;
            b_j.w = joint.head.extend(1.0);
            let parent_inverse = b_parent.invert().ok_or_else(|| {
                anyhow::anyhow!("bind pose of the parent of {} is singular", joint.name)
            })?;
            let joint_inverse = b_j
                .invert()
                .ok_or_else(|| anyhow::anyhow!("bind pose of {} is singular", joint.name))?;

            let bone_id = skeleton.add_bone(&joint.name, parent_id);
            let bone = skeleton.get_bone_mut(bone_id).expect("bone was just added");
            bone.local_transform = parent_inverse * b_j;
            bone.inverse_bind_pose = joint_inverse;

            parent_id = Some(bone_id);
            b_parent = b_j;
            chain_bones.push(bone_id);
        }

        bone_ids.push(chain_bones);
    }

    Ok(bone_ids)
}

pub fn write_chain_weights(
    skin: &mut SkinData,
    root: BoneId,
    bone_ids: &[Vec<BoneId>],
    influences: &[VertexInfluences],
) {
    for inf in influences {
        let mut mapped: Vec<(u32, f32)> = Vec::new();

        for (target, weight) in &inf.influences {
            let bone_idx = match target {
                InfluenceTarget::Root => root,
                InfluenceTarget::Joint { chain, joint } => bone_ids[*chain][*joint],
            };
            mapped.push((bone_idx, *weight));
        }

        mapped.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut indices = [0u32; 4];
        let mut weights = [0.0f32; 4];
        let mut total_weight = 0.0;

        for (i, &(bone_idx, weight)) in mapped.iter().take(4).enumerate() {
            indices[i] = bone_idx;
            weights[i] = weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            for w in &mut weights {
                *w /= total_weight;
            }
        }

        let vert_idx = inf.vertex as usize;
        if vert_idx < skin.bone_indices.len() {
            skin.bone_indices[vert_idx] =
                cgmath::Vector4::new(indices[0], indices[1], indices[2], indices[3]);
            skin.bone_weights[vert_idx] =
                cgmath::Vector4::new(weights[0], weights[1], weights[2], weights[3]);
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

    use crate::garment::components::chain_gen::GeneratedJoint;

    use thyllore_model_core::apply_skinning;

    use super::*;

    fn make_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        let root = skeleton.add_bone("root", None);
        if let Some(bone) = skeleton.get_bone_mut(root) {
            bone.inverse_bind_pose = Matrix4::from_translation(Vector3::new(0.0, 1.5, 0.0))
                .invert()
                .unwrap();
        }
        skeleton
    }

    fn make_chain() -> Vec<GeneratedChain> {
        use crate::garment::components::chain_gen::ChainRole;
        use crate::garment::components::chain_gen::{joint_name, ChainGenParams, ChainGenSettings};
        use crate::garment::components::submesh_role::SubmeshSelector;

        let params = ChainGenParams {
            role: ChainRole::Hair,
            target: SubmeshSelector::Mesh("test".to_string()),
            settings: ChainGenSettings::default(),
        };
        let joints = vec![
            GeneratedJoint {
                name: joint_name(&params, 0, 0),
                head: Vector3::new(0.0, 1.4, 0.0),
            },
            GeneratedJoint {
                name: joint_name(&params, 0, 1),
                head: Vector3::new(0.0, 1.3, 0.0),
            },
        ];
        let tail = joints.last().unwrap().head;

        vec![GeneratedChain {
            column: 0,
            joints,
            tail,
            vertices: vec![0],
        }]
    }

    fn make_skin(positions: &[Vector3<f32>]) -> SkinData {
        let count = positions.len();
        SkinData {
            skeleton_id: 0,
            bone_indices: vec![cgmath::Vector4::new(0, 0, 0, 0); count],
            bone_weights: vec![cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0); count],
            base_positions: positions.to_vec(),
            base_normals: vec![Vector3::new(0.0, 1.0, 0.0); count],
        }
    }

    #[test]
    fn test_bone_count_and_parent_chain() {
        let mut skeleton = make_skeleton();
        let root = 0;
        let chains = make_chain();

        let bone_ids = append_chain_bones(&mut skeleton, root, &chains).unwrap();

        assert_eq!(skeleton.bone_count(), 3);
        assert_eq!(bone_ids.len(), 1);
        assert_eq!(bone_ids[0].len(), 2);

        let joint0_id = bone_ids[0][0];
        let joint1_id = bone_ids[0][1];

        let bone0 = skeleton.get_bone(joint0_id).unwrap();
        assert_eq!(bone0.parent_id, Some(root));

        let bone1 = skeleton.get_bone(joint1_id).unwrap();
        assert_eq!(bone1.parent_id, Some(joint0_id));

        assert_eq!(skeleton.root_bone_ids, vec![root]);
    }

    #[test]
    fn test_rest_pose_skinning_identity() {
        let mut skeleton = make_skeleton();
        let root = 0;
        let chains = make_chain();

        let bone_ids = append_chain_bones(&mut skeleton, root, &chains).unwrap();

        let positions = vec![Vector3::new(0.0, 1.4, 0.0), Vector3::new(0.0, 1.3, 0.0)];
        let mut skin = make_skin(&positions);

        let influences = vec![
            VertexInfluences {
                vertex: 0,
                influences: vec![(InfluenceTarget::Joint { chain: 0, joint: 0 }, 1.0)],
            },
            VertexInfluences {
                vertex: 1,
                influences: vec![(InfluenceTarget::Joint { chain: 0, joint: 1 }, 1.0)],
            },
        ];

        write_chain_weights(&mut skin, root, &bone_ids, &influences);

        let global_transforms: Vec<Matrix4<f32>> = skeleton
            .bones
            .iter()
            .map(|b| b.inverse_bind_pose.invert().unwrap())
            .collect();

        let mut out_positions = vec![Vector3::new(0.0, 0.0, 0.0); positions.len()];
        let mut out_normals = vec![Vector3::new(0.0, 0.0, 0.0); positions.len()];

        apply_skinning(
            &skin,
            &global_transforms,
            &skeleton,
            &mut out_positions,
            &mut out_normals,
        );

        for (i, (expected, actual)) in positions.iter().zip(out_positions.iter()).enumerate() {
            let diff = (expected - *actual).magnitude();
            assert!(
                diff < 1e-5,
                "vertex {}: expected {:?}, got {:?}",
                i,
                expected,
                actual
            );
        }
    }

    #[test]
    fn test_weights_left_padded_and_sum_to_one() {
        let mut skeleton = make_skeleton();
        let root = 0;
        let chains = make_chain();

        let bone_ids = append_chain_bones(&mut skeleton, root, &chains).unwrap();

        let positions = vec![Vector3::new(0.0, 1.35, 0.0)];
        let mut skin = make_skin(&positions);

        let influences = vec![VertexInfluences {
            vertex: 0,
            influences: vec![
                (InfluenceTarget::Joint { chain: 0, joint: 0 }, 0.5),
                (InfluenceTarget::Joint { chain: 0, joint: 1 }, 0.5),
            ],
        }];

        write_chain_weights(&mut skin, root, &bone_ids, &influences);

        let indices = skin.bone_indices[0];
        let weights = skin.bone_weights[0];

        assert_eq!(indices.x, bone_ids[0][0]);
        assert_eq!(indices.y, bone_ids[0][1]);
        assert_eq!(indices.z, 0);
        assert_eq!(indices.w, 0);

        let sum: f32 = weights.x + weights.y + weights.z + weights.w;
        assert!((sum - 1.0).abs() < 1e-6, "weight sum {} != 1.0", sum);
        assert!((weights.x - 0.5).abs() < 1e-6);
        assert!((weights.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_inverse_bind_pose_translation_matches_joint_head() {
        let mut skeleton = make_skeleton();
        let root = 0;
        let chains = make_chain();

        let bone_ids = append_chain_bones(&mut skeleton, root, &chains).unwrap();

        for (k, joint) in chains[0].joints.iter().enumerate() {
            let bone = skeleton.get_bone(bone_ids[0][k]).unwrap();
            let b_j = bone.inverse_bind_pose.invert().unwrap();
            let translation = Vector3::new(b_j.w.x, b_j.w.y, b_j.w.z);
            let diff = (joint.head - translation).magnitude();
            assert!(
                diff < 1e-5,
                "bone {} inverse_bind_pose^-1 translation {:?} != joint head {:?}",
                k,
                translation,
                joint.head
            );
        }
    }

    #[test]
    fn test_joint1_local_transform_translation_is_head1_minus_head0() {
        let mut skeleton = make_skeleton();
        let root = 0;
        let chains = make_chain();

        let bone_ids = append_chain_bones(&mut skeleton, root, &chains).unwrap();

        let joint0 = chains[0].joints[0].head;
        let joint1 = chains[0].joints[1].head;
        let expected_diff = joint1 - joint0;

        let bone1 = skeleton.get_bone(bone_ids[0][1]).unwrap();
        let actual_translation = Vector3::new(
            bone1.local_transform.w.x,
            bone1.local_transform.w.y,
            bone1.local_transform.w.z,
        );

        let diff = (expected_diff - actual_translation).magnitude();
        assert!(
            diff < 1e-5,
            "joint1 local_transform translation {:?} != head1 - head0 {:?}",
            actual_translation,
            expected_diff
        );
    }
}
