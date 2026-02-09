use crate::asset::AssetStorage;
use crate::ecs::resource::{HierarchyDisplayMode, ObjectIdReadback};
use crate::ecs::systems::hierarchy_systems::{
    hierarchy_deselect_all, hierarchy_select, hierarchy_toggle_selection,
};
use crate::ecs::world::{Entity, MeshRef, World};

pub fn find_entity_by_object_id(
    world: &World,
    assets: &AssetStorage,
    object_id: u32,
) -> Option<Entity> {
    if object_id == 0 {
        return None;
    }

    let mesh_index = (object_id - 1) as usize;
    let mesh_asset = assets.find_mesh_by_graphics_index(mesh_index)?;
    let target_asset_id = mesh_asset.id;

    world
        .iter_components::<MeshRef>()
        .find(|(_, mesh_ref)| mesh_ref.mesh_asset_id == target_asset_id)
        .map(|(entity, _)| entity)
}

pub fn apply_mesh_selection(
    world: &mut World,
    assets: &AssetStorage,
    readback: &mut ObjectIdReadback,
) {
    let Some(object_id) = readback.last_read_object_id.take() else {
        return;
    };

    let is_shift = readback.is_shift;
    let is_ctrl = readback.is_ctrl;

    if object_id == 0 {
        if !is_shift && !is_ctrl {
            let mut state = world.resource_mut::<crate::ecs::resource::HierarchyState>();
            hierarchy_deselect_all(&mut state);
            state.display_mode = HierarchyDisplayMode::Entities;
        }
        return;
    }

    let Some(entity) = find_entity_by_object_id(world, assets, object_id) else {
        return;
    };

    let mut state = world.resource_mut::<crate::ecs::resource::HierarchyState>();
    if is_shift || is_ctrl {
        hierarchy_toggle_selection(&mut state, entity);
    } else {
        hierarchy_select(&mut state, entity);
    }
    state.display_mode = HierarchyDisplayMode::Entities;
}
