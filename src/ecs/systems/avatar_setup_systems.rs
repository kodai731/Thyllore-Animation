use std::collections::HashSet;
use std::path::Path;

use cgmath::Matrix4;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::naming::HumanoidNamingRules;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::infer::{collect_unresolved_roles, infer_mapping};
use thyllore_avatar_core::humanoid::systems::mapping_io::{
    humanoid_mapping_path, load_mapping, save_mapping,
};
use thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose;
use thyllore_avatar_core::humanoid::systems::spring_prefix::find_prefix_chain_roots;
use thyllore_avatar_core::humanoid::systems::validate::validate_mapping;
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::rank::{rank_stats, Platform};
use thyllore_avatar_core::vrchat::rank_thresholds::default_thresholds;
use thyllore_avatar_core::vrchat::sidecar::{
    build_sidecar, sidecar_path, write_sidecar_json, AvatarSidecar, SidecarInput,
    SidecarSpringChain,
};
use thyllore_model_core::MeshMorph;

use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{AvatarSetupState, ExpressionLibraryState, ModelState};
use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_add;
use crate::ecs::world::{Entity, World};
use crate::ecs::{find_mesh_morph, MeshRef};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

fn skeleton_to_bone_inputs(skeleton: &Skeleton) -> Vec<BoneInput> {
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

fn compute_bone_global_transform(skeleton: &Skeleton, bone_index: usize) -> Matrix4<f32> {
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

fn find_first_skeleton(assets: &AssetStorage) -> Option<&Skeleton> {
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

pub(crate) fn find_model_path(world: &World) -> Option<String> {
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
    let (mapping, missing_bone_names) = load_or_infer_mapping(Path::new(&model_path), &bones);
    let stats = count_avatar_stats(graphics, Some(skeleton), find_spring_bone_setup(world));

    let mut state = world.resource_mut::<AvatarSetupState>();
    state.bones = bones;
    state.mapping = mapping;
    state.missing_bone_names = missing_bone_names;
    state.stats = stats;
    refresh_avatar_validation(&mut state);
    state.source_model_path = model_path;
}

fn load_or_infer_mapping(model_path: &Path, bones: &[BoneInput]) -> (HumanoidMapping, Vec<String>) {
    let mapping_path = humanoid_mapping_path(model_path);
    if mapping_path.exists() {
        match load_mapping(&mapping_path, bones) {
            Ok(loaded) => return loaded,
            Err(error) => log_warn!(
                "Failed to load humanoid mapping {}: {}",
                mapping_path.display(),
                error
            ),
        }
    }

    let (mapping, _) = infer_mapping(bones, &HumanoidNamingRules::default());
    (mapping, Vec::new())
}

fn refresh_avatar_validation(state: &mut AvatarSetupState) {
    state.unresolved = collect_unresolved_roles(&state.mapping);
    state.issues = validate_mapping(&state.mapping, &state.bones);
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
        log_warn!("Cannot save humanoid mapping: no model loaded");
        return;
    };
    let state = world.resource::<AvatarSetupState>();

    let mapping_path = humanoid_mapping_path(Path::new(&model_path));
    match save_mapping(&mapping_path, &state.mapping, &state.bones) {
        Ok(()) => log!("Saved humanoid mapping to {}", mapping_path.display()),
        Err(error) => log_warn!(
            "Failed to save humanoid mapping {}: {}",
            mapping_path.display(),
            error
        ),
    }
}

pub fn add_spring_chains_by_prefix(world: &mut World, assets: &AssetStorage, prefix: &str) {
    let Some(skeleton) = find_first_skeleton(assets) else {
        log_warn!("Cannot add spring chains: no skeleton loaded");
        return;
    };
    let Some(spring_entity) = find_spring_bone_setup_entity(world) else {
        log_warn!("Cannot add spring chains: no SpringBoneSetup entity found");
        return;
    };

    let bones = skeleton_to_bone_inputs(skeleton);
    let roots = find_prefix_chain_roots(&bones, prefix);
    if roots.is_empty() {
        log_warn!("No spring chain roots found for prefix \"{}\"", prefix);
        return;
    }

    for chain in roots {
        handle_spring_chain_add(
            world,
            spring_entity,
            chain.root as BoneId,
            chain.length,
            skeleton,
        );
    }
}

fn find_spring_bone_setup_entity(world: &World) -> Option<Entity> {
    world
        .iter_components::<WithSpringBone>()
        .map(|(entity, _)| entity)
        .find(|&entity| world.has_component::<SpringBoneSetup>(entity))
}

pub fn export_avatar_sidecar(world: &World, assets: &AssetStorage, graphics: &GraphicsResources) {
    let Some(model_path) = find_model_path(world) else {
        log_warn!("Cannot export avatar sidecar: no model loaded");
        return;
    };

    let sidecar = build_avatar_sidecar(world, assets, graphics);
    let path = sidecar_path(Path::new(&model_path));
    match write_sidecar_json(&path, &sidecar) {
        Ok(()) => log!("Exported avatar sidecar to {}", path.display()),
        Err(error) => log_warn!(
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
) -> AvatarSidecar {
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
    let spring_chains = find_spring_bone_setup(world)
        .map(|setup| collect_sidecar_spring_chains(setup, &state.bones))
        .unwrap_or_default();

    build_sidecar(SidecarInput {
        mapping: &state.mapping,
        bones: &state.bones,
        channel_names: &channel_names,
        expression_mesh_name: expression_morph.map(|morph| morph.source_mesh.as_str()),
        library: &library_state.library,
        spring_chains: &spring_chains,
        stats: &state.stats,
    })
}

pub(crate) fn find_expression_morph<'a>(
    world: &World,
    assets: &AssetStorage,
    graphics: &'a GraphicsResources,
) -> Option<&'a MeshMorph> {
    world
        .iter_components::<MeshRef>()
        .filter_map(|(entity, _)| find_mesh_morph(world, entity, assets, graphics))
        .find(|morph| !morph.channels.is_empty())
}

fn collect_sidecar_spring_chains(
    setup: &SpringBoneSetup,
    bones: &[BoneInput],
) -> Vec<SidecarSpringChain> {
    setup
        .chains
        .iter()
        .filter_map(|chain| {
            let first_joint = chain.joints.first()?;
            let root = bones.get(first_joint.bone_id as usize)?;
            Some(SidecarSpringChain {
                root: root.name.clone(),
                stiffness: first_joint.stiffness,
                gravity: first_joint.gravity_power,
                drag: first_joint.drag_force,
                colliders: Vec::new(),
            })
        })
        .collect()
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
