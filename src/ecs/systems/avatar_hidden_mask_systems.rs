use std::collections::BTreeMap;

use cgmath::{InnerSpace, Matrix4, Vector3};
use thyllore_avatar_core::garment::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_avatar_core::garment::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::garment::systems::hidden_mask::{
    find_hidden_triangle_indices, merge_mask_meshes,
};

use crate::animation::{Skeleton, SkinData};
use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, NodeAssets};
use crate::ecs::systems::avatar_submesh_role_systems::{
    collect_submesh_meshes, skin_geometry_to_model,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_vulkan_core::resource::mesh_buffer::MeshBuffer;

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
                .filter(|(key, _)| state.submesh_roles.get(key) == Some(&role))
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

    let garments = build_garment_mask_mesh(&garment_pairs, assets, graphics);

    let hidden_triangles: BTreeMap<SubmeshKey, Vec<u32>> = skin_submeshes
        .iter()
        .filter_map(|(key, mesh_index)| {
            let body = model_space_mask_mesh(key, &graphics.meshes[*mesh_index], assets)?;
            let hidden = find_hidden_triangle_indices(&body, &garments, params);
            Some((key.clone(), hidden))
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

/// The submesh in the model space shared by every skin of its skeleton, or none when it is not skinned.
fn model_space_mask_mesh(
    key: &SubmeshKey,
    mesh: &MeshBuffer,
    assets: &AssetStorage,
) -> Option<MaskMesh> {
    let Some(skin) = mesh.skin_data.as_ref() else {
        msg_warn!(
            "{}/{} has no skin data — skipped by the hidden mask",
            key.mesh,
            key.material
        );
        return None;
    };
    let Some(skeleton) = assets.get_skeleton_by_skeleton_id(skin.skeleton_id) else {
        msg_warn!(
            "{}/{} is skinned to an unknown skeleton — skipped by the hidden mask",
            key.mesh,
            key.material
        );
        return None;
    };
    let geometry_to_model = skin_geometry_to_model(skeleton, skeleton_root_bone(skeleton));
    Some(skin_to_mask_mesh(
        skin,
        &mesh.vertex_data.indices,
        geometry_to_model,
    ))
}

fn skeleton_root_bone(skeleton: &Skeleton) -> usize {
    skeleton
        .bones
        .iter()
        .position(|bone| bone.parent_id.is_none())
        .unwrap_or(0)
}

fn skin_to_mask_mesh(
    skin: &SkinData,
    indices: &[u32],
    geometry_to_model: Matrix4<f32>,
) -> MaskMesh {
    let positions: Vec<Vector3<f32>> = skin
        .base_positions
        .iter()
        .map(|p| (geometry_to_model * p.extend(1.0)).truncate())
        .collect();
    let normals: Vec<Vector3<f32>> = skin
        .base_normals
        .iter()
        .map(|n| (geometry_to_model * n.extend(0.0)).truncate().normalize())
        .collect();
    let triangles: Vec<[u32; 3]> = indices
        .chunks_exact(3)
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
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> MaskMesh {
    let meshes: Vec<MaskMesh> = garment_pairs
        .iter()
        .filter_map(|(key, mesh_index)| {
            model_space_mask_mesh(key, &graphics.meshes[*mesh_index], assets)
        })
        .collect();
    merge_mask_meshes(&meshes)
}
