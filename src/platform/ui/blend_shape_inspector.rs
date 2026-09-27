use std::collections::HashSet;

use imgui::Condition;
use thyllore_avatar_core::expression::components::grouping::{ChannelGroup, ExpressionGrouping};
use thyllore_avatar_core::expression::systems::grouping::group_channels;

use crate::asset::AssetStorage;
use crate::ecs::component::MorphWeights;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{BlendShapeInspectorState, ExpressionLibraryState};
use crate::ecs::systems::{find_mesh_morph, find_morph_channel_names, find_morph_siblings};
use crate::ecs::world::{Children, Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn build_blend_shape_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    let morph_entities = collect_morph_entities(world, entity);
    if morph_entities.is_empty() {
        return;
    }

    let Some(mut inspector_state) = world.get_resource_mut::<BlendShapeInspectorState>() else {
        return;
    };

    let representatives = collect_sibling_representatives(world, &morph_entities, assets, graphics);

    for representative in representatives {
        let Some(morph_weights) = world.get_component::<MorphWeights>(representative) else {
            continue;
        };
        let Some(channel_names) = find_morph_channel_names(world, representative, assets, graphics)
        else {
            continue;
        };

        let header = format!(
            "Blend Shapes ({})###blend_shapes_{}",
            format_section_title(world, representative, assets, graphics),
            representative
        );
        if !ui.collapsing_header(&header, imgui::TreeNodeFlags::DEFAULT_OPEN) {
            continue;
        }

        let id_token = ui.push_id_int(representative as i32);
        build_blend_shape_toolbar(ui, ui_events, representative, &mut inspector_state);
        build_channel_groups(
            ui,
            ui_events,
            representative,
            &channel_names,
            &morph_weights.weights,
            &mut inspector_state,
        );
        build_presets_section(ui, ui_events, world, representative);
        id_token.end();
    }
}

fn collect_morph_entities(world: &World, entity: Entity) -> Vec<Entity> {
    let children = world
        .get_component::<Children>(entity)
        .map(|children| children.0.clone())
        .unwrap_or_default();

    std::iter::once(entity)
        .chain(children)
        .filter(|&candidate| world.has_component::<MorphWeights>(candidate))
        .collect()
}

fn collect_sibling_representatives(
    world: &World,
    morph_entities: &[Entity],
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Vec<Entity> {
    let mut covered_entities = HashSet::new();
    let mut representatives = Vec::new();
    for &morph_entity in morph_entities {
        if covered_entities.contains(&morph_entity) {
            continue;
        }
        covered_entities.extend(find_morph_siblings(world, morph_entity, assets, graphics));
        representatives.push(morph_entity);
    }
    representatives
}

fn format_section_title(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> String {
    find_mesh_morph(world, entity, assets, graphics)
        .map(|morph| morph.source_mesh.clone())
        .filter(|source_mesh| !source_mesh.is_empty())
        .unwrap_or_else(|| format!("entity {}", entity))
}

fn build_blend_shape_toolbar(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    inspector_state: &mut BlendShapeInspectorState,
) {
    ui.set_next_item_width(-1.0);
    ui.input_text("##blend_shape_search", &mut inspector_state.search)
        .hint("Search...")
        .build();

    ui.checkbox("Mirror L/R", &mut inspector_state.mirror_edit);
    ui.same_line();
    if ui.button("Reset") {
        ui_events.send(UIEvent::ResetMorphWeights { entity });
    }
    ui.same_line();
    if ui.button("Key") {
        ui_events.send(UIEvent::KeyMorphWeights { entity });
    }
}

fn build_channel_groups(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    channel_names: &[String],
    weights: &[f32],
    inspector_state: &mut BlendShapeInspectorState,
) {
    for group in group_channels(channel_names, &ExpressionGrouping::default()) {
        let visible_channels = filter_channels(&group, channel_names, &inspector_state.search);
        if visible_channels.is_empty() {
            continue;
        }

        let expanded_key = format!("{}/{}", entity, group.name);
        let is_expanded = *inspector_state
            .expanded
            .entry(expanded_key.clone())
            .or_insert(!group.collapsed);
        let Some(tree_token) = ui
            .tree_node_config(&group.name)
            .opened(is_expanded, Condition::Always)
            .push()
        else {
            inspector_state.expanded.insert(expanded_key, false);
            continue;
        };
        inspector_state.expanded.insert(expanded_key, true);

        for channel in visible_channels {
            build_channel_slider(
                ui,
                ui_events,
                entity,
                channel,
                &channel_names[channel],
                weights,
            );
        }
        tree_token.end();
    }
}

fn filter_channels(group: &ChannelGroup, channel_names: &[String], search: &str) -> Vec<usize> {
    let search = search.to_lowercase();
    group
        .channel_indices
        .iter()
        .copied()
        .filter(|&channel| channel_names[channel].to_lowercase().contains(&search))
        .collect()
}

fn build_channel_slider(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    channel: usize,
    channel_name: &str,
    weights: &[f32],
) {
    let Some(&current_weight) = weights.get(channel) else {
        return;
    };

    let mut weight = current_weight;
    let label = format!("{}##channel_{}", channel_name, channel);
    if ui.slider(&label, 0.0f32, 1.0f32, &mut weight) {
        ui_events.send(UIEvent::SetMorphWeight {
            entity,
            channel,
            weight,
        });
    }
}

fn build_presets_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    entity: Entity,
) {
    let Some(mut library_state) = world.get_resource_mut::<ExpressionLibraryState>() else {
        return;
    };
    let Some(tree_token) = ui.tree_node("Presets") else {
        return;
    };

    for (index, preset) in library_state.library.presets.iter().enumerate() {
        let weights_label = if preset.weights.is_empty() {
            "(empty)".to_string()
        } else {
            format!("({} weights)", preset.weights.len())
        };
        ui.text(format!("{} {}", preset.name, weights_label));
        ui.same_line();
        if ui.small_button(format!("Apply##preset_{}", index)) {
            ui_events.send(UIEvent::ApplyExpressionPreset {
                entity,
                preset_index: index,
            });
        }
    }

    ui.separator();
    ui.input_text("##capture_name", &mut library_state.capture_name)
        .hint("Preset name")
        .build();
    ui.same_line();
    if ui.button("Capture") && !library_state.capture_name.is_empty() {
        ui_events.send(UIEvent::CaptureExpressionPreset {
            entity,
            name: library_state.capture_name.clone(),
        });
    }
    if ui.button("Save presets") {
        ui_events.send(UIEvent::SaveExpressionLibrary);
    }

    tree_token.end();
}
