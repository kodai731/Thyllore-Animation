use crate::descriptor::RRBillboardDescriptorSet;
use crate::resource::gpu_resource::GpuResource;
use crate::resource::image::RRImage;

#[must_use]
#[derive(Clone, Debug, Default, GpuResource)]
pub struct BillboardRenderState {
    pub descriptor_set: RRBillboardDescriptorSet,
    pub texture: Option<RRImage>,
}
