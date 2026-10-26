use thyllore_vulkan_core::types::DeviceAddress;

/// Device address of every water instance's UBO slot, read by the water closest hit shader
/// through the hit record; filled once when the water pipeline is created.
#[derive(Debug, Default, Clone)]
pub struct WaterTraceBlocks {
    slot_addresses: Vec<DeviceAddress>,
}

impl WaterTraceBlocks {
    pub fn new(slot_addresses: Vec<DeviceAddress>) -> Self {
        Self { slot_addresses }
    }

    pub fn slot_address(&self, ordinal: usize) -> DeviceAddress {
        self.slot_addresses.get(ordinal).copied().unwrap_or(0)
    }
}
