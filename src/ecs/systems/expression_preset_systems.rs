use std::path::Path;

use thyllore_avatar_core::expression::components::preset::ExpressionLibrary;
use thyllore_avatar_core::expression::systems::library_io::{
    expression_library_path, load_library, save_library,
};
use thyllore_avatar_core::expression::systems::preset::{apply_preset, capture_preset};
use thyllore_avatar_core::vrchat::gesture::gesture_template_library;

use crate::asset::AssetStorage;
use crate::ecs::component::MorphWeights;
use crate::ecs::resource::{ExpressionLibraryState, ModelState};
use crate::ecs::systems::morph_weight_systems::for_each_morph_sibling;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn sync_expression_library(world: &mut World) {
    let Some(model_path) = world
        .get_resource::<ModelState>()
        .map(|model_state| model_state.model_path.clone())
    else {
        return;
    };
    if model_path.is_empty() {
        return;
    }

    let Some(mut state) = world.get_resource_mut::<ExpressionLibraryState>() else {
        return;
    };
    if state.source_model_path == model_path {
        return;
    }

    state.library = load_expression_library_or_templates(Path::new(&model_path));
    state.source_model_path = model_path;
}

fn load_expression_library_or_templates(model_path: &Path) -> ExpressionLibrary {
    let library_path = expression_library_path(model_path);
    if !library_path.exists() {
        return gesture_template_library();
    }

    match load_library(&library_path) {
        Ok(library) => library,
        Err(error) => {
            log_warn!(
                "Failed to load expression library {}: {}",
                library_path.display(),
                error
            );
            gesture_template_library()
        }
    }
}

pub fn apply_expression_preset(
    world: &mut World,
    entity: Entity,
    preset_index: usize,
    channel_names: &[String],
) {
    let Some(application) = world
        .get_resource::<ExpressionLibraryState>()
        .and_then(|state| {
            state
                .library
                .presets
                .get(preset_index)
                .map(|preset| apply_preset(preset, channel_names))
        })
    else {
        return;
    };

    if !application.unknown_channels.is_empty() {
        log_warn!(
            "Expression preset {} has unknown channels: {}",
            preset_index,
            application.unknown_channels.join(", ")
        );
    }

    let Some(morph_weights) = world.get_component_mut::<MorphWeights>(entity) else {
        return;
    };
    if morph_weights.weights.len() != application.weights.len() {
        log_warn!(
            "Cannot apply expression preset {}: {} weights for {} channels",
            preset_index,
            morph_weights.weights.len(),
            application.weights.len()
        );
        return;
    }
    morph_weights.weights = application.weights;
}

pub fn capture_expression_preset(
    world: &mut World,
    entity: Entity,
    name: &str,
    channel_names: &[String],
) {
    let Some(weights) = world
        .get_component::<MorphWeights>(entity)
        .map(|morph_weights| morph_weights.weights.clone())
    else {
        return;
    };
    if weights.len() != channel_names.len() {
        log_warn!(
            "Cannot capture expression preset {}: {} weights for {} channels",
            name,
            weights.len(),
            channel_names.len()
        );
        return;
    }

    let preset = capture_preset(name, channel_names, &weights);
    let Some(mut state) = world.get_resource_mut::<ExpressionLibraryState>() else {
        return;
    };
    let presets = &mut state.library.presets;
    match presets
        .iter_mut()
        .find(|existing| existing.name == preset.name)
    {
        Some(existing) => *existing = preset,
        None => presets.push(preset),
    }
}

pub fn save_expression_library(world: &World) {
    let Some(model_path) = world
        .get_resource::<ModelState>()
        .map(|model_state| model_state.model_path.clone())
    else {
        return;
    };
    if model_path.is_empty() {
        log_warn!("Cannot save expression library: no model loaded");
        return;
    }
    let Some(state) = world.get_resource::<ExpressionLibraryState>() else {
        return;
    };

    let library_path = expression_library_path(Path::new(&model_path));
    match save_library(&library_path, &state.library) {
        Ok(()) => log!("Saved expression library to {}", library_path.display()),
        Err(error) => log_warn!(
            "Failed to save expression library {}: {}",
            library_path.display(),
            error
        ),
    }
}

pub fn apply_expression_preset_on_siblings(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    entity: Entity,
    preset_index: usize,
) {
    for_each_morph_sibling(
        world,
        assets,
        graphics,
        entity,
        |world, sibling, channel_names| {
            apply_expression_preset(world, sibling, preset_index, channel_names);
        },
    );
}
