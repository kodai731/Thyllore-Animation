use cgmath::SquareMatrix;
use thyllore_avatar_core::garment::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainGenReport, ChainGenRequest, ChainRole, ChainWeightRecord,
    GeneratedBoneRecord, GeneratedChain, InfluenceTarget, VertexInfluences,
};
use thyllore_avatar_core::garment::components::submesh_role::{SubmeshKey, SubmeshRole};
use thyllore_avatar_core::garment::systems::chain_bones::{
    append_chain_bones, write_chain_weights,
};
use thyllore_avatar_core::garment::systems::chain_gen::{compute_chain_weights, place_chains};

use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, BoneGizmoData, FbxModelCache, NodeAssets};
use crate::ecs::systems::avatar_setup_systems::skeleton_to_bone_inputs;
use crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes;
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::world::World;
use crate::exporter::fbx_exporter::clusters_from_skin_data;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub struct GeneratedChainBones {
    pub mesh_index: usize,
    pub report: ChainGenReport,
}

pub fn generate_chain_bones(
    world: &World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    request: &ChainGenRequest,
) -> anyhow::Result<GeneratedChainBones> {
    let (mesh_index, role) = resolve_submesh(world, assets, graphics, &request.submesh)?;

    let mesh = &graphics.meshes[mesh_index];
    let skin = mesh
        .skin_data
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("mesh has no skin_data"))?;
    let skeleton_id = skin.skeleton_id;

    let positions = skin.base_positions.clone();
    let triangles: Vec<[u32; 3]> = mesh
        .vertex_data
        .indices
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let skeleton = find_skeleton_mut(assets, skeleton_id)?;
    let root = *skeleton
        .bone_name_to_id
        .get(&request.root_bone)
        .ok_or_else(|| anyhow::anyhow!("root bone '{}' not found", request.root_bone))?;
    let root_position = bind_position(skeleton, root)?;

    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position,
        up: cgmath::Vector3::new(0.0, 1.0, 0.0),
    };
    let params = ChainGenParams {
        role,
        joint_count: request.joint_count,
        column_count: request.column_count,
        root_blend_width: None,
    };
    let chains = place_chains(&input, &params)?;
    let influences = compute_chain_weights(&input, &params, &chains);
    let bone_ids = append_chain_bones(skeleton, root, &chains)?;

    let report = build_report(
        skeleton,
        &request.submesh,
        &chains,
        &bone_ids,
        root,
        &influences,
        &positions,
    );

    if let Some(skin) = graphics.meshes[mesh_index].skin_data.as_mut() {
        write_chain_weights(skin, root, &bone_ids, &influences);
    }

    Ok(GeneratedChainBones { mesh_index, report })
}

fn resolve_submesh(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    submesh: &SubmeshKey,
) -> anyhow::Result<(usize, ChainRole)> {
    let node_assets = world.resource::<NodeAssets>();
    let pairs = collect_submesh_meshes(assets, graphics, &node_assets.nodes);
    let available: Vec<String> = pairs
        .iter()
        .map(|(k, _)| format!("{}/{}", k.mesh, k.material))
        .collect();
    let mesh_index = pairs
        .iter()
        .find(|(key, _)| key == submesh)
        .map(|(_, i)| *i)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "submesh not found: {}/{}; available: {}",
                submesh.mesh,
                submesh.material,
                available.join(", ")
            )
        })?;

    let state = world.resource::<AvatarSetupState>();
    let entry = state.submesh_roles.get(submesh).ok_or_else(|| {
        anyhow::anyhow!(
            "submesh role not set: {}:{}",
            submesh.mesh,
            submesh.material
        )
    })?;

    let role = match entry.role {
        SubmeshRole::Hair => ChainRole::Hair,
        SubmeshRole::Garment => ChainRole::Skirt,
        _ => anyhow::bail!(
            "submesh role {:?} is not Hair or Garment: {}:{}",
            entry.role,
            submesh.mesh,
            submesh.material
        ),
    };

    Ok((mesh_index, role))
}

fn find_skeleton_mut(
    assets: &mut AssetStorage,
    skeleton_id: crate::animation::SkeletonId,
) -> anyhow::Result<&mut thyllore_model_core::Skeleton> {
    assets
        .skeletons
        .values_mut()
        .find(|sa| sa.skeleton_id == skeleton_id)
        .map(|sa| &mut sa.skeleton)
        .ok_or_else(|| anyhow::anyhow!("skeleton not found: {:?}", skeleton_id))
}

fn bind_position(
    skeleton: &thyllore_model_core::Skeleton,
    bone: thyllore_model_core::BoneId,
) -> anyhow::Result<cgmath::Vector3<f32>> {
    let bone_data = skeleton
        .get_bone(bone)
        .ok_or_else(|| anyhow::anyhow!("bone not found: {:?}", bone))?;
    let root_matrix = bone_data
        .inverse_bind_pose
        .invert()
        .ok_or_else(|| anyhow::anyhow!("inverse_bind_pose is singular"))?;
    Ok(cgmath::Vector3::new(
        root_matrix.w.x,
        root_matrix.w.y,
        root_matrix.w.z,
    ))
}

fn build_report(
    skeleton: &thyllore_model_core::Skeleton,
    submesh: &SubmeshKey,
    chains: &[GeneratedChain],
    bone_ids: &[Vec<thyllore_model_core::BoneId>],
    root: thyllore_model_core::BoneId,
    influences: &[VertexInfluences],
    positions: &[cgmath::Vector3<f32>],
) -> ChainGenReport {
    let submesh_str = format!("{}:{}", submesh.mesh, submesh.material);

    let bones: Vec<GeneratedBoneRecord> = chains
        .iter()
        .enumerate()
        .flat_map(|(ci, chain)| {
            chain.joints.iter().enumerate().map(move |(ji, joint)| {
                let parent_id = if ji == 0 { root } else { bone_ids[ci][ji - 1] };
                let parent_name = &skeleton.bones[parent_id as usize].name;
                GeneratedBoneRecord {
                    name: joint.name.clone(),
                    parent: parent_name.clone(),
                    head: [joint.head.x, joint.head.y, joint.head.z],
                }
            })
        })
        .collect();

    let mut vertex_weights: Vec<ChainWeightRecord> = Vec::new();
    for influence in influences {
        for (target, weight) in &influence.influences {
            let bone_name = match target {
                InfluenceTarget::Root => skeleton.bones[root as usize].name.clone(),
                InfluenceTarget::Joint { chain, joint } => {
                    let bid = bone_ids[*chain][*joint];
                    skeleton.bones[bid as usize].name.clone()
                }
            };
            let pos = positions[influence.vertex as usize];
            vertex_weights.push(ChainWeightRecord {
                mesh: submesh_str.clone(),
                vertex: influence.vertex,
                bone: bone_name,
                weight: *weight,
                position: [pos.x, pos.y, pos.z],
            });
        }
    }

    ChainGenReport {
        submesh: submesh_str,
        bones,
        vertex_weights,
    }
}

pub fn apply_chain_generation(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    generated: GeneratedChainBones,
) {
    let report = generated.report;
    let bone_count = report.bones.len();

    sync_fbx_clusters(world, assets, graphics, generated.mesh_index);

    reset_spring_bone_state(world);

    let skeleton_id = world.resource::<BoneGizmoData>().cached_skeleton_id;

    if let Some(skeleton_id) = skeleton_id {
        if let Some(skeleton) = assets.get_skeleton_by_skeleton_id(skeleton_id) {
            let rest_globals = crate::ecs::compute_pose_global_transforms(
                skeleton,
                &crate::ecs::create_pose_from_rest(skeleton),
            );
            if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                bone_gizmo.bone_local_offsets =
                    crate::ecs::systems::bone_gizmo_systems::compute_bone_local_offsets(
                        skeleton,
                        &rest_globals,
                    );
                bone_gizmo.cached_global_transforms = rest_globals;
            }
        }
    }

    {
        let mut state = world.resource_mut::<AvatarSetupState>();
        if let Some(skeleton_id) = skeleton_id {
            if let Some(skeleton_asset) = assets.get_skeleton_by_skeleton_id(skeleton_id) {
                state.bones = skeleton_to_bone_inputs(skeleton_asset);
            }
        }
        state.chain_gen_report = Some(report);
    }

    msg_info!("Generated {} chain bones", bone_count);
}

fn sync_fbx_clusters(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    mesh_index: usize,
) {
    let skin = match graphics.meshes[mesh_index].skin_data.as_ref() {
        Some(s) => s,
        None => return,
    };
    let skeleton_id = skin.skeleton_id;
    let skeleton = match assets.get_skeleton_by_skeleton_id(skeleton_id) {
        Some(s) => s,
        None => return,
    };

    let clusters = clusters_from_skin_data(skin, skeleton);

    let mut fbx_cache = world.resource_mut::<FbxModelCache>();
    if let Some(fbx_model) = fbx_cache.fbx_model_mut() {
        if fbx_model.fbx_data.len() <= mesh_index {
            msg_warn!(
                "fbx_data has {} entries, mesh_index is {}",
                fbx_model.fbx_data.len(),
                mesh_index
            );
            return;
        }
        fbx_model.fbx_data[mesh_index].clusters = clusters;
    }
}
