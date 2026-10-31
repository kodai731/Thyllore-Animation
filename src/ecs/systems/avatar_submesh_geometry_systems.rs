use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshKey;

/// One submesh inside the merged vertex range.
pub(crate) struct SubmeshPart {
    pub(crate) key: SubmeshKey,
    pub(crate) mesh_index: usize,
    pub(crate) vertex_offset: u32,
    pub(crate) vertex_count: u32,
}

impl SubmeshPart {
    pub(crate) fn contains(&self, vertex: u32) -> bool {
        vertex >= self.vertex_offset && vertex < self.vertex_offset + self.vertex_count
    }
}

/// The skin vertices and triangles of several submeshes concatenated into one index space.
pub(crate) struct MergedSubmeshGeometry {
    pub(crate) parts: Vec<SubmeshPart>,
    pub(crate) positions: Vec<cgmath::Vector3<f32>>,
    pub(crate) triangles: Vec<[u32; 3]>,
    pub(crate) tex_coords: Vec<[f32; 2]>,
    pub(crate) skeleton_id: crate::animation::SkeletonId,
}

fn submesh_tex_coords(
    vertices: &[thyllore_model_core::mesh::Vertex],
    vertex_count: usize,
) -> Option<Vec<[f32; 2]>> {
    if vertices.len() != vertex_count {
        return None;
    }
    Some(
        vertices
            .iter()
            .map(|v| [v.tex_coord.x, v.tex_coord.y])
            .collect(),
    )
}

pub(crate) fn merge_submesh_geometry(
    graphics: &GraphicsResources,
    submeshes: Vec<(SubmeshKey, usize)>,
) -> anyhow::Result<MergedSubmeshGeometry> {
    let mut parts = Vec::new();
    let mut positions: Vec<cgmath::Vector3<f32>> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut submesh_uvs: Vec<Option<Vec<[f32; 2]>>> = Vec::new();
    let mut skeleton_id = None;

    for (key, mesh_index) in submeshes {
        let mesh = &graphics.meshes[mesh_index];
        let skin = mesh
            .skin_data
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}/{} has no skin_data", key.mesh, key.material))?;
        match skeleton_id {
            None => skeleton_id = Some(skin.skeleton_id),
            Some(id) if id != skin.skeleton_id => {
                anyhow::bail!(
                    "{}/{} is skinned to another skeleton",
                    key.mesh,
                    key.material
                )
            }
            Some(_) => {}
        }

        let vertex_offset = positions.len() as u32;
        positions.extend_from_slice(&skin.base_positions);
        triangles.extend(mesh.vertex_data.indices.chunks_exact(3).map(|c| {
            [
                c[0] + vertex_offset,
                c[1] + vertex_offset,
                c[2] + vertex_offset,
            ]
        }));

        submesh_uvs.push(submesh_tex_coords(
            &mesh.vertex_data.vertices,
            skin.base_positions.len(),
        ));

        parts.push(SubmeshPart {
            key,
            mesh_index,
            vertex_offset,
            vertex_count: skin.base_positions.len() as u32,
        });
    }

    let tex_coords = submesh_uvs
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .map(|uvs| uvs.concat())
        .unwrap_or_default();

    let Some(skeleton_id) = skeleton_id else {
        anyhow::bail!("no submesh to generate chains on");
    };
    Ok(MergedSubmeshGeometry {
        parts,
        positions,
        triangles,
        tex_coords,
        skeleton_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_math_core::{Vec2, Vec3};
    use thyllore_model_core::mesh::Vertex;

    fn vertex_with_uv(u: f32, v: f32) -> Vertex {
        Vertex {
            pos: Vec3::new(0.0, 0.0, 0.0),
            color: Default::default(),
            tex_coord: Vec2::new(u, v),
            normal: Default::default(),
        }
    }

    #[test]
    fn test_submesh_tex_coords_matching_count() {
        let vertices: Vec<Vertex> = vec![vertex_with_uv(0.1, 0.2), vertex_with_uv(0.3, 0.4)];
        let result = submesh_tex_coords(&vertices, 2);
        assert!(result.is_some());
        let tc = result.unwrap();
        assert_eq!(tc.len(), 2);
        assert!((tc[0][0] - 0.1).abs() < 1e-6);
        assert!((tc[0][1] - 0.2).abs() < 1e-6);
        assert!((tc[1][0] - 0.3).abs() < 1e-6);
        assert!((tc[1][1] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_submesh_tex_coords_mismatching_count() {
        let vertices: Vec<Vertex> = vec![vertex_with_uv(0.1, 0.2)];
        let result = submesh_tex_coords(&vertices, 3);
        assert!(result.is_none());
    }
}
