use super::{resolve_selected_wind, ui_command::WindUiCommand};
use crate::asset::AssetStorage;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::effect_edit::{apply_effect_preset, apply_effect_update};
use crate::ecs::world::World;
use crate::hooks::effect_ui_event::EffectUiQueue;

pub fn dispatch_wind_ui_events(world: &mut World, _assets: &mut AssetStorage) {
    let commands = match world.get_resource_mut::<EffectUiQueue<WindUiCommand>>() {
        Some(mut queue) => queue.drain(),
        None => return,
    };

    for command in commands {
        match command {
            WindUiCommand::UpdateEffect { entity, effect } => {
                apply_effect_update::<WindTornadoEffect>(world, entity, *effect);
            }
            WindUiCommand::ApplyPreset(name) => {
                let Some(target) = resolve_selected_wind(world) else {
                    continue;
                };
                apply_effect_preset::<WindTornadoEffect>(world, target, &name);
            }
            WindUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<WindRenderSettings>() {
                    *settings = new_settings;
                }
            }
        }
    }
}
