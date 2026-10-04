use std::path::PathBuf;
use std::rc::Rc;

use crate::hooks::batch_capture::BatchCapture;

/// Commands recorded during event dispatch and applied by `App`, one queue per stage of the apply order.
pub struct CommandQueue<C> {
    commands: Vec<C>,
}

impl<C> Default for CommandQueue<C> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<C> CommandQueue<C> {
    pub fn push(&mut self, command: C) {
        self.commands.push(command);
    }

    pub fn take(&mut self) -> Vec<C> {
        std::mem::take(&mut self.commands)
    }
}

pub type EntityRemovalQueue = CommandQueue<EntityRemovalCommand>;
pub type SceneLoadQueue = CommandQueue<SceneLoadCommand>;
pub type AssetEditQueue = CommandQueue<AssetEditCommand>;
pub type OutputQueue = CommandQueue<OutputCommand>;

/// Applied first: the entity ids were collected from the scene as it was before any load.
#[derive(Clone, Debug)]
pub enum EntityRemovalCommand {
    DeleteEntities { entities: Vec<u64> },
}

#[derive(Clone, Debug)]
pub enum SceneLoadCommand {
    LoadModel {
        path: String,
    },
    LoadModelAdditive {
        path: String,
    },
    #[cfg(feature = "auto-rig")]
    LoadModelFromMemory {
        glb_data: Vec<u8>,
        source: crate::ecs::events::ModelLoadSource,
    },
    SpawnDebugPrimitive {
        kind: crate::ecs::events::DebugPrimitiveKind,
    },
}

/// Applied after the loads: these edit assets of the model that is loaded.
#[derive(Clone, Debug)]
pub enum AssetEditCommand {
    LoadClipFromFile { path: PathBuf },
    LoadRecipeFromFile { path: PathBuf },
    AssignMaterialTexture { material: String, path: PathBuf },
}

/// Applied last: these only read the scene the earlier stages produced.
#[derive(Clone, Debug)]
pub enum OutputCommand {
    TakeScreenshot,
    #[cfg(debug_assertions)]
    DebugShadowInfo,
    #[cfg(debug_assertions)]
    DebugBillboardDepth,
    DumpDebugInfo,
    DumpAnimationDebug,
    CaptureNow(Rc<dyn BatchCapture>),
    SaveClipToFile {
        source_id: u64,
        path: PathBuf,
    },
    SaveSpringBoneBake {
        baked_id: u64,
        path: PathBuf,
    },
    ExportClipFbx {
        source_id: u64,
        path: PathBuf,
    },
    ExportClipGltf {
        source_id: u64,
        path: PathBuf,
    },
    ExportClipGltfAnimationOnly {
        source_id: u64,
        path: PathBuf,
    },
    ExportModelGltf {
        path: PathBuf,
    },
}
