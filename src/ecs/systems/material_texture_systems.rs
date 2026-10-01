use std::path::Path;

use thyllore_avatar_core::material::components::texture_remap::MaterialTextureRemap;
use thyllore_avatar_core::material::systems::texture_remap_io::{
    load_material_texture_remap, make_model_relative_path, material_texture_remap_path,
    save_material_texture_remap,
};

use crate::ecs::resource::{
    MaterialTextureSaveState, MaterialTextureSlot, MaterialTextureState, SceneLoadCommand,
    SceneLoadQueue,
};
use crate::ecs::systems::find_model_path;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn load_model_texture_remap(model_path: &str) -> MaterialTextureRemap {
    if Path::new(model_path).file_stem().is_none() {
        return MaterialTextureRemap::default();
    }
    let remap_path = material_texture_remap_path(Path::new(model_path));
    if !remap_path.exists() {
        return MaterialTextureRemap::default();
    }

    load_material_texture_remap(&remap_path).unwrap_or_else(|error| {
        log_warn!(
            "Failed to load material textures {}: {}",
            remap_path.display(),
            error
        );
        MaterialTextureRemap::default()
    })
}

pub fn sync_material_textures(world: &mut World, graphics: &GraphicsResources) {
    let Some(model_path) = find_model_path(world) else {
        return;
    };
    let is_synced = world
        .get_resource::<MaterialTextureState>()
        .is_none_or(|state| state.source_model_path == model_path);
    if is_synced {
        return;
    }

    let remap = load_model_texture_remap(&model_path);
    let slots = collect_material_names(graphics)
        .into_iter()
        .map(|material| MaterialTextureSlot {
            texture: remap.textures.get(&material).cloned(),
            material,
        })
        .collect();

    let mut state = world.resource_mut::<MaterialTextureState>();
    state.slots = slots;
    state.save_state = MaterialTextureSaveState::Saved;
    state.source_model_path = model_path;
}

fn collect_material_names(graphics: &GraphicsResources) -> Vec<String> {
    let mut material_names: Vec<String> = Vec::new();
    for material_id in &graphics.mesh_material_ids {
        let Some(material) = graphics.materials.get(*material_id) else {
            continue;
        };
        if !material_names.contains(&material.name) {
            material_names.push(material.name.clone());
        }
    }
    material_names
}

pub fn set_material_texture(world: &mut World, material: &str, texture_file: Option<&Path>) {
    let mut state = world.resource_mut::<MaterialTextureState>();

    let texture = match texture_file {
        Some(texture_file) => {
            let model_path = Path::new(&state.source_model_path);
            match make_model_relative_path(texture_file, model_path) {
                Ok(relative_path) => Some(relative_path),
                Err(error) => {
                    msg_error!(
                        "Cannot assign texture {} to material {}: {}",
                        texture_file.display(),
                        material,
                        error
                    );
                    return;
                }
            }
        }
        None => None,
    };

    let Some(slot) = state
        .slots
        .iter_mut()
        .find(|slot| slot.material == material)
    else {
        msg_error!("Cannot assign texture: no material named {}", material);
        return;
    };
    if slot.texture == texture {
        return;
    }
    slot.texture = texture;
    state.save_state = MaterialTextureSaveState::Edited;
}

pub fn save_material_textures(world: &mut World) {
    let mut state = world.resource_mut::<MaterialTextureState>();
    if state.source_model_path.is_empty() {
        msg_warn!("Cannot save material textures: no model loaded");
        return;
    }

    let remap = build_texture_remap(&state.slots);
    let remap_path = material_texture_remap_path(Path::new(&state.source_model_path));
    if let Err(error) = save_material_texture_remap(&remap_path, &remap) {
        let reason = format!("Cannot write {}: {}", remap_path.display(), error);
        msg_error!("{}", reason);
        state.save_state = MaterialTextureSaveState::SaveFailed { reason };
        return;
    }
    msg_info!("Saved material textures to {}", remap_path.display());

    state.save_state = MaterialTextureSaveState::Saved;
    world
        .resource_mut::<SceneLoadQueue>()
        .push(SceneLoadCommand::LoadModel {
            path: state.source_model_path.clone(),
        });
}

fn build_texture_remap(slots: &[MaterialTextureSlot]) -> MaterialTextureRemap {
    MaterialTextureRemap {
        textures: slots
            .iter()
            .filter_map(|slot| {
                slot.texture
                    .as_ref()
                    .map(|texture| (slot.material.clone(), texture.clone()))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_world(slots: Vec<MaterialTextureSlot>) -> World {
        let mut world = World::new();
        world.insert_resource(MaterialTextureState {
            source_model_path: "/models/avatar/FBX/avatar.fbx".to_string(),
            slots,
            save_state: MaterialTextureSaveState::Saved,
        });
        world.insert_resource(SceneLoadQueue::default());
        world
    }

    fn make_slot(material: &str, texture: Option<&str>) -> MaterialTextureSlot {
        MaterialTextureSlot {
            material: material.to_string(),
            texture: texture.map(str::to_string),
        }
    }

    #[test]
    fn test_set_material_texture_stores_model_relative_path() {
        let mut world = make_world(vec![make_slot("skin", None)]);

        set_material_texture(
            &mut world,
            "skin",
            Some(Path::new("/models/avatar/Textures/skin.png")),
        );

        let state = world.resource::<MaterialTextureState>();
        assert_eq!(
            state.slots,
            vec![make_slot("skin", Some("../Textures/skin.png"))]
        );
        assert_eq!(state.save_state, MaterialTextureSaveState::Edited);
    }

    #[test]
    fn test_clearing_an_empty_slot_is_not_an_edit() {
        let mut world = make_world(vec![make_slot("skin", None)]);

        set_material_texture(&mut world, "skin", None);

        let state = world.resource::<MaterialTextureState>();
        assert_eq!(state.save_state, MaterialTextureSaveState::Saved);
    }

    #[test]
    fn test_unknown_material_is_ignored() {
        let mut world = make_world(vec![make_slot("skin", None)]);

        set_material_texture(
            &mut world,
            "hair",
            Some(Path::new("/models/avatar/hair.png")),
        );

        let state = world.resource::<MaterialTextureState>();
        assert_eq!(state.slots, vec![make_slot("skin", None)]);
        assert_eq!(state.save_state, MaterialTextureSaveState::Saved);
    }

    #[test]
    fn test_save_failure_is_kept_in_state_without_reload() {
        let mut world = make_world(vec![make_slot("skin", Some("skin.png"))]);
        world
            .resource_mut::<MaterialTextureState>()
            .source_model_path = "/missing_material_texture_dir/avatar.fbx".to_string();

        save_material_textures(&mut world);

        assert!(world.resource_mut::<SceneLoadQueue>().take().is_empty());
        let state = world.resource::<MaterialTextureState>();
        assert!(matches!(
            state.save_state,
            MaterialTextureSaveState::SaveFailed { .. }
        ));
    }

    #[test]
    fn test_remap_keeps_assigned_slots_only() {
        let remap = build_texture_remap(&[
            make_slot("skin", Some("../Textures/skin.png")),
            make_slot("hair", None),
        ]);

        assert_eq!(remap.textures.len(), 1);
        assert_eq!(
            remap.textures.get("skin").map(String::as_str),
            Some("../Textures/skin.png")
        );
    }
}
