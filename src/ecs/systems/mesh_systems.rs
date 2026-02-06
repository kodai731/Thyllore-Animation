use crate::ecs::component::mesh::{
    MeshData, VertexAttributeValues, VertexFormat, VertexLayout,
};

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
        _ => panic!("Format mismatch in write_attribute_value"),
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

        let layout = VertexLayout::from_mesh_data(&mesh);
        assert_eq!(layout.stride, 24);

        let buffer = create_interleaved_buffer(&mesh, &layout);
        assert_eq!(buffer.len(), 48);
    }
}
