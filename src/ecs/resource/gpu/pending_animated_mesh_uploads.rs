use thyllore_vulkan_core::resource::SkinStreamsUpload;
use thyllore_vulkan_core::{AnimatedMeshUpload, GeometryUpload};

/// Whether the TLAS must be updated this frame because an instance moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstanceTransforms {
    #[default]
    Unchanged,
    Changed,
}

/// What the next submitted command buffer owes the device; cleared only once it is submitted.
#[derive(Clone, Default)]
pub struct PendingAnimatedMeshUploads {
    uploads: Vec<AnimatedMeshUpload>,
    pub instance_transforms: InstanceTransforms,
}

impl PendingAnimatedMeshUploads {
    pub fn uploads(&self) -> &[AnimatedMeshUpload] {
        &self.uploads
    }

    /// Keeps one upload per mesh; a skin stream copy that was never submitted stays owed.
    pub fn queue_upload(&mut self, upload: AnimatedMeshUpload) {
        let Some(queued) = self
            .uploads
            .iter_mut()
            .find(|queued| queued.mesh_id == upload.mesh_id)
        else {
            self.uploads.push(upload);
            return;
        };

        let owes_stream_copy = matches!(
            queued.geometry,
            GeometryUpload::Skinning(SkinStreamsUpload::Staged)
        );
        queued.geometry = match upload.geometry {
            GeometryUpload::Skinning(SkinStreamsUpload::Unchanged) if owes_stream_copy => {
                GeometryUpload::Skinning(SkinStreamsUpload::Staged)
            }
            GeometryUpload::Skinning(SkinStreamsUpload::Unchanged)
            | GeometryUpload::Skinning(SkinStreamsUpload::Staged)
            | GeometryUpload::StagedVertices => upload.geometry,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skinning(mesh_id: usize, streams: SkinStreamsUpload) -> AnimatedMeshUpload {
        AnimatedMeshUpload {
            mesh_id,
            geometry: GeometryUpload::Skinning(streams),
        }
    }

    #[test]
    fn unsubmitted_stream_copy_survives_the_next_frames_upload() {
        let mut pending = PendingAnimatedMeshUploads::default();
        pending.queue_upload(skinning(3, SkinStreamsUpload::Staged));
        pending.queue_upload(skinning(3, SkinStreamsUpload::Unchanged));

        assert_eq!(pending.uploads().len(), 1);
        assert!(matches!(
            pending.uploads()[0].geometry,
            GeometryUpload::Skinning(SkinStreamsUpload::Staged)
        ));
    }

    #[test]
    fn meshes_queue_independently() {
        let mut pending = PendingAnimatedMeshUploads::default();
        pending.queue_upload(skinning(1, SkinStreamsUpload::Unchanged));
        pending.queue_upload(AnimatedMeshUpload {
            mesh_id: 2,
            geometry: GeometryUpload::StagedVertices,
        });

        assert_eq!(pending.uploads().len(), 2);
        assert!(matches!(
            pending.uploads()[0].geometry,
            GeometryUpload::Skinning(SkinStreamsUpload::Unchanged)
        ));
    }
}
