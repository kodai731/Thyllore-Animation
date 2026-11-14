use thyllore_file_format_core::fbx::FbxData;

use crate::components::fbx::*;
use crate::fbx_animation::UidAllocator;

/// A shape without a single non-zero normal delta carries no normal information worth writing.
fn has_normal_deltas(deltas: &[thyllore_model_core::SparseDelta]) -> bool {
    deltas.iter().any(|d| d.delta.iter().any(|v| *v != 0.0))
}

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
            if channel.position_deltas.is_empty() {
                continue;
            }
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
            let normals: Vec<f64> = if has_normal_deltas(&channel.normal_deltas) {
                channel
                    .normal_deltas
                    .iter()
                    .flat_map(|d| [d.delta[0] as f64, d.delta[1] as f64, d.delta[2] as f64])
                    .collect()
            } else {
                Vec::new()
            };

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

#[cfg(test)]
mod tests {
    use thyllore_model_core::{MorphChannel, SparseDelta};

    use super::*;

    fn delta(vertex_index: u32, delta: [f32; 3]) -> SparseDelta {
        SparseDelta {
            vertex_index,
            delta,
        }
    }

    fn channel(name: &str, positions: Vec<SparseDelta>, normals: Vec<SparseDelta>) -> MorphChannel {
        MorphChannel {
            name: name.to_string(),
            default_weight: 0.0,
            position_deltas: positions,
            normal_deltas: normals,
        }
    }

    fn export_channels(channels: Vec<MorphChannel>) -> Vec<FbxBlendShapeChannelExport> {
        let mut fbx_data = FbxData::new();
        fbx_data.morph.channels = channels;
        let geometries = vec![FbxGeometryExport {
            uid: 1,
            mesh_model_uid: 2,
            positions: Vec::new(),
            polygon_vertex_index: Vec::new(),
            normals: Vec::new(),
            uv_values: Vec::new(),
        }];
        let mut uids = UidAllocator::new();

        build_blend_shape_exports(&[fbx_data], &geometries, &mut uids, 1.0)
            .pop()
            .map(|export| export.channels)
            .unwrap_or_default()
    }

    #[test]
    fn empty_channels_are_not_exported() {
        let channels = export_channels(vec![
            channel("empty", Vec::new(), Vec::new()),
            channel("moves", vec![delta(3, [0.0, 1.0, 0.0])], Vec::new()),
        ]);

        let names: Vec<&str> = channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["moves"]);
        assert_eq!(channels[0].indexes, [3]);
    }

    #[test]
    fn all_zero_normal_deltas_are_omitted_and_real_ones_kept() {
        let channels = export_channels(vec![
            channel(
                "flat",
                vec![delta(0, [1.0, 0.0, 0.0])],
                vec![delta(0, [0.0, 0.0, 0.0])],
            ),
            channel(
                "tilted",
                vec![delta(0, [1.0, 0.0, 0.0])],
                vec![delta(0, [0.0, 0.5, 0.0])],
            ),
        ]);

        assert!(channels[0].normals.is_empty());
        assert_eq!(channels[1].normals, [0.0, 0.5, 0.0]);
    }
}
