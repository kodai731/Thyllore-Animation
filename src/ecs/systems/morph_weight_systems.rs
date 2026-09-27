use std::path::Path;

use thyllore_avatar_core::expression::components::grouping::ExpressionGrouping;
use thyllore_avatar_core::expression::components::preset::ExpressionLibrary;
use thyllore_avatar_core::expression::systems::grouping::group_channels;
use thyllore_avatar_core::expression::systems::library_io::{
    expression_library_path, load_library, save_library,
};
use thyllore_avatar_core::expression::systems::preset::{apply_preset, capture_preset};
use thyllore_avatar_core::expression::systems::side::find_mirror_channel;
use thyllore_avatar_core::vrchat::gesture::gesture_template_library;
use thyllore_model_core::MeshMorph;

use crate::asset::AssetStorage;
use crate::ecs::component::{AppliedMorphWeights, MorphWeights};
use crate::ecs::resource::{BlendShapeInspectorState, ExpressionLibraryState, ModelState};
use crate::ecs::world::{Entity, MeshRef, Parent, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::mesh_buffer::MeshBuffer;

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

pub fn apply_morph_weights(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &mut GraphicsResources,
) -> Vec<usize> {
    let pending = collect_pending_morph_updates(world, assets, graphics);

    let mut updated_meshes = Vec::with_capacity(pending.len());
    for (entity, mesh_idx, weights) in pending {
        apply_morph_to_mesh(&mut graphics.meshes[mesh_idx], &weights);
        world.insert_component(entity, AppliedMorphWeights { weights });
        updated_meshes.push(mesh_idx);
    }

    updated_meshes
}

pub fn find_morph_channel_names(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Option<Vec<String>> {
    let morph = find_mesh_morph(world, entity, assets, graphics)?;
    Some(
        morph
            .channels
            .iter()
            .map(|channel| channel.name.clone())
            .collect(),
    )
}

pub fn find_mesh_morph<'a>(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &'a GraphicsResources,
) -> Option<&'a MeshMorph> {
    let mesh_ref = world.get_component::<MeshRef>(entity)?;
    let mesh_idx = assets.get_mesh(mesh_ref.mesh_asset_id)?.graphics_mesh_index;
    graphics.meshes.get(mesh_idx).map(|mesh| &mesh.morph)
}

pub fn set_morph_weight(
    world: &mut World,
    entity: Entity,
    channel: usize,
    weight: f32,
    channel_names: &[String],
) {
    let clamped = weight.clamp(0.0, 1.0);

    let mirror_edit = world
        .get_resource::<BlendShapeInspectorState>()
        .is_some_and(|state| state.mirror_edit);
    let mirror_channel = if mirror_edit {
        find_mirror_channel_index(channel_names, channel)
    } else {
        None
    };

    let Some(morph_weights) = world.get_component_mut::<MorphWeights>(entity) else {
        return;
    };

    for target in std::iter::once(channel).chain(mirror_channel) {
        if let Some(slot) = morph_weights.weights.get_mut(target) {
            *slot = clamped;
        }
    }
}

fn find_mirror_channel_index(channel_names: &[String], channel: usize) -> Option<usize> {
    let mirror_name = find_mirror_channel(channel_names.get(channel)?)?;
    channel_names.iter().position(|name| *name == mirror_name)
}

pub fn find_morph_siblings(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Vec<Entity> {
    let Some(source_mesh) = find_mesh_morph(world, entity, assets, graphics)
        .map(|morph| &morph.source_mesh)
        .filter(|source_mesh| !source_mesh.is_empty())
    else {
        return vec![entity];
    };
    let Some(parent) = world.get_component::<Parent>(entity).map(|parent| parent.0) else {
        return vec![entity];
    };

    let is_sibling = |candidate: Entity| {
        candidate == entity
            || (world
                .get_component::<Parent>(candidate)
                .map(|parent| parent.0)
                == Some(parent)
                && find_mesh_morph(world, candidate, assets, graphics)
                    .is_some_and(|morph| morph.source_mesh == *source_mesh))
    };

    world
        .iter_components::<MorphWeights>()
        .map(|(candidate, _)| candidate)
        .filter(|&candidate| is_sibling(candidate))
        .collect()
}

pub fn reset_morph_weights(world: &mut World, entity: Entity, channel_names: &[String]) {
    let preserved_channels: Vec<usize> =
        group_channels(channel_names, &ExpressionGrouping::default())
            .into_iter()
            .filter(|group| group.exclude_from_reset)
            .flat_map(|group| group.channel_indices)
            .collect();

    let Some(morph_weights) = world.get_component_mut::<MorphWeights>(entity) else {
        return;
    };

    for (channel, weight) in morph_weights.weights.iter_mut().enumerate() {
        if !preserved_channels.contains(&channel) {
            *weight = 0.0;
        }
    }
}

fn collect_pending_morph_updates(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Vec<(Entity, usize, Vec<f32>)> {
    world
        .iter_components::<MorphWeights>()
        .filter_map(|(entity, morph_weights)| {
            let mesh_ref = world.get_component::<MeshRef>(entity)?;
            let mesh_idx = assets.get_mesh(mesh_ref.mesh_asset_id)?.graphics_mesh_index;
            let mesh = graphics.meshes.get(mesh_idx)?;
            if mesh.morph.channels.is_empty() {
                return None;
            }

            let is_unchanged = world
                .get_component::<AppliedMorphWeights>(entity)
                .is_some_and(|applied| applied.weights == morph_weights.weights);
            if is_unchanged {
                return None;
            }

            Some((entity, mesh_idx, morph_weights.weights.clone()))
        })
        .collect()
}

fn apply_morph_to_mesh(mesh: &mut MeshBuffer, weights: &[f32]) {
    let base_positions: Vec<[f32; 3]> = mesh
        .base_vertices
        .iter()
        .map(|v| [v.pos.x, v.pos.y, v.pos.z])
        .collect();

    let morphed =
        thyllore_model_core::accumulate_morph_positions(&base_positions, &mesh.morph, weights);

    for (i, pos) in morphed.iter().enumerate() {
        if i < mesh.vertex_data.vertices.len() {
            mesh.vertex_data.vertices[i].pos.x = pos[0];
            mesh.vertex_data.vertices[i].pos.y = pos[1];
            mesh.vertex_data.vertices[i].pos.z = pos[2];
        }
    }

    if let Some(ref mut skin_data) = mesh.skin_data {
        for (i, pos) in morphed.iter().enumerate() {
            if i < skin_data.base_positions.len() {
                skin_data.base_positions[i].x = pos[0];
                skin_data.base_positions[i].y = pos[1];
                skin_data.base_positions[i].z = pos[2];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vulkanr::data::Vertex;
    use thyllore_math_core::{Vec2, Vec3, Vec4};
    use thyllore_model_core::{MeshMorph, MorphChannel, SparseDelta};

    fn make_test_mesh() -> MeshBuffer {
        let vertices = vec![
            Vertex::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(0.0, 0.0),
            ),
            Vertex::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(1.0, 0.0),
            ),
            Vertex::new(
                Vec3::new(0.0, 1.0, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ),
        ];
        let mut mesh = MeshBuffer::default();
        mesh.vertex_data.vertices = vertices.clone();
        mesh.base_vertices = vertices;
        mesh.morph = MeshMorph {
            source_mesh: String::new(),
            channels: vec![MorphChannel {
                name: "test".to_string(),
                position_deltas: vec![
                    SparseDelta {
                        vertex_index: 0,
                        delta: [0.0, 1.0, 0.0],
                    },
                    SparseDelta {
                        vertex_index: 2,
                        delta: [0.0, -0.5, 0.0],
                    },
                ],
                normal_deltas: Vec::new(),
            }],
        };
        mesh
    }

    fn make_morph_world(channel_count: usize) -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert_component(entity, MorphWeights::zeroed(channel_count));
        (world, entity)
    }

    fn channel_names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn morph_weights_of(world: &World, entity: Entity) -> Vec<f32> {
        world
            .get_component::<MorphWeights>(entity)
            .unwrap()
            .weights
            .clone()
    }

    #[test]
    fn test_apply_morph_to_mesh_full_weight() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[1.0]);

        assert!((mesh.vertex_data.vertices[0].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.y - 1.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.z - 0.0).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[1].pos.x - 1.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[1].pos.y - 0.0).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[2].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[2].pos.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_half_weight() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[0.5]);

        assert!((mesh.vertex_data.vertices[0].pos.y - 0.5).abs() < 1e-6);

        assert!((mesh.vertex_data.vertices[2].pos.y - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_zero_weight_is_noop() {
        let mut mesh = make_test_mesh();
        apply_morph_to_mesh(&mut mesh, &[0.0]);

        assert!((mesh.vertex_data.vertices[0].pos.x - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[0].pos.y - 0.0).abs() < 1e-6);
        assert!((mesh.vertex_data.vertices[2].pos.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_morph_to_mesh_idempotent() {
        let mut mesh1 = make_test_mesh();
        apply_morph_to_mesh(&mut mesh1, &[1.0]);
        apply_morph_to_mesh(&mut mesh1, &[0.5]);

        let mut mesh2 = make_test_mesh();
        apply_morph_to_mesh(&mut mesh2, &[0.5]);

        for i in 0..3 {
            assert!(
                (mesh1.vertex_data.vertices[i].pos.x - mesh2.vertex_data.vertices[i].pos.x).abs()
                    < 1e-6
            );
            assert!(
                (mesh1.vertex_data.vertices[i].pos.y - mesh2.vertex_data.vertices[i].pos.y).abs()
                    < 1e-6
            );
            assert!(
                (mesh1.vertex_data.vertices[i].pos.z - mesh2.vertex_data.vertices[i].pos.z).abs()
                    < 1e-6
            );
        }
    }

    #[test]
    fn test_set_morph_weight_clamps() {
        let names = channel_names(&["eye_angry", "mouth_smile"]);
        let (mut world, entity) = make_morph_world(names.len());

        set_morph_weight(&mut world, entity, 0, 1.5, &names);
        set_morph_weight(&mut world, entity, 1, -0.5, &names);

        assert_eq!(morph_weights_of(&world, entity), vec![1.0, 0.0]);
    }

    #[test]
    fn test_set_morph_weight_mirrors_when_enabled() {
        let names = channel_names(&["eye_angry_L", "eye_angry_R", "mouth_smile"]);
        let (mut world, entity) = make_morph_world(names.len());
        world.insert_resource(BlendShapeInspectorState {
            mirror_edit: true,
            ..Default::default()
        });

        set_morph_weight(&mut world, entity, 1, 0.4, &names);

        assert_eq!(morph_weights_of(&world, entity), vec![0.4, 0.4, 0.0]);
    }

    #[test]
    fn test_set_morph_weight_does_not_mirror_when_disabled() {
        let names = channel_names(&["eye_angry_L", "eye_angry_R"]);
        let (mut world, entity) = make_morph_world(names.len());
        world.insert_resource(BlendShapeInspectorState::default());

        set_morph_weight(&mut world, entity, 0, 0.7, &names);

        assert_eq!(morph_weights_of(&world, entity), vec![0.7, 0.0]);
    }

    #[test]
    fn test_reset_morph_weights_keeps_excluded_groups() {
        let names = channel_names(&["eye_angry", "Shrink", "mouth_smile"]);
        let (mut world, entity) = make_morph_world(names.len());
        world.insert_component(
            entity,
            MorphWeights {
                weights: vec![0.5, 1.0, 0.3],
            },
        );

        reset_morph_weights(&mut world, entity, &names);

        assert_eq!(morph_weights_of(&world, entity), vec![0.0, 1.0, 0.0]);
    }
}
