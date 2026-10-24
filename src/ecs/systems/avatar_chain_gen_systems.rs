use cgmath::{InnerSpace, SquareMatrix};
use thyllore_anim_core::spring_bone::{
    ensure_fitted_collider, fit_bone_capsule, ChainOrigin, FittedCapsule, SpringBoneSetup,
    SpringChain, SpringJointParam, FITTED_BODY_GROUP_NAME,
};
use thyllore_avatar_core::garment::components::chain_gen::{
    chain_name, chain_role_of_submesh, ChainGenInput, ChainGenParams, ChainGenReport,
    ChainGenRequest, ChainGenSettings, ChainRole, ChainWeightRecord, GeneratedBoneRecord,
    GeneratedChain, GeneratedJoint, HeadCapsuleRecord, InfluenceTarget, RootBandRecord,
    StrandPlacement, VertexInfluences,
};
use thyllore_avatar_core::garment::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshSelector,
};
use thyllore_avatar_core::garment::systems::chain_bones::{
    append_chain_bones, write_chain_weights,
};
use thyllore_avatar_core::garment::systems::chain_gen::{
    component_labels, compute_chain_weights, hair_root_band, place_chains_with_strands, RootBand,
};
use thyllore_avatar_core::garment::systems::head_capsule::{hair_head_capsule, HeadCapsule};
use thyllore_avatar_core::garment::systems::joint_radius::joint_half_thicknesses;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_math_core::world_y_axis;

use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, NodeAssets};
use crate::ecs::systems::avatar_setup_systems::compute_bone_global_transform;
use crate::ecs::systems::avatar_setup_systems::find_or_create_spring_bone_setup_entity;
use crate::ecs::systems::avatar_submesh_role_systems::collect_submesh_meshes;
use crate::ecs::systems::skeleton_edit_systems::{
    capture_skeleton_snapshot, record_skeleton_edit, refresh_after_skeleton_edit,
    remove_bone_subtree,
};
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub struct GeneratedChainBones {
    pub report: ChainGenReport,
    pub chains: Vec<SpringChain>,
    pub root_collider: Option<FittedCapsule>,
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

fn generate_chain_bones(
    world: &World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    request: &ChainGenRequest,
) -> anyhow::Result<GeneratedChainBones> {
    let (submeshes, role) = resolve_target(world, assets, graphics, &request.target)?;
    let geometry = merge_submesh_geometry(graphics, submeshes)?;

    let root = find_root_bone(world, request.root)?;
    let attachment_geometry =
        collect_skin_vertices_of_bone(world, assets, graphics, geometry.skeleton_id, root);
    let body_collider = fit_body_collider(world, assets, graphics, geometry.skeleton_id, root);
    let skeleton = find_skeleton_mut(assets, geometry.skeleton_id)?;
    let root_global = compute_bone_global_transform(skeleton, root as usize);
    let geometry_to_model = root_global * skeleton.bones[root as usize].inverse_bind_pose;
    let model_to_geometry = geometry_to_model
        .invert()
        .ok_or_else(|| anyhow::anyhow!("geometry transform of the skin is singular"))?;

    let to_model = |p: &cgmath::Vector3<f32>| (geometry_to_model * p.extend(1.0)).truncate();
    let model_positions: Vec<cgmath::Vector3<f32>> =
        geometry.positions.iter().map(to_model).collect();
    let attachment_positions: Vec<cgmath::Vector3<f32>> =
        attachment_geometry.iter().map(to_model).collect();
    let input = ChainGenInput {
        positions: &model_positions,
        triangles: &geometry.triangles,
        root_position: root_global.w.truncate(),
        attachment_positions: &attachment_positions,
        tex_coords: &geometry.tex_coords,
    };
    let params = ChainGenParams {
        role,
        target: request.target.clone(),
        settings: request.settings.clone(),
    };
    let model_to_root_local = root_global
        .invert()
        .ok_or_else(|| anyhow::anyhow!("global transform of the root bone is singular"))?;
    let head_capsule = hair_head_capsule(&input, &params);
    let root_collider = match head_capsule {
        Some(capsule) => Some(head_capsule_collider(&capsule, root, model_to_root_local)),
        None => body_collider,
    };
    let placement = place_chains_with_strands(&input, &params)?;
    let chains = placement.chains;
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
        ReportSource {
            parts: &geometry.parts,
            chains: &chains,
            bone_ids: &bone_ids,
            root,
            influences: &influences,
            input: &input,
            strands: placement.strands,
            root_band: hair_root_band(&input, &params),
            head_capsule,
            component_labels: &component_labels(&input),
        },
    );

    for part in &geometry.parts {
        let part_influences = influences_of_part(part, &influences);
        if let Some(skin) = graphics.meshes[part.mesh_index].skin_data.as_mut() {
            write_chain_weights(skin, root, &bone_ids, &part_influences);
        }
    }

    let half_thicknesses = joint_half_thicknesses(&model_positions, &chains, &influences);
    let spring_chains = chains
        .iter()
        .zip(&bone_ids)
        .zip(&half_thicknesses)
        .map(|((chain, joint_bones), radii)| SpringChain {
            name: chain_name(&params, chain.column),
            joints: joint_bones
                .iter()
                .zip(radii)
                .map(|(&bone_id, &hit_radius)| SpringJointParam {
                    bone_id,
                    hit_radius,
                    ..SpringJointParam::default()
                })
                .collect(),
            origin: ChainOrigin::Generated {
                target: request.target.clone(),
                role,
            },
            ..SpringChain::default()
        })
        .collect();

    Ok(GeneratedChainBones {
        report,
        chains: spring_chains,
        root_collider,
    })
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
        let part_role = match (chain_role_of_submesh(entry.role), target) {
            (Some(chain_role), _) => chain_role,
            (None, SubmeshSelector::Mesh(_)) => continue,
            (None, SubmeshSelector::Submesh(_)) => anyhow::bail!(
                "submesh role {:?} is not Hair or Garment: {}/{}",
                entry.role,
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
fn skin_role_skins<'a>(
    world: &World,
    assets: &AssetStorage,
    graphics: &'a GraphicsResources,
    skeleton_id: crate::animation::SkeletonId,
) -> Vec<&'a thyllore_model_core::SkinData> {
    let node_assets = world.resource::<NodeAssets>();
    let state = world.resource::<AvatarSetupState>();
    let mut skin_meshes: Vec<usize> = collect_submesh_meshes(assets, graphics, &node_assets.nodes)
        .into_iter()
        .filter(|(key, _)| {
            state
                .submesh_roles
                .get(key)
                .is_some_and(|entry| entry.role == SubmeshRole::Skin)
        })
        .map(|(_, mesh_index)| mesh_index)
        .collect();
    skin_meshes.sort_unstable();
    skin_meshes.dedup();

    skin_meshes
        .into_iter()
        .filter_map(|mesh_index| graphics.meshes[mesh_index].skin_data.as_ref())
        .filter(|skin| skin.skeleton_id == skeleton_id)
        .collect()
}

/// Collider offsets are in the bone's frame: the spring solver places them at `global[bone] * offset`.
fn head_capsule_collider(
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

fn fit_body_collider(
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

fn collect_skin_vertices_of_bone(
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

struct MergedGeometry {
    parts: Vec<ChainGenPart>,
    positions: Vec<cgmath::Vector3<f32>>,
    triangles: Vec<[u32; 3]>,
    tex_coords: Vec<[f32; 2]>,
    skeleton_id: crate::animation::SkeletonId,
}

fn submesh_tex_coords(
    vertices: &[thyllore_model_core::mesh::Vertex],
    vertex_count: usize,
) -> Option<Vec<[f32; 2]>> {
    if vertices.len() != vertex_count {
        return None;
    }
    Some(
        vertices
            .iter()
            .map(|v| [v.tex_coord.x, v.tex_coord.y])
            .collect(),
    )
}

fn merge_submesh_geometry(
    graphics: &GraphicsResources,
    submeshes: Vec<(SubmeshKey, usize)>,
) -> anyhow::Result<MergedGeometry> {
    let mut parts = Vec::new();
    let mut positions: Vec<cgmath::Vector3<f32>> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut submesh_uvs: Vec<Option<Vec<[f32; 2]>>> = Vec::new();
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

        submesh_uvs.push(submesh_tex_coords(
            &mesh.vertex_data.vertices,
            skin.base_positions.len(),
        ));

        parts.push(ChainGenPart {
            key,
            mesh_index,
            vertex_offset,
            vertex_count: skin.base_positions.len() as u32,
        });
    }

    let tex_coords = submesh_uvs
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .map(|uvs| uvs.concat())
        .unwrap_or_default();

    let Some(skeleton_id) = skeleton_id else {
        anyhow::bail!("no submesh to generate chains on");
    };
    Ok(MergedGeometry {
        parts,
        positions,
        triangles,
        tex_coords,
        skeleton_id,
    })
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
    world: &World,
    root: HumanoidRole,
) -> anyhow::Result<thyllore_model_core::BoneId> {
    let state = world.resource::<AvatarSetupState>();
    let bone_index = state.mapping.by_role.get(&root).copied().ok_or_else(|| {
        anyhow::anyhow!(
            "humanoid role {} is not mapped; confirm the mapping in Avatar Setup",
            root.unity_name()
        )
    })?;
    Ok(bone_index as thyllore_model_core::BoneId)
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

struct ReportSource<'a> {
    parts: &'a [ChainGenPart],
    chains: &'a [GeneratedChain],
    bone_ids: &'a [Vec<thyllore_model_core::BoneId>],
    root: thyllore_model_core::BoneId,
    influences: &'a [VertexInfluences],
    input: &'a ChainGenInput<'a>,
    strands: Option<StrandPlacement>,
    root_band: Option<RootBand>,
    head_capsule: Option<HeadCapsule>,
    component_labels: &'a [usize],
}

fn to_array(p: cgmath::Vector3<f32>) -> [f32; 3] {
    [p.x, p.y, p.z]
}

fn root_band_record(band: RootBand, root_position: cgmath::Vector3<f32>) -> RootBandRecord {
    let root_height = root_position.dot(world_y_axis());
    RootBandRecord {
        equator_height: root_height - band.equator_progress,
        height: root_height - band.progress,
        center: to_array(band.center),
    }
}

fn bone_records(
    skeleton: &thyllore_model_core::Skeleton,
    source: &ReportSource,
) -> Vec<GeneratedBoneRecord> {
    source
        .chains
        .iter()
        .enumerate()
        .flat_map(|(ci, chain)| {
            let last_joint = chain.joints.len().saturating_sub(1);
            chain.joints.iter().enumerate().map(move |(ji, joint)| {
                let parent_id = if ji == 0 {
                    source.root
                } else {
                    source.bone_ids[ci][ji - 1]
                };
                GeneratedBoneRecord {
                    name: joint.name.clone(),
                    parent: skeleton.bones[parent_id as usize].name.clone(),
                    head: to_array(joint.head),
                    tail: (ji == last_joint).then(|| to_array(chain.tail)),
                }
            })
        })
        .collect()
}

fn weight_records(
    skeleton: &thyllore_model_core::Skeleton,
    source: &ReportSource,
) -> Vec<ChainWeightRecord> {
    let bone_name_of = |target_bone: &InfluenceTarget| match target_bone {
        InfluenceTarget::Root => skeleton.bones[source.root as usize].name.clone(),
        InfluenceTarget::Joint { chain, joint } => {
            let bid = source.bone_ids[*chain][*joint];
            skeleton.bones[bid as usize].name.clone()
        }
    };

    let mut vertex_weights = Vec::new();
    for part in source.parts {
        let part_label = format!("{}:{}", part.key.mesh, part.key.material);
        for influence in influences_of_part(part, source.influences) {
            let merged_vertex = (influence.vertex + part.vertex_offset) as usize;
            let position = to_array(source.input.positions[merged_vertex]);
            let component = source.component_labels[merged_vertex] as u32;
            vertex_weights.extend(influence.influences.iter().map(|(target_bone, weight)| {
                ChainWeightRecord {
                    mesh: part_label.clone(),
                    vertex: influence.vertex,
                    component,
                    bone: bone_name_of(target_bone),
                    weight: *weight,
                    position,
                }
            }));
        }
    }
    vertex_weights
}

fn build_report(
    skeleton: &thyllore_model_core::Skeleton,
    target: &SubmeshSelector,
    source: ReportSource,
) -> ChainGenReport {
    let (strand_mode, strand_centerlines) = match &source.strands {
        Some(strands) => (Some(strands.mode), strands.centerlines.clone()),
        None => (None, Vec::new()),
    };
    ChainGenReport {
        target: target.to_string(),
        bones: bone_records(skeleton, &source),
        vertex_weights: weight_records(skeleton, &source),
        root_band: source
            .root_band
            .map(|band| root_band_record(band, source.input.root_position)),
        head_capsule: source.head_capsule.map(|capsule| HeadCapsuleRecord {
            bottom: to_array(capsule.bottom),
            top: to_array(capsule.top),
            radius: capsule.radius,
        }),
        attachment_positions: source
            .input
            .attachment_positions
            .iter()
            .copied()
            .map(to_array)
            .collect(),
        strand_mode,
        strand_centerlines,
    }
}

/// Replaces the chains generated earlier on the same target: their bones go, their weights fold back
/// into the root, and the new chains are registered as spring chains.
pub fn generate_and_apply_chain_bones(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    request: &ChainGenRequest,
) -> anyhow::Result<()> {
    let before = capture_skeleton_snapshot(world, assets, graphics);
    let replaced = remove_generated_chains(world, assets, graphics, &request.target)?;
    let generated = generate_chain_bones(world, assets, graphics, request)?;
    apply_chain_generation(world, assets, graphics, generated, replaced);
    record_skeleton_edit(world, assets, graphics, before, "Generate chains");
    Ok(())
}

fn remove_generated_chains(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    target: &SubmeshSelector,
) -> anyhow::Result<usize> {
    let mut removed_chains = 0;
    while let Some(root_joint) = first_generated_root_joint(world, target) {
        remove_bone_subtree(world, assets, graphics, root_joint)?;
        removed_chains += 1;
    }
    Ok(removed_chains)
}

fn first_generated_root_joint(
    world: &World,
    target: &SubmeshSelector,
) -> Option<thyllore_model_core::BoneId> {
    world
        .iter_components::<SpringBoneSetup>()
        .flat_map(|(_, setup)| setup.chains.iter())
        .find(|chain| chain.origin.is_generated_on(target))
        .and_then(|chain| chain.joints.first())
        .map(|joint| joint.bone_id)
}

fn apply_chain_generation(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    generated: GeneratedChainBones,
    replaced_chains: usize,
) {
    let report = generated.report;
    let bone_count = report.bones.len();
    let chain_count = generated.chains.len();

    let has_root_collider = generated.root_collider.is_some();
    register_spring_chains(world, generated.chains, generated.root_collider);
    refresh_after_skeleton_edit(world, assets, graphics);
    world.resource_mut::<AvatarSetupState>().chain_gen_report = Some(report);

    let collider_note = if has_root_collider {
        " with a fitted root collider"
    } else {
        " without a root collider (no skin vertices on the root bone)"
    };
    match replaced_chains {
        0 => msg_info!(
            "Generated {} chain bones in {} spring chains{}",
            bone_count,
            chain_count,
            collider_note
        ),
        replaced => msg_info!(
            "Generated {} chain bones in {} spring chains{}, replacing {} chains",
            bone_count,
            chain_count,
            collider_note,
            replaced
        ),
    }
}

fn register_spring_chains(
    world: &mut World,
    chains: Vec<SpringChain>,
    root_collider: Option<FittedCapsule>,
) {
    let Some(entity) = find_or_create_spring_bone_setup_entity(world) else {
        msg_warn!("Generated chains are not spring chains: no animated model entity");
        return;
    };
    let world_units_per_skeleton_unit = spring_space_for_entity(world, entity).world_scale();
    let Some(mut setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };

    let fitted_body_group = match root_collider {
        Some(capsule) => Some(ensure_fitted_collider(&mut setup, &capsule)),
        None => setup
            .collider_groups
            .iter()
            .find(|group| group.name == FITTED_BODY_GROUP_NAME)
            .map(|group| group.id),
    };
    for mut chain in chains {
        chain.id = setup.next_chain_id;
        setup.next_chain_id += 1;
        chain.collider_group_ids.extend(fitted_body_group);
        for joint in &mut chain.joints {
            joint.hit_radius *= world_units_per_skeleton_unit;
        }
        setup.chains.push(chain);
    }

    reset_spring_bone_state(world);
}

pub fn set_chain_gen_settings(world: &mut World, settings: ChainGenSettings) {
    world.resource_mut::<AvatarSetupState>().chain_gen = settings;
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_math_core::{Vec2, Vec3};
    use thyllore_model_core::mesh::Vertex;

    fn vertex_with_uv(u: f32, v: f32) -> Vertex {
        Vertex {
            pos: Vec3::new(0.0, 0.0, 0.0),
            color: Default::default(),
            tex_coord: Vec2::new(u, v),
            normal: Default::default(),
        }
    }

    #[test]
    fn test_submesh_tex_coords_matching_count() {
        let vertices: Vec<Vertex> = vec![vertex_with_uv(0.1, 0.2), vertex_with_uv(0.3, 0.4)];
        let result = submesh_tex_coords(&vertices, 2);
        assert!(result.is_some());
        let tc = result.unwrap();
        assert_eq!(tc.len(), 2);
        assert!((tc[0][0] - 0.1).abs() < 1e-6);
        assert!((tc[0][1] - 0.2).abs() < 1e-6);
        assert!((tc[1][0] - 0.3).abs() < 1e-6);
        assert!((tc[1][1] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_submesh_tex_coords_mismatching_count() {
        let vertices: Vec<Vertex> = vec![vertex_with_uv(0.1, 0.2)];
        let result = submesh_tex_coords(&vertices, 3);
        assert!(result.is_none());
    }
}
