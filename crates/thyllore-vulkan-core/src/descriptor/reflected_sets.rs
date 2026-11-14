use anyhow::{anyhow, Result};

use crate::core::device::RRDevice;
use crate::descriptor::reflected_layout::{
    DescriptorSetWriter, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::resource::gpu_resource::{build_gpu_resource, GpuResource, OwnedDescriptorSet};
use crate::types::DescriptorSet;

/// How many sets one layout owns: one row per frame slot, one column per history index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SetGrid {
    frame_slots: usize,
    history_count: usize,
}

impl SetGrid {
    pub const SINGLE: Self = Self::new(1, 1);

    pub const fn new(frame_slots: usize, history_count: usize) -> Self {
        Self {
            frame_slots,
            history_count,
        }
    }

    pub const fn per_frame_slot(frame_slots: usize) -> Self {
        Self::new(frame_slots, 1)
    }

    pub const fn per_history(history_count: usize) -> Self {
        Self::new(1, history_count)
    }

    fn set_count(self) -> usize {
        self.frame_slots * self.history_count
    }

    fn flat_index(self, slot: SetSlot) -> Result<usize> {
        if slot.frame_slot >= self.frame_slots || slot.history_index >= self.history_count {
            return Err(anyhow!(
                "descriptor set slot (frame {}, history {}) is outside the {} x {} grid",
                slot.frame_slot,
                slot.history_index,
                self.frame_slots,
                self.history_count
            ));
        }
        Ok(slot.frame_slot * self.history_count + slot.history_index)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SetSlot {
    frame_slot: usize,
    history_index: usize,
}

impl SetSlot {
    pub const SINGLE: Self = Self::new(0, 0);

    pub const fn new(frame_slot: usize, history_index: usize) -> Self {
        Self {
            frame_slot,
            history_index,
        }
    }

    pub const fn frame_slot(frame_slot: usize) -> Self {
        Self::new(frame_slot, 0)
    }

    pub const fn history(history_index: usize) -> Self {
        Self::new(0, history_index)
    }
}

/// A reflected layout together with the grid of descriptor sets allocated from it.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct ReflectedDescriptorSets {
    layout: ReflectedSetLayout,
    sets: Vec<OwnedDescriptorSet>,
    #[gpu_resource(skip)]
    grid: SetGrid,
}

impl ReflectedDescriptorSets {
    pub unsafe fn create(
        rrdevice: &RRDevice,
        spec: &ReflectedLayoutSpec,
        grid: SetGrid,
    ) -> Result<Self> {
        build_gpu_resource(rrdevice, |descriptor_sets: &mut Self| {
            descriptor_sets.grid = grid;
            descriptor_sets.layout = ReflectedSetLayout::create(rrdevice, spec)?;
            let layout = &descriptor_sets.layout;

            descriptor_sets.sets = layout
                .allocate_sets(rrdevice, grid.set_count())?
                .into_iter()
                .map(|set| OwnedDescriptorSet::new(set, layout.handle))
                .collect();
            Ok(())
        })
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        &self.layout
    }

    pub fn set(&self, slot: SetSlot) -> Result<DescriptorSet> {
        let index = self.grid.flat_index(slot)?;
        Ok(DescriptorSet(self.sets[index].handle()))
    }

    pub fn writer(&self, slot: SetSlot) -> Result<DescriptorSetWriter<'_>> {
        Ok(self.layout.descriptor_writer(self.set(slot)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_of_a_grid_map_to_distinct_indices_in_frame_major_order() {
        let grid = SetGrid::new(2, 3);

        let indices: Vec<usize> = (0..2)
            .flat_map(|frame_slot| (0..3).map(move |history| SetSlot::new(frame_slot, history)))
            .map(|slot| grid.flat_index(slot).expect("slot inside the grid"))
            .collect();

        assert_eq!(indices, (0..grid.set_count()).collect::<Vec<_>>());
    }

    #[test]
    fn a_slot_outside_either_axis_is_rejected() {
        let grid = SetGrid::per_frame_slot(2);

        assert!(grid.flat_index(SetSlot::frame_slot(2)).is_err());
        assert!(grid.flat_index(SetSlot::history(1)).is_err());
        assert!(SetGrid::SINGLE.flat_index(SetSlot::SINGLE).is_ok());
    }
}
