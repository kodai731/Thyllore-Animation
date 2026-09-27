use thyllore_importer_core::fbx::fbx::FbxData;

use crate::components::fbx::*;
use crate::fbx_animation::UidAllocator;

pub(crate) fn build_blend_shape_exports(
    fbx_data_list: &[FbxData],
    geometries: &[FbxGeometryExport],
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> Vec<FbxBlendShapeExport> {
    let mut blend_shapes = Vec::new();

    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        if fbx_data.morph.channels.is_empty() {
            continue;
        }

        let geometry_uid = if i < geometries.len() {
            geometries[i].uid
        } else {
            continue;
        };

        let deformer_uid = uid_alloc.allocate();
        let mut channels = Vec::new();

        for channel in &fbx_data.morph.channels {
            let channel_uid = uid_alloc.allocate();
            let shape_uid = uid_alloc.allocate();

            let indexes: Vec<i32> = channel
                .position_deltas
                .iter()
                .map(|d| d.vertex_index as i32)
                .collect();
            let vertices: Vec<f64> = channel
                .position_deltas
                .iter()
                .flat_map(|d| {
                    [
                        (d.delta[0] * inv_unit_scale) as f64,
                        (d.delta[1] * inv_unit_scale) as f64,
                        (d.delta[2] * inv_unit_scale) as f64,
                    ]
                })
                .collect();
            let normals: Vec<f64> = channel
                .normal_deltas
                .iter()
                .flat_map(|d| [d.delta[0] as f64, d.delta[1] as f64, d.delta[2] as f64])
                .collect();

            channels.push(FbxBlendShapeChannelExport {
                channel_uid,
                shape_uid,
                name: channel.name.clone(),
                indexes,
                vertices,
                normals,
            });
        }

        if !channels.is_empty() {
            blend_shapes.push(FbxBlendShapeExport {
                source_mesh: fbx_data.morph.source_mesh.clone(),
                deformer_uid,
                geometry_uid,
                channels,
            });
        }
    }

    blend_shapes
}
