use std::any::Any;

use super::ui_command::WindUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::write_wind_transform;
use crate::ecs::world::World;

pub fn apply_wind_ui_command(world: &mut World, _assets: &mut AssetStorage, command: &dyn Any) {
    let Some(command) = command.downcast_ref::<WindUiCommand>() else {
        return;
    };

    match command {
        WindUiCommand::UpdateEffect { entity, effect } => {
            if !world.has_component::<WindTornadoEffect>(*entity) {
                return;
            }
            write_wind_transform(world, *entity, effect.position, effect.rotation);
            if let Some(current) = world.get_component_mut::<WindTornadoEffect>(*entity) {
                *current = effect.as_ref().clone();
            }
        }
        WindUiCommand::ApplyPreset(name) => {
            crate::ecs::systems::apply_wind_preset_to_selected(world, name);
        }
        WindUiCommand::UpdateRenderSettings(new_settings) => {
            if let Some(mut settings) = world.get_resource_mut::<WindRenderSettings>() {
                *settings = *new_settings;
            }
        }
    }
}
