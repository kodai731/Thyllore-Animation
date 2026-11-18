use thyllore_vulkan_core::AnimatedMeshUpload;

/// Whether the TLAS must be updated this frame because an instance moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstanceTransforms {
    #[default]
    Unchanged,
    Changed,
}

#[derive(Default)]
pub struct PendingAnimatedMeshUploads {
    pub uploads: Vec<AnimatedMeshUpload>,
    pub instance_transforms: InstanceTransforms,
}
