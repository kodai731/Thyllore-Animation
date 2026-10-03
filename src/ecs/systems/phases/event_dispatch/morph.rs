use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::systems::{
    apply_expression_preset_on_siblings, capture_expression_preset, find_morph_channel_names,
    key_morph_weights, remove_expression_preset, reset_morph_weights_on_siblings,
    save_expression_library, set_morph_weight_on_siblings,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn dispatch_morph_weight_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    for event in events {
        match event {
            UIEvent::SetMorphWeight {
                entity,
                channel,
                weight,
            } => {
                set_morph_weight_on_siblings(world, assets, graphics, *entity, channel, *weight);
            }
            UIEvent::ResetMorphWeights { entity } => {
                reset_morph_weights_on_siblings(world, assets, graphics, *entity);
            }
            UIEvent::ApplyExpressionPreset {
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
            UIEvent::CaptureExpressionPreset { entity, name } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                capture_expression_preset(world, *entity, name, &channel_names);
            }
            UIEvent::RemoveExpressionPreset { preset_index } => {
                remove_expression_preset(world, *preset_index);
            }
            UIEvent::SaveExpressionLibrary => save_expression_library(world),
            UIEvent::KeyMorphWeights { entity } => {
                key_morph_weights(world, assets, graphics, *entity);
            }
            _ => {}
        }
    }
}
