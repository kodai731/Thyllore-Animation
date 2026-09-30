use std::collections::HashMap;

use thyllore_model_core::{MorphChannel, SparseDelta};

use super::fbx::{FbxModel, MeshSplitInfo};

fn remap_shape_offsets(
    offset_vertices: &[u32],
    offsets: &[[f32; 3]],
    vertex_map: &HashMap<u32, u32>,
) -> Vec<SparseDelta> {
    if offsets.is_empty() {
        return Vec::new();
    }
    let mut deltas = Vec::new();
    for (i, &ctrl_idx) in offset_vertices.iter().enumerate() {
        if let Some(&mapped) = vertex_map.get(&ctrl_idx) {
            let offset = offsets.get(i).copied().unwrap_or([0.0, 0.0, 0.0]);
            deltas.push(SparseDelta {
                vertex_index: mapped,
                delta: offset,
            });
        }
    }
    deltas
}

fn resolve_target_shape(channel: &ufbx::BlendChannel) -> Option<&ufbx::BlendShape> {
    if channel.keyframes.len() > 1 {
        log_warn!(
            "Blend channel '{}' has {} in-between keyframes; only the full-weight shape is used",
            channel.element.name,
            channel.keyframes.len()
        );
    }

    if let Some(shape) = channel.target_shape.as_deref() {
        return Some(shape);
    }

    if let Some(keyframe) = channel.keyframes.last() {
        return Some(&*keyframe.shape);
    }

    log_warn!(
        "Blend channel '{}' has no target_shape and no keyframes — skipping",
        channel.element.name
    );
    None
}

pub(super) fn extract_blend_shapes(
    scene: &ufbx::Scene,
    fbx_model: &mut FbxModel,
    split_infos: &[MeshSplitInfo],
    mesh_to_node: &HashMap<usize, String>,
) {
    for ufbx_mesh in &scene.meshes {
        if ufbx_mesh.blend_deformers.is_empty() {
            continue;
        }

        let typed_id = ufbx_mesh.element.typed_id as usize;
        let source_name = mesh_to_node.get(&typed_id).cloned().unwrap_or_default();

        for blend_deformer in &ufbx_mesh.blend_deformers {
            for channel in &blend_deformer.channels {
                let channel_name = channel.element.name.to_string();

                let shape = match resolve_target_shape(&channel) {
                    Some(s) => s,
                    None => continue,
                };

                let offset_vertices: Vec<u32> = shape.offset_vertices.iter().copied().collect();
                let position_offsets: Vec<[f32; 3]> = shape
                    .position_offsets
                    .iter()
                    .map(|v| [v.x as f32, v.y as f32, v.z as f32])
                    .collect();
                let normal_offsets: Vec<[f32; 3]> = shape
                    .normal_offsets
                    .iter()
                    .map(|v| [v.x as f32, v.y as f32, v.z as f32])
                    .collect();

                for (fbx_idx, info) in split_infos.iter().enumerate() {
                    if info.ufbx_mesh_typed_id != typed_id {
                        continue;
                    }

                    let position_deltas =
                        remap_shape_offsets(&offset_vertices, &position_offsets, &info.vertex_map);

                    let normal_deltas =
                        remap_shape_offsets(&offset_vertices, &normal_offsets, &info.vertex_map);

                    fbx_model.fbx_data[fbx_idx].morph.source_mesh = source_name.clone();

                    fbx_model.fbx_data[fbx_idx]
                        .morph
                        .channels
                        .push(MorphChannel {
                            name: channel_name.clone(),
                            default_weight: channel.weight as f32,
                            position_deltas,
                            normal_deltas,
                        });
                }
            }
        }

        log!("Extracted blend shapes for mesh typed_id={}", typed_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_overlap() {
        let mut vertex_map = HashMap::new();
        vertex_map.insert(0u32, 0u32);
        vertex_map.insert(1u32, 1u32);
        vertex_map.insert(2u32, 2u32);

        let offset_vertices: Vec<u32> = vec![0, 1, 2];
        let offsets: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];

        let deltas = remap_shape_offsets(&offset_vertices, &offsets, &vertex_map);

        assert_eq!(deltas.len(), 3);
        assert_eq!(deltas[0].vertex_index, 0);
        assert_eq!(deltas[0].delta, [1.0, 0.0, 0.0]);
        assert_eq!(deltas[1].vertex_index, 1);
        assert_eq!(deltas[1].delta, [0.0, 2.0, 0.0]);
        assert_eq!(deltas[2].vertex_index, 2);
        assert_eq!(deltas[2].delta, [0.0, 0.0, 3.0]);
    }

    #[test]
    fn test_partial_overlap() {
        let mut vertex_map = HashMap::new();
        vertex_map.insert(1u32, 0u32);
        vertex_map.insert(3u32, 1u32);

        let offset_vertices: Vec<u32> = vec![0, 1, 2, 3];
        let offsets: [[f32; 3]; 4] = [
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 3.0],
            [4.0, 0.0, 0.0],
        ];

        let deltas = remap_shape_offsets(&offset_vertices, &offsets, &vertex_map);

        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0].vertex_index, 0);
        assert_eq!(deltas[0].delta, [0.0, 2.0, 0.0]);
        assert_eq!(deltas[1].vertex_index, 1);
        assert_eq!(deltas[1].delta, [4.0, 0.0, 0.0]);
    }

    #[test]
    fn test_no_overlap() {
        let mut vertex_map = HashMap::new();
        vertex_map.insert(10u32, 0u32);

        let offset_vertices: Vec<u32> = vec![0, 1, 2];
        let offsets: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];

        let deltas = remap_shape_offsets(&offset_vertices, &offsets, &vertex_map);

        assert!(deltas.is_empty());
    }

    #[test]
    fn test_empty() {
        let vertex_map: HashMap<u32, u32> = HashMap::new();
        let offset_vertices: Vec<u32> = vec![];
        let offsets: [[f32; 3]; 0] = [];

        let deltas = remap_shape_offsets(&offset_vertices, &offsets, &vertex_map);

        assert!(deltas.is_empty());
    }

    #[test]
    fn test_missing_offsets_yield_no_deltas() {
        let mut vertex_map = HashMap::new();
        vertex_map.insert(0u32, 0u32);
        vertex_map.insert(1u32, 1u32);

        let offset_vertices: Vec<u32> = vec![0, 1];
        let offsets: [[f32; 3]; 0] = [];

        let deltas = remap_shape_offsets(&offset_vertices, &offsets, &vertex_map);

        assert!(deltas.is_empty());
    }
}
