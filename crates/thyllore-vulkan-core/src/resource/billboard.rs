use crate::descriptor::RRBillboardDescriptorSet;
use crate::resource::GpuResource;
use crate::resource::RRImage;

#[must_use]
#[derive(Clone, Debug, Default, GpuResource)]
pub struct BillboardRenderState {
    pub descriptor_set: RRBillboardDescriptorSet,
    pub texture: Option<RRImage>,
}
