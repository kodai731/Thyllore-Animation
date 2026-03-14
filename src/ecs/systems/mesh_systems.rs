use cgmath::Vector3;

use crate::ecs::component::mesh::{
    MeshData, VertexAttribute, VertexAttributeValues, VertexFormat, VertexLayout,
};
use crate::ecs::resource::MeshAssets;

pub fn mesh_calculate_model_bounds(
    assets: &MeshAssets,
) -> Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)> {
    if assets.meshes.is_empty() {
        return None;
    }

    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut has_vertices = false;

    for mesh in &assets.meshes {
        for vertex in &mesh.vertex_data.vertices {
            has_vertices = true;
            min.x = min.x.min(vertex.pos.x);
            min.y = min.y.min(vertex.pos.y);
            min.z = min.z.min(vertex.pos.z);
            max.x = max.x.max(vertex.pos.x);
            max.y = max.y.max(vertex.pos.y);
            max.z = max.z.max(vertex.pos.z);
        }
    }

    if !has_vertices {
        return None;
    }

    let center = Vector3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );

    Some((min, max, center))
}

pub fn validate_mesh_data(mesh: &MeshData) -> Result<(), String> {
    if mesh.vertex_count() == 0 && mesh.attribute_ids().next().is_none() {
        return Err("MeshData has no attributes".to_string());
    }

    let vertex_count = mesh.vertex_count();
    for id in mesh.attribute_ids() {
        if let Some(values) = mesh.attribute(*id) {
            if values.len() != vertex_count {
                return Err(format!(
                    "Attribute {:?} has {} vertices, expected {}",
                    id,
                    values.len(),
                    vertex_count
                ));
            }
        }
    }

    Ok(())
}

pub fn compute_vertex_layout(mesh: &MeshData) -> VertexLayout {
    let mut attributes = Vec::new();
    let mut offset = 0u32;

    for id in mesh.attribute_ids() {
        if let Some(values) = mesh.attribute(*id) {
            let format = values.format();
            let attr = VertexAttribute::new(*id, format, id.default_location());
            attributes.push(attr);
            offset += format.size();
        }
    }

    VertexLayout {
        attributes,
        stride: offset,
    }
}

pub fn create_interleaved_buffer(mesh: &MeshData, layout: &VertexLayout) -> Vec<u8> {
    let vertex_count = mesh.vertex_count();
    if vertex_count == 0 {
        return Vec::new();
    }

    let buffer_size = vertex_count * layout.stride as usize;
    let mut buffer = vec![0u8; buffer_size];

    for vertex_idx in 0..vertex_count {
        let mut offset = 0usize;
        for attr in &layout.attributes {
            if let Some(values) = mesh.attribute(attr.id) {
                let dst_offset = vertex_idx * layout.stride as usize + offset;
                write_attribute_value(&mut buffer, dst_offset, values, vertex_idx, attr.format);
            }
            offset += attr.format.size() as usize;
        }
    }

    buffer
}

fn write_attribute_value(
    buffer: &mut [u8],
    offset: usize,
    values: &VertexAttributeValues,
    index: usize,
    format: VertexFormat,
) {
    match (values, format) {
        (VertexAttributeValues::Float32x2(data), VertexFormat::Float32x2) => {
            write_f32_array(&mut buffer[offset..], &data[index]);
        }
        (VertexAttributeValues::Float32x3(data), VertexFormat::Float32x3) => {
            write_f32_array(&mut buffer[offset..], &data[index]);
        }
        (VertexAttributeValues::Float32x4(data), VertexFormat::Float32x4) => {
            write_f32_array(&mut buffer[offset..], &data[index]);
        }
        _ => unreachable!(
            "Format mismatch in write_attribute_value: values={:?}, format={:?}",
            std::mem::discriminant(values),
            format
        ),
    }
}

fn write_f32_array<const N: usize>(buffer: &mut [u8], values: &[f32; N]) {
    for (i, &value) in values.iter().enumerate() {
        let bytes = value.to_le_bytes();
        let start = i * 4;
        buffer[start..start + 4].copy_from_slice(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::mesh::presets::{COLOR, POSITION};
    use crate::ecs::component::mesh::{MeshData, PrimitiveTopology};

    #[test]
    fn test_interleaved_buffer_creation() {
        let mesh = MeshData::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(POSITION, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])
            .with_inserted_attribute(COLOR, vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);

        let layout = compute_vertex_layout(&mesh);
        assert_eq!(layout.stride, 24);

        let buffer = create_interleaved_buffer(&mesh, &layout);
        assert_eq!(buffer.len(), 48);
    }
}
