use super::{resolve_selected_wind, ui_command::WindUiCommand};
use crate::asset::AssetStorage;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::effect_edit::{apply_effect_preset, apply_effect_update};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

impl UiCommand for WindUiCommand {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            WindUiCommand::UpdateEffect { entity, effect } => {
                apply_effect_update::<WindTornadoEffect>(world, entity, *effect);
            }
            WindUiCommand::ApplyPreset(name) => {
                let Some(target) = resolve_selected_wind(world) else {
                    return;
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
