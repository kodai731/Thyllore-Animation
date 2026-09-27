use crate::asset::AssetStorage;
use crate::ecs::component::{AppliedMorphWeights, MorphWeights};
use crate::ecs::world::{Entity, MeshRef, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::mesh_buffer::MeshBuffer;

pub fn apply_morph_weights(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &mut GraphicsResources,
) -> Vec<usize> {
    let pending = collect_pending_morph_updates(world, assets, graphics);

    let mut updated_meshes = Vec::with_capacity(pending.len());
    for (entity, mesh_idx, weights) in pending {
        apply_morph_to_mesh(&mut graphics.meshes[mesh_idx], &weights);
        world.insert_component(entity, AppliedMorphWeights { weights });
        updated_meshes.push(mesh_idx);
    }

    updated_meshes
}

fn collect_pending_morph_updates(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Vec<(Entity, usize, Vec<f32>)> {
    world
        .iter_components::<MorphWeights>()
        .filter_map(|(entity, morph_weights)| {
            let mesh_ref = world.get_component::<MeshRef>(entity)?;
            let mesh_idx = assets.get_mesh(mesh_ref.mesh_asset_id)?.graphics_mesh_index;
            let mesh = graphics.meshes.get(mesh_idx)?;
            if mesh.morph.channels.is_empty() {
                return None;
            }

            let is_unchanged = world
                .get_component::<AppliedMorphWeights>(entity)
                .is_some_and(|applied| applied.weights == morph_weights.weights);
            if is_unchanged {
                return None;
            }

            Some((entity, mesh_idx, morph_weights.weights.clone()))
        })
        .collect()
}

fn apply_morph_to_mesh(mesh: &mut MeshBuffer, weights: &[f32]) {
    let base_positions: Vec<[f32; 3]> = mesh
        .base_vertices
        .iter()
        .map(|v| [v.pos.x, v.pos.y, v.pos.z])
        .collect();

    let morphed =
        thyllore_model_core::accumulate_morph_positions(&base_positions, &mesh.morph, weights);

    for (i, pos) in morphed.iter().enumerate() {
        if i < mesh.vertex_data.vertices.len() {
            mesh.vertex_data.vertices[i].pos.x = pos[0];
            mesh.vertex_data.vertices[i].pos.y = pos[1];
            mesh.vertex_data.vertices[i].pos.z = pos[2];
        }
    }

    if let Some(ref mut skin_data) = mesh.skin_data {
        for (i, pos) in morphed.iter().enumerate() {
            if i < skin_data.base_positions.len() {
                skin_data.base_positions[i].x = pos[0];
                skin_data.base_positions[i].y = pos[1];
                skin_data.base_positions[i].z = pos[2];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vulkanr::data::Vertex;
    use thyllore_math_core::{Vec2, Vec3, Vec4};
    use thyllore_model_core::{MeshMorph, MorphChannel, SparseDelta};

    fn make_test_mesh() -> MeshBuffer {
        let vertices = vec![
            Vertex::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(0.0, 0.0),
            ),
            Vertex::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(1.0, 0.0),
            ),
            Vertex::new(
                Vec3::new(0.0, 1.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ),
        ];
        let mut mesh = MeshBuffer::default();
        mesh.vertex_data.vertices = vertices.clone();
        mesh.base_vertices = vertices;
        mesh.morph = MeshMorph {
            channels: vec![MorphChannel {
                name: "test".to_string(),
                position_deltas: vec![
                    SparseDelta {
                        vertex_index: 0,
                        delta: [0.0, 1.0, 0.0],
                    },
                    SparseDelta {
                        vertex_index: 2,
                        delta: [0.0, -0.5, 0.0],
                    },
                ],
                normal_deltas: Vec::new(),
            }],
        };
        mesh
    }

    #[test]
    fn test_apply_morph_to_mesh_full_weight() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[1.0]);

        assert!((mesh.vertex_data.vertices[0].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.y - 1.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.z - 0.0).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[1].pos.x - 1.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[1].pos.y - 0.0).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[2].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[2].pos.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_half_weight() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[0.5]);

        assert!((mesh.vertex_data.vertices[0].pos.y - 0.5).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[2].pos.y - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_zero_weight_is_noop() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[0.0]);

        assert!((mesh.vertex_data.vertices[0].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.y - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[2].pos.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_idempotent() {
        let mut mesh1 = make_test_mesh();
        apply_morph_to_mesh(&mut mesh1, &[1.0]);
        apply_morph_to_mesh(&mut mesh1, &[0.5]);

        let mut mesh2 = make_test_mesh();
        apply_morph_to_mesh(&mut mesh2, &[0.5]);

        for i in 0..3 {
            assert!(
                (mesh1.vertex_data.vertices[i].pos.x - mesh2.vertex_data.vertices[i].pos.x).abs()
                    < 1e-6
            );
            assert!(
                (mesh1.vertex_data.vertices[i].pos.y - mesh2.vertex_data.vertices[i].pos.y).abs()
                    < 1e-6
            );
            assert!(
                (mesh1.vertex_data.vertices[i].pos.z - mesh2.vertex_data.vertices[i].pos.z).abs()
                    < 1e-6
            );
        }
    }
}
