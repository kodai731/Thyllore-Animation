use cgmath::SquareMatrix;
use thyllore_avatar_core::garment::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainGenReport, ChainGenRequest, ChainGenSettings, ChainRole,
    ChainWeightRecord, GeneratedBoneRecord, GeneratedChain, GeneratedJoint, InfluenceTarget,
    VertexInfluences,
};
use thyllore_avatar_core::garment::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshSelector,
};
use thyllore_avatar_core::garment::systems::chain_bones::{
    append_chain_bones, write_chain_weights,
};
use thyllore_avatar_core::garment::systems::chain_gen::{compute_chain_weights, place_chains};

use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, NodeAssets};
use crate::ecs::systems::avatar_setup_systems::compute_bone_global_transform;
use crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes;
use crate::ecs::systems::skeleton_edit_systems::refresh_after_skeleton_edit;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub struct GeneratedChainBones {
    pub report: ChainGenReport,
}

struct ChainGenPart {
    key: SubmeshKey,
    mesh_index: usize,
    vertex_offset: u32,
    vertex_count: u32,
}

impl ChainGenPart {
    fn contains(&self, vertex: u32) -> bool {
        vertex >= self.vertex_offset && vertex < self.vertex_offset + self.vertex_count
    }
}

pub fn generate_chain_bones(
    world: &World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    request: &ChainGenRequest,
) -> anyhow::Result<GeneratedChainBones> {
    let (submeshes, role) = resolve_target(world, assets, graphics, &request.target)?;
    let (parts, positions, triangles, skeleton_id) = merge_submesh_geometry(graphics, submeshes)?;

    let root = find_root_bone(assets, skeleton_id, &request.root_bone)?;
    let attachment_geometry =
        collect_skin_vertices_of_bone(world, assets, graphics, skeleton_id, root);
    let skeleton = find_skeleton_mut(assets, skeleton_id)?;
    let root_global = compute_bone_global_transform(skeleton, root as usize);
    let geometry_to_model = root_global * skeleton.bones[root as usize].inverse_bind_pose;
    let model_to_geometry = geometry_to_model
        .invert()
        .ok_or_else(|| anyhow::anyhow!("geometry transform of the skin is singular"))?;

    let to_model = |p: &cgmath::Vector3<f32>| (geometry_to_model * p.extend(1.0)).truncate();
    let model_positions: Vec<cgmath::Vector3<f32>> = positions.iter().map(to_model).collect();
    let attachment_positions: Vec<cgmath::Vector3<f32>> =
        attachment_geometry.iter().map(to_model).collect();
    let input = ChainGenInput {
        positions: &model_positions,
        triangles: &triangles,
        root_position: root_global.w.truncate(),
        attachment_positions: &attachment_positions,
    };
    let params = ChainGenParams {
        role,
        settings: request.settings.clone(),
    };
    let chains = place_chains(&input, &params)?;
    let influences = compute_chain_weights(&input, &params, &chains);

    let geometry_chains: Vec<GeneratedChain> = chains
        .iter()
        .map(|chain| GeneratedChain {
            joints: chain
                .joints
                .iter()
                .map(|joint| GeneratedJoint {
                    name: joint.name.clone(),
                    head: (model_to_geometry * joint.head.extend(1.0)).truncate(),
                })
                .collect(),
            tail: (model_to_geometry * chain.tail.extend(1.0)).truncate(),
            ..chain.clone()
        })
        .collect();
    let bone_ids = append_chain_bones(skeleton, root, &geometry_chains)?;

    let report = build_report(
        skeleton,
        &request.target,
        &parts,
        &chains,
        &bone_ids,
        root,
        &influences,
        &model_positions,
    );

    for part in &parts {
        let part_influences = influences_of_part(part, &influences);
        if let Some(skin) = graphics.meshes[part.mesh_index].skin_data.as_mut() {
            write_chain_weights(skin, root, &bone_ids, &part_influences);
        }
    }

    Ok(GeneratedChainBones { report })
}

fn resolve_target(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    target: &SubmeshSelector,
) -> anyhow::Result<(Vec<(SubmeshKey, usize)>, ChainRole)> {
    let node_assets = world.resource::<NodeAssets>();
    let pairs = collect_submesh_meshes(assets, graphics, &node_assets.nodes);
    let matching: Vec<(SubmeshKey, usize)> = pairs
        .iter()
        .filter(|(key, _)| target.matches(key))
        .cloned()
        .collect();
    if matching.is_empty() {
        let available: Vec<String> = pairs
            .iter()
            .map(|(k, _)| format!("{}/{}", k.mesh, k.material))
            .collect();
        anyhow::bail!(
            "submesh not found: {}; available: {}",
            target,
            available.join(", ")
        );
    }

    let state = world.resource::<AvatarSetupState>();
    let mut selected = Vec::new();
    let mut role: Option<ChainRole> = None;
    for (key, mesh_index) in matching {
        let entry = state.submesh_roles.get(&key).ok_or_else(|| {
            anyhow::anyhow!("submesh role not set: {}/{}", key.mesh, key.material)
        })?;
        let part_role = match (entry.role, target) {
            (SubmeshRole::Hair, _) => ChainRole::Hair,
            (SubmeshRole::Garment, _) => ChainRole::Skirt,
            (_, SubmeshSelector::Mesh(_)) => continue,
            (other, SubmeshSelector::Submesh(_)) => anyhow::bail!(
                "submesh role {:?} is not Hair or Garment: {}/{}",
                other,
                key.mesh,
                key.material
            ),
        };
        match role {
            Some(previous) if previous != part_role => anyhow::bail!(
                "submeshes of {} mix hair and garment roles; select one submesh",
                target
            ),
            _ => role = Some(part_role),
        }
        selected.push((key, mesh_index));
    }

    let Some(role) = role else {
        anyhow::bail!("no hair or garment submesh in {}", target);
    };
    Ok((selected, role))
}

fn dominant_bone(indices: cgmath::Vector4<u32>, weights: cgmath::Vector4<f32>) -> Option<u32> {
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

/// Geometry-space positions of the skin-role vertices the bone drives most, the surface the chains hang from.
fn collect_skin_vertices_of_bone(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    skeleton_id: crate::animation::SkeletonId,
    bone: thyllore_model_core::BoneId,
) -> Vec<cgmath::Vector3<f32>> {
    let node_assets = world.resource::<NodeAssets>();
    let state = world.resource::<AvatarSetupState>();
    let skin_meshes = collect_submesh_meshes(assets, graphics, &node_assets.nodes)
        .into_iter()
        .filter(|(key, _)| {
            state
                .submesh_roles
                .get(key)
                .is_some_and(|entry| entry.role == SubmeshRole::Skin)
        })
        .map(|(_, mesh_index)| mesh_index);

    let mut positions = Vec::new();
    for mesh_index in skin_meshes {
        let Some(skin) = graphics.meshes[mesh_index].skin_data.as_ref() else {
            continue;
        };
        if skin.skeleton_id != skeleton_id {
            continue;
        }
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

type MergedGeometry = (
    Vec<ChainGenPart>,
    Vec<cgmath::Vector3<f32>>,
    Vec<[u32; 3]>,
    crate::animation::SkeletonId,
);

fn merge_submesh_geometry(
    graphics: &GraphicsResources,
    submeshes: Vec<(SubmeshKey, usize)>,
) -> anyhow::Result<MergedGeometry> {
    let mut parts = Vec::new();
    let mut positions: Vec<cgmath::Vector3<f32>> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut skeleton_id = None;

    for (key, mesh_index) in submeshes {
        let mesh = &graphics.meshes[mesh_index];
        let skin = mesh
            .skin_data
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}/{} has no skin_data", key.mesh, key.material))?;
        match skeleton_id {
            None => skeleton_id = Some(skin.skeleton_id),
            Some(id) if id != skin.skeleton_id => {
                anyhow::bail!(
                    "{}/{} is skinned to another skeleton",
                    key.mesh,
                    key.material
                )
            }
            Some(_) => {}
        }

        let vertex_offset = positions.len() as u32;
        positions.extend_from_slice(&skin.base_positions);
        triangles.extend(mesh.vertex_data.indices.chunks_exact(3).map(|c| {
            [
                c[0] + vertex_offset,
                c[1] + vertex_offset,
                c[2] + vertex_offset,
            ]
        }));
        parts.push(ChainGenPart {
            key,
            mesh_index,
            vertex_offset,
            vertex_count: skin.base_positions.len() as u32,
        });
    }

    let Some(skeleton_id) = skeleton_id else {
        anyhow::bail!("no submesh to generate chains on");
    };
    Ok((parts, positions, triangles, skeleton_id))
}

fn influences_of_part(
    part: &ChainGenPart,
    influences: &[VertexInfluences],
) -> Vec<VertexInfluences> {
    influences
        .iter()
        .filter(|influence| part.contains(influence.vertex))
        .map(|influence| VertexInfluences {
            vertex: influence.vertex - part.vertex_offset,
            influences: influence.influences.clone(),
        })
        .collect()
}

fn find_root_bone(
    assets: &AssetStorage,
    skeleton_id: crate::animation::SkeletonId,
    root_bone: &str,
) -> anyhow::Result<thyllore_model_core::BoneId> {
    let skeleton = assets
        .skeletons
        .values()
        .find(|sa| sa.skeleton_id == skeleton_id)
        .map(|sa| &sa.skeleton)
        .ok_or_else(|| anyhow::anyhow!("skeleton not found: {:?}", skeleton_id))?;
    skeleton
        .bone_name_to_id
        .get(root_bone)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("root bone '{}' not found", root_bone))
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

fn build_report(
    skeleton: &thyllore_model_core::Skeleton,
    target: &SubmeshSelector,
    parts: &[ChainGenPart],
    chains: &[GeneratedChain],
    bone_ids: &[Vec<thyllore_model_core::BoneId>],
    root: thyllore_model_core::BoneId,
    influences: &[VertexInfluences],
    positions: &[cgmath::Vector3<f32>],
) -> ChainGenReport {
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
    for part in parts {
        let part_label = format!("{}:{}", part.key.mesh, part.key.material);
        for influence in influences_of_part(part, influences) {
            let pos = positions[(influence.vertex + part.vertex_offset) as usize];
            for (target_bone, weight) in &influence.influences {
                let bone_name = match target_bone {
                    InfluenceTarget::Root => skeleton.bones[root as usize].name.clone(),
                    InfluenceTarget::Joint { chain, joint } => {
                        let bid = bone_ids[*chain][*joint];
                        skeleton.bones[bid as usize].name.clone()
                    }
                };
                vertex_weights.push(ChainWeightRecord {
                    mesh: part_label.clone(),
                    vertex: influence.vertex,
                    bone: bone_name,
                    weight: *weight,
                    position: [pos.x, pos.y, pos.z],
                });
            }
        }
    }

    ChainGenReport {
        target: target.to_string(),
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

    refresh_after_skeleton_edit(world, assets, graphics);
    world.resource_mut::<AvatarSetupState>().chain_gen_report = Some(report);

    msg_info!("Generated {} chain bones", bone_count);
}

pub fn set_chain_gen_settings(world: &mut World, settings: ChainGenSettings) {
    world.resource_mut::<AvatarSetupState>().chain_gen = settings;
}
