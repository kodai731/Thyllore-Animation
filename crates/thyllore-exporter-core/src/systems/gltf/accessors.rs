use std::collections::HashMap;

use cgmath::Quaternion;
use gltf::json::accessor::{ComponentType, GenericComponentType, Type};
use gltf::json::validation::{Checked, USize64};
use gltf::json::{self, Index};

use thyllore_anim_core::{BoneId, Skeleton};

pub(crate) fn append_scalar_accessor(
    root: &mut json::Root,
    bin: &mut Vec<u8>,
    buffer_index: Index<json::Buffer>,
    data: &[f32],
) -> Index<json::Accessor> {
    let (min_val, max_val) = compute_min_max_scalar(data);
    let byte_offset = append_f32_data(bin, data);
    let byte_length = (data.len() * 4) as u64;

    let view_index = root.push(json::buffer::View {
        buffer: buffer_index,
        byte_length: USize64::from(byte_length),
        byte_offset: Some(USize64::from(byte_offset as u64)),
        byte_stride: None,
        name: None,
        target: None,
        extensions: None,
        extras: Default::default(),
    });

    root.push(json::Accessor {
        buffer_view: Some(view_index),
        byte_offset: None,
        count: USize64::from(data.len() as u64),
        component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
        extensions: None,
        extras: Default::default(),
        type_: Checked::Valid(Type::Scalar),
        min: Some(serde_json::json!([min_val])),
        max: Some(serde_json::json!([max_val])),
        name: None,
        normalized: false,
        sparse: None,
    })
}

pub(crate) fn append_vec3_accessor(
    root: &mut json::Root,
    bin: &mut Vec<u8>,
    buffer_index: Index<json::Buffer>,
    data: &[f32],
) -> Index<json::Accessor> {
    let count = data.len() / 3;
    let (min_vals, max_vals) = compute_min_max_vec3(data);
    let byte_offset = append_f32_data(bin, data);
    let byte_length = (data.len() * 4) as u64;

    let view_index = root.push(json::buffer::View {
        buffer: buffer_index,
        byte_length: USize64::from(byte_length),
        byte_offset: Some(USize64::from(byte_offset as u64)),
        byte_stride: None,
        name: None,
        target: None,
        extensions: None,
        extras: Default::default(),
    });

    root.push(json::Accessor {
        buffer_view: Some(view_index),
        byte_offset: None,
        count: USize64::from(count as u64),
        component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
        extensions: None,
        extras: Default::default(),
        type_: Checked::Valid(Type::Vec3),
        min: Some(serde_json::json!(min_vals)),
        max: Some(serde_json::json!(max_vals)),
        name: None,
        normalized: false,
        sparse: None,
    })
}

pub(crate) fn append_vec4_accessor(
    root: &mut json::Root,
    bin: &mut Vec<u8>,
    buffer_index: Index<json::Buffer>,
    data: &[f32],
) -> Index<json::Accessor> {
    let count = data.len() / 4;
    let (min_vals, max_vals) = compute_min_max_vec4(data);
    let byte_offset = append_f32_data(bin, data);
    let byte_length = (data.len() * 4) as u64;

    let view_index = root.push(json::buffer::View {
        buffer: buffer_index,
        byte_length: USize64::from(byte_length),
        byte_offset: Some(USize64::from(byte_offset as u64)),
        byte_stride: None,
        name: None,
        target: None,
        extensions: None,
        extras: Default::default(),
    });

    root.push(json::Accessor {
        buffer_view: Some(view_index),
        byte_offset: None,
        count: USize64::from(count as u64),
        component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
        extensions: None,
        extras: Default::default(),
        type_: Checked::Valid(Type::Vec4),
        min: Some(serde_json::json!(min_vals)),
        max: Some(serde_json::json!(max_vals)),
        name: None,
        normalized: false,
        sparse: None,
    })
}

pub(crate) fn append_f32_data(bin: &mut Vec<u8>, data: &[f32]) -> usize {
    pad_to_4byte_alignment(bin);
    let byte_offset = bin.len();

    for &val in data {
        bin.extend_from_slice(&val.to_le_bytes());
    }

    byte_offset
}

pub(crate) fn pad_to_4byte_alignment(bin: &mut Vec<u8>) {
    let remainder = bin.len() % 4;
    if remainder != 0 {
        let padding = 4 - remainder;
        bin.extend(std::iter::repeat(0u8).take(padding));
    }
}

pub(crate) fn build_bone_to_node_map(
    skeleton: &Skeleton,
    nodes: &[json::scene::Node],
) -> HashMap<BoneId, u32> {
    let mut map = HashMap::new();

    let node_name_to_index: HashMap<&str, u32> = nodes
        .iter()
        .enumerate()
        .filter_map(|(i, node)| node.name.as_ref().map(|name| (name.as_str(), i as u32)))
        .collect();

    for bone in &skeleton.bones {
        if let Some(&node_index) = node_name_to_index.get(bone.name.as_str()) {
            map.insert(bone.id, node_index);
        }
    }

    map
}

pub(crate) fn quaternion_to_gltf_array(q: Quaternion<f32>) -> [f32; 4] {
    [q.v.x, q.v.y, q.v.z, q.s]
}

pub(crate) fn compute_min_max_scalar(data: &[f32]) -> (f32, f32) {
    let min = data.iter().copied().fold(f32::INFINITY, f32::min);
    let max = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}

pub(crate) fn compute_min_max_vec3(data: &[f32]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for chunk in data.chunks(3) {
        for i in 0..3 {
            min[i] = min[i].min(chunk[i]);
            max[i] = max[i].max(chunk[i]);
        }
    }

    (min, max)
}

pub(crate) fn compute_min_max_vec4(data: &[f32]) -> ([f32; 4], [f32; 4]) {
    let mut min = [f32::INFINITY; 4];
    let mut max = [f32::NEG_INFINITY; 4];

    for chunk in data.chunks(4) {
        for i in 0..4 {
            min[i] = min[i].min(chunk[i]);
            max[i] = max[i].max(chunk[i]);
        }
    }

    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Quaternion;
    use thyllore_anim_core::Skeleton;

    fn create_test_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        skeleton.add_bone("Hips", None);
        skeleton.add_bone("Spine", Some(0));
        skeleton.add_bone("Head", Some(1));
        skeleton
    }

    fn create_test_nodes() -> Vec<json::scene::Node> {
        vec![
            json::scene::Node {
                name: Some("Armature".to_string()),
                ..Default::default()
            },
            json::scene::Node {
                name: Some("Hips".to_string()),
                ..Default::default()
            },
            json::scene::Node {
                name: Some("Spine".to_string()),
                ..Default::default()
            },
            json::scene::Node {
                name: Some("Head".to_string()),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn test_bone_to_node_mapping() {
        let skeleton = create_test_skeleton();
        let nodes = create_test_nodes();

        let map = build_bone_to_node_map(&skeleton, &nodes);

        assert_eq!(map.get(&0), Some(&1));
        assert_eq!(map.get(&1), Some(&2));
        assert_eq!(map.get(&2), Some(&3));
    }

    #[test]
    fn test_bone_to_node_mapping_missing_nodes() {
        let skeleton = create_test_skeleton();
        let nodes = vec![json::scene::Node {
            name: Some("Hips".to_string()),
            ..Default::default()
        }];

        let map = build_bone_to_node_map(&skeleton, &nodes);

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&0), Some(&0));
        assert!(!map.contains_key(&1));
    }

    #[test]
    fn test_quaternion_to_gltf_array() {
        let q = Quaternion::new(1.0, 0.2, 0.3, 0.4);
        let arr = quaternion_to_gltf_array(q);

        assert_eq!(arr[0], 0.2);
        assert_eq!(arr[1], 0.3);
        assert_eq!(arr[2], 0.4);
        assert_eq!(arr[3], 1.0);
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let arr = quaternion_to_gltf_array(q);

        assert_eq!(arr, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_append_f32_data_alignment() {
        let mut bin = vec![0u8; 5];
        let data = [1.0f32, 2.0];

        let offset = append_f32_data(&mut bin, &data);

        assert_eq!(offset, 8);
        assert_eq!(bin.len(), 16);
    }

    #[test]
    fn test_compute_min_max_scalar() {
        let data = [3.0, 1.0, 4.0, 1.5, 9.0];
        let (min, max) = compute_min_max_scalar(&data);

        assert_eq!(min, 1.0);
        assert_eq!(max, 9.0);
    }

    #[test]
    fn test_compute_min_max_vec3() {
        let data = [1.0, 2.0, 3.0, -1.0, 5.0, 0.0];
        let (min, max) = compute_min_max_vec3(&data);

        assert_eq!(min, [-1.0, 2.0, 0.0]);
        assert_eq!(max, [1.0, 5.0, 3.0]);
    }

    #[test]
    fn test_pad_to_4byte_alignment() {
        let mut bin = vec![0u8; 5];
        pad_to_4byte_alignment(&mut bin);
        assert_eq!(bin.len(), 8);

        let mut bin2 = vec![0u8; 8];
        pad_to_4byte_alignment(&mut bin2);
        assert_eq!(bin2.len(), 8);
    }
}
