use super::ui_command::WaterUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::systems::write_water_transform;
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
                if !world.has_component::<WaterTorusEffect>(entity) {
                    continue;
                }
                write_water_transform(world, entity, effect.position, effect.rotation);
                if let Some(current) = world.get_component_mut::<WaterTorusEffect>(entity) {
                    *current = *effect;
                }
            }
            WaterUiCommand::ApplyPreset(name) => {
                crate::ecs::systems::apply_water_preset_to_selected(world, &name);
            }
            WaterUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<WaterRenderSettings>() {
                    *settings = new_settings;
                }
            }
        }
    }
}
