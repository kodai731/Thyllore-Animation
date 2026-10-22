use cgmath::Vector3;
use thyllore_anim_core::spring_bone::{
    add_chain, apply_ramp, chains_in_scope as anim_chains_in_scope,
    chains_under_bone as anim_chains_under_bone, ChainRamp, RampScope, RampShape, SpringBoneSetup,
    SpringChainId, SpringJointParam,
};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::spring_prefix::find_prefix_chain_roots;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar, SidecarSpring,
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

pub fn chains_from_prefix(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    prefix: &str,
) -> anyhow::Result<SidecarSpring> {
    let mut setup = spring_setup_from_sidecar(spring, bones);
    let skeleton = skeleton_from_bones(bones);

    let roots = find_prefix_chain_roots(bones, prefix);
    if roots.is_empty() {
        anyhow::bail!("no bones found matching prefix \"{}\"", prefix);
    }

    for root in roots {
        let root_bone_id = root.root as BoneId;
        let already_exists = setup.chains.iter().any(|chain| {
            chain
                .joints
                .first()
                .is_some_and(|joint| joint.bone_id == root_bone_id)
        });
        if already_exists {
            continue;
        }
        add_chain(&mut setup, root_bone_id, root.length, &skeleton);
    }

    Ok(build_sidecar_spring(&setup, bones))
}

#[derive(serde::Deserialize)]
pub struct RampInput {
    pub root: JointValues,
    pub tip: JointValues,
    pub shape: String,
}

#[derive(serde::Deserialize)]
pub struct JointValues {
    pub stiffness: f32,
    pub drag_force: f32,
    pub gravity_power: f32,
    pub gravity_dir: [f32; 3],
    pub hit_radius: f32,
}

impl JointValues {
    fn to_joint_param(&self) -> SpringJointParam {
        SpringJointParam {
            bone_id: 0,
            stiffness: self.stiffness,
            drag_force: self.drag_force,
            gravity_power: self.gravity_power,
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
    Ok(ChainRamp { root, tip, shape })
}

pub fn apply_ramp_to_chains(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    chain_names: &[String],
    ramp: &RampInput,
) -> anyhow::Result<SidecarSpring> {
    let mut setup = spring_setup_from_sidecar(spring, bones);

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
        let new_chain = apply_ramp(old_chain, &chain_ramp);
        setup.chains[chain_idx] = new_chain;
    }

    Ok(build_sidecar_spring(&setup, bones))
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
    let setup = spring_setup_from_sidecar(spring, bones);
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
    let setup = spring_setup_from_sidecar(spring, bones);
    let skeleton = skeleton_from_bones(bones);

    let bone_id = skeleton
        .bone_name_to_id
        .get(bone_name)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("bone \"{}\" not found", bone_name))?;

    let ids = anim_chains_under_bone(&setup, &skeleton, bone_id);

    Ok(chain_names_of(&setup, &ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &str, parent: Option<usize>) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent,
            rest_position: [0.0, 0.0, 0.0],
        }
    }

    fn test_bones() -> Vec<BoneInput> {
        vec![
            bone("Hips", None),
            bone("Head", Some(0)),
            bone("hair_a", Some(1)),
            bone("hair_a.001", Some(2)),
            bone("hair_a.002", Some(3)),
            bone("hair_b", Some(1)),
            bone("hair_b.001", Some(5)),
        ]
    }

    fn ramp_input(shape: &str) -> RampInput {
        RampInput {
            root: JointValues {
                stiffness: 1.0,
                drag_force: 0.0,
                gravity_power: 0.0,
                gravity_dir: [0.0, -1.0, 0.0],
                hit_radius: 0.0,
            },
            tip: JointValues {
                stiffness: 0.0,
                drag_force: 0.0,
                gravity_power: 0.0,
                gravity_dir: [0.0, -1.0, 0.0],
                hit_radius: 0.0,
            },
            shape: shape.to_string(),
        }
    }

    #[test]
    fn spring_edit_bridge_builds_two_chains_from_prefix() {
        let bones = test_bones();
        let spring = SidecarSpring::default();

        let result = chains_from_prefix(&spring, &bones, "hair_").unwrap();

        assert_eq!(result.chains.len(), 2);

        let chain_a = result
            .chains
            .iter()
            .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_a"))
            .expect("chain starting with hair_a");
        assert_eq!(chain_a.joints.len(), 3);
        assert_eq!(chain_a.joints.first().unwrap().bone, "hair_a");
        assert_eq!(chain_a.joints[1].bone, "hair_a.001");
        assert_eq!(chain_a.joints[2].bone, "hair_a.002");

        let chain_b = result
            .chains
            .iter()
            .find(|c| c.joints.first().is_some_and(|j| j.bone == "hair_b"))
            .expect("chain starting with hair_b");
        assert_eq!(chain_b.joints.len(), 2);
        assert_eq!(chain_b.joints.first().unwrap().bone, "hair_b");
        assert_eq!(chain_b.joints[1].bone, "hair_b.001");
    }

    #[test]
    fn spring_edit_bridge_idempotent_call_does_not_duplicate_chains() {
        let bones = test_bones();
        let spring = SidecarSpring::default();

        let first = chains_from_prefix(&spring, &bones, "hair_").unwrap();
        assert_eq!(first.chains.len(), 2);

        let second = chains_from_prefix(&first, &bones, "hair_").unwrap();

        assert_eq!(second.chains.len(), 2);

        let first_names: Vec<_> = first.chains.iter().map(|c| c.name.clone()).collect();
        let second_names: Vec<_> = second.chains.iter().map(|c| c.name.clone()).collect();
        assert_eq!(first_names, second_names);
    }

    #[test]
    fn spring_edit_bridge_unknown_prefix_returns_error() {
        let bones = test_bones();
        let spring = SidecarSpring::default();

        let result = chains_from_prefix(&spring, &bones, "nonexistent_");
        assert!(result.is_err());
    }

    #[test]
    fn spring_edit_bridge_ramp_linear_interpolation() {
        let bones = test_bones();
        let spring = chains_from_prefix(&SidecarSpring::default(), &bones, "hair_a").unwrap();
        let ramp = ramp_input("linear");

        let result =
            apply_ramp_to_chains(&spring, &bones, &["Chain_0".to_string()], &ramp).unwrap();

        assert_eq!(result.chains.len(), 1);
        let chain = &result.chains[0];
        assert_eq!(chain.joints.len(), 3);
        assert!((chain.joints.first().unwrap().stiffness - 1.0).abs() < 1e-6);
        assert!((chain.joints[1].stiffness - 0.5).abs() < 1e-6);
        assert!((chain.joints[2].stiffness - 0.0).abs() < 1e-6);
    }

    #[test]
    fn spring_edit_bridge_chains_under_bone_head() {
        let bones = test_bones();
        let spring = chains_from_prefix(&SidecarSpring::default(), &bones, "hair_").unwrap();

        let result = chains_under_bone(&spring, &bones, "Head").unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&"Chain_0".to_string()));
        assert!(result.contains(&"Chain_1".to_string()));
    }

    #[test]
    fn spring_edit_bridge_chains_in_scope_sibling_chains() {
        let bones = test_bones();
        let spring = chains_from_prefix(&SidecarSpring::default(), &bones, "hair_").unwrap();

        let result = chains_in_scope(&spring, &bones, "Chain_0", "sibling_chains").unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn spring_edit_bridge_unknown_shape_returns_error() {
        let bones = test_bones();
        let spring = chains_from_prefix(&SidecarSpring::default(), &bones, "hair_a").unwrap();
        let mut ramp = ramp_input("linear");
        ramp.shape = "unknown".to_string();

        let result = apply_ramp_to_chains(&spring, &bones, &["Chain_0".to_string()], &ramp);
        assert!(result.is_err());
    }
}
