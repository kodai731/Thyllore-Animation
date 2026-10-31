use crate::animation::spring_bone::SpringForceModel;
use crate::animation::BoneId;
use crate::ecs::component::CapsuleFitParams;
use crate::ecs::resource::AvatarSetupState;
use crate::ecs::systems::batch_run_systems::BatchAction;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::systems::phases::event_dispatch::spring_bone::{
    DumpSkinnedVertices, DumpSpringTails, FitSpringParams, SetSpringForceModel,
};
use crate::ecs::world::World;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshKey;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

#[derive(Debug)]
pub struct AddSpringChainsUnder {
    root: HumanoidRole,
    min_joints: u32,
}

impl BatchAction for AddSpringChainsUnder {
    fn name(&self) -> &'static str {
        "add_spring_chains_under"
    }
    fn apply(&self, world: &mut World) {
        let root = world
            .resource::<AvatarSetupState>()
            .mapping
            .by_role
            .get(&self.root)
            .copied();
        match root {
            Some(root) => world.send_command(AvatarSetupEvent::AddSpringChainsUnder {
                root: root as BoneId,
                min_joints: self.min_joints,
            }),
            None => msg_error!(
                "add_spring_chains_under: humanoid role {} is not mapped",
                self.root.unity_name()
            ),
        }
    }
}

pub(super) fn add_spring_chains_under_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("add_spring_chains_under=")?;
    let mut parts = spec.split(',');
    let Some(root) = parts
        .next()
        .and_then(|role| HumanoidRole::from_unity_name(role.trim()))
    else {
        return Some(Err(anyhow!("{ADD_SPRING_CHAINS_UNDER_USAGE}")));
    };
    let mut min_joints = 2;
    for part in parts {
        let Some(value) = part.trim().strip_prefix("min_joints=") else {
            return Some(Err(anyhow!("{ADD_SPRING_CHAINS_UNDER_USAGE}")));
        };
        match value.parse::<u32>() {
            Ok(parsed) if parsed >= 1 => min_joints = parsed,
            _ => return Some(Err(anyhow!("{ADD_SPRING_CHAINS_UNDER_USAGE}"))),
        }
    }
    Some(Ok(Box::new(AddSpringChainsUnder { root, min_joints })))
}

#[derive(Debug)]
pub struct DumpSpringTailsAction {
    path: PathBuf,
    sample_rate: f32,
}

impl BatchAction for DumpSpringTailsAction {
    fn name(&self) -> &'static str {
        "dump_spring_tails"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(DumpSpringTails {
            path: self.path.clone(),
            sample_rate: self.sample_rate,
        });
    }
}

pub(super) fn dump_spring_tails_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let suffix = text.strip_prefix("dump_spring_tails=")?;
    let parts: Vec<&str> = suffix.split(',').collect();
    if parts.len() != 2 {
        return Some(Err(anyhow!(
            "dump_spring_tails requires exactly 2 comma-separated values: path,sample_rate"
        )));
    }
    let path = PathBuf::from(parts[0].trim());
    if path.as_os_str().is_empty() {
        return Some(Err(anyhow!("dump_spring_tails: path must not be empty")));
    }
    let Ok(sample_rate) = parts[1].trim().parse::<f32>() else {
        return Some(Err(anyhow!(
            "dump_spring_tails: invalid sample_rate '{}'",
            parts[1]
        )));
    };
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Some(Err(anyhow!(
            "dump_spring_tails: sample_rate must be positive and finite, got {}",
            sample_rate
        )));
    }
    Some(Ok(Box::new(DumpSpringTailsAction { path, sample_rate })))
}

#[derive(Debug)]
pub struct DumpSkinnedVerticesAction {
    path: PathBuf,
    sample_rate: f32,
    submesh: SubmeshKey,
}

impl BatchAction for DumpSkinnedVerticesAction {
    fn name(&self) -> &'static str {
        "dump_skinned_vertices"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(DumpSkinnedVertices {
            path: self.path.clone(),
            sample_rate: self.sample_rate,
            submesh: self.submesh.clone(),
        });
    }
}

pub(super) fn dump_skinned_vertices_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let suffix = text.strip_prefix("dump_skinned_vertices=")?;
    let parts: Vec<&str> = suffix.splitn(3, ',').collect();
    if parts.len() != 3 {
        return Some(Err(anyhow!(
            "dump_skinned_vertices requires exactly 3 comma-separated values: path,sample_rate,mesh/material"
        )));
    }
    let path = PathBuf::from(parts[0].trim());
    if path.as_os_str().is_empty() {
        return Some(Err(anyhow!(
            "dump_skinned_vertices: path must not be empty"
        )));
    }
    let Ok(sample_rate) = parts[1].trim().parse::<f32>() else {
        return Some(Err(anyhow!(
            "dump_skinned_vertices: invalid sample_rate '{}'",
            parts[1]
        )));
    };
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Some(Err(anyhow!(
            "dump_skinned_vertices: sample_rate must be positive and finite, got {}",
            sample_rate
        )));
    }
    let mesh_material = parts[2].trim();
    let (mesh, material) = match mesh_material.rsplit_once('/') {
        Some((m, mat)) => (m.to_string(), mat.to_string()),
        None => {
            return Some(Err(anyhow!(
                "dump_skinned_vertices: submesh must be in mesh/material format"
            )))
        }
    };
    let submesh = SubmeshKey { mesh, material };
    Some(Ok(Box::new(DumpSkinnedVerticesAction {
        path,
        sample_rate,
        submesh,
    })))
}

#[derive(Debug)]
pub struct FitSpringParamsAction {
    reference: PathBuf,
    sample_rate: f32,
    initial_step: f32,
    max_iterations: usize,
    tolerance: f32,
}

impl BatchAction for FitSpringParamsAction {
    fn name(&self) -> &'static str {
        "fit_spring_params"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(FitSpringParams {
            reference: self.reference.clone(),
            sample_rate: self.sample_rate,
            params: thyllore_math_core::NelderMeadParams {
                initial_step: self.initial_step,
                max_iterations: self.max_iterations,
                tolerance: self.tolerance,
            },
        });
    }
}

pub(super) fn fit_spring_params_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let suffix = text.strip_prefix("fit_spring_params=")?;
    let parts: Vec<&str> = suffix.split(',').collect();
    if parts.len() != 5 {
        return Some(Err(anyhow!(
            "fit_spring_params requires exactly 5 comma-separated values: reference,sample_rate,initial_step,max_iterations,tolerance"
        )));
    }
    let reference = PathBuf::from(parts[0].trim());
    if reference.as_os_str().is_empty() {
        return Some(Err(anyhow!(
            "fit_spring_params: reference path must not be empty"
        )));
    }
    let Ok(sample_rate) = parts[1].trim().parse::<f32>() else {
        return Some(Err(anyhow!(
            "fit_spring_params: invalid sample_rate '{}'",
            parts[1]
        )));
    };
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Some(Err(anyhow!(
            "fit_spring_params: sample_rate must be positive and finite, got {}",
            sample_rate
        )));
    }
    let Ok(initial_step) = parts[2].trim().parse::<f32>() else {
        return Some(Err(anyhow!(
            "fit_spring_params: invalid initial_step '{}'",
            parts[2]
        )));
    };
    if initial_step <= 0.0 {
        return Some(Err(anyhow!(
            "fit_spring_params: initial_step must be positive, got {}",
            initial_step
        )));
    }
    let Ok(max_iterations) = parts[3].trim().parse::<usize>() else {
        return Some(Err(anyhow!(
            "fit_spring_params: invalid max_iterations '{}'",
            parts[3]
        )));
    };
    if max_iterations < 1 {
        return Some(Err(anyhow!(
            "fit_spring_params: max_iterations must be at least 1, got {}",
            max_iterations
        )));
    }
    let Ok(tolerance) = parts[4].trim().parse::<f32>() else {
        return Some(Err(anyhow!(
            "fit_spring_params: invalid tolerance '{}'",
            parts[4]
        )));
    };
    Some(Ok(Box::new(FitSpringParamsAction {
        reference,
        sample_rate,
        initial_step,
        max_iterations,
        tolerance,
    })))
}

#[derive(Debug)]
pub struct FitBodyColliders {
    params: CapsuleFitParams,
}

impl BatchAction for FitBodyColliders {
    fn name(&self) -> &'static str {
        "fit_body_colliders"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::FitBodyColliders {
            params: self.params.clone(),
        });
    }
}

pub(super) fn fit_body_colliders_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("fit_body_colliders=")?;
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 4 {
        return Some(Err(anyhow!("{FIT_BODY_COLLIDERS_USAGE}")));
    }
    let parsed = (
        parts[0].trim().parse::<f32>().ok(),
        parts[1].trim().parse::<f32>().ok(),
        parts[2].trim().parse::<f32>().ok(),
        parts[3].trim().parse::<u32>().ok(),
    );

    Some(match parsed {
        (Some(min_weight), Some(radius_percentile), Some(radius_scale), Some(min_vertex_count)) => {
            Ok(Box::new(FitBodyColliders {
                params: CapsuleFitParams {
                    min_weight,
                    radius_percentile,
                    radius_scale,
                    min_vertex_count,
                },
            }))
        }
        _ => Err(anyhow!("{FIT_BODY_COLLIDERS_USAGE}")),
    })
}

#[derive(Debug)]
pub struct SpringForceModelAction {
    force_model: SpringForceModel,
}

impl BatchAction for SpringForceModelAction {
    fn name(&self) -> &'static str {
        "spring_force_model"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(SetSpringForceModel {
            force_model: self.force_model,
        });
    }
}

pub(super) fn spring_force_model_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("spring_force_model=")?;
    let Some(force_model) = SpringForceModel::from_key(spec) else {
        return Some(Err(anyhow!(
            "spring_force_model expects vrm or rest_relative, got {spec:?}"
        )));
    };
    Some(Ok(Box::new(SpringForceModelAction { force_model })))
}
crate::batch_action!("add_spring_chains_under", add_spring_chains_under_parse);
crate::batch_action!("fit_body_colliders", fit_body_colliders_parse);
crate::batch_action!("dump_spring_tails", dump_spring_tails_parse);
crate::batch_action!("dump_skinned_vertices", dump_skinned_vertices_parse);
crate::batch_action!("fit_spring_params", fit_spring_params_parse);
crate::batch_action!("spring_force_model", spring_force_model_parse);

const FIT_BODY_COLLIDERS_USAGE: &str = "fit_body_colliders expects fit_body_colliders=<min_weight>,<radius_percentile>,<radius_scale>,<min_vertex_count>";

const ADD_SPRING_CHAINS_UNDER_USAGE: &str =
    "add_spring_chains_under expects add_spring_chains_under=<humanoid role>[,min_joints=<n>]";
