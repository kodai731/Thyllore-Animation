use crate::ecs::resource::AvatarSetupState;
use crate::ecs::systems::batch_run_systems::BatchAction;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use anyhow::{anyhow, Result};
use thyllore_avatar_core::cloth_setup::components::chain_gen::{ChainGenRequest, ChainGenSettings};
use thyllore_avatar_core::cloth_setup::components::hidden_mask::HiddenMaskParams;
use thyllore_avatar_core::cloth_setup::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshSelector,
};
use thyllore_avatar_core::cloth_setup::systems::chain_gen;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_cli_core::scalar_assignment_parse;

#[derive(Debug)]
pub struct GenerateChainBones {
    request: ChainGenRequest,
}

impl BatchAction for GenerateChainBones {
    fn name(&self) -> &'static str {
        "generate_chain_bones"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::GenerateChainBones {
            request: self.request.clone(),
        });
    }
}

pub(super) fn generate_chain_bones_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("generate_chain_bones=")?;
    let mut parts = spec.split(',');
    let (Some(target_text), Some(root_bone)) = (parts.next(), parts.next()) else {
        return Some(Err(anyhow!("{GENERATE_CHAIN_BONES_USAGE}")));
    };

    let target = match target_text.parse::<SubmeshSelector>() {
        Ok(target) => target,
        Err(_) => return Some(Err(anyhow!("{GENERATE_CHAIN_BONES_USAGE}"))),
    };
    let Some(root) = HumanoidRole::from_unity_name(root_bone.trim()) else {
        return Some(Err(anyhow!(
            "generate_chain_bones: root must be a humanoid role such as Head or Hips, got \"{}\"",
            root_bone.trim()
        )));
    };

    let setting_names = chain_gen::setting_names();
    let assignments: Vec<(String, f32)> = match parts
        .map(|part| scalar_assignment_parse(part.trim(), &setting_names))
        .collect::<std::result::Result<_, String>>()
    {
        Ok(assignments) => assignments,
        Err(message) => return Some(Err(anyhow!("generate_chain_bones: {message}"))),
    };
    let mut settings = ChainGenSettings::default();
    if let Err(e) = chain_gen::apply_setting_assignments(&mut settings, &assignments) {
        return Some(Err(e));
    }
    if settings.joint_count == 0 {
        return Some(Err(anyhow!(
            "generate_chain_bones: joint_count must be at least 1"
        )));
    }

    let request = ChainGenRequest {
        target,
        root,
        settings,
    };
    Some(Ok(Box::new(GenerateChainBones { request })))
}

#[derive(Debug)]
pub struct SubmeshRoleAction {
    pub(super) role: SubmeshRole,
    pub(super) target: SubmeshSelector,
}

impl BatchAction for SubmeshRoleAction {
    fn name(&self) -> &'static str {
        "submesh_role"
    }
    fn apply(&self, world: &mut World) {
        let keys: Vec<SubmeshKey> = world
            .resource::<AvatarSetupState>()
            .submesh_roles
            .keys()
            .filter(|key| self.target.matches(key))
            .cloned()
            .collect();
        if keys.is_empty() {
            log_warn!("submesh_role: no submesh matches {}", self.target);
        }
        for key in keys {
            world.send_command(AvatarSetupEvent::SetSubmeshRole {
                key,
                role: self.role,
            });
        }
    }
}

pub(super) fn submesh_role_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("submesh_role=")?;
    Some(submesh_role_spec_parse(spec).map(|action| Box::new(action) as Box<dyn BatchAction>))
}

pub(super) fn submesh_role_spec_parse(spec: &str) -> Result<SubmeshRoleAction> {
    let Some((role_str, target_text)) = spec.split_once(':') else {
        return Err(anyhow!("{SUBMESH_ROLE_USAGE}"));
    };
    let target = target_text
        .parse::<SubmeshSelector>()
        .map_err(|_| anyhow!("{SUBMESH_ROLE_USAGE}"))?;
    let role =
        serde_json::from_value::<SubmeshRole>(serde_json::Value::String(role_str.to_string()))
            .map_err(|e| anyhow!("unknown submesh role {:?}: {}", role_str, e))?;
    Ok(SubmeshRoleAction { role, target })
}

#[derive(Debug)]
pub struct HideCoveredSkin {
    params: HiddenMaskParams,
}

impl BatchAction for HideCoveredSkin {
    fn name(&self) -> &'static str {
        "hide_covered_skin"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::ComputeHiddenTriangles {
            params: self.params.clone(),
        });
    }
}

pub(super) fn hide_covered_skin_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("hide_covered_skin=")?;
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 4 {
        return Some(Err(anyhow!("{HIDE_COVERED_SKIN_USAGE}")));
    }
    let parsed = (
        parts[0].trim().parse::<f32>().ok(),
        parts[1].trim().parse::<f32>().ok(),
        parts[2].trim().parse::<usize>().ok(),
        parts[3].trim().parse::<f32>().ok(),
    );

    Some(match parsed {
        (Some(cover_distance), Some(cone_angle_deg), Some(ray_count), Some(hidden_ratio)) => {
            if cover_distance <= 0.0 {
                return Some(Err(anyhow!("cover_distance must be > 0")));
            }
            if cone_angle_deg < 0.0 || cone_angle_deg > 90.0 {
                return Some(Err(anyhow!("cone_angle_deg must be in range [0, 90]")));
            }
            if ray_count < 1 {
                return Some(Err(anyhow!("ray_count must be >= 1")));
            }
            if hidden_ratio < 0.0 || hidden_ratio > 1.0 {
                return Some(Err(anyhow!("hidden_ratio must be in range [0, 1]")));
            }
            Ok(Box::new(HideCoveredSkin {
                params: HiddenMaskParams {
                    cover_distance,
                    cone_angle_deg,
                    ray_count,
                    hidden_ratio,
                },
            }))
        }
        _ => Err(anyhow!("{HIDE_COVERED_SKIN_USAGE}")),
    })
}

const GENERATE_CHAIN_BONES_USAGE: &str = "generate_chain_bones expects generate_chain_bones=<mesh>[/<material>],<root humanoid role>[,<setting>=<value>...]";

const SUBMESH_ROLE_USAGE: &str = "submesh_role expects submesh_role=<role>:<mesh>[/<material>]";

const HIDE_COVERED_SKIN_USAGE: &str = "hide_covered_skin expects hide_covered_skin=<cover_distance>,<cone_angle_deg>,<ray_count>,<hidden_ratio>";
crate::batch_action!("generate_chain_bones", generate_chain_bones_parse);
crate::batch_action!("submesh_role", submesh_role_parse);
crate::batch_action!("hide_covered_skin", hide_covered_skin_parse);
