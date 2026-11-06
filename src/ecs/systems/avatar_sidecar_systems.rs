use crate::animation::Skeleton;
use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, ExpressionLibraryState, MaterialTextureState};
use crate::ecs::systems::avatar_hidden_mask_systems::restore_hidden_triangles;
use crate::ecs::systems::avatar_setup_systems::{
    animated_spring_space, find_first_skeleton, find_model_path,
    find_or_create_spring_bone_setup_entity, find_spring_bone_setup,
};
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::world::{Entity, World};
use crate::ecs::{find_mesh_morph, MeshRef};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use std::collections::BTreeMap;
use std::path::Path;
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshKey;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar::{
    build_sidecar, count_viseme_channels, read_sidecar_json, sidecar_path, write_sidecar_json,
    AvatarSidecar, SidecarInput,
};
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, rest_frames_from_globals, spring_setup_from_sidecar, BoneRestFrame,
    SidecarSpring,
};
use thyllore_model_core::MeshMorph;

/// Each bone's rest frame in the sidecar's model space, taken at the model's current scale.
pub(crate) fn current_rest_frames(
    world: &World,
    skeleton: &Skeleton,
    bones: &[BoneInput],
) -> anyhow::Result<Vec<BoneRestFrame>> {
    let rest_globals = crate::ecs::compute_pose_global_transforms(
        skeleton,
        &crate::ecs::create_pose_from_rest(skeleton),
    );
    rest_frames_from_globals(
        &rest_globals,
        bones,
        animated_spring_space(world).world_scale(),
    )
}

/// Restore the setup an earlier export wrote beside the model: submesh roles, the hidden skin
/// triangles and spring bones.
pub(crate) fn apply_avatar_sidecar(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    model_path: &Path,
) {
    let path = sidecar_path(model_path);
    if !path.exists() {
        return;
    }
    let sidecar = match read_sidecar_json(&path) {
        Ok(sidecar) => sidecar,
        Err(error) => {
            msg_warn!("Ignoring avatar sidecar {}: {}", path.display(), error);
            return;
        }
    };

    {
        let mut state = world.resource_mut::<AvatarSetupState>();
        for role in &sidecar.submesh_roles {
            let key = SubmeshKey {
                mesh: role.mesh.clone(),
                material: role.material.clone(),
            };
            if let Some(assigned) = state.submesh_roles.get_mut(&key) {
                *assigned = role.role;
            }
        }
    }
    restore_hidden_triangles(world, assets, graphics, &sidecar.hidden_triangles);

    let Some(skeleton) = find_first_skeleton(assets) else {
        return;
    };
    let bones = world.resource::<AvatarSetupState>().bones.clone();
    let frames = match current_rest_frames(world, skeleton, &bones) {
        Ok(frames) => frames,
        Err(error) => {
            msg_error!(
                "Avatar sidecar {}: spring bones not restored: {error:#}",
                path.display()
            );
            return;
        }
    };
    let spring_setup = spring_setup_from_sidecar(&sidecar.spring, &bones, &frames);

    let chain_count = spring_setup.chains.len();
    let collider_count = spring_setup.colliders.len();
    if chain_count > 0 || collider_count > 0 {
        let Some(spring_entity) = find_or_create_spring_bone_setup_entity(world) else {
            msg_warn!("Avatar sidecar has spring bones but no animated model entity");
            return;
        };
        world.insert_component(spring_entity, spring_setup);
        reset_spring_bone_state(world);
    }
    msg_info!(
        "Applied avatar sidecar {}: {} submesh roles, {} spring chains, {} colliders",
        path.display(),
        sidecar.submesh_roles.len(),
        chain_count,
        collider_count
    );
}

pub fn export_avatar_sidecar(world: &World, assets: &AssetStorage, graphics: &GraphicsResources) {
    let Some(model_path) = find_model_path(world) else {
        msg_error!("Cannot export avatar sidecar: no model loaded");
        return;
    };
    export_avatar_sidecar_beside(world, assets, graphics, Path::new(&model_path));
}

/// Write the sidecar next to `model_path`, the FBX the setup belongs to.
pub fn export_avatar_sidecar_beside(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    model_path: &Path,
) {
    let path = sidecar_path(model_path);
    let sidecar = match build_avatar_sidecar(world, assets, graphics) {
        Ok(sidecar) => sidecar,
        Err(error) => {
            msg_error!(
                "Failed to export avatar sidecar {}: {error:#}",
                path.display()
            );
            return;
        }
    };
    match write_sidecar_json(&path, &sidecar) {
        Ok(()) => msg_info!("Exported avatar sidecar to {}", path.display()),
        Err(error) => msg_error!(
            "Failed to export avatar sidecar {}: {}",
            path.display(),
            error
        ),
    }
}

fn build_avatar_sidecar(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> anyhow::Result<AvatarSidecar> {
    let state = world.resource::<AvatarSetupState>();
    let library_state = world.resource::<ExpressionLibraryState>();

    let expression_morph = find_expression_morph(world, assets, graphics);
    let channel_names: Vec<String> = expression_morph
        .map(|morph| {
            morph
                .channels
                .iter()
                .map(|channel| channel.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let spring = match (find_spring_bone_setup(world), find_first_skeleton(assets)) {
        (Some(setup), Some(skeleton)) => {
            let frames = current_rest_frames(world, skeleton, &state.bones)?;
            build_sidecar_spring(setup, &state.bones, &frames)
        }
        _ => SidecarSpring::default(),
    };
    let materials: BTreeMap<String, String> = world
        .resource::<MaterialTextureState>()
        .slots
        .iter()
        .filter_map(|slot| {
            slot.texture
                .as_ref()
                .map(|texture| (slot.material.clone(), texture.clone()))
        })
        .collect();

    Ok(build_sidecar(SidecarInput {
        mapping: &state.mapping,
        bones: &state.bones,
        channel_names: &channel_names,
        expression_mesh_name: expression_morph.map(|morph| morph.source_mesh.as_str()),
        library: &library_state.library,
        spring: &spring,
        materials: &materials,
        stats: &state.stats,
        submesh_roles: &state.submesh_roles,
        hidden_triangles: &state.hidden_triangles,
    }))
}

pub(crate) fn find_expression_morph_entity(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Option<Entity> {
    world
        .iter_components::<MeshRef>()
        .filter_map(|(entity, _)| {
            find_mesh_morph(world, entity, assets, graphics).map(|morph| (entity, morph))
        })
        .filter(|(_, morph)| !morph.channels.is_empty())
        .max_by_key(|(_, morph)| {
            let channel_names = morph.channel_names();
            (count_viseme_channels(&channel_names), channel_names.len())
        })
        .map(|(entity, _)| entity)
}

pub(crate) fn find_expression_morph<'a>(
    world: &World,
    assets: &AssetStorage,
    graphics: &'a GraphicsResources,
) -> Option<&'a MeshMorph> {
    let entity = find_expression_morph_entity(world, assets, graphics)?;
    find_mesh_morph(world, entity, assets, graphics)
}
