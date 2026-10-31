use crate::animation::spring_bone::SpringSpace;
use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{AvatarSetupState, ModelState, NodeAssets};
use crate::ecs::systems::avatar_sidecar_systems::apply_avatar_sidecar;
use crate::ecs::systems::avatar_stats_systems::count_avatar_stats;
use crate::ecs::systems::avatar_submesh_role_systems::refresh_submesh_roles;
use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_add;
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::{Animator, Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use cgmath::Matrix4;
use std::path::Path;
use thyllore_avatar_core::humanoid::components::avatar_rig::AvatarRig;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::geometry_checks::check_mapping_geometry;
use thyllore_avatar_core::humanoid::systems::humanoid_frame::derive_humanoid_frame;
use thyllore_avatar_core::humanoid::systems::mapping_io::{
    humanoid_mapping_path, load_or_infer_rig, save_mapping,
};
use thyllore_avatar_core::humanoid::systems::name_match::{
    collect_unresolved_roles, infer_mapping,
};
use thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose;
use thyllore_avatar_core::humanoid::systems::structural_chains::find_chains_under;
use thyllore_avatar_core::humanoid::systems::validate::validate_mapping;
use thyllore_avatar_core::vrchat::rank::{rank_stats, Platform};
use thyllore_avatar_core::vrchat::rank_thresholds::default_thresholds;

pub fn animated_spring_space(world: &World) -> SpringSpace {
    world
        .iter_components::<Animator>()
        .map(|(entity, _)| spring_space_for_entity(world, entity))
        .next()
        .unwrap_or(SpringSpace::IDENTITY)
}

pub fn skeleton_to_bone_inputs(skeleton: &Skeleton) -> Vec<BoneInput> {
    skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(bone_index, bone)| {
            let global_transform = compute_bone_global_transform(skeleton, bone_index);
            BoneInput {
                name: bone.name.clone(),
                parent: bone.parent_id.map(|parent_id| parent_id as usize),
                rest_position: [
                    global_transform.w.x,
                    global_transform.w.y,
                    global_transform.w.z,
                ],
            }
        })
        .collect()
}

pub(crate) fn compute_bone_global_transform(
    skeleton: &Skeleton,
    bone_index: usize,
) -> Matrix4<f32> {
    let bone = &skeleton.bones[bone_index];
    let mut global_transform = bone.local_transform;
    let mut parent_id = bone.parent_id;
    while let Some(id) = parent_id {
        let parent = &skeleton.bones[id as usize];
        global_transform = parent.local_transform * global_transform;
        parent_id = parent.parent_id;
    }
    global_transform
}

pub(crate) fn find_first_skeleton(assets: &AssetStorage) -> Option<&Skeleton> {
    assets
        .skeletons
        .values()
        .next()
        .map(|skeleton_asset| &skeleton_asset.skeleton)
}

pub(crate) fn find_spring_bone_setup(world: &World) -> Option<&SpringBoneSetup> {
    world
        .iter_components::<SpringBoneSetup>()
        .next()
        .map(|(_, setup)| setup)
}

pub fn find_model_path(world: &World) -> Option<String> {
    world
        .get_resource::<ModelState>()
        .map(|model_state| model_state.model_path.clone())
        .filter(|model_path| !model_path.is_empty())
}

pub fn sync_avatar_setup(world: &mut World, assets: &AssetStorage, graphics: &GraphicsResources) {
    let Some(model_path) = find_model_path(world) else {
        return;
    };
    let is_synced = world
        .get_resource::<AvatarSetupState>()
        .is_none_or(|state| state.source_model_path == model_path);
    if is_synced {
        return;
    }
    let Some(skeleton) = find_first_skeleton(assets) else {
        return;
    };

    let bones = skeleton_to_bone_inputs(skeleton);
    let imported = world.resource::<ModelState>().imported_humanoid.clone();
    let (mapping, missing_bone_names) =
        load_or_infer_mapping(Path::new(&model_path), &bones, imported.as_ref());
    let stats = count_avatar_stats(graphics, Some(skeleton), find_spring_bone_setup(world));

    let node_assets = world.resource::<NodeAssets>();
    let mut state = world.resource_mut::<AvatarSetupState>();
    state.bones = bones;
    state.mapping = mapping;
    state.missing_bone_names = missing_bone_names;
    state.stats = stats;
    refresh_submesh_roles(&mut state, assets, graphics, &node_assets.nodes);
    refresh_avatar_validation(&mut state);
    state.source_model_path = model_path.clone();
    drop(state);
    drop(node_assets);

    apply_avatar_sidecar(world, assets, graphics, Path::new(&model_path));
}

pub(crate) fn load_or_infer_mapping(
    model_path: &Path,
    bones: &[BoneInput],
    imported: Option<&HumanoidMapping>,
) -> (HumanoidMapping, Vec<String>) {
    match load_or_infer_rig(model_path, bones, imported) {
        Ok((AvatarRig::Humanoid(m), missing)) => (m, missing),
        Ok((AvatarRig::Inferred(m), _)) => (m, Vec::new()),
        Ok((AvatarRig::Generic, _)) => (HumanoidMapping::default(), Vec::new()),
        Err(error) => {
            log_warn!(
                "Failed to load or infer humanoid mapping {}: {}",
                model_path.display(),
                error
            );
            let (mapping, _) = infer_mapping(bones);
            (mapping, Vec::new())
        }
    }
}

pub(crate) fn refresh_avatar_validation(state: &mut AvatarSetupState) {
    state.unresolved = collect_unresolved_roles(&state.mapping);
    state.issues = validate_mapping(&state.mapping, &state.bones);
    state.geometry_warnings = derive_humanoid_frame(&state.mapping, &state.bones)
        .map(|frame| check_mapping_geometry(&state.mapping, &state.bones, &frame))
        .unwrap_or_default();
    state.rest_pose = detect_rest_pose(&state.mapping, &state.bones);
    state.rank = Some(rank_stats(
        &state.stats,
        &default_thresholds(),
        state.platform,
    ));
}

pub fn open_avatar_setup(world: &mut World) {
    world.resource_mut::<AvatarSetupState>().is_open = true;
}

pub fn set_humanoid_role(world: &mut World, role: HumanoidRole, bone: Option<usize>) {
    let mut state = world.resource_mut::<AvatarSetupState>();
    match bone {
        Some(bone_index) if bone_index < state.bones.len() => {
            state.mapping.by_role.insert(role, bone_index);
        }
        Some(bone_index) => {
            log_warn!("Cannot map {:?} to missing bone {}", role, bone_index);
            return;
        }
        None => {
            state.mapping.by_role.remove(&role);
        }
    }
    refresh_avatar_validation(&mut state);
}

pub fn set_avatar_rank_platform(world: &mut World, platform: Platform) {
    let mut state = world.resource_mut::<AvatarSetupState>();
    state.platform = platform;
    refresh_avatar_validation(&mut state);
}

pub fn save_humanoid_mapping(world: &World) {
    let Some(model_path) = find_model_path(world) else {
        msg_error!("Cannot save humanoid mapping: no model loaded");
        return;
    };
    let state = world.resource::<AvatarSetupState>();

    if !state.issues.is_empty() {
        let issue_list: Vec<String> = state.issues.iter().map(|issue| issue.to_string()).collect();
        msg_error!("Cannot save humanoid mapping: {}", issue_list.join("; "));
        return;
    }

    let mapping_path = humanoid_mapping_path(Path::new(&model_path));
    match save_mapping(&mapping_path, &state.mapping, &state.bones) {
        Ok(()) => msg_info!("Saved humanoid mapping to {}", mapping_path.display()),
        Err(error) => msg_error!(
            "Failed to save humanoid mapping {}: {}",
            mapping_path.display(),
            error
        ),
    }
}

/// Registers every run of non-humanoid bones under `root` as a spring chain; runs that already head
/// a chain are kept as they are.
pub fn add_spring_chains_under(
    world: &mut World,
    assets: &AssetStorage,
    root: BoneId,
    min_joints: u32,
) {
    let Some(skeleton) = find_first_skeleton(assets) else {
        msg_error!("Cannot add spring chains: no skeleton loaded");
        return;
    };
    let Some(spring_entity) = find_or_create_spring_bone_setup_entity(world) else {
        msg_error!("Cannot add spring chains: no animated model loaded");
        return;
    };

    let mapping = world.resource::<AvatarSetupState>().mapping.clone();
    let chains = find_chains_under(skeleton, &mapping, root, min_joints as usize);
    let root_name = skeleton
        .get_bone(root)
        .map(|bone| bone.name.as_str())
        .unwrap_or_default();
    if chains.is_empty() {
        msg_error!(
            "No bone runs of at least {} joints under \"{}\"",
            min_joints,
            root_name
        );
        return;
    }

    let existing_roots: Vec<BoneId> = world
        .get_component::<SpringBoneSetup>(spring_entity)
        .map(|setup| {
            setup
                .chains
                .iter()
                .filter_map(|chain| chain.joints.first().map(|joint| joint.bone_id))
                .collect()
        })
        .unwrap_or_default();
    let mut added = 0;
    for chain in chains {
        let chain_root = chain.joints[0];
        if existing_roots.contains(&chain_root) {
            continue;
        }
        handle_spring_chain_add(
            world,
            spring_entity,
            chain_root,
            chain.joints.len() as u32,
            skeleton,
        );
        added += 1;
    }
    msg_info!("Added {} spring chains under \"{}\"", added, root_name);
}

pub fn find_or_create_spring_bone_setup_entity(world: &mut World) -> Option<Entity> {
    if let Some(entity) = find_spring_bone_setup_entity(world) {
        return Some(entity);
    }

    let model_entity = world
        .iter_components::<Animator>()
        .map(|(entity, _)| entity)
        .next()?;
    world.insert_component(model_entity, SpringBoneSetup::default());
    world.insert_component(model_entity, WithSpringBone);
    Some(model_entity)
}

fn find_spring_bone_setup_entity(world: &World) -> Option<Entity> {
    world
        .iter_components::<WithSpringBone>()
        .map(|(entity, _)| entity)
        .find(|&entity| world.has_component::<SpringBoneSetup>(entity))
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;
    use crate::ecs::component::{SpringChain, SpringJointParam};

    fn make_chain_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("chain");
        let root = skeleton.add_bone("Hips", None);
        let spine = skeleton.add_bone("Spine", Some(root));
        let chest = skeleton.add_bone("Chest", Some(spine));
        skeleton.bones[root as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
        skeleton.bones[spine as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.5, 0.0));
        skeleton.bones[chest as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 0.25, 0.1));
        skeleton
    }

    #[test]
    fn test_skeleton_to_bone_inputs_accumulates_parent_chain() {
        let bones = skeleton_to_bone_inputs(&make_chain_skeleton());

        assert_eq!(bones.len(), 3);
        assert_eq!(bones[0].name, "Hips");
        assert_eq!(bones[0].parent, None);
        assert_eq!(bones[1].parent, Some(0));
        assert_eq!(bones[2].parent, Some(1));
        assert_eq!(bones[0].rest_position, [0.0, 1.0, 0.0]);
        assert_eq!(bones[1].rest_position, [0.0, 1.5, 0.0]);
        assert_eq!(bones[2].rest_position, [0.0, 1.75, 0.1]);
    }

    #[test]
    fn test_spring_setup_is_created_on_the_animated_model_when_missing() {
        let mut world = World::new();
        let model = world
            .entity()
            .with_name("model")
            .with_animator(Animator::new())
            .build();

        let created = find_or_create_spring_bone_setup_entity(&mut world);
        let found_again = find_or_create_spring_bone_setup_entity(&mut world);

        assert_eq!(created, Some(model));
        assert_eq!(found_again, Some(model));
        assert!(world.has_component::<SpringBoneSetup>(model));
        assert!(world.has_component::<WithSpringBone>(model));
    }

    #[test]
    fn test_spring_setup_is_not_created_without_an_animated_model() {
        let mut world = World::new();

        assert_eq!(find_or_create_spring_bone_setup_entity(&mut world), None);
    }
}
