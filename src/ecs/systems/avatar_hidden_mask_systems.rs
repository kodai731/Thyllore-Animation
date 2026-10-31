use std::collections::BTreeMap;

use thyllore_avatar_core::cloth_setup::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_avatar_core::cloth_setup::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::cloth_setup::systems::hidden_mask::{
    find_hidden_triangle_indices, mask_mesh_from_skin, merge_mask_meshes,
};
use thyllore_avatar_core::vrchat::sidecar::SidecarHiddenTriangles;

use crate::animation::Skeleton;
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

    let hidden_triangles: BTreeMap<SubmeshKey, Vec<u32>> = if garment_pairs.is_empty() {
        msg_warn!("No Garment-role submeshes found — the hidden mask is cleared");
        BTreeMap::new()
    } else {
        let garments = build_garment_mask_mesh(&garment_pairs, assets, graphics);
        skin_submeshes
            .iter()
            .filter_map(|(key, mesh_index)| {
                let body = model_space_mask_mesh(key, &graphics.meshes[*mesh_index], assets)?;
                let hidden = find_hidden_triangle_indices(&body, &garments, params);
                Some((key.clone(), hidden))
            })
            .collect()
    };
    let total: usize = hidden_triangles.values().map(Vec::len).sum();

    let previously_hidden: Vec<SubmeshKey> = world
        .resource::<AvatarSetupState>()
        .hidden_triangles
        .keys()
        .cloned()
        .collect();
    let meshes = mask_targets(&pairs, &previously_hidden, &hidden_triangles);
    world.resource_mut::<AvatarSetupState>().hidden_triangles = hidden_triangles;

    msg_info!(
        "Computed {} hidden triangles across {} skin submeshes",
        total,
        skin_submeshes.len()
    );
    push_apply_hidden_triangles(world, meshes);
}

/// Every submesh the new mask touches or the previous mask touched, with its new mask; a submesh the
/// new mask leaves out gets an empty one so its triangles come back.
fn mask_targets(
    pairs: &[(SubmeshKey, usize)],
    previously_hidden: &[SubmeshKey],
    hidden_triangles: &BTreeMap<SubmeshKey, Vec<u32>>,
) -> Vec<(usize, Vec<u32>)> {
    pairs
        .iter()
        .filter(|(key, _)| previously_hidden.contains(key) || hidden_triangles.contains_key(key))
        .map(|(key, mesh_index)| {
            (
                *mesh_index,
                hidden_triangles.get(key).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

fn push_apply_hidden_triangles(world: &mut World, meshes: Vec<(usize, Vec<u32>)>) {
    if meshes.is_empty() {
        return;
    }
    world
        .resource_mut::<crate::ecs::resource::OutputQueue>()
        .push(crate::ecs::resource::OutputCommand::ApplyHiddenTriangles { meshes });
}

/// Restore the hidden triangles a sidecar saved: into the setup state and onto the skin meshes.
pub fn restore_hidden_triangles(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    saved: &[SidecarHiddenTriangles],
) {
    if saved.is_empty() {
        return;
    }
    let pairs = {
        let node_assets = world.resource::<NodeAssets>();
        collect_submesh_meshes(assets, graphics, &node_assets.nodes)
    };

    let mut hidden_triangles: BTreeMap<SubmeshKey, Vec<u32>> = BTreeMap::new();
    let mut meshes: Vec<(usize, Vec<u32>)> = Vec::new();
    for entry in saved {
        let key = SubmeshKey {
            mesh: entry.mesh.clone(),
            material: entry.material.clone(),
        };
        let Some(&(_, mesh_index)) = pairs.iter().find(|(candidate, _)| *candidate == key) else {
            msg_warn!(
                "Avatar sidecar hides triangles of unknown submesh {}/{}",
                key.mesh,
                key.material
            );
            continue;
        };
        let triangle_count = (graphics.meshes[mesh_index].vertex_data.indices.len() / 3) as u32;
        if let Some(&out_of_range) = entry.triangles.iter().find(|&&t| t >= triangle_count) {
            msg_warn!(
                "Avatar sidecar hides triangle {} of {}/{}, which has {} triangles; mask ignored",
                out_of_range,
                key.mesh,
                key.material,
                triangle_count
            );
            continue;
        }
        meshes.push((mesh_index, entry.triangles.clone()));
        hidden_triangles.insert(key, entry.triangles.clone());
    }

    world.resource_mut::<AvatarSetupState>().hidden_triangles = hidden_triangles;
    push_apply_hidden_triangles(world, meshes);
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
    Some(mask_mesh_from_skin(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(mesh: &str) -> SubmeshKey {
        SubmeshKey {
            mesh: mesh.to_string(),
            material: "mat".to_string(),
        }
    }

    #[test]
    fn mask_targets_clear_submeshes_the_new_mask_leaves_out() {
        let pairs = vec![(key("body"), 0), (key("arm"), 1), (key("hair"), 2)];
        let previously_hidden = vec![key("body"), key("arm")];
        let hidden: BTreeMap<SubmeshKey, Vec<u32>> = [(key("body"), vec![3, 4])].into();

        let targets = mask_targets(&pairs, &previously_hidden, &hidden);

        assert_eq!(targets, vec![(0, vec![3, 4]), (1, Vec::new())]);
    }

    #[test]
    fn mask_targets_with_no_garment_clear_every_previous_mask() {
        let pairs = vec![(key("body"), 0), (key("arm"), 1)];
        let previously_hidden = vec![key("arm")];

        let targets = mask_targets(&pairs, &previously_hidden, &BTreeMap::new());

        assert_eq!(targets, vec![(1, Vec::new())]);
    }
}
