use std::any::Any;

use super::ui_command::WaterUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::systems::{resolve_selected_water, write_water_transform};
use crate::ecs::world::World;

pub fn apply_water_ui_command(world: &mut World, _assets: &mut AssetStorage, command: &dyn Any) {
    let Some(command) = command.downcast_ref::<WaterUiCommand>() else {
        return;
    };

    match command {
        WaterUiCommand::UpdateEffect(effect) => {
            let Some(target) = resolve_selected_water(world) else {
                return;
            };
            write_water_transform(world, target, effect.position, effect.rotation);
            if let Some(current) = world.get_component_mut::<WaterTorusEffect>(target) {
                *current = effect.as_ref().clone();
            }
        }
        WaterUiCommand::ApplyPreset(name) => {
            crate::ecs::systems::apply_water_preset_to_selected(world, name);
        }
        WaterUiCommand::UpdateRenderSettings(new_settings) => {
            if let Some(mut settings) = world.get_resource_mut::<WaterRenderSettings>() {
                *settings = new_settings.clone();
            }
        }
    }
}
