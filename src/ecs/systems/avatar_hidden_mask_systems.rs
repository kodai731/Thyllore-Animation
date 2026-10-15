use std::collections::BTreeMap;

use cgmath::Vector3;
use thyllore_avatar_core::garment::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_avatar_core::garment::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::garment::systems::hidden_mask::find_hidden_triangle_indices;

use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, NodeAssets};
use crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes;
use crate::ecs::world::World;
use crate::vulkanr::data::VertexData;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn compute_hidden_triangles(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    params: &HiddenMaskParams,
) {
    let pairs = {
        let node_assets = world.resource::<NodeAssets>();
        collect_submesh_meshes(assets, graphics, &node_assets.nodes)
    };
    let (skin_submeshes, garment_pairs) = {
        let state = world.resource::<AvatarSetupState>();
        let filter_by_role = |role: SubmeshRole| -> Vec<(SubmeshKey, usize)> {
            pairs
                .iter()
                .filter(|(key, _)| state.submesh_roles.get(key).map(|e| e.role) == Some(role))
                .cloned()
                .collect()
        };
        (
            filter_by_role(SubmeshRole::Skin),
            filter_by_role(SubmeshRole::Garment),
        )
    };

    if garment_pairs.is_empty() {
        msg_warn!("No Garment-role submeshes found — hidden triangle computation skipped");
        return;
    }

    let garments = build_garment_mask_mesh(&garment_pairs, graphics);

    let hidden_triangles: BTreeMap<SubmeshKey, Vec<u32>> = skin_submeshes
        .iter()
        .map(|(key, mesh_index)| {
            let body = mesh_to_mask_mesh(&graphics.meshes[*mesh_index].vertex_data);
            let hidden = find_hidden_triangle_indices(&body, &garments, params);
            (key.clone(), hidden)
        })
        .collect();
    let total: usize = hidden_triangles.values().map(Vec::len).sum();

    let meshes: Vec<(usize, Vec<u32>)> = skin_submeshes
        .iter()
        .map(|(key, mesh_index)| {
            (
                *mesh_index,
                hidden_triangles.get(key).cloned().unwrap_or_default(),
            )
        })
        .collect();

    world.resource_mut::<AvatarSetupState>().hidden_triangles = hidden_triangles;

    msg_info!(
        "Computed {} hidden triangles across {} skin submeshes",
        total,
        skin_submeshes.len()
    );

    world
        .resource_mut::<crate::ecs::resource::OutputQueue>()
        .push(crate::ecs::resource::OutputCommand::ApplyHiddenTriangles { meshes });
}

fn mesh_to_mask_mesh(vertex_data: &VertexData) -> MaskMesh {
    let positions: Vec<Vector3<f32>> = vertex_data
        .vertices
        .iter()
        .map(|v| Vector3::new(v.pos.x, v.pos.y, v.pos.z))
        .collect();
    let normals: Vec<Vector3<f32>> = vertex_data
        .vertices
        .iter()
        .map(|v| Vector3::new(v.normal.x, v.normal.y, v.normal.z))
        .collect();
    let triangles: Vec<[u32; 3]> = vertex_data
        .indices
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    MaskMesh {
        positions,
        normals,
        triangles,
    }
}

fn build_garment_mask_mesh(
    garment_pairs: &[(SubmeshKey, usize)],
    graphics: &GraphicsResources,
) -> MaskMesh {
    let mut merged = MaskMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        triangles: Vec::new(),
    };

    for (_key, mesh_index) in garment_pairs {
        let garment = mesh_to_mask_mesh(&graphics.meshes[*mesh_index].vertex_data);
        let offset = merged.positions.len() as u32;
        merged.positions.extend(garment.positions);
        merged.normals.extend(garment.normals);
        merged.triangles.extend(
            garment
                .triangles
                .iter()
                .map(|[a, b, c]| [a + offset, b + offset, c + offset]),
        );
    }

    merged
}
