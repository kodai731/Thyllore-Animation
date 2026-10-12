use crate::app::App;
use crate::vulkanr::vulkan::*;

use anyhow::{anyhow, Result};

impl App {
    pub(crate) unsafe fn get_memory_type_index(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_properties = self
            .instance
            .get_physical_device_memory_properties(self.rrdevice.physical_device);

        for i in 0..mem_properties.memory_type_count {
            let has_type = (type_filter & (1 << i)) != 0;
            let has_properties = mem_properties.memory_types[i as usize]
                .property_flags
                .contains(properties);

            if has_type && has_properties {
                return Ok(i);
            }
        }

        Err(anyhow!("Failed to find suitable memory type"))
    }
}
