use std::any::Any;

use super::ui_command::LightningUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::LightningEffect;
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::{
    clear_lightning_target, resolve_selected_lightning, spawn_lightning_target,
    spawn_lightning_waypoint, write_lightning_transform,
};
use crate::ecs::world::World;

pub fn apply_lightning_ui_command(
    world: &mut World,
    _assets: &mut AssetStorage,
    command: &dyn Any,
) {
    let Some(command) = command.downcast_ref::<LightningUiCommand>() else {
        return;
    };

    match command {
        LightningUiCommand::UpdateEffect { entity, effect } => {
            if !world.has_component::<LightningEffect>(*entity) {
                return;
            }
            write_lightning_transform(world, *entity, effect.position, effect.rotation);
            if let Some(current) = world.get_component_mut::<LightningEffect>(*entity) {
                *current = effect.as_ref().clone();
            }
        }
        LightningUiCommand::ApplyPreset(name) => {
            crate::ecs::systems::apply_lightning_preset_to_selected(world, name);
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
                crate::ecs::systems::remove_lightning_waypoint(world, lightning, *index);
            }
        }
        LightningUiCommand::UpdateRenderSettings(new_settings) => {
            if let Some(mut settings) = world.get_resource_mut::<LightningRenderSettings>() {
                *settings = *new_settings;
            }
        }
    }
}
