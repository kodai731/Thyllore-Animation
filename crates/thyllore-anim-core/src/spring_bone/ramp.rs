use std::sync::LazyLock;

use thyllore_model_core::{BoneId, Skeleton};
use thyllore_scene_core::{SceneFields, UiParam};

use crate::spring_bone::{SpringBoneSetup, SpringChain, SpringChainId, SpringJointParam};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RampShape {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
}

impl RampShape {
    pub const ALL: [RampShape; 3] = [RampShape::Linear, RampShape::EaseIn, RampShape::EaseOut];

    pub fn label(&self) -> &'static str {
        match self {
            RampShape::Linear => "Linear",
            RampShape::EaseIn => "Ease In",
            RampShape::EaseOut => "Ease Out",
        }
    }

    pub fn weight(&self, t: f32) -> f32 {
        match self {
            RampShape::Linear => t,
            RampShape::EaseIn => t * t,
            RampShape::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct ChainRamp {
    #[nested]
    pub root: SpringJointParam,
    #[nested]
    pub tip: SpringJointParam,
    pub shape: RampShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampEnd {
    Root,
    Tip,
}

impl ChainRamp {
    pub fn set_end(&mut self, end: RampEnd, joint: SpringJointParam) {
        match end {
            RampEnd::Root => self.root = joint,
            RampEnd::Tip => self.tip = joint,
        }
    }

    pub fn load_ends(&mut self, chain: &SpringChain) {
        let joints = &chain.joints;
        if joints.is_empty() {
            return;
        }
        self.root = joints[0].clone();
        self.tip = joints[joints.len() - 1].clone();
    }
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}

pub fn apply_ramp(chain: &SpringChain, ramp: &ChainRamp) -> SpringChain {
    let n = chain.joints.len();
    let mut out = chain.clone();

    for (i, joint) in out.joints.iter_mut().enumerate() {
        let t = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let w = ramp.shape.weight(t);

        joint.force = ramp.root.force.lerp(&ramp.tip.force, w);
        joint.drag_force = lerp(ramp.root.drag_force, ramp.tip.drag_force, w);
        joint.hit_radius = lerp(ramp.root.hit_radius, ramp.tip.hit_radius, w);
        joint.gravity_dir = ramp.root.gravity_dir;
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RampScope {
    #[default]
    ThisChain,
    SiblingChains,
    AllChains,
}

impl RampScope {
    pub const ALL: [RampScope; 3] = [
        RampScope::ThisChain,
        RampScope::SiblingChains,
        RampScope::AllChains,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            RampScope::ThisChain => "This Chain",
            RampScope::SiblingChains => "Sibling Chains",
            RampScope::AllChains => "All Chains",
        }
    }
}

fn root_parent(chain: &SpringChain, skeleton: &Skeleton) -> Option<BoneId> {
    let first_joint = chain.joints.first()?;
    skeleton
        .get_bone(first_joint.bone_id)
        .and_then(|b| b.parent_id)
}

/// The chains whose root joint hangs directly under `bone_id`.
pub fn chains_under_bone(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    bone_id: BoneId,
) -> Vec<SpringChainId> {
    setup
        .chains
        .iter()
        .filter(|c| root_parent(c, skeleton) == Some(bone_id))
        .map(|c| c.id)
        .collect()
}

pub fn chains_in_scope(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    chain_id: SpringChainId,
    scope: RampScope,
) -> Vec<SpringChainId> {
    match scope {
        RampScope::ThisChain => vec![chain_id],
        RampScope::AllChains => setup.chains.iter().map(|c| c.id).collect(),
        RampScope::SiblingChains => {
            let target_chain = setup.chains.iter().find(|c| c.id == chain_id);
            let Some(target_chain) = target_chain else {
                return Vec::new();
            };
            let Some(parent) = root_parent(target_chain, skeleton) else {
                return Vec::new();
            };
            chains_under_bone(setup, skeleton, parent)
        }
    }
}

pub fn joint_count_of(setup: &SpringBoneSetup, chain_ids: &[SpringChainId]) -> usize {
    setup
        .chains
        .iter()
        .filter(|c| chain_ids.contains(&c.id))
        .map(|c| c.joints.len())
        .sum()
}

pub static CHAIN_RAMP_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out: Vec<UiParam> = Vec::new();
    ChainRamp::collect_ui("", "", &mut out);
    out
});

#[cfg(test)]
mod tests;
