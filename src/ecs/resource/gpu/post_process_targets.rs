use crate::vulkanr::resource::BloomMipTarget;

/// Remembers which image generations a frame slot's descriptor set was last written with,
/// so a transient that landed on the same pooled image needs no rewrite.
#[derive(Debug, Default)]
pub struct BoundGenerations {
    per_slot: Vec<Option<Vec<u64>>>,
}

impl BoundGenerations {
    pub fn is_bound(&self, frame_slot: usize, generations: &[u64]) -> bool {
        self.per_slot
            .get(frame_slot)
            .and_then(|bound| bound.as_deref())
            == Some(generations)
    }

    pub fn mark_bound(&mut self, frame_slot: usize, generations: Vec<u64>) {
        if self.per_slot.len() <= frame_slot {
            self.per_slot.resize(frame_slot + 1, None);
        }
        self.per_slot[frame_slot] = Some(generations);
    }

    fn forget(&mut self) {
        self.per_slot.clear();
    }
}

/// The bloom mips and descriptor bindings the core post-process nodes prepared for the transients
/// the graph assigned this frame.
#[derive(Debug, Default)]
pub struct PostProcessFrameTargets {
    pub bloom_mips: Vec<BloomMipTarget>,
    pub tonemap_bound: BoundGenerations,
    pub histogram_bound: BoundGenerations,
    pub bloom_bound: BoundGenerations,
}

impl PostProcessFrameTargets {
    pub fn forget_bindings(&mut self) {
        self.tonemap_bound.forget();
        self.histogram_bound.forget();
        self.bloom_bound.forget();
    }
}
