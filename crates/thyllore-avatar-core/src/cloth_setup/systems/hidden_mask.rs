use cgmath::{InnerSpace, Matrix4, Vector3};
use thyllore_math_core::{cone_directions, TriangleBvh};
use thyllore_model_core::SkinData;

use crate::cloth_setup::components::hidden_mask::{HiddenMaskParams, MaskMesh};

pub fn find_hidden_body_vertices(
    body_positions: &[Vector3<f32>],
    body_normals: &[Vector3<f32>],
    garment: &TriangleBvh,
    params: &HiddenMaskParams,
) -> Vec<bool> {
    let mut hidden = Vec::with_capacity(body_positions.len());
    for i in 0..body_positions.len() {
        let origin = body_positions[i] + body_normals[i] * 1e-4;
        let dirs = cone_directions(
            body_normals[i],
            params.cone_angle_deg.to_radians(),
            params.ray_count,
        );
        let hit_count = dirs
            .iter()
            .filter(|&&dir| {
                garment
                    .raycast(origin, dir, params.cover_distance)
                    .is_some()
            })
            .count();
        let ratio = if dirs.is_empty() {
            0.0
        } else {
            hit_count as f32 / dirs.len() as f32
        };
        hidden.push(ratio >= params.hidden_ratio);
    }
    hidden
}

pub fn find_hidden_body_triangles(triangles: &[[u32; 3]], hidden_vertices: &[bool]) -> Vec<bool> {
    triangles
        .iter()
        .map(|tri| {
            hidden_vertices[tri[0] as usize]
                && hidden_vertices[tri[1] as usize]
                && hidden_vertices[tri[2] as usize]
        })
        .collect()
}

pub fn find_hidden_triangle_indices(
    body: &MaskMesh,
    garments: &MaskMesh,
    params: &HiddenMaskParams,
) -> Vec<u32> {
    if garments.positions.is_empty() {
        return Vec::new();
    }
    let garment_bvh = TriangleBvh::build(&garments.positions, &garments.triangles);
    let hidden_vertices =
        find_hidden_body_vertices(&body.positions, &body.normals, &garment_bvh, params);
    find_hidden_body_triangles(&body.triangles, &hidden_vertices)
        .iter()
        .enumerate()
        .filter(|(_, hidden)| **hidden)
        .map(|(index, _)| index as u32)
        .collect()
}

/// A skin's geometry-space vertices and index buffer carried into model space.
pub fn mask_mesh_from_skin(
    skin: &SkinData,
    indices: &[u32],
    geometry_to_model: Matrix4<f32>,
) -> MaskMesh {
    let positions = skin
        .base_positions
        .iter()
        .map(|p| (geometry_to_model * p.extend(1.0)).truncate())
        .collect();
    let normals = skin
        .base_normals
        .iter()
        .map(|n| (geometry_to_model * n.extend(0.0)).truncate().normalize())
        .collect();
    let triangles = indices
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    MaskMesh {
        positions,
        normals,
        triangles,
    }
}

pub fn merge_mask_meshes(meshes: &[MaskMesh]) -> MaskMesh {
    let mut merged = MaskMesh::default();
    for mesh in meshes {
        let offset = merged.positions.len() as u32;
        merged.positions.extend_from_slice(&mesh.positions);
        merged.normals.extend_from_slice(&mesh.normals);
        merged.triangles.extend(
            mesh.triangles
                .iter()
                .map(|[a, b, c]| [a + offset, b + offset, c + offset]),
        );
    }
    merged
}

pub fn order_visible_triangles_first(indices: &[u32], hidden_triangles: &[u32]) -> (Vec<u32>, u32) {
    let num_triangles = indices.len() / 3;
    let mut is_hidden = vec![false; num_triangles];
    for &t in hidden_triangles {
        if t < num_triangles as u32 {
            is_hidden[t as usize] = true;
        }
    }
    let mut reordered = Vec::with_capacity(indices.len());
    for t in 0..num_triangles {
        if !is_hidden[t] {
            reordered.extend_from_slice(&indices[3 * t..3 * t + 3]);
        }
    }
    let visible_count = reordered.len() as u32;
    for t in 0..num_triangles {
        if is_hidden[t] {
            reordered.extend_from_slice(&indices[3 * t..3 * t + 3]);
        }
    }
    (reordered, visible_count)
}

#[cfg(test)]
mod tests;
