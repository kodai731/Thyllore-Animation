use thyllore_vulkan_core::AnimatedMeshUpload;

#[derive(Default)]
pub struct PendingAnimatedMeshUploads {
    pub uploads: Vec<AnimatedMeshUpload>,
}
