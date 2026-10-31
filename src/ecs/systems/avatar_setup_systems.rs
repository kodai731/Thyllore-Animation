use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use cgmath::Matrix4;
use thyllore_avatar_core::garment::components::submesh_role::SubmeshKey;
use thyllore_avatar_core::humanoid::components::avatar_rig::AvatarRig;
use thyllore_avatar_core::humanoid::components::body_collider_roles::body_collider_bone_indices;
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
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::rank::{rank_stats, Platform};
use thyllore_avatar_core::vrchat::rank_thresholds::default_thresholds;
use thyllore_avatar_core::vrchat::sidecar::{
    build_sidecar, count_viseme_channels, read_sidecar_json, sidecar_path, write_sidecar_json,
    AvatarSidecar, SidecarInput,
};
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, rest_frames_from_globals, spring_setup_from_sidecar, BoneRestFrame,
    SidecarSpring,
};
use thyllore_model_core::MeshMorph;

use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::component::{CapsuleFitParams, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{
    AvatarSetupState, ExpressionLibraryState, MaterialTextureState, ModelState, NodeAssets,
};
use crate::ecs::systems::avatar_hidden_mask_systems::restore_hidden_triangles;
use crate::ecs::systems::avatar_submesh_role_systems::{refresh_submesh_roles, skin_role_skins};
use crate::ecs::systems::spring_bone_edit_systems::{
    handle_spring_chain_add, reset_spring_bone_state,
};
use crate::ecs::world::{Animator, Entity, World};
use crate::ecs::{find_mesh_morph, MeshRef};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use crate::animation::spring_bone::{
    fit_bone_capsules, replace_fitted_body_colliders, SpringSpace,
};
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;

/// Each bone's rest frame in the sidecar's model space, taken at the model's current scale.
fn current_rest_frames(
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

fn count_avatar_stats(
    graphics: &GraphicsResources,
    skeleton: Option<&Skeleton>,
    spring_setup: Option<&SpringBoneSetup>,
) -> AvatarStats {
    let mut stats = count_mesh_stats(graphics);
    stats.bones = skeleton.map_or(0, |skeleton| skeleton.bones.len() as u32);
    if let Some(setup) = spring_setup {
        stats.spring_chains = setup.chains.len() as u32;
        stats.spring_transforms = setup
            .chains
            .iter()
            .map(|chain| chain.joints.len() as u32)
            .sum();
        stats.spring_colliders = setup.colliders.len() as u32;
    }
    stats
}

fn count_mesh_stats(graphics: &GraphicsResources) -> AvatarStats {
    let materials: HashSet<_> = graphics.mesh_material_ids.iter().collect();
    let morph_meshes: HashSet<&str> = graphics
        .meshes
        .iter()
        .map(|mesh| mesh.morph.source_mesh.as_str())
        .filter(|source_mesh| !source_mesh.is_empty())
        .collect();

    AvatarStats {
        triangles: graphics
            .meshes
            .iter()
            .map(|mesh| (mesh.vertex_data.indices.len() / 3) as u32)
            .sum(),
        meshes: graphics.meshes.len() as u32,
        skinned_meshes: graphics
            .meshes
            .iter()
            .filter(|mesh| mesh.skin_data.is_some())
            .count() as u32,
        materials: materials.len() as u32,
        morph_meshes: morph_meshes.len() as u32,
        texture_bytes: 0,
        ..Default::default()
    }
}

pub(crate) fn find_first_skeleton(assets: &AssetStorage) -> Option<&Skeleton> {
    assets
        .skeletons
        .values()
        .next()
        .map(|skeleton_asset| &skeleton_asset.skeleton)
}

fn find_spring_bone_setup(world: &World) -> Option<&SpringBoneSetup> {
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

/// Restore the setup an earlier export wrote beside the model: submesh roles, the hidden skin
/// triangles and spring bones.
fn apply_avatar_sidecar(
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

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;
    use crate::animation::SkinData;
    use crate::ecs::component::{SpringChain, SpringJointParam};
    use thyllore_vulkan_core::resource::mesh_buffer::MeshBuffer;

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

    fn make_mesh(triangle_count: usize, source_mesh: &str, skin: Option<SkinData>) -> MeshBuffer {
        let mut mesh = MeshBuffer::default();
        mesh.vertex_data.indices = vec![0; triangle_count * 3];
        mesh.morph = MeshMorph {
            source_mesh: source_mesh.to_string(),
            channels: Vec::new(),
        };
        mesh.skin_data = skin;
        mesh
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

    #[test]
    fn test_count_avatar_stats() {
        let graphics = GraphicsResources {
            meshes: vec![
                make_mesh(10, "Face", Some(SkinData::default())),
                make_mesh(4, "Face", None),
                make_mesh(2, "", Some(SkinData::default())),
            ],
            mesh_material_ids: vec![0, 1, 1],
            ..Default::default()
        };
        let setup = SpringBoneSetup {
            chains: vec![SpringChain {
                joints: vec![SpringJointParam::default(), SpringJointParam::default()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let skeleton = make_chain_skeleton();

        let stats = count_avatar_stats(&graphics, Some(&skeleton), Some(&setup));

        assert_eq!(stats.triangles, 16);
        assert_eq!(stats.meshes, 3);
        assert_eq!(stats.skinned_meshes, 2);
        assert_eq!(stats.materials, 2);
        assert_eq!(stats.morph_meshes, 1);
        assert_eq!(stats.bones, 3);
        assert_eq!(stats.spring_chains, 1);
        assert_eq!(stats.spring_transforms, 2);
        assert_eq!(stats.spring_colliders, 0);
        assert_eq!(stats.texture_bytes, 0);
    }

    #[test]
    fn test_count_avatar_stats_without_skeleton_or_springs() {
        let graphics = GraphicsResources::default();

        let stats = count_avatar_stats(&graphics, None, None);

        assert_eq!(stats.triangles, 0);
        assert_eq!(stats.bones, 0);
        assert_eq!(stats.spring_chains, 0);
    }
}
