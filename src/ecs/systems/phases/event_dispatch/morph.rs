use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::systems::{
    apply_expression_preset_on_siblings, capture_expression_preset, find_morph_channel_names,
    key_morph_weights, remove_expression_preset, reset_morph_weights_on_siblings,
    save_expression_library, set_morph_weight_on_siblings,
};
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum MorphEvent {
    SetMorphWeight {
        entity: Entity,
        channel: String,
        weight: f32,
    },
    ResetMorphWeights {
        entity: Entity,
    },
    ApplyExpressionPreset {
        entity: Entity,
        preset_index: usize,
    },
    CaptureExpressionPreset {
        entity: Entity,
        name: String,
    },
    RemoveExpressionPreset {
        preset_index: usize,
    },
    SaveExpressionLibrary,
    KeyMorphWeights {
        entity: Entity,
    },
}

impl UiCommand for MorphEvent {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    ) {
        dispatch_morph_weight_events(&[*self], world, assets, graphics);
    }
}

fn dispatch_morph_weight_events(
    events: &[MorphEvent],
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    for event in events {
        match event {
            MorphEvent::SetMorphWeight {
                entity,
                channel,
                weight,
            } => {
                set_morph_weight_on_siblings(world, assets, graphics, *entity, channel, *weight);
            }
            MorphEvent::ResetMorphWeights { entity } => {
                reset_morph_weights_on_siblings(world, assets, graphics, *entity);
            }
            MorphEvent::ApplyExpressionPreset {
                entity,
                preset_index,
            } => {
                apply_expression_preset_on_siblings(
                    world,
                    assets,
                    graphics,
                    *entity,
                    *preset_index,
                );
            }
            MorphEvent::CaptureExpressionPreset { entity, name } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                capture_expression_preset(world, *entity, name, &channel_names);
            }
            MorphEvent::RemoveExpressionPreset { preset_index } => {
                remove_expression_preset(world, *preset_index);
            }
            MorphEvent::SaveExpressionLibrary => save_expression_library(world),
            MorphEvent::KeyMorphWeights { entity } => {
                key_morph_weights(world, assets, graphics, *entity);
            }
        }
    }
}
