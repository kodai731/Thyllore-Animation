use super::ui_command::LightningUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::LightningEffect;
use crate::ecs::events::UiCommand;
use crate::ecs::systems::effect_edit::resolve_selected_effect;
use crate::ecs::systems::{
    clear_lightning_target, spawn_lightning_target, spawn_lightning_waypoint,
};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

impl UiCommand for LightningUiCommand {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            LightningUiCommand::AddTarget => {
                if let Some(lightning) = resolve_selected_effect::<LightningEffect>(world) {
                    spawn_lightning_target(world, lightning);
                }
            }
            LightningUiCommand::ClearTarget => {
                if let Some(lightning) = resolve_selected_effect::<LightningEffect>(world) {
                    clear_lightning_target(world, lightning);
                }
            }
            LightningUiCommand::AddWaypoint => {
                if let Some(lightning) = resolve_selected_effect::<LightningEffect>(world) {
                    spawn_lightning_waypoint(world, lightning);
                }
            }
            LightningUiCommand::RemoveWaypoint(index) => {
                if let Some(lightning) = resolve_selected_effect::<LightningEffect>(world) {
                    crate::ecs::systems::remove_lightning_waypoint(world, lightning, index);
                }
            }
        }
    }
}
