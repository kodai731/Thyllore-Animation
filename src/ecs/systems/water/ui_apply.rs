use super::{resolve_selected_water, ui_command::WaterUiCommand};
use crate::asset::AssetStorage;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::WaterRenderSettings;
use crate::ecs::systems::effect_edit::{apply_effect_preset, apply_effect_update};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

impl UiCommand for WaterUiCommand {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            WaterUiCommand::UpdateEffect { entity, effect } => {
                apply_effect_update::<WaterTorusEffect>(world, entity, *effect);
            }
            WaterUiCommand::ApplyPreset(name) => {
                let Some(target) = resolve_selected_water(world) else {
                    return;
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
