use crate::animation::editable::SourceClipId;
use crate::ecs::events::EventQueue;
use crate::ecs::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipExportFormat {
    Fbx,
    Gltf,
    GltfAnimationOnly,
}

/// A file dialog the platform opens for the engine; the result comes back as a queued command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogRequest {
    LoadClip,
    SaveClip(SourceClipId),
    ExportClip {
        source_id: SourceClipId,
        format: ClipExportFormat,
    },
    ExportModelGltf,
    SaveSpringBoneBake,
    PickMaterialTexture {
        material: String,
    },
}

pub fn send_dialog_request(world: &World, request: DialogRequest) {
    world
        .resource_mut::<EventQueue<DialogRequest>>()
        .send(request);
}
