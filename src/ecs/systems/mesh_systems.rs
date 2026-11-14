use glam::Vec3;

use crate::ecs::resource::MeshAssets;

pub fn mesh_calculate_model_bounds(assets: &MeshAssets) -> Option<(Vec3, Vec3, Vec3)> {
    if assets.meshes.is_empty() {
        return None;
    }

    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
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

    let center = Vec3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );

    Some((min, max, center))
}
