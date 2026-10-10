use super::ui_command::LightningUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::LightningEffect;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::effect_edit::{apply_effect_preset, apply_effect_update};
use crate::ecs::systems::{
    clear_lightning_target, resolve_selected_lightning, spawn_lightning_target,
    spawn_lightning_waypoint,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

impl UiCommand for LightningUiCommand {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            LightningUiCommand::UpdateEffect { entity, effect } => {
                apply_effect_update::<LightningEffect>(world, entity, *effect);
            }
            LightningUiCommand::ApplyPreset(name) => {
                let Some(target) = resolve_selected_lightning(world) else {
                    return;
                };
                apply_effect_preset::<LightningEffect>(world, target, &name);
            }
            LightningUiCommand::AddTarget => {
                if let Some(lightning) = resolve_selected_lightning(world) {
                    spawn_lightning_target(world, lightning);
                }
            }
            LightningUiCommand::ClearTarget => {
                if let Some(lightning) = resolve_selected_lightning(world) {
                    clear_lightning_target(world, lightning);
                }
            }
            LightningUiCommand::AddWaypoint => {
                if let Some(lightning) = resolve_selected_lightning(world) {
                    spawn_lightning_waypoint(world, lightning);
                }
            }
            LightningUiCommand::RemoveWaypoint(index) => {
                if let Some(lightning) = resolve_selected_lightning(world) {
                    crate::ecs::systems::remove_lightning_waypoint(world, lightning, index);
                }
            }
            LightningUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<LightningRenderSettings>() {
                    *settings = new_settings;
                }
            }
        }
    }
}
