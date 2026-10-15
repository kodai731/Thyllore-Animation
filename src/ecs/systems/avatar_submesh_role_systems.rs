use thyllore_avatar_core::garment::components::submesh_role::{
    merge_guessed_roles, SubmeshKey, SubmeshRole, SubmeshRoleEntry, SubmeshRoleSource,
};

use crate::asset::AssetStorage;
use crate::ecs::resource::AvatarSetupState;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::MaterialId;

pub(crate) fn collect_submesh_keys(assets: &AssetStorage) -> Vec<SubmeshKey> {
    let mut keys: Vec<SubmeshKey> = assets
        .meshes
        .values()
        .map(|mesh| SubmeshKey {
            mesh: mesh.name.clone(),
            material: find_material_name(assets, mesh.material_id),
        })
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn find_material_name(assets: &AssetStorage, material_id: Option<MaterialId>) -> String {
    material_id
        .and_then(|id| {
            assets
                .materials
                .values()
                .find(|material| material.material_id == id)
        })
        .map(|material| material.name.clone())
        .unwrap_or_default()
}

pub fn refresh_submesh_roles(state: &mut AvatarSetupState, assets: &AssetStorage) {
    let keys = collect_submesh_keys(assets);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{MaterialAsset, MeshAsset};

    fn add_submesh(assets: &mut AssetStorage, mesh: &str, material: &str, material_id: MaterialId) {
        assets.add_material(MaterialAsset {
            id: 0,
            name: material.to_string(),
            material_id,
            properties: Default::default(),
        });
        assets.add_mesh(MeshAsset {
            id: 0,
            name: mesh.to_string(),
            graphics_mesh_index: 0,
            object_index: 0,
            material_id: Some(material_id),
            skeleton_id: None,
            node_index: None,
            render_to_gbuffer: true,
        });
    }

    fn key(mesh: &str, material: &str) -> SubmeshKey {
        SubmeshKey {
            mesh: mesh.to_string(),
            material: material.to_string(),
        }
    }

    #[test]
    fn test_collect_submesh_keys_splits_same_mesh_by_material() {
        let mut assets = AssetStorage::default();
        add_submesh(&mut assets, "Body", "Skin", 1);
        add_submesh(&mut assets, "Body", "Shirt", 2);

        let keys = collect_submesh_keys(&assets);

        assert_eq!(keys, vec![key("Body", "Shirt"), key("Body", "Skin")]);
    }

    #[test]
    fn test_refresh_submesh_roles_keeps_user_choice() {
        let mut assets = AssetStorage::default();
        add_submesh(&mut assets, "Body", "Skin", 1);
        add_submesh(&mut assets, "Body", "Shirt", 2);
        let mut state = AvatarSetupState::default();
        refresh_submesh_roles(&mut state, &assets);

        let user_entry = SubmeshRoleEntry {
            role: SubmeshRole::Accessory,
            source: SubmeshRoleSource::User,
        };
        state.submesh_roles.insert(key("Body", "Skin"), user_entry);
        refresh_submesh_roles(&mut state, &assets);

        assert_eq!(state.submesh_roles.len(), 2);
        assert_eq!(state.submesh_roles[&key("Body", "Skin")], user_entry);
    }
}
