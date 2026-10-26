use cgmath::{InnerSpace, Vector3};

use crate::garment::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_math_core::TriangleBvh;

fn cone_directions(normal: Vector3<f32>, cone_angle_deg: f32, count: usize) -> Vec<Vector3<f32>> {
    let mut directions = Vec::with_capacity(count);
    if count == 0 {
        return directions;
    }
    directions.push(normal);
    if count == 1 {
        return directions;
    }
    let reference_axis = if normal.y.abs() < 0.9 {
        Vector3::unit_y()
    } else {
        Vector3::unit_x()
    };
    let tangent = (reference_axis - normal * normal.dot(reference_axis)).normalize();
    let bitangent = normal.cross(tangent).normalize();
    let cone_rad = cone_angle_deg.to_radians();
    for i in 1..count {
        let azimuth = 2.0 * std::f32::consts::PI * (i as f32) / (count - 1) as f32;
        let cos_a = azimuth.cos();
        let sin_a = azimuth.sin();
        let tangent_cone = tangent * cos_a + bitangent * sin_a;
        let dir = (normal * cone_rad.cos() + tangent_cone * cone_rad.sin()).normalize();
        directions.push(dir);
    }
    directions
}

pub fn find_hidden_body_vertices(
    body_positions: &[Vector3<f32>],
    body_normals: &[Vector3<f32>],
    garment: &TriangleBvh,
    params: &HiddenMaskParams,
) -> Vec<bool> {
    let mut hidden = Vec::with_capacity(body_positions.len());
    for i in 0..body_positions.len() {
        let origin = body_positions[i] + body_normals[i] * 1e-4;
        let dirs = cone_directions(body_normals[i], params.cone_angle_deg, params.ray_count);
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
