use thyllore_avatar_core::garment::components::submesh_role::{
    merge_guessed_roles, SubmeshKey, SubmeshRole, SubmeshRoleEntry, SubmeshRoleSource,
};

use crate::asset::{AssetStorage, MeshAsset};
use crate::ecs::resource::{AvatarSetupState, HierarchyState, NodeAssets};
use crate::ecs::systems::hierarchy_systems::{hierarchy_reveal_entity, hierarchy_select};
use crate::ecs::world::{Entity, MeshRef, World};
use crate::vulkanr::resource::graphics_resource::{GraphicsResources, NodeData};

pub(crate) fn collect_submesh_keys(
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    nodes: &[NodeData],
) -> Vec<SubmeshKey> {
    collect_submesh_meshes(assets, graphics, nodes)
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

pub(crate) fn collect_submesh_meshes(
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    nodes: &[NodeData],
) -> Vec<(SubmeshKey, usize)> {
    let mut pairs: Vec<(SubmeshKey, usize)> = assets
        .meshes
        .values()
        .map(|mesh_asset| {
            let node_name = mesh_asset
                .node_index
                .and_then(|node_index| nodes.get(node_index))
                .map(|node| node.name.as_str());
            let material_name = find_material_name(graphics, mesh_asset.graphics_mesh_index);
            let key = build_submesh_key(mesh_asset, node_name, material_name);
            (key, mesh_asset.graphics_mesh_index)
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs.dedup_by(|a, b| a.0 == b.0);
    pairs
}

fn find_material_name(graphics: &GraphicsResources, graphics_mesh_index: usize) -> Option<&str> {
    let material_id = graphics.mesh_material_ids.get(graphics_mesh_index)?;
    graphics
        .materials
        .get(*material_id)
        .map(|material| material.name.as_str())
}

fn build_submesh_key(
    mesh_asset: &MeshAsset,
    node_name: Option<&str>,
    material_name: Option<&str>,
) -> SubmeshKey {
    SubmeshKey {
        mesh: node_name.unwrap_or(&mesh_asset.name).to_string(),
        material: material_name.unwrap_or_default().to_string(),
    }
}

pub fn refresh_submesh_roles(
    state: &mut AvatarSetupState,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    nodes: &[NodeData],
) {
    let keys = collect_submesh_keys(assets, graphics, nodes);
    state.submesh_roles = merge_guessed_roles(&state.submesh_roles, &keys);
}

pub fn set_submesh_role(world: &mut World, key: SubmeshKey, role: SubmeshRole) {
    let mut state = world.resource_mut::<AvatarSetupState>();
    let Some(entry) = state.submesh_roles.get_mut(&key) else {
        log_warn!(
            "Cannot set submesh role for missing submesh ({}, {})",
            key.mesh,
            key.material
        );
        return;
    };
    *entry = SubmeshRoleEntry {
        role,
        source: SubmeshRoleSource::User,
    };
}

/// The scene entity that renders the submesh, found through its mesh asset.
pub fn find_submesh_entity(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    key: &SubmeshKey,
) -> Option<Entity> {
    let nodes = &world.resource::<NodeAssets>().nodes;
    let (_, mesh_index) = collect_submesh_meshes(assets, graphics, nodes)
        .into_iter()
        .find(|(candidate, _)| candidate == key)?;
    let asset_id = assets
        .meshes
        .iter()
        .find(|(_, mesh_asset)| mesh_asset.graphics_mesh_index == mesh_index)
        .map(|(id, _)| *id)?;
    world
        .iter_components::<MeshRef>()
        .find(|(_, mesh_ref)| mesh_ref.mesh_asset_id == asset_id)
        .map(|(entity, _)| entity)
}

pub fn select_submesh_entity(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    key: &SubmeshKey,
) {
    let Some(entity) = find_submesh_entity(world, assets, graphics, key) else {
        log_warn!("No entity renders submesh ({}, {})", key.mesh, key.material);
        return;
    };
    hierarchy_select(&mut world.resource_mut::<HierarchyState>(), entity);
    hierarchy_reveal_entity(world, entity);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mesh_asset(name: &str) -> MeshAsset {
        MeshAsset {
            id: 0,
            name: name.to_string(),
            graphics_mesh_index: 0,
            object_index: 0,
            material_id: None,
            skeleton_id: None,
            node_index: None,
            render_to_gbuffer: true,
        }
    }

    fn key(mesh: &str, material: &str) -> SubmeshKey {
        SubmeshKey {
            mesh: mesh.to_string(),
            material: material.to_string(),
        }
    }

    #[test]
    fn test_build_submesh_key_uses_node_and_material_names() {
        let mesh_asset = make_mesh_asset("test_humanoid_01");

        let submesh_key = build_submesh_key(&mesh_asset, Some("Body"), Some("Skin"));

        assert_eq!(submesh_key, key("Body", "Skin"));
    }

    #[test]
    fn test_build_submesh_key_falls_back_to_mesh_asset_name_without_node() {
        let mesh_asset = make_mesh_asset("test_humanoid_01");

        let submesh_key = build_submesh_key(&mesh_asset, None, Some("Skin"));

        assert_eq!(submesh_key, key("test_humanoid_01", "Skin"));
    }

    #[test]
    fn test_build_submesh_key_leaves_material_empty_without_material() {
        let mesh_asset = make_mesh_asset("test_humanoid_01");

        let submesh_key = build_submesh_key(&mesh_asset, Some("Body"), None);

        assert_eq!(submesh_key, key("Body", ""));
    }

    #[test]
    fn test_build_submesh_key_without_node_and_material() {
        let mesh_asset = make_mesh_asset("test_humanoid_01");

        let submesh_key = build_submesh_key(&mesh_asset, None, None);

        assert_eq!(submesh_key, key("test_humanoid_01", ""));
    }

    #[test]
    fn select_submesh_entity_selects_and_reveals_the_mesh_entity() {
        let mut world = World::new();
        world.insert_resource(NodeAssets::default());
        world.insert_resource(HierarchyState::default());
        let mut assets = AssetStorage::new();
        let graphics = GraphicsResources::default();
        let asset_id = assets.add_mesh(make_mesh_asset("test_humanoid_01"));
        let parent = world.entity().with_name("model").build();
        let entity = world
            .entity()
            .with_name("test_humanoid_01")
            .with_parent(parent)
            .with_mesh(asset_id, 0)
            .build();

        select_submesh_entity(&mut world, &assets, &graphics, &key("test_humanoid_01", ""));

        let state = world.resource::<HierarchyState>();
        assert_eq!(state.selected_entity, Some(entity));
        assert!(state.scroll_to_selected);
        assert_eq!(
            find_submesh_entity(&world, &assets, &graphics, &key("missing", "")),
            None
        );
    }

    #[test]
    fn test_refresh_submesh_roles_keeps_user_choice() {
        let mut assets = AssetStorage::default();
        assets.add_mesh(make_mesh_asset("Body"));
        assets.add_mesh(make_mesh_asset("Hair"));
        let graphics = GraphicsResources::default();
        let mut state = AvatarSetupState::default();
        refresh_submesh_roles(&mut state, &assets, &graphics, &[]);

        let user_entry = SubmeshRoleEntry {
            role: SubmeshRole::Accessory,
            source: SubmeshRoleSource::User,
        };
        state.submesh_roles.insert(key("Body", ""), user_entry);
        refresh_submesh_roles(&mut state, &assets, &graphics, &[]);

        assert_eq!(state.submesh_roles.len(), 2);
        assert_eq!(state.submesh_roles[&key("Body", "")], user_entry);
    }
}
