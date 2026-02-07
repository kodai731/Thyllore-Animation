use super::{MeshData, VertexAttribute};

#[derive(Clone, Debug)]
pub struct VertexLayout {
    pub attributes: Vec<VertexAttribute>,
    pub stride: u32,
}

impl VertexLayout {
    pub fn from_mesh_data(mesh: &MeshData) -> Self {
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

        Self {
            attributes,
            stride: offset,
        }
    }

    pub fn from_attributes(attrs: &[VertexAttribute]) -> Self {
        let stride: u32 = attrs.iter().map(|a| a.format.size()).sum();
        Self {
            attributes: attrs.to_vec(),
            stride,
        }
    }
}
