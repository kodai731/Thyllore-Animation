use cgmath::Vector3;
use thyllore_anim_core::spring_bone::{
    add_chain, apply_ramp, chains_in_scope as anim_chains_in_scope,
    chains_under_bone as anim_chains_under_bone, chains_with_radius as anim_chains_with_radius,
    ChainRamp, JointForce, RampScope, RampShape, SpringBoneSetup, SpringChainGizmoMode,
    SpringChainId, SpringJointParam,
};
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::humanoid::systems::structural_chains::find_chains_under;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, BoneRestFrame, SidecarJointForce,
    SidecarSpring,
};
use thyllore_model_core::{BoneId, Skeleton};

pub fn skeleton_from_bones(bones: &[BoneInput]) -> Skeleton {
    let mut skeleton = Skeleton::new("spring_edit");

    for bone in bones {
        let parent_id = bone.parent.map(|p| p as BoneId);
        skeleton.add_bone(&bone.name, parent_id);
    }

    skeleton
}

fn resolve_root_bone(
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    root_name: &str,
) -> anyhow::Result<BoneId> {
    if let Some(&id) = skeleton.bone_name_to_id.get(root_name) {
        return Ok(id);
    }
    HumanoidRole::from_unity_name(root_name)
        .and_then(|role| mapping.by_role.get(&role))
        .map(|&index| index as BoneId)
        .ok_or_else(|| anyhow::anyhow!("bone \"{}\" not found", root_name))
}

pub fn resolve_root_bone_name(bones: &[BoneInput], root_name: &str) -> anyhow::Result<String> {
    let skeleton = skeleton_from_bones(bones);
    let (mapping, _) = infer_mapping(bones);
    let resolved_id = resolve_root_bone(&skeleton, &mapping, root_name)?;
    Ok(bones[resolved_id as usize].name.clone())
}

/// Every run of non-humanoid bones under `root_name` becomes a chain; a run that already heads a
/// chain is kept. The humanoid bones are inferred from the bone names, as the collider fit does.
pub fn add_chains_under_bone(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    root_name: &str,
    min_joints: u32,
) -> anyhow::Result<SidecarSpring> {
    let mut setup =
        spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));
    let skeleton = skeleton_from_bones(bones);
    let (mapping, _) = infer_mapping(bones);
    let root = resolve_root_bone(&skeleton, &mapping, root_name)?;

    let chains = find_chains_under(&skeleton, &mapping, root, min_joints as usize);
    if chains.is_empty() {
        anyhow::bail!(
            "no bone runs of at least {} joints under \"{}\"",
            min_joints,
            root_name
        );
    }

    for chain in chains {
        let chain_root = chain.joints[0];
        let already_exists = setup.chains.iter().any(|existing| {
            existing
                .joints
                .first()
                .is_some_and(|joint| joint.bone_id == chain_root)
        });
        if already_exists {
            continue;
        }
        add_chain(&mut setup, chain_root, chain.joints.len() as u32, &skeleton);
    }

    Ok(build_sidecar_spring(
        &setup,
        bones,
        &BoneRestFrame::identity_for(bones.len()),
    ))
}

#[derive(serde::Deserialize)]
pub struct RampInput {
    pub root: JointValues,
    pub tip: JointValues,
    pub shape: String,
    #[serde(default)]
    pub rigid_fraction: f32,
}

/// The joint values the addon sends, in the sidecar joint's shape without the bone.
#[derive(serde::Deserialize)]
pub struct JointValues {
    pub force: SidecarJointForce,
    pub drag_force: f32,
    pub gravity_dir: [f32; 3],
    pub hit_radius: f32,
}

impl JointValues {
    fn to_joint_param(&self) -> SpringJointParam {
        SpringJointParam {
            bone_id: 0,
            force: JointForce::from(self.force),
            drag_force: self.drag_force,
            gravity_dir: Vector3::from(self.gravity_dir),
            hit_radius: self.hit_radius,
        }
    }
}

fn parse_ramp_shape(shape: &str) -> anyhow::Result<RampShape> {
    match shape {
        "linear" => Ok(RampShape::Linear),
        "ease_in" => Ok(RampShape::EaseIn),
        "ease_out" => Ok(RampShape::EaseOut),
        other => anyhow::bail!("unknown ramp shape \"{}\"", other),
    }
}

fn to_chain_ramp(ramp: &RampInput) -> anyhow::Result<ChainRamp> {
    let root = ramp.root.to_joint_param();
    let tip = ramp.tip.to_joint_param();
    let shape = parse_ramp_shape(&ramp.shape)?;
    Ok(ChainRamp {
        root,
        tip,
        shape,
        rigid_fraction: ramp.rigid_fraction,
    })
}

pub fn apply_ramp_to_chains(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    chain_names: &[String],
    ramp: &RampInput,
) -> anyhow::Result<SidecarSpring> {
    let mut setup =
        spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));

    if chain_names.is_empty() {
        anyhow::bail!("chain_names must not be empty");
    }

    let chain_ramp = to_chain_ramp(ramp)?;

    for chain_name in chain_names {
        let chain_idx = setup
            .chains
            .iter()
            .position(|c| c.name == *chain_name)
            .ok_or_else(|| anyhow::anyhow!("chain \"{}\" not found", chain_name))?;

        let old_chain = &setup.chains[chain_idx];
        let new_chain = apply_ramp(old_chain, &chain_ramp)
            .map_err(|mismatch| anyhow::anyhow!("chain \"{}\": {mismatch}", chain_name))?;
        setup.chains[chain_idx] = new_chain;
    }

    Ok(build_sidecar_spring(
        &setup,
        bones,
        &BoneRestFrame::identity_for(bones.len()),
    ))
}

fn parse_scope(scope: &str) -> anyhow::Result<RampScope> {
    match scope {
        "this_chain" => Ok(RampScope::ThisChain),
        "sibling_chains" => Ok(RampScope::SiblingChains),
        "all_chains" => Ok(RampScope::AllChains),
        other => anyhow::bail!("unknown scope \"{}\"", other),
    }
}

fn chain_names_of(setup: &SpringBoneSetup, ids: &[SpringChainId]) -> Vec<String> {
    ids.iter()
        .filter_map(|id| {
            setup
                .chains
                .iter()
                .find(|c| c.id == *id)
                .map(|c| c.name.clone())
        })
        .collect()
}

pub fn chains_in_scope(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    chain_name: &str,
    scope: &str,
) -> anyhow::Result<Vec<String>> {
    let setup = spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));
    let skeleton = skeleton_from_bones(bones);

    let chain_id = setup
        .chains
        .iter()
        .find(|c| c.name == chain_name)
        .map(|c| c.id)
        .ok_or_else(|| anyhow::anyhow!("chain \"{}\" not found", chain_name))?;

    let scope = parse_scope(scope)?;
    let ids = anim_chains_in_scope(&setup, &skeleton, chain_id, scope);

    Ok(chain_names_of(&setup, &ids))
}

pub fn chains_under_bone(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    bone_name: &str,
) -> anyhow::Result<Vec<String>> {
    let setup = spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));
    let skeleton = skeleton_from_bones(bones);

    let bone_id = skeleton
        .bone_name_to_id
        .get(bone_name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("bone \"{}\" not found", bone_name))?;

    let ids = anim_chains_under_bone(&setup, &skeleton, bone_id);

    Ok(chain_names_of(&setup, &ids))
}

/// Chains whose joint hit radii the Blender gizmo draws: those holding the active bone.
pub fn chains_with_radius(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    active: Option<usize>,
    mode: &str,
) -> anyhow::Result<Vec<String>> {
    let setup = spring_setup_from_sidecar(spring, bones, &BoneRestFrame::identity_for(bones.len()));

    let gizmo_mode = match mode {
        "all" => SpringChainGizmoMode::All,
        "none" => SpringChainGizmoMode::None,
        other => anyhow::bail!("unknown mode \"{}\"", other),
    };

    let ids = anim_chains_with_radius(&setup, active, gizmo_mode);
    Ok(chain_names_of(&setup, &ids))
}

#[cfg(test)]
mod tests;
