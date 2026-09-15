use vulkanalia::vk;

/// Device address of every water instance's UBO slot, read by the water closest hit shader
/// through the hit record; filled once when the water pipeline is created.
#[derive(Debug, Default, Clone)]
pub struct WaterTraceBlocks {
    slot_addresses: Vec<vk::DeviceAddress>,
}

impl WaterTraceBlocks {
    pub fn new(slot_addresses: Vec<vk::DeviceAddress>) -> Self {
        Self { slot_addresses }
    }

    pub fn slot_address(&self, ordinal: usize) -> vk::DeviceAddress {
        self.slot_addresses.get(ordinal).copied().unwrap_or(0)
    }
}
