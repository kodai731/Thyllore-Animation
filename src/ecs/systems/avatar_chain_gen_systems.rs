use crate::asset::AssetStorage;
use crate::ecs::resource::{AvatarSetupState, NodeAssets};
use crate::ecs::systems::avatar_collider_systems::{fit_body_collider, head_capsule_collider};
use crate::ecs::systems::avatar_setup_systems::compute_bone_global_transform;
use crate::ecs::systems::avatar_setup_systems::find_or_create_spring_bone_setup_entity;
use crate::ecs::systems::avatar_submesh_geometry_systems::{merge_submesh_geometry, SubmeshPart};
use crate::ecs::systems::avatar_submesh_role_systems::{
    collect_submesh_meshes, skin_geometry_to_model, skin_role_surfaces,
};
use crate::ecs::systems::skeleton_edit_systems::{
    capture_skeleton_snapshot, record_skeleton_edit, refresh_after_skeleton_edit,
    remove_bone_subtree, restore_skeleton_snapshot,
};
use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use cgmath::{InnerSpace, SquareMatrix};
use thyllore_anim_core::spring_bone::{
    add_generated_chains, ChainOrigin, FittedCapsule, SpringBoneSetup, SpringChain,
    SpringJointParam,
};
use thyllore_avatar_core::cloth_setup::components::chain_gen::{
    chain_name, chain_role_of_submesh, ChainGenInput, ChainGenParams, ChainGenReport,
    ChainGenRequest, ChainGenSettings, ChainPlacement, ChainRole, ChainWeightRecord,
    GeneratedBoneRecord, GeneratedChain, GeneratedJoint, HeadCapsuleRecord, InfluenceTarget,
    RootBandRecord, StrandPlacement, VertexInfluences,
};
use thyllore_avatar_core::cloth_setup::components::submesh_role::{SubmeshKey, SubmeshSelector};
use thyllore_avatar_core::cloth_setup::systems::chain_bones::{
    append_chain_bones, write_chain_weights,
};
use thyllore_avatar_core::cloth_setup::systems::chain_gen::hair_head_capsule;
use thyllore_avatar_core::cloth_setup::systems::chain_gen::{
    component_labels, compute_chain_weights, hair_root_band, place_chains_with_strands,
    root_attachment_positions, RootBand,
};
use thyllore_avatar_core::cloth_setup::systems::joint_radius::joint_half_thicknesses;
use thyllore_avatar_core::collider::components::capsule::HeadCapsule;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_math_core::{array3_from_vec, world_y_axis};

pub struct GeneratedChainBones {
    pub report: ChainGenReport,
    pub chains: Vec<SpringChain>,
    pub root_collider: Option<FittedCapsule>,
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
    let body_collider = fit_body_collider(world, assets, graphics, geometry.skeleton_id, root);
    let geometry_to_model = {
        let skeleton = find_skeleton_mut(assets, geometry.skeleton_id)?;
        skin_geometry_to_model(skeleton, root as usize)
    };
    let skins = skin_role_surfaces(
        world,
        assets,
        graphics,
        geometry.skeleton_id,
        geometry_to_model,
    );
    let skeleton = find_skeleton_mut(assets, geometry.skeleton_id)?;
    let root_global = compute_bone_global_transform(skeleton, root as usize);
    let model_to_geometry = geometry_to_model
        .invert()
        .ok_or_else(|| anyhow::anyhow!("geometry transform of the skin is singular"))?;

    let to_model = |p: &cgmath::Vector3<f32>| (geometry_to_model * p.extend(1.0)).truncate();
    let model_positions: Vec<cgmath::Vector3<f32>> =
        geometry.positions.iter().map(to_model).collect();
    let input = ChainGenInput {
        positions: &model_positions,
        triangles: &geometry.triangles,
        root_position: root_global.w.truncate(),
        root_bone: root,
        skins: &skins,
        tex_coords: &geometry.tex_coords,
        covered_vertices: &[],
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
    let ChainPlacement {
        chains,
        strands,
        covered,
    } = place_chains_with_strands(&input, &params)?;
    let influences = compute_chain_weights(&input, &params, &chains, &covered);

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
            strands,
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
        let submesh_role = *state.submesh_roles.get(&key).ok_or_else(|| {
            anyhow::anyhow!("submesh role not set: {}/{}", key.mesh, key.material)
        })?;
        let part_role = match (chain_role_of_submesh(submesh_role), target) {
            (Some(chain_role), _) => chain_role,
            (None, SubmeshSelector::Mesh(_)) => continue,
            (None, SubmeshSelector::Submesh(_)) => anyhow::bail!(
                "submesh role {:?} is not Hair or Garment: {}/{}",
                submesh_role,
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

fn influences_of_part(
    part: &SubmeshPart,
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
    let bone_index = state.mapping.bone_of(root).ok_or_else(|| {
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
    parts: &'a [SubmeshPart],
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

fn root_band_record(band: RootBand, root_position: cgmath::Vector3<f32>) -> RootBandRecord {
    let root_height = root_position.dot(world_y_axis());
    RootBandRecord {
        equator_height: root_height - band.equator_progress,
        height: root_height - band.progress,
        center: array3_from_vec(band.center),
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
                    head: array3_from_vec(joint.head),
                    tail: (ji == last_joint).then(|| array3_from_vec(chain.tail)),
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
        InfluenceTarget::Bone(bone) => skeleton.bones[*bone as usize].name.clone(),
    };

    let mut vertex_weights = Vec::new();
    for part in source.parts {
        let part_label = format!("{}:{}", part.key.mesh, part.key.material);
        for influence in influences_of_part(part, source.influences) {
            let merged_vertex = (influence.vertex + part.vertex_offset) as usize;
            let position = array3_from_vec(source.input.positions[merged_vertex]);
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
            bottom: array3_from_vec(capsule.bottom),
            top: array3_from_vec(capsule.top),
            radius: capsule.radius,
        }),
        attachment_positions: root_attachment_positions(source.input.skins, source.input.root_bone)
            .into_iter()
            .map(array3_from_vec)
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
    match replace_generated_chains(world, assets, graphics, request) {
        Ok((replaced, generated)) => {
            apply_chain_generation(world, assets, graphics, generated, replaced);
            record_skeleton_edit(world, assets, graphics, before, "Generate chains");
            Ok(())
        }
        Err(e) => {
            if let Some(before) = before {
                restore_skeleton_snapshot(world, assets, graphics, before);
            }
            Err(e)
        }
    }
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

fn replace_generated_chains(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    request: &ChainGenRequest,
) -> anyhow::Result<(usize, GeneratedChainBones)> {
    let replaced = remove_generated_chains(world, assets, graphics, &request.target)?;
    let generated = generate_chain_bones(world, assets, graphics, request)?;
    Ok((replaced, generated))
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
    let spring_space = spring_space_for_entity(world, entity);
    let Some(mut setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    add_generated_chains(&mut setup, chains, root_collider, spring_space);

    reset_spring_bone_state(world);
}

pub fn set_chain_gen_settings(world: &mut World, settings: ChainGenSettings) {
    world.resource_mut::<AvatarSetupState>().chain_gen = settings;
}
