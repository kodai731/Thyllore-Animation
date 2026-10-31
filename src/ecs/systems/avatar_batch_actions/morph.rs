use crate::ecs::component::MorphWeights;
use crate::ecs::systems::batch_run_systems::{unit_action_parse, BatchAction};
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::world::{Entity, World};
use anyhow::{anyhow, Result};

pub(super) fn find_morph_entities(world: &World) -> Vec<Entity> {
    let mut entities: Vec<Entity> = world
        .iter_components::<MorphWeights>()
        .map(|(entity, _)| entity)
        .collect();
    entities.sort_unstable();
    entities
}

pub(super) fn send_to_morph_entities(world: &World, build_event: impl Fn(Entity) -> MorphEvent) {
    for entity in find_morph_entities(world) {
        world.send_command(build_event(entity));
    }
}

#[derive(Debug)]
pub struct SetMorphWeight {
    pub(super) channel: String,
    pub(super) weight: f32,
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

pub(super) fn morph_weight_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
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

pub(super) fn capture_expression_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
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
crate::batch_action!("morph_weight", morph_weight_parse);
crate::batch_action!("key_morph_weights", unit_action_parse::<KeyMorphWeights>);
crate::batch_action!("capture_expression", capture_expression_parse);
