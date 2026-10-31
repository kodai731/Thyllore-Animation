use std::collections::BTreeMap;

use cgmath::{Matrix4, SquareMatrix};

use thyllore_anim_core::Skeleton;
use thyllore_file_format_core::fbx::components::ClusterInfo;
use thyllore_model_core::{SkinData, INFLUENCES_PER_VERTEX};

pub fn clusters_from_skin_data(skin: &SkinData, skeleton: &Skeleton) -> Vec<ClusterInfo> {
    let mut bone_map: BTreeMap<u32, (Vec<usize>, Vec<f32>)> = BTreeMap::new();

    for (k, weights) in skin.bone_weights.iter().enumerate() {
        let indices = &skin.bone_indices[k];
        for s in 0..INFLUENCES_PER_VERTEX {
            let w = weights[s];
            if w > 0.0 {
                let bone_id = indices[s];
                let entry = bone_map
                    .entry(bone_id)
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                entry.0.push(k);
                entry.1.push(w);
            }
        }
    }

    let mut clusters = Vec::new();

    for (bone_id, (vertex_indices, vertex_weights)) in bone_map {
        let bone = match skeleton.bones.get(bone_id as usize) {
            Some(b) => b,
            None => continue,
        };

        let inverse_bind_pose = bone.inverse_bind_pose;
        let transform_link = match inverse_bind_pose.invert() {
            Some(inv) => inv,
            None => continue,
        };

        clusters.push(ClusterInfo {
            bone_name: bone.name.clone(),
            transform: Matrix4::identity(),
            transform_link,
            inverse_bind_pose,
            vertex_indices: vertex_indices.into_iter().collect(),
            vertex_weights: vertex_weights.into_iter().collect(),
        });
    }

    clusters
}

#[cfg(test)]
mod tests {
    use cgmath::{Matrix4, SquareMatrix};

    use super::*;

    fn make_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        skeleton.add_bone("bone0", None);
        skeleton.add_bone("bone1", Some(0));
        skeleton
    }

    fn make_skin_data() -> SkinData {
        SkinData {
            skeleton_id: 0,
            bone_indices: vec![
                cgmath::Vector4::new(0, 1, 0, 0),
                cgmath::Vector4::new(0, 1, 0, 0),
                cgmath::Vector4::new(0, 0, 0, 0),
            ],
            bone_weights: vec![
                cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
                cgmath::Vector4::new(0.5, 0.5, 0.0, 0.0),
                cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0),
            ],
            base_positions: vec![
                cgmath::Vector3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::new(1.0, 0.0, 0.0),
                cgmath::Vector3::new(2.0, 0.0, 0.0),
            ],
            base_normals: vec![
                cgmath::Vector3::new(0.0, 1.0, 0.0),
                cgmath::Vector3::new(0.0, 1.0, 0.0),
                cgmath::Vector3::new(0.0, 1.0, 0.0),
            ],
        }
    }

    #[test]
    fn test_clusters_count_and_content() {
        let skeleton = make_skeleton();
        let skin = make_skin_data();
        let clusters = clusters_from_skin_data(&skin, &skeleton);

        assert_eq!(clusters.len(), 2);

        let cluster0 = &clusters[0];
        assert_eq!(cluster0.bone_name, "bone0");
        assert_eq!(cluster0.vertex_indices, vec![0, 1]);
        assert!((cluster0.vertex_weights[0] - 1.0).abs() < 1e-5);
        assert!((cluster0.vertex_weights[1] - 0.5).abs() < 1e-5);

        let cluster1 = &clusters[1];
        assert_eq!(cluster1.bone_name, "bone1");
        assert_eq!(cluster1.vertex_indices, vec![1]);
        assert!((cluster1.vertex_weights[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_transform_link_is_inverse_of_inverse_bind_pose() {
        let skeleton = make_skeleton();
        let skin = make_skin_data();
        let clusters = clusters_from_skin_data(&skin, &skeleton);

        for cluster in &clusters {
            let product = cluster.transform_link * cluster.inverse_bind_pose;
            let identity: Matrix4<f32> = Matrix4::identity();
            for r in 0..4 {
                for c in 0..4 {
                    let expected = if r == c { 1.0 } else { 0.0 };
                    assert!(
                        (product[r][c] - expected).abs() < 1e-5,
                        "transform_link * inverse_bind_pose not identity at [{},{}]: got {} expected {}",
                        r, c, product[r][c], expected
                    );
                }
            }
        }
    }
}
