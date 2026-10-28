use std::collections::HashMap;

use crate::resource::{RenderTargetKey, TransientDesc, TransientHandle};
use crate::vulkan::*;

/// Symbolic name of a frame-lifetime image a pass asks the graph for (`"effect.scene_color"`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TransientSlot(pub &'static str);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TransientRequest {
    pub slot: TransientSlot,
    pub desc: TransientDesc,
}

impl TransientRequest {
    pub const fn new(slot: TransientSlot, desc: TransientDesc) -> Self {
        Self { slot, desc }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CoreTarget {
    HdrColor,
    OnionSkinGhost,
    GBufferPosition,
    GBufferNormal,
    GBufferAlbedo,
    GBufferObjectId,
    GBufferShadowMask,
    Offscreen,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetRef {
    Storage(RenderTargetKey),
    Transient(TransientSlot),
    Core(CoreTarget),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderStage {
    Fragment,
    Compute,
    RayTracing,
}

impl ShaderStage {
    fn pipeline_stage(self) -> vk::PipelineStageFlags {
        match self {
            ShaderStage::Fragment => vk::PipelineStageFlags::FRAGMENT_SHADER,
            ShaderStage::Compute => vk::PipelineStageFlags::COMPUTE_SHADER,
            ShaderStage::RayTracing => vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR,
        }
    }
}

impl From<ShaderStage> for vk::PipelineStageFlags {
    fn from(stage: ShaderStage) -> Self {
        stage.pipeline_stage()
    }
}

impl std::ops::BitOr for ShaderStage {
    type Output = vk::PipelineStageFlags;

    fn bitor(self, other: ShaderStage) -> vk::PipelineStageFlags {
        self.pipeline_stage() | other.pipeline_stage()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttachmentStart {
    Discard,
    Sampled,
    ColorAttachment,
}

impl AttachmentStart {
    fn initial_layout(self) -> vk::ImageLayout {
        match self {
            AttachmentStart::Discard => vk::ImageLayout::UNDEFINED,
            AttachmentStart::Sampled => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            AttachmentStart::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetAccess {
    Attachment(AttachmentStart),
    Sampled(ShaderStage),
    StorageRead(ShaderStage),
    StorageReadWrite(ShaderStage),
    TransferSrc,
    TransferDst,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TargetUse {
    pub target: TargetRef,
    pub access: TargetAccess,
}

impl TargetUse {
    pub const fn new(target: TargetRef, access: TargetAccess) -> Self {
        Self { target, access }
    }
}

/// First and last node index that touches each transient slot, computed from the declarations of
/// every node before anything is recorded. The graph acquires a slot at its first node and releases it
/// after its last, so slots with disjoint lifetimes and equal descs share one pooled image.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TransientLifetimes {
    first_use: HashMap<TransientSlot, usize>,
    last_use: HashMap<TransientSlot, usize>,
}

impl TransientLifetimes {
    pub fn from_node_uses(node_uses: &[Vec<TargetUse>]) -> Self {
        let mut lifetimes = Self::default();
        for (node_index, uses) in node_uses.iter().enumerate() {
            for target_use in uses {
                let TargetRef::Transient(slot) = target_use.target else {
                    continue;
                };
                lifetimes.first_use.entry(slot).or_insert(node_index);
                lifetimes.last_use.insert(slot, node_index);
            }
        }
        lifetimes
    }

    pub fn starting_at(&self, node_index: usize) -> Vec<TransientSlot> {
        let mut slots: Vec<TransientSlot> = self
            .first_use
            .iter()
            .filter(|(_, first)| **first == node_index)
            .map(|(slot, _)| *slot)
            .collect();
        slots.sort_by_key(|slot| slot.0);
        slots
    }

    pub fn ending_at(&self, node_index: usize) -> Vec<TransientSlot> {
        let mut slots: Vec<TransientSlot> = self
            .last_use
            .iter()
            .filter(|(_, last)| **last == node_index)
            .map(|(slot, _)| *slot)
            .collect();
        slots.sort_by_key(|slot| slot.0);
        slots
    }

    pub fn is_used(&self, slot: TransientSlot) -> bool {
        self.first_use.contains_key(&slot)
    }
}

/// The transient handles the graph assigned for the current frame, keyed by slot.
#[derive(Debug, Default)]
pub struct FrameTransients {
    handles: HashMap<TransientSlot, TransientHandle>,
}

impl FrameTransients {
    pub fn clear(&mut self) {
        self.handles.clear();
    }

    pub fn insert(&mut self, slot: TransientSlot, handle: TransientHandle) {
        self.handles.insert(slot, handle);
    }

    pub fn get(&self, slot: TransientSlot) -> Option<TransientHandle> {
        self.handles.get(&slot).copied()
    }

    pub fn handle(&self, slot: TransientSlot) -> Result<TransientHandle> {
        self.get(slot)
            .ok_or_else(|| anyhow!("transient slot {} was not acquired this frame", slot.0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ImageState {
    layout: vk::ImageLayout,
    stage: vk::PipelineStageFlags,
    access: vk::AccessFlags,
    writes: bool,
}

const UNKNOWN_STATE: ImageState = ImageState {
    layout: vk::ImageLayout::UNDEFINED,
    stage: vk::PipelineStageFlags::TOP_OF_PIPE,
    access: vk::AccessFlags::empty(),
    writes: false,
};

impl TargetAccess {
    fn entry_state(self) -> ImageState {
        match self {
            TargetAccess::Attachment(start) => ImageState {
                layout: start.initial_layout(),
                stage: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                access: vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                writes: true,
            },
            TargetAccess::Sampled(stage) => ImageState {
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                stage: stage.pipeline_stage(),
                access: vk::AccessFlags::SHADER_READ,
                writes: false,
            },
            TargetAccess::StorageRead(stage) => ImageState {
                layout: vk::ImageLayout::GENERAL,
                stage: stage.pipeline_stage(),
                access: vk::AccessFlags::SHADER_READ,
                writes: false,
            },
            TargetAccess::StorageReadWrite(stage) => ImageState {
                layout: vk::ImageLayout::GENERAL,
                stage: stage.pipeline_stage(),
                access: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
                writes: true,
            },
            TargetAccess::TransferSrc => ImageState {
                layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                stage: vk::PipelineStageFlags::TRANSFER,
                access: vk::AccessFlags::TRANSFER_READ,
                writes: false,
            },
            TargetAccess::TransferDst => ImageState {
                layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                stage: vk::PipelineStageFlags::TRANSFER,
                access: vk::AccessFlags::TRANSFER_WRITE,
                writes: true,
            },
        }
    }

    fn exit_state(self) -> ImageState {
        let mut state = self.entry_state();
        if let TargetAccess::Attachment(_) = self {
            state.layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        }
        state
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PendingBarrier {
    pub(crate) src_stage: vk::PipelineStageFlags,
    pub(crate) dst_stage: vk::PipelineStageFlags,
    pub(crate) image: vk::Image,
    pub(crate) old_layout: vk::ImageLayout,
    pub(crate) new_layout: vk::ImageLayout,
    pub(crate) src_access: vk::AccessFlags,
    pub(crate) dst_access: vk::AccessFlags,
}

impl PendingBarrier {
    pub fn image_memory_barrier(&self) -> vk::ImageMemoryBarrier {
        vk::ImageMemoryBarrier::builder()
            .image(self.image)
            .old_layout(self.old_layout)
            .new_layout(self.new_layout)
            .src_access_mask(self.src_access)
            .dst_access_mask(self.dst_access)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build()
    }
}

/// Tracks the layout and last access of every color image the pass graph touches
/// and derives the barrier a pass needs before it runs.
#[derive(Debug, Default)]
pub struct ImageStateTracker {
    states: HashMap<vk::Image, ImageState>,
}

impl ImageStateTracker {
    pub fn forget(&mut self, image: crate::types::Image) {
        self.states.remove(&image.raw());
    }

    /// Records an image that was transitioned to SHADER_READ_ONLY_OPTIMAL outside the graph
    /// (history images cleared at creation), so its first read needs no barrier.
    pub fn mark_shader_read_only(&mut self, image: crate::types::Image) {
        self.states.insert(
            image.raw(),
            ImageState {
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                access: vk::AccessFlags::SHADER_READ,
                writes: false,
            },
        );
    }

    pub fn clear(&mut self) {
        self.states.clear();
    }

    pub fn transition(
        &mut self,
        image: crate::types::Image,
        access: TargetAccess,
    ) -> Option<PendingBarrier> {
        let image = image.raw();
        let current = self.states.get(&image).copied().unwrap_or(UNKNOWN_STATE);
        let entry = access.entry_state();
        self.states.insert(image, access.exit_state());

        let render_pass_discards_contents =
            matches!(access, TargetAccess::Attachment(AttachmentStart::Discard));
        if render_pass_discards_contents {
            let image_untouched = current == UNKNOWN_STATE;
            if image_untouched {
                return None;
            }
            return Some(PendingBarrier {
                src_stage: current.stage,
                dst_stage: entry.stage,
                image,
                old_layout: current.layout,
                new_layout: current.layout,
                src_access: current.access,
                dst_access: entry.access,
            });
        }

        let hazard = current.layout != entry.layout || current.writes || entry.writes;
        if !hazard {
            return None;
        }

        Some(PendingBarrier {
            src_stage: current.stage,
            dst_stage: entry.stage,
            image,
            old_layout: current.layout,
            new_layout: entry.layout,
            src_access: current.access,
            dst_access: entry.access,
        })
    }
}

pub unsafe fn record_pending_barriers(
    rrdevice: &crate::core::device::RRDevice,
    command_buffer: vk::CommandBuffer,
    barriers: &[PendingBarrier],
) {
    if barriers.is_empty() {
        return;
    }

    let src_stage = barriers
        .iter()
        .fold(vk::PipelineStageFlags::empty(), |acc, b| acc | b.src_stage);
    let dst_stage = barriers
        .iter()
        .fold(vk::PipelineStageFlags::empty(), |acc, b| acc | b.dst_stage);
    let image_barriers: Vec<vk::ImageMemoryBarrier> = barriers
        .iter()
        .map(PendingBarrier::image_memory_barrier)
        .collect();

    rrdevice.device().cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &image_barriers,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(id: u64) -> crate::types::Image {
        crate::types::Image(vk::Image::from_raw(id))
    }

    #[test]
    fn first_use_transitions_from_undefined() {
        let mut tracker = ImageStateTracker::default();
        let barrier = tracker
            .transition(image(1), TargetAccess::TransferDst)
            .expect("barrier");
        assert_eq!(barrier.old_layout, vk::ImageLayout::UNDEFINED);
        assert_eq!(barrier.new_layout, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        assert_eq!(barrier.src_stage, vk::PipelineStageFlags::TOP_OF_PIPE);
    }

    #[test]
    fn read_after_read_in_same_layout_needs_no_barrier() {
        let mut tracker = ImageStateTracker::default();
        tracker.transition(image(1), TargetAccess::Sampled(ShaderStage::Fragment));
        assert!(tracker
            .transition(image(1), TargetAccess::Sampled(ShaderStage::Compute))
            .is_none());
    }

    #[test]
    fn attachment_with_loaded_contents_transitions_to_initial_layout() {
        let mut tracker = ImageStateTracker::default();
        tracker.transition(
            image(1),
            TargetAccess::StorageReadWrite(ShaderStage::Compute),
        );
        let attachment = TargetAccess::Attachment(AttachmentStart::Sampled);
        let barrier = tracker.transition(image(1), attachment).expect("barrier");
        assert_eq!(barrier.old_layout, vk::ImageLayout::GENERAL);
        assert_eq!(
            barrier.new_layout,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        );
    }

    #[test]
    fn write_after_read_in_same_layout_needs_barrier() {
        let mut tracker = ImageStateTracker::default();
        tracker.transition(image(1), TargetAccess::StorageRead(ShaderStage::Fragment));
        let barrier = tracker
            .transition(
                image(1),
                TargetAccess::StorageReadWrite(ShaderStage::Compute),
            )
            .expect("barrier");
        assert_eq!(barrier.old_layout, vk::ImageLayout::GENERAL);
        assert_eq!(barrier.new_layout, vk::ImageLayout::GENERAL);
        assert_eq!(barrier.src_stage, vk::PipelineStageFlags::FRAGMENT_SHADER);
        assert_eq!(barrier.dst_stage, vk::PipelineStageFlags::COMPUTE_SHADER);
    }

    #[test]
    fn attachment_with_undefined_initial_layout_skips_barrier_on_untouched_image() {
        let mut tracker = ImageStateTracker::default();
        let attachment = TargetAccess::Attachment(AttachmentStart::Discard);
        assert!(tracker.transition(image(1), attachment).is_none());
    }

    #[test]
    fn attachment_with_undefined_initial_layout_orders_after_previous_use_and_lands_on_final() {
        let mut tracker = ImageStateTracker::default();
        tracker.transition(image(1), TargetAccess::TransferDst);
        let attachment = TargetAccess::Attachment(AttachmentStart::Discard);
        let ordering = tracker.transition(image(1), attachment).expect("barrier");
        assert_eq!(ordering.old_layout, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        assert_eq!(ordering.new_layout, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        assert_eq!(ordering.src_stage, vk::PipelineStageFlags::TRANSFER);
        assert_eq!(
            ordering.dst_stage,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        );
        let barrier = tracker
            .transition(image(1), TargetAccess::TransferSrc)
            .expect("barrier");
        assert_eq!(
            barrier.old_layout,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        );
        assert_eq!(
            barrier.src_stage,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        );
    }

    #[test]
    fn marked_shader_read_only_image_is_read_without_barrier() {
        let mut tracker = ImageStateTracker::default();
        tracker.mark_shader_read_only(image(1));
        assert!(tracker
            .transition(image(1), TargetAccess::Sampled(ShaderStage::Fragment))
            .is_none());
    }

    fn use_of(slot: &'static str, access: TargetAccess) -> TargetUse {
        TargetUse::new(TargetRef::Transient(TransientSlot(slot)), access)
    }

    #[test]
    fn lifetimes_span_first_to_last_declaring_node() {
        let sampled = TargetAccess::Sampled(ShaderStage::Fragment);
        let node_uses = vec![
            vec![use_of("a", TargetAccess::TransferDst)],
            vec![use_of("a", sampled), use_of("b", TargetAccess::TransferDst)],
            vec![],
            vec![use_of("b", sampled)],
        ];
        let lifetimes = TransientLifetimes::from_node_uses(&node_uses);
        assert_eq!(lifetimes.starting_at(0), vec![TransientSlot("a")]);
        assert_eq!(lifetimes.ending_at(1), vec![TransientSlot("a")]);
        assert_eq!(lifetimes.starting_at(1), vec![TransientSlot("b")]);
        assert_eq!(lifetimes.ending_at(3), vec![TransientSlot("b")]);
        assert!(lifetimes.starting_at(2).is_empty());
        assert!(!lifetimes.is_used(TransientSlot("c")));
    }

    #[test]
    fn forget_resets_to_undefined() {
        let mut tracker = ImageStateTracker::default();
        tracker.transition(image(1), TargetAccess::Sampled(ShaderStage::Fragment));
        tracker.forget(image(1));
        let barrier = tracker
            .transition(image(1), TargetAccess::Sampled(ShaderStage::Fragment))
            .expect("barrier");
        assert_eq!(barrier.old_layout, vk::ImageLayout::UNDEFINED);
    }
}
