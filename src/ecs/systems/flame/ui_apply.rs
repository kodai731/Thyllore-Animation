use super::ui_command::FlameUiCommand;
use crate::asset::AssetStorage;
use crate::ecs::component::{FlameEffect, FlameTrail};
use crate::ecs::resource::{FlameRenderSettings, FlameUIState};
use crate::ecs::systems::{resolve_selected_flame, write_flame_transform};
use crate::ecs::world::World;
use crate::hooks::effect_ui_event::EffectUiQueue;

pub fn dispatch_flame_ui_events(world: &mut World, _assets: &mut AssetStorage) {
    let commands = match world.get_resource_mut::<EffectUiQueue<FlameUiCommand>>() {
        Some(mut queue) => queue.drain(),
        None => return,
    };

    for command in commands {
        match command {
            FlameUiCommand::UpdateEffect { entity, effect } => {
                if !world.has_component::<FlameEffect>(entity) {
                    continue;
                }
                write_flame_transform(world, entity, effect.position, effect.rotation);
                if let Some(current) = world.get_component_mut::<FlameEffect>(entity) {
                    *current = *effect;
                }
            }
            FlameUiCommand::UpdateBaked(baked) => {
                let Some(target) = resolve_selected_flame(world) else {
                    continue;
                };
                world.insert_component(target, *baked);
            }
            FlameUiCommand::ApplyPreset(name) => {
                crate::ecs::systems::apply_flame_preset_to_selected(world, &name);
            }
            FlameUiCommand::ApplyTextureFit {
                path,
                blend,
                groups,
                profile,
            } => {
                crate::ecs::systems::apply_flame_texture_fit_to_selected(
                    world,
                    &path,
                    blend,
                    thyllore_effect_core::TextureFitGroups {
                        silhouette: groups[0],
                        color: groups[1],
                        turbulence: groups[2],
                        tilt: groups[3],
                    },
                    profile,
                    "ui",
                );
            }
            FlameUiCommand::ApplyStyle { path, groups } => {
                crate::ecs::systems::apply_flame_style_to_selected(
                    world,
                    &path,
                    thyllore_effect_core::StyleGroups {
                        motion: groups[0],
                        texture: groups[1],
                        optics: groups[2],
                    },
                );
            }
            FlameUiCommand::SaveStyle { name } => {
                if crate::ecs::systems::save_flame_style_of_selected(world, &name).is_some() {
                    if let Some(mut flame_ui) = world.get_resource_mut::<FlameUIState>() {
                        flame_ui.style_scan_done = false;
                        flame_ui.style_scan.clear();
                    }
                }
            }
            FlameUiCommand::UpdateRenderSettings(new_settings) => {
                if let Some(mut settings) = world.get_resource_mut::<FlameRenderSettings>() {
                    *settings = new_settings;
                }
            }
            FlameUiCommand::UpdateTrailEnabled(enabled) => {
                let Some(target) = resolve_selected_flame(world) else {
                    continue;
                };
                if let Some(trail) = world.get_component_mut::<FlameTrail>(target) {
                    trail.state.enabled = enabled;
                } else {
                    world.insert_component(
                        target,
                        FlameTrail {
                            state: thyllore_effect_core::FlameTrailState {
                                enabled,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    );
                }
            }
            FlameUiCommand::UpdateTrailFade(fade) => {
                let Some(target) = resolve_selected_flame(world) else {
                    continue;
                };
                if let Some(trail) = world.get_component_mut::<FlameTrail>(target) {
                    trail.state.fade_seconds = fade;
                } else {
                    world.insert_component(
                        target,
                        FlameTrail {
                            state: thyllore_effect_core::FlameTrailState {
                                fade_seconds: fade,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    );
                }
            }
            FlameUiCommand::DumpWallProbe { viewport_size } => {
                crate::ecs::systems::perform_flame_wall_probe_dump(world, viewport_size);
            }
        }
    }
}
