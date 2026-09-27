use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::systems::{find_morph_channel_names, reset_morph_weights, set_morph_weight};
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
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                set_morph_weight(world, *entity, *channel, *weight, &channel_names);
            }
            UIEvent::ResetMorphWeights { entity } => {
                let channel_names =
                    find_morph_channel_names(world, *entity, assets, graphics).unwrap_or_default();
                reset_morph_weights(world, *entity, &channel_names);
            }
            _ => {}
        }
    }
}
