use crate::animation::spring_bone::{chains_skinned_to, SpringChainId, SpringSpace};
use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::world::{Animator, Entity, Parent, Transform, World};
use crate::ecs::MeshRef;
use crate::hooks::model_load::LoadedModel;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

/// The uniform scale the entity's ancestry applies to its skeleton; a mirrored axis counts by its magnitude.
pub fn spring_space_for_entity(world: &World, entity: Entity) -> SpringSpace {
    let mut world_scale = 1.0;
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(transform) = world.get_component::<Transform>(e) {
            let scale = transform.scale;
            world_scale *= (scale.x.abs() + scale.y.abs() + scale.z.abs()) / 3.0;
        }
        current = world.get_component::<Parent>(e).map(|p| p.0);
    }
    SpringSpace::from_world_scale(world_scale).unwrap_or(SpringSpace::IDENTITY)
}

/// The chains whose joints skin the mesh of `entity`; empty for an entity without a skinned mesh.
pub fn chains_skinned_to_entity(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    setup: &SpringBoneSetup,
    entity: Entity,
) -> Vec<SpringChainId> {
    let Some(mesh_ref) = world.get_component::<MeshRef>(entity) else {
        return Vec::new();
    };
    let Some(mesh_index) = assets
        .get_mesh(mesh_ref.mesh_asset_id)
        .map(|mesh| mesh.graphics_mesh_index)
    else {
        return Vec::new();
    };
    match graphics
        .meshes
        .get(mesh_index)
        .and_then(|mesh| mesh.skin_data.as_ref())
    {
        Some(skin) => chains_skinned_to(setup, skin),
        None => Vec::new(),
    }
}

pub fn spring_bone_attach_loaded(world: &mut World, _assets: &AssetStorage, loaded: &LoadedModel) {
    let Some(setup) = loaded.load_result.spring_bone_setup.as_ref() else {
        return;
    };
    if !world.has_component::<Animator>(loaded.entity) {
        return;
    }

    world.insert_component(loaded.entity, setup.clone());
    world.insert_component(loaded.entity, WithSpringBone);

    log!(
        "Applied VRMC spring bones to entity {}: {} chains, {} colliders",
        loaded.entity,
        setup.chains.len(),
        setup.colliders.len()
    );
}

crate::model_load_hook!("spring_bone", Rig, spring_bone_attach_loaded);
