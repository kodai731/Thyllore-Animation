use super::ui_command::LightningUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::LightningEffect;
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::{
    clear_lightning_target, resolve_selected_lightning, spawn_lightning_target,
    spawn_lightning_waypoint, write_lightning_transform,
};
use crate::ecs::world::World;
use crate::hooks::effect_ui_event::EffectUiQueue;

pub fn dispatch_lightning_ui_events(world: &mut World, _assets: &mut AssetStorage) {
    let commands = match world.get_resource_mut::<EffectUiQueue<LightningUiCommand>>() {
        Some(mut queue) => queue.drain(),
        None => return,
    };

    for command in commands {
        match command {
            LightningUiCommand::UpdateEffect { entity, effect } => {
                if !world.has_component::<LightningEffect>(entity) {
                    continue;
                }
                write_lightning_transform(world, entity, effect.position, effect.rotation);
                if let Some(current) = world.get_component_mut::<LightningEffect>(entity) {
                    *current = *effect;
                }
            }
            LightningUiCommand::ApplyPreset(name) => {
                crate::ecs::systems::apply_lightning_preset_to_selected(world, &name);
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
