use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::systems::{
    apply_expression_preset, capture_expression_preset, find_morph_channel_names,
    find_morph_siblings, reset_morph_weights, save_expression_library, set_morph_weight,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn dispatch_morph_weight_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    for event in events {
        match event {
            UIEvent::SetMorphWeight {
                entity,
                channel,
                weight,
            } => {
                let Some(channel_name) = find_morph_channel_names(world, *entity, assets, graphics)
                    .and_then(|names| names.get(*channel).cloned())
                else {
                    continue;
                };
                let siblings = find_morph_siblings(world, *entity, assets, graphics);
                for sibling in siblings {
                    let channel_names = find_morph_channel_names(world, sibling, assets, graphics)
                        .unwrap_or_default();
                    let Some(sibling_channel) =
                        channel_names.iter().position(|name| *name == channel_name)
                    else {
                        continue;
                    };
                    set_morph_weight(world, sibling, sibling_channel, *weight, &channel_names);
                }
            }
            UIEvent::ResetMorphWeights { entity } => {
                let siblings = find_morph_siblings(world, *entity, assets, graphics);
                for sibling in siblings {
                    let channel_names = find_morph_channel_names(world, sibling, assets, graphics)
                        .unwrap_or_default();
                    reset_morph_weights(world, sibling, &channel_names);
                }
            }
            UIEvent::ApplyExpressionPreset {
                entity,
                preset_index,
            } => {
                let siblings = find_morph_siblings(world, *entity, assets, graphics);
                for sibling in siblings {
                    let channel_names = find_morph_channel_names(world, sibling, assets, graphics)
                        .unwrap_or_default();
                    apply_expression_preset(world, sibling, *preset_index, &channel_names);
                }
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
