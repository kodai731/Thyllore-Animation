use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::systems::{
    apply_expression_preset, capture_expression_preset, find_morph_channel_names,
    reset_morph_weights, save_expression_library, set_morph_weight, sync_expression_library,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn dispatch_morph_weight_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    sync_expression_library(world);

    for event in events {
        match event {
            UIEvent::SetMorphWeight {
                entity,
                channel,
                weight,
            } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                set_morph_weight(world, *entity, *channel, *weight, &channel_names);
            }
            UIEvent::ResetMorphWeights { entity } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                reset_morph_weights(world, *entity, &channel_names);
            }
            UIEvent::ApplyExpressionPreset {
                entity,
                preset_index,
            } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                apply_expression_preset(world, *entity, *preset_index, &channel_names);
            }
            UIEvent::CaptureExpressionPreset { entity, name } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                capture_expression_preset(world, *entity, name, &channel_names);
            }
            UIEvent::SaveExpressionLibrary => save_expression_library(world),
            _ => {}
        }
    }
}
