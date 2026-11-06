use thyllore_math_core::scale_translation;
use thyllore_model_core::scale_morph;

use crate::fbx::components::{FbxData, FbxModel};

/// Scale every length of a loaded FBX model by `factor`; rotations, normals, UVs and the unit declaration stay.
pub fn scale_fbx_model(model: &mut FbxModel, factor: f32) {
    for data in &mut model.fbx_data {
        scale_fbx_data(data, factor);
    }
    for node in model.nodes.values_mut() {
        node.local_transform = scale_translation(&node.local_transform, factor);
        node.default_translation = node.default_translation.map(|c| c * factor);
    }
    for animation in &mut model.animations {
        for bone_animation in animation.bone_animations.values_mut() {
            for key in &mut bone_animation.translation_keys {
                key.value = key.value.map(|c| c * factor);
            }
        }
    }
}

fn scale_fbx_data(data: &mut FbxData, factor: f32) {
    for position in data
        .positions
        .iter_mut()
        .chain(data.local_positions.iter_mut())
    {
        *position *= factor;
    }
    for cluster in &mut data.clusters {
        cluster.transform = scale_translation(&cluster.transform, factor);
        cluster.transform_link = scale_translation(&cluster.transform_link, factor);
        cluster.inverse_bind_pose = scale_translation(&cluster.inverse_bind_pose, factor);
    }
    for part in &mut data.mesh_parts {
        for position in &mut part.local_positions {
            *position *= factor;
        }
        part.local_transform = scale_translation(&part.local_transform, factor);
    }
    scale_morph(&mut data.morph, factor);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cgmath::{Matrix4, Vector3};

    use super::*;
    use crate::fbx::components::{
        BoneAnimation, BoneNode, ClusterInfo, FbxAnimation, KeyFrame, MeshPart,
    };

    #[test]
    fn fbx_model_scale_touches_every_length_and_nothing_else() {
        let translation = Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0));
        let mut data = FbxData::new();
        data.positions = vec![Vector3::new(1.0, 0.0, 0.0)];
        data.local_positions = vec![Vector3::new(0.0, 1.0, 0.0)];
        data.normals = vec![Vector3::new(0.0, 0.0, 1.0)];
        data.clusters.push(ClusterInfo {
            bone_name: "b".into(),
            transform: translation,
            transform_link: translation,
            inverse_bind_pose: translation,
            vertex_indices: vec![0],
            vertex_weights: vec![1.0],
        });
        data.mesh_parts.push(MeshPart {
            mesh_name: "m".into(),
            local_positions: vec![Vector3::new(2.0, 0.0, 0.0)],
            parent_bone: None,
            local_transform: translation,
            vertex_offset: 0,
            vertex_count: 1,
        });
        let mut model = FbxModel::default();
        model.fbx_data.push(data);
        model.nodes.insert(
            "b".into(),
            BoneNode {
                name: "b".into(),
                parent: None,
                local_transform: translation,
                default_translation: [1.0, 2.0, 3.0],
                default_rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
                default_scaling: [1.0, 1.0, 1.0],
            },
        );
        model.animations.push(FbxAnimation {
            name: "a".into(),
            duration: 1.0,
            bone_animations: HashMap::from([(
                "b".to_string(),
                BoneAnimation {
                    bone_name: "b".into(),
                    translation_keys: vec![KeyFrame {
                        time: 0.0,
                        value: [1.0, 2.0, 3.0],
                    }],
                    rotation_keys: Vec::new(),
                    scale_keys: Vec::new(),
                },
            )]),
        });
        model.unit_scale = 0.01;

        scale_fbx_model(&mut model, 10.0);

        let data = &model.fbx_data[0];
        assert_eq!(data.positions[0], Vector3::new(10.0, 0.0, 0.0));
        assert_eq!(data.local_positions[0], Vector3::new(0.0, 10.0, 0.0));
        assert_eq!(data.normals[0], Vector3::new(0.0, 0.0, 1.0));
        assert_eq!(
            data.clusters[0].inverse_bind_pose.w.truncate(),
            Vector3::new(10.0, 20.0, 30.0)
        );
        assert_eq!(
            data.clusters[0].transform_link.w.truncate(),
            Vector3::new(10.0, 20.0, 30.0)
        );
        assert_eq!(
            data.mesh_parts[0].local_positions[0],
            Vector3::new(20.0, 0.0, 0.0)
        );
        assert_eq!(
            data.mesh_parts[0].local_transform.w.truncate(),
            Vector3::new(10.0, 20.0, 30.0)
        );
        let node = &model.nodes["b"];
        assert_eq!(node.default_translation, [10.0, 20.0, 30.0]);
        assert_eq!(
            node.local_transform.w.truncate(),
            Vector3::new(10.0, 20.0, 30.0)
        );
        assert_eq!(
            model.animations[0].bone_animations["b"].translation_keys[0].value,
            [10.0, 20.0, 30.0]
        );
        assert_eq!(model.unit_scale, 0.01);
    }
}
