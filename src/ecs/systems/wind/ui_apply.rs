use super::ui_command::WindUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::write_wind_transform;
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
                if !world.has_component::<WindTornadoEffect>(entity) {
                    continue;
                }
                write_wind_transform(world, entity, effect.position, effect.rotation);
                if let Some(current) = world.get_component_mut::<WindTornadoEffect>(entity) {
                    *current = *effect;
                }
            }
            WindUiCommand::ApplyPreset(name) => {
                crate::ecs::systems::apply_wind_preset_to_selected(world, &name);
            }
            WindUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<WindRenderSettings>() {
                    *settings = new_settings;
                }
            }
        }
    }
}
