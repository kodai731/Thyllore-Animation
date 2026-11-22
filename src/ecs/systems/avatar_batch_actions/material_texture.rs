use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::ecs::resource::{AssetEditCommand, AssetEditQueue};
use crate::ecs::systems::batch_run_systems::{unit_action_parse, BatchAction};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;

#[derive(Debug)]
pub(super) struct AssignMaterialTexture {
    material: String,
    texture_file: PathBuf,
}

impl BatchAction for AssignMaterialTexture {
    fn name(&self) -> &'static str {
        "assign_material_texture"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<AssetEditQueue>()
            .push(AssetEditCommand::AssignMaterialTexture {
                material: self.material.clone(),
                path: self.texture_file.clone(),
            });
    }
}

pub(super) fn assign_material_texture_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let spec = text.strip_prefix("assign_material_texture=")?;
    Some(
        assign_material_texture_spec_parse(spec)
            .map(|action| Box::new(action) as Box<dyn BatchAction>),
    )
}

pub(super) fn assign_material_texture_spec_parse(spec: &str) -> Result<AssignMaterialTexture> {
    let (material, texture_file) = spec
        .split_once(':')
        .map(|(material, texture_file)| (material.trim(), texture_file.trim()))
        .filter(|(material, texture_file)| !material.is_empty() && !texture_file.is_empty())
        .ok_or_else(|| anyhow!(ASSIGN_MATERIAL_TEXTURE_USAGE))?;

    Ok(AssignMaterialTexture {
        material: material.to_string(),
        texture_file: PathBuf::from(texture_file),
    })
}

#[derive(Debug, Default)]
pub(super) struct SaveMaterialTextures;

impl BatchAction for SaveMaterialTextures {
    fn name(&self) -> &'static str {
        "save_material_textures"
    }
    fn apply(&self, world: &mut World) {
        world.send_command(AvatarSetupEvent::SaveMaterialTextures);
    }
}

const ASSIGN_MATERIAL_TEXTURE_USAGE: &str =
    "assign_material_texture expects assign_material_texture=<material>:<texture file>";

crate::batch_action!("assign_material_texture", assign_material_texture_parse);
crate::batch_action!(
    "save_material_textures",
    unit_action_parse::<SaveMaterialTextures>
);
