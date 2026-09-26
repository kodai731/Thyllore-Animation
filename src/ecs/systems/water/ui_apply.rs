use super::{resolve_selected_water, ui_command::WaterUiCommand};
use crate::asset::AssetStorage;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::systems::effect_edit::{apply_effect_preset, apply_effect_update};
use crate::ecs::world::World;
use crate::hooks::effect_ui_event::EffectUiQueue;

pub fn dispatch_water_ui_events(world: &mut World, _assets: &mut AssetStorage) {
    let commands = match world.get_resource_mut::<EffectUiQueue<WaterUiCommand>>() {
        Some(mut queue) => queue.drain(),
        None => return,
    };

    for command in commands {
        match command {
            WaterUiCommand::UpdateEffect { entity, effect } => {
                apply_effect_update::<WaterTorusEffect>(world, entity, *effect);
            }
            WaterUiCommand::ApplyPreset(name) => {
                let Some(target) = resolve_selected_water(world) else {
                    continue;
                };
                apply_effect_preset::<WaterTorusEffect>(world, target, &name);
            }
            WaterUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<WaterRenderSettings>() {
                    *settings = new_settings;
                }
            }
        }
    }
}
