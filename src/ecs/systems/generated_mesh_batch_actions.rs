use anyhow::{anyhow, Result};

use crate::ecs::events::ModelLoadSource;
use crate::ecs::resource::{SceneLoadCommand, SceneLoadQueue};
use crate::ecs::systems::batch_run_systems::BatchAction;
use crate::ecs::world::World;

/// Loads a GLB file as an in-memory model, the way a generated mesh arrives from the mesh server.
#[derive(Debug)]
struct LoadGeneratedMesh {
    glb_file: String,
}

impl BatchAction for LoadGeneratedMesh {
    fn name(&self) -> &'static str {
        "load_generated_mesh"
    }

    fn apply(&self, world: &mut World) {
        let glb_data = match std::fs::read(&self.glb_file) {
            Ok(glb_data) => glb_data,
            Err(error) => {
                log_error!(
                    "load_generated_mesh: cannot read {}: {}",
                    self.glb_file,
                    error
                );
                return;
            }
        };

        world
            .resource_mut::<SceneLoadQueue>()
            .push(SceneLoadCommand::LoadModelFromMemory {
                glb_data,
                source: ModelLoadSource::TextToMeshOutput,
            });
    }
}

fn load_generated_mesh_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let glb_file = text.strip_prefix("load_generated_mesh=")?.trim();
    Some(if glb_file.is_empty() {
        Err(anyhow!(
            "load_generated_mesh expects load_generated_mesh=<glb file>"
        ))
    } else {
        Ok(Box::new(LoadGeneratedMesh {
            glb_file: glb_file.to_string(),
        }) as Box<dyn BatchAction>)
    })
}

crate::batch_action!("load_generated_mesh", load_generated_mesh_parse);
