use std::path::PathBuf;

use anyhow::{anyhow, Result};

use thyllore_avatar_core::garment::components::submesh_role::{SubmeshKey, SubmeshRole};

use crate::ecs::component::CapsuleFitParams;
use crate::ecs::component::MorphWeights;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::systems::phases::event_dispatch::spring_bone::DumpSpringTails;
use crate::ecs::world::{Entity, World};

use super::avatar_export_systems::dump_morph_track_samples;
use super::batch_run_systems::{unit_action_parse, BatchAction};

fn find_morph_entities(world: &World) -> Vec<Entity> {
    let mut entities: Vec<Entity> = world
        .iter_components::<MorphWeights>()
        .map(|(entity, _)| entity)
        .collect();
    entities.sort_unstable();
    entities
}

fn send_to_morph_entities(world: &World, build_event: impl Fn(Entity) -> MorphEvent) {
    for entity in find_morph_entities(world) {
        world.send_command(build_event(entity));
    }
}

#[derive(Debug)]
pub struct SetMorphWeight {
    channel: String,
    weight: f32,
}

impl BatchAction for SetMorphWeight {
    fn name(&self) -> &'static str {
        "morph_weight"
    }
    fn apply(&self, world: &mut World) {
        send_to_morph_entities(world, |entity| MorphEvent::SetMorphWeight {
            entity,
            channel: self.channel.clone(),
            weight: self.weight,
        });
    }
}

fn morph_weight_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("morph_weight=")?;
    let parsed = spec
        .rsplit_once(':')
        .and_then(|(channel, weight)| Some((channel.trim(), weight.trim().parse::<f32>().ok()?)))
        .filter(|(channel, weight)| !channel.is_empty() && weight.is_finite());

    Some(match parsed {
        Some((channel, weight)) => Ok(Box::new(SetMorphWeight {
            channel: channel.to_string(),
            weight,
        })),
        None => Err(anyhow!(
            "morph_weight expects morph_weight=<channel>:<weight>, got '{text}'"
        )),
    })
}

#[derive(Debug, Default)]
pub struct KeyMorphWeights;

impl BatchAction for KeyMorphWeights {
    fn name(&self) -> &'static str {
        "key_morph_weights"
    }
    fn apply(&self, world: &mut World) {
        send_to_morph_entities(world, |entity| MorphEvent::KeyMorphWeights { entity });
    }
}

#[derive(Debug)]
pub struct CaptureExpression {
    preset_name: String,
}

impl BatchAction for CaptureExpression {
    fn name(&self) -> &'static str {
        "capture_expression"
    }
    fn apply(&self, world: &mut World) {
        let Some(entity) = find_morph_entities(world).into_iter().next() else {
            log_warn!("capture_expression: no entity has morph weights");
            return;
        };
        world.send_command(MorphEvent::CaptureExpressionPreset {
            entity,
            name: self.preset_name.clone(),
        });
    }
}

fn capture_expression_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let preset_name = text.strip_prefix("capture_expression=")?.trim();
    Some(if preset_name.is_empty() {
        Err(anyhow!(
            "capture_expression expects capture_expression=<name>"
        ))
    } else {
        Ok(Box::new(CaptureExpression {
            preset_name: preset_name.to_string(),
        }))
    })
}

#[derive(Debug)]
pub struct AddSpringChains {
    prefix: String,
}

impl BatchAction for AddSpringChains {
    fn name(&self) -> &'static str {
        "add_spring_chains"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::AddSpringChainsByPrefix {
            prefix: self.prefix.clone(),
        });
    }
}

fn add_spring_chains_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let prefix = text.strip_prefix("add_spring_chains=")?.trim();
    Some(if prefix.is_empty() {
        Err(anyhow!(
            "add_spring_chains expects add_spring_chains=<bone name prefix>"
        ))
    } else {
        Ok(Box::new(AddSpringChains {
            prefix: prefix.to_string(),
        }))
    })
}

#[derive(Debug, Default)]
pub struct ExportUnityAvatar;

impl BatchAction for ExportUnityAvatar {
    fn name(&self) -> &'static str {
        "export_unity_avatar"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::ExportUnityAvatar);
    }
}

#[derive(Debug, Default)]
pub struct DumpMorphTrackSamples;

impl BatchAction for DumpMorphTrackSamples {
    fn name(&self) -> &'static str {
        "dump_morph_track_samples"
    }
    fn apply(&self, world: &mut World) {
        dump_morph_track_samples(world);
    }
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

fn dump_spring_tails_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
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

const FIT_BODY_COLLIDERS_USAGE: &str = "fit_body_colliders expects fit_body_colliders=<min_weight>,<radius_percentile>,<radius_scale>,<min_vertex_count>";

fn fit_body_colliders_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("fit_body_colliders=")?;
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 4 {
        return Some(Err(anyhow!("{FIT_BODY_COLLIDERS_USAGE}")));
    }
    let parsed = (
        parts[0].trim().parse::<f32>().ok(),
        parts[1].trim().parse::<f32>().ok(),
        parts[2].trim().parse::<f32>().ok(),
        parts[3].trim().parse::<usize>().ok(),
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
pub struct SubmeshRoleAction {
    role: SubmeshRole,
    mesh: String,
    material: String,
}

impl BatchAction for SubmeshRoleAction {
    fn name(&self) -> &'static str {
        "submesh_role"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::SetSubmeshRole {
            key: SubmeshKey {
                mesh: self.mesh.clone(),
                material: self.material.clone(),
            },
            role: self.role,
        });
    }
}

fn submesh_role_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("submesh_role=")?;
    let (role_str, rest) = match spec.split_once(':') {
        Some(parts) => parts,
        None => return Some(Err(anyhow!("submesh_role expects role:mesh/material"))),
    };
    let (mesh, material) = match rest.rsplit_once('/') {
        Some((m, mat)) => (m.to_string(), mat.to_string()),
        None => (rest.to_string(), String::new()),
    };
    match serde_json::from_value::<SubmeshRole>(serde_json::Value::String(role_str.to_string())) {
        Ok(role) => Some(Ok(Box::new(SubmeshRoleAction {
            role,
            mesh,
            material,
        }))),
        Err(e) => Some(Err(anyhow!("unknown submesh role {:?}: {}", role_str, e))),
    }
}

crate::batch_action!("morph_weight", morph_weight_parse);
crate::batch_action!("key_morph_weights", unit_action_parse::<KeyMorphWeights>);
crate::batch_action!("capture_expression", capture_expression_parse);
crate::batch_action!("add_spring_chains", add_spring_chains_parse);
crate::batch_action!("fit_body_colliders", fit_body_colliders_parse);
crate::batch_action!(
    "export_unity_avatar",
    unit_action_parse::<ExportUnityAvatar>
);
crate::batch_action!(
    "dump_morph_track_samples",
    unit_action_parse::<DumpMorphTrackSamples>
);
crate::batch_action!("dump_spring_tails", dump_spring_tails_parse);
crate::batch_action!("submesh_role", submesh_role_parse);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    #[test]
    fn test_morph_weight_parses_channel_names_with_dots_and_colons() {
        let action = morph_weight_parse("morph_weight=vrc.v_aa:0.25")
            .expect("prefix matches")
            .expect("spec is valid");

        assert_eq!(
            format!("{:?}", action),
            "SetMorphWeight { channel: \"vrc.v_aa\", weight: 0.25 }"
        );
    }

    #[test]
    fn test_morph_weight_rejects_malformed_specs() {
        assert!(morph_weight_parse("key_morph_weights").is_none());
        for spec in [
            "morph_weight=smile",
            "morph_weight=:0.5",
            "morph_weight=smile:x",
        ] {
            assert!(
                morph_weight_parse(spec).expect("prefix matches").is_err(),
                "{spec} must be rejected"
            );
        }
    }

    #[test]
    fn test_morph_weight_reaches_every_morph_entity_by_channel_name() {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        let first = world.entity().with_name("first").build();
        let second = world.entity().with_name("second").build();
        world.insert_component(first, MorphWeights { weights: vec![0.0] });
        world.insert_component(second, MorphWeights { weights: vec![0.0] });

        SetMorphWeight {
            channel: "smile".to_string(),
            weight: 0.5,
        }
        .apply(&mut world);

        let sent: Vec<String> = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        let expected: Vec<String> = [first, second]
            .into_iter()
            .map(|entity| {
                format!(
                    "{:?}",
                    MorphEvent::SetMorphWeight {
                        entity,
                        channel: "smile".to_string(),
                        weight: 0.5,
                    }
                )
            })
            .collect();
        assert_eq!(sent, expected);
    }

    #[test]
    fn test_named_actions_reject_empty_values() {
        assert!(capture_expression_parse("capture_expression=")
            .expect("prefix matches")
            .is_err());
        assert!(add_spring_chains_parse("add_spring_chains= ")
            .expect("prefix matches")
            .is_err());
    }

    #[test]
    fn test_fit_body_colliders_parses_correct_format() {
        let action = fit_body_colliders_parse("fit_body_colliders=0.5,0.9,0.85,8")
            .expect("prefix matches")
            .expect("spec is valid");

        assert_eq!(
            format!("{:?}", action),
            "FitBodyColliders { params: CapsuleFitParams { min_weight: 0.5, radius_percentile: 0.9, radius_scale: 0.85, min_vertex_count: 8 } }"
        );
    }

    #[test]
    fn test_fit_body_colliders_rejects_malformed_specs() {
        assert!(fit_body_colliders_parse("fit_body_colliders=0.5,0.9,0.85")
            .expect("prefix matches")
            .is_err());
        assert!(fit_body_colliders_parse("fit_body_colliders=0.5,0.9")
            .expect("prefix matches")
            .is_err());
        assert!(fit_body_colliders_parse("fit_body_colliders=a,0.9,0.85,8")
            .expect("prefix matches")
            .is_err());
    }

    #[test]
    fn test_dump_spring_tails_parses_correct_format() {
        let action = dump_spring_tails_parse("dump_spring_tails=output.csv,60.0")
            .expect("prefix matches")
            .expect("spec is valid");

        assert_eq!(action.name(), "dump_spring_tails");
    }

    #[test]
    fn test_dump_spring_tails_rejects_malformed_specs() {
        assert!(dump_spring_tails_parse("dump_spring_tails=output.csv")
            .expect("prefix matches")
            .is_err());
        assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,abc")
            .expect("prefix matches")
            .is_err());
        assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,0")
            .expect("prefix matches")
            .is_err());
        assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,-1")
            .expect("prefix matches")
            .is_err());
    }

    #[test]
    fn test_submesh_role_parses_correct_format() {
        let action: Box<dyn BatchAction> = submesh_role_parse("submesh_role=hair:Body/Hair_mat")
            .expect("prefix matches")
            .expect("spec is valid");

        assert_eq!(action.name(), "submesh_role");
        assert_eq!(
            format!("{:?}", action),
            "SubmeshRoleAction { role: Hair, mesh: \"Body\", material: \"Hair_mat\" }"
        );
    }

    #[test]
    fn test_submesh_role_rejects_malformed_specs() {
        assert!(submesh_role_parse("submesh_role=fur:Body/x")
            .expect("prefix matches")
            .is_err());
        assert!(submesh_role_parse("submesh_role=skin")
            .expect("prefix matches")
            .is_err());
    }
}
