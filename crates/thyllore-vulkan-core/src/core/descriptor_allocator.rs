use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

const INITIAL_SETS_PER_POOL: u32 = 16;
const MAX_SETS_PER_POOL: u32 = 512;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolSignature {
    descriptors_per_set: Vec<(i32, u32)>,
}

impl PoolSignature {
    pub fn from_bindings(bindings: &[vk::DescriptorSetLayoutBinding]) -> Self {
        let mut counts: BTreeMap<i32, u32> = BTreeMap::new();
        for binding in bindings {
            *counts.entry(binding.descriptor_type.as_raw()).or_default() +=
                binding.descriptor_count;
        }
        Self {
            descriptors_per_set: counts.into_iter().collect(),
        }
    }

    fn pool_sizes(&self, max_sets: u32) -> Vec<vk::DescriptorPoolSize> {
        self.descriptors_per_set
            .iter()
            .map(|&(raw_type, count)| {
                vk::DescriptorPoolSize::builder()
                    .type_(vk::DescriptorType::from_raw(raw_type))
                    .descriptor_count(count * max_sets)
                    .build()
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct PoolBlock {
    pool: vk::DescriptorPool,
    capacity: u32,
    used: u32,
}

impl PoolBlock {
    fn remaining(&self) -> u32 {
        self.capacity - self.used
    }
}

#[derive(Clone, Debug, Default)]
struct SetRecycler {
    by_layout: HashMap<vk::DescriptorSetLayout, Vec<vk::DescriptorSet>>,
}

impl SetRecycler {
    fn take(&mut self, layout: vk::DescriptorSetLayout, count: usize) -> Vec<vk::DescriptorSet> {
        let Some(sets) = self.by_layout.get_mut(&layout) else {
            return Vec::new();
        };
        let keep = sets.len().saturating_sub(count);
        sets.split_off(keep)
    }

    fn put(
        &mut self,
        layout: vk::DescriptorSetLayout,
        sets: impl IntoIterator<Item = vk::DescriptorSet>,
    ) {
        self.by_layout.entry(layout).or_default().extend(sets);
    }

    fn forget_layout(&mut self, layout: vk::DescriptorSetLayout) -> usize {
        self.by_layout.remove(&layout).map_or(0, |sets| sets.len())
    }
}

#[derive(Clone, Debug, Default)]
pub struct DescriptorAllocator {
    pools: BTreeMap<PoolSignature, Vec<PoolBlock>>,
    recycled: SetRecycler,
}

impl DescriptorAllocator {
    pub unsafe fn allocate(
        &mut self,
        device: &Device,
        layout: vk::DescriptorSetLayout,
        signature: &PoolSignature,
        count: usize,
    ) -> Result<Vec<vk::DescriptorSet>> {
        let mut sets = self.recycled.take(layout, count);
        let mut remaining = (count - sets.len()) as u32;

        while remaining > 0 {
            let block = self.block_with_room(device, signature)?;
            let batch = remaining.min(block.remaining());
            let layouts = vec![layout; batch as usize];
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(block.pool)
                .set_layouts(&layouts);
            sets.extend(device.allocate_descriptor_sets(&info)?);
            block.used += batch;
            remaining -= batch;
        }

        Ok(sets)
    }

    unsafe fn block_with_room(
        &mut self,
        device: &Device,
        signature: &PoolSignature,
    ) -> Result<&mut PoolBlock> {
        let blocks = self.pools.entry(signature.clone()).or_default();
        let has_room = blocks.last().is_some_and(|block| block.remaining() > 0);
        if !has_room {
            let capacity = blocks.last().map_or(INITIAL_SETS_PER_POOL, |block| {
                (block.capacity * 2).min(MAX_SETS_PER_POOL)
            });
            let pool_sizes = signature.pool_sizes(capacity);
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(capacity);
            let pool = device.create_descriptor_pool(&info, None)?;
            blocks.push(PoolBlock {
                pool,
                capacity,
                used: 0,
            });
        }
        blocks
            .last_mut()
            .ok_or_else(|| anyhow!("descriptor pool block missing after creation"))
    }

    /// The caller guarantees no pending command buffer binds the sets, as for `vkFreeDescriptorSets`.
    pub fn release(
        &mut self,
        layout: vk::DescriptorSetLayout,
        sets: impl IntoIterator<Item = vk::DescriptorSet>,
    ) {
        self.recycled.put(layout, sets);
    }

    pub fn forget_layout(&mut self, layout: vk::DescriptorSetLayout) {
        self.recycled.forget_layout(layout);
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        for block in self.pools.values().flatten() {
            device.destroy_descriptor_pool(block.pool, None);
        }
        self.pools.clear();
        self.recycled = SetRecycler::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vulkanalia::vk::Handle;

    fn layout(id: u64) -> vk::DescriptorSetLayout {
        vk::DescriptorSetLayout::from_raw(id)
    }

    fn set(id: u64) -> vk::DescriptorSet {
        vk::DescriptorSet::from_raw(id)
    }

    #[test]
    fn take_from_unknown_layout_returns_nothing() {
        let mut recycler = SetRecycler::default();
        assert!(recycler.take(layout(1), 3).is_empty());
    }

    #[test]
    fn put_then_take_round_trips_up_to_the_requested_count() {
        let mut recycler = SetRecycler::default();
        recycler.put(layout(1), [set(10), set(11), set(12)]);

        let taken = recycler.take(layout(1), 2);
        assert_eq!(taken, vec![set(11), set(12)]);
        assert_eq!(recycler.take(layout(1), 5), vec![set(10)]);
        assert!(recycler.take(layout(1), 1).is_empty());
    }

    #[test]
    fn sets_are_only_reused_by_their_own_layout() {
        let mut recycler = SetRecycler::default();
        recycler.put(layout(1), [set(10)]);

        assert!(recycler.take(layout(2), 1).is_empty());
        assert_eq!(recycler.take(layout(1), 1), vec![set(10)]);
    }

    #[test]
    fn forget_layout_drops_its_sets_and_reports_how_many() {
        let mut recycler = SetRecycler::default();
        recycler.put(layout(1), [set(10), set(11)]);

        assert_eq!(recycler.forget_layout(layout(1)), 2);
        assert_eq!(recycler.forget_layout(layout(1)), 0);
        assert!(recycler.take(layout(1), 1).is_empty());
    }
}
