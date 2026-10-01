use anyhow::{anyhow, Result};

use crate::ecs::component::MorphWeights;
use crate::ecs::events::{UIEvent, UIEventQueue};
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

fn send_to_morph_entities(world: &mut World, build_event: impl Fn(Entity) -> UIEvent) {
    let entities = find_morph_entities(world);
    let mut ui_events = world.resource_mut::<UIEventQueue>();
    for entity in entities {
        ui_events.send(build_event(entity));
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
        send_to_morph_entities(world, |entity| UIEvent::SetMorphWeight {
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
        send_to_morph_entities(world, |entity| UIEvent::KeyMorphWeights { entity });
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
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::CaptureExpressionPreset {
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
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::AddSpringChainsByPrefix {
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
pub struct ExportAvatarSidecar;

impl BatchAction for ExportAvatarSidecar {
    fn name(&self) -> &'static str {
        "export_avatar_sidecar"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::ExportAvatarSidecar);
    }
}

#[derive(Debug, Default)]
pub struct ExportExpressionAnims;

impl BatchAction for ExportExpressionAnims {
    fn name(&self) -> &'static str {
        "export_expression_anims"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::ExportExpressionAnims);
    }
}

#[derive(Debug, Default)]
pub struct ExportMorphTrackAnim;

impl BatchAction for ExportMorphTrackAnim {
    fn name(&self) -> &'static str {
        "export_morph_track_anim"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::ExportMorphTrackAnim);
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

crate::batch_action!("morph_weight", morph_weight_parse);
crate::batch_action!("key_morph_weights", unit_action_parse::<KeyMorphWeights>);
crate::batch_action!("capture_expression", capture_expression_parse);
crate::batch_action!("add_spring_chains", add_spring_chains_parse);
crate::batch_action!(
    "export_avatar_sidecar",
    unit_action_parse::<ExportAvatarSidecar>
);
crate::batch_action!(
    "export_expression_anims",
    unit_action_parse::<ExportExpressionAnims>
);
crate::batch_action!(
    "export_morph_track_anim",
    unit_action_parse::<ExportMorphTrackAnim>
);
crate::batch_action!(
    "dump_morph_track_samples",
    unit_action_parse::<DumpMorphTrackSamples>
);

#[cfg(test)]
mod tests {
    use super::*;

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
        world.insert_resource(UIEventQueue::default());
        let first = world.entity().with_name("first").build();
        let second = world.entity().with_name("second").build();
        world.insert_component(first, MorphWeights { weights: vec![0.0] });
        world.insert_component(second, MorphWeights { weights: vec![0.0] });

        SetMorphWeight {
            channel: "smile".to_string(),
            weight: 0.5,
        }
        .apply(&mut world);

        let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
        let targets: Vec<Entity> = events
            .iter()
            .filter_map(|event| match event {
                UIEvent::SetMorphWeight {
                    entity,
                    channel,
                    weight,
                } if channel == "smile" && *weight == 0.5 => Some(*entity),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![first, second]);
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
}
