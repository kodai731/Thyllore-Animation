use crate::animation::spring_bone::{
    compare_tail_trajectories, SpringBoneSetup, SpringSpace, SpringTailSample,
};
use crate::animation::{AnimationClip, Skeleton};
use crate::ecs::component::{ConstraintSet, SpringChain};
use thyllore_math_core::{minimize_nelder_mead, NelderMeadParams};

use super::spring_bone_bake_systems::{simulate_spring_tails, BakeConfig};

#[derive(Clone, Debug, PartialEq)]
pub struct SpringChainFit {
    pub chain: String,
    pub before_rms: f32,
    pub after_rms: f32,
    pub stiffness: f32,
    pub drag_force: f32,
    pub gravity_power: f32,
}

pub struct SpringFitInputs<'a> {
    pub config: &'a BakeConfig,
    pub skeleton: &'a Skeleton,
    pub clip: &'a AnimationClip,
    pub constraints: Option<&'a ConstraintSet>,
    pub looping: bool,
    pub reference: &'a [SpringTailSample],
}

fn clamp_values(values: &[f32]) -> [f32; 3] {
    [
        values[0].max(0.0),
        values[1].clamp(0.0, 1.0),
        values[2].max(0.0),
    ]
}

fn with_chain_values(
    setup: &SpringBoneSetup,
    chain_name: &str,
    values: [f32; 3],
) -> SpringBoneSetup {
    let mut out = setup.clone();
    for chain in &mut out.chains {
        if chain.name == chain_name {
            for joint in &mut chain.joints {
                joint.stiffness = values[0];
                joint.drag_force = values[1];
                joint.gravity_power = values[2];
            }
        }
    }
    out
}

fn chain_rms(
    inputs: &SpringFitInputs,
    setup: &SpringBoneSetup,
    chain_name: &str,
    values: [f32; 3],
) -> f32 {
    let eval_setup = with_chain_values(setup, chain_name, values);
    let simulated = simulate_spring_tails(
        inputs.config,
        &eval_setup,
        inputs.skeleton,
        inputs.clip,
        inputs.constraints,
        inputs.looping,
    );
    let errors = compare_tail_trajectories(inputs.reference, &simulated);
    for e in errors {
        if e.chain == chain_name {
            return e.rms;
        }
    }
    f32::MAX
}

fn chain_start_values(chain: &SpringChain) -> Option<[f32; 3]> {
    let first = chain.joints.first()?;
    Some([first.stiffness, first.drag_force, first.gravity_power])
}

pub fn fit_spring_params(
    inputs: &SpringFitInputs,
    setup: &SpringBoneSetup,
    params: &NelderMeadParams,
) -> (SpringBoneSetup, Vec<SpringChainFit>) {
    let mut current_setup = setup.clone();
    let mut fits: Vec<SpringChainFit> = Vec::new();

    for chain in &setup.chains {
        if !chain.enabled {
            continue;
        }

        let has_reference = inputs.reference.iter().any(|s| s.chain == chain.name);
        if !has_reference {
            continue;
        }

        let start_values = match chain_start_values(chain) {
            Some(v) => v,
            None => continue,
        };

        let before_rms = chain_rms(inputs, &current_setup, &chain.name, start_values);

        let objective = |x: &[f32]| -> f32 {
            let clamped = clamp_values(x);
            chain_rms(inputs, &current_setup, &chain.name, clamped)
        };

        let result = minimize_nelder_mead(objective, &start_values, params);
        let best_clamped = clamp_values(&result.point);

        for joint in &mut current_setup
            .chains
            .iter_mut()
            .find(|c| c.name == chain.name)
            .unwrap()
            .joints
        {
            joint.stiffness = best_clamped[0];
            joint.drag_force = best_clamped[1];
            joint.gravity_power = best_clamped[2];
        }

        let after_rms = chain_rms(inputs, &current_setup, &chain.name, best_clamped);

        fits.push(SpringChainFit {
            chain: chain.name.clone(),
            before_rms,
            after_rms,
            stiffness: best_clamped[0],
            drag_force: best_clamped[1],
            gravity_power: best_clamped[2],
        });
    }

    (current_setup, fits)
}

#[cfg(test)]
mod tests;
