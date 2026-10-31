use crate::animation::spring_bone::{fit_bone_capsules, replace_fitted_body_colliders};
use crate::asset::AssetStorage;
use crate::ecs::component::{CapsuleFitParams, SpringBoneSetup};
use crate::ecs::resource::AvatarSetupState;
use crate::ecs::systems::avatar_setup_systems::{
    find_first_skeleton, find_or_create_spring_bone_setup_entity,
};
use crate::ecs::systems::avatar_submesh_role_systems::skin_role_skins;
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_anim_core::spring_bone::{fit_bone_capsule, FittedCapsule};
use thyllore_avatar_core::collider::components::capsule::HeadCapsule;
use thyllore_avatar_core::humanoid::components::body_collider_roles::body_collider_bone_indices;

pub fn fit_body_colliders(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    params: &CapsuleFitParams,
) {
    let Some(skeleton) = find_first_skeleton(assets) else {
        msg_error!("Cannot fit body colliders: no skeleton loaded");
        return;
    };

    let skin_data = skin_role_skins(world, assets, graphics, skeleton.id);
    if skin_data.is_empty() {
        msg_error!(
            "Cannot fit body colliders: no mesh with the skin role is bound to this skeleton"
        );
        return;
    }

    let fitted = fit_bone_capsules(&skin_data, skeleton, params);

    let allowed = body_collider_bone_indices(&world.resource::<AvatarSetupState>().mapping);
    let fitted: Vec<_> = fitted
        .into_iter()
        .filter(|c| allowed.contains(&(c.bone_id as usize)))
        .collect();

    if fitted.is_empty() {
        msg_error!("Cannot fit body colliders: no humanoid body bones are mapped");
        return;
    }

    let Some(spring_entity) = find_or_create_spring_bone_setup_entity(world) else {
        msg_error!("Cannot fit body colliders: no animated model loaded");
        return;
    };

    let Some(mut setup) = world.get_component_mut::<SpringBoneSetup>(spring_entity) else {
        msg_error!("Cannot fit body colliders: spring bone setup missing");
        return;
    };
    replace_fitted_body_colliders(&mut setup, &fitted);

    reset_spring_bone_state(world);

    msg_info!("Fitted {} body colliders", fitted.len());
}

pub(crate) fn dominant_bone(
    indices: cgmath::Vector4<u32>,
    weights: cgmath::Vector4<f32>,
) -> Option<u32> {
    let candidates = [
        (indices.x, weights.x),
        (indices.y, weights.y),
        (indices.z, weights.z),
        (indices.w, weights.w),
    ];
    candidates
        .into_iter()
        .filter(|(_, weight)| *weight > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(bone, _)| bone)
}

/// Collider offsets are in the bone's frame: the spring solver places them at `global[bone] * offset`.
pub(crate) fn head_capsule_collider(
    capsule: &HeadCapsule,
    root: thyllore_model_core::BoneId,
    model_to_root_local: cgmath::Matrix4<f32>,
) -> FittedCapsule {
    let to_local = |p: cgmath::Vector3<f32>| (model_to_root_local * p.extend(1.0)).truncate();
    FittedCapsule {
        bone_id: root,
        head: to_local(capsule.bottom),
        tail: to_local(capsule.top),
        radius: capsule.radius,
    }
}

pub(crate) fn fit_body_collider(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    skeleton_id: crate::animation::SkeletonId,
    root: thyllore_model_core::BoneId,
) -> Option<FittedCapsule> {
    let skins = skin_role_skins(world, assets, graphics, skeleton_id);
    let skeleton = assets
        .skeletons
        .values()
        .find(|sa| sa.skeleton_id == skeleton_id)?;
    let params = &world.resource::<AvatarSetupState>().body_collider_fit;
    fit_bone_capsule(&skins, skeleton.skeleton.get_bone(root)?, params)
}

pub(crate) fn collect_skin_vertices_of_bone(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    skeleton_id: crate::animation::SkeletonId,
    bone: thyllore_model_core::BoneId,
) -> Vec<cgmath::Vector3<f32>> {
    let mut positions = Vec::new();
    for skin in skin_role_skins(world, assets, graphics, skeleton_id) {
        positions.extend(
            skin.base_positions
                .iter()
                .zip(&skin.bone_indices)
                .zip(&skin.bone_weights)
                .filter(|((_, indices), weights)| dominant_bone(**indices, **weights) == Some(bone))
                .map(|((position, _), _)| *position),
        );
    }
    positions
}
