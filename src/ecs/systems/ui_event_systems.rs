use std::path::PathBuf;
use std::rc::Rc;

use crate::hooks::batch_capture::BatchCapture;

#[derive(Clone, Debug)]
pub enum DeferredAction {
    LoadModel {
        path: String,
    },
    TakeScreenshot,
    #[cfg(debug_assertions)]
    DebugShadowInfo,
    #[cfg(debug_assertions)]
    DebugBillboardDepth,
    DumpDebugInfo,
    DumpAnimationDebug,
    CaptureNow(Rc<dyn BatchCapture>),
    LoadClipFromFile {
        path: PathBuf,
    },
    SaveClipToFile {
        source_id: u64,
        path: PathBuf,
    },
    SaveSpringBoneBake {
        baked_id: u64,
        path: PathBuf,
    },
    #[cfg(feature = "auto-rig")]
    LoadModelFromMemory {
        glb_data: Vec<u8>,
        source: crate::ecs::events::ModelLoadSource,
    },
    LoadModelAdditive {
        path: String,
    },
    SpawnDebugPrimitive {
        kind: crate::ecs::events::DebugPrimitiveKind,
    },
    DeleteEntities {
        entities: Vec<u64>,
    },
}
